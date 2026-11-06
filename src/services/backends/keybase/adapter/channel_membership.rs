use super::*;

pub(super) fn start_leave(
    sender: Sender<BackendEvent>,
    store: Arc<LocalStore>,
    workspace_id: WorkspaceId,
    conversation: ProviderConversationRef,
) {
    thread::spawn(move || {
        let raw_id = provider_ref_to_conversation_id_bytes(&conversation);
        let id = ConversationId::new(conversation.0.clone());
        let command = BackendCommand::LeaveChannel {
            workspace_id: workspace_id.clone(),
            conversation_id: id.clone(),
        };
        let mut outcome = BackendTaskOutcome::new(&sender, command.clone());
        let result = (|| -> io::Result<u64> {
            let generation = store.conversation_membership_generation(&id)?;
            if !store
                .get_conversation(&id)?
                .is_some_and(|summary| summary.can_leave_channel())
            {
                return Err(io::Error::other(
                    "This channel cannot be left individually.",
                ));
            }
            let raw_id = raw_id.ok_or_else(|| io::Error::other("Invalid channel ID"))?;
            let path = socket_path().ok_or_else(|| io::Error::other("Keybase is unavailable"))?;
            let runtime = Builder::new_current_thread().enable_all().build()?;
            runtime.block_on(async {
                tokio::time::timeout(Duration::from_secs(30), async {
                    let transport = FramedMsgpackTransport::connect(&path).await?;
                    let mut client = KeybaseRpcClient::new(transport);
                    leave_rpc(&mut client, &raw_id).await
                })
                .await
                .map_err(|_| io::Error::other("Leaving the channel timed out. Please try again."))?
            })?;
            Ok(generation)
        })();
        match result {
            Ok(generation) => {
                // A newer notification owns membership, even if this RPC succeeded.
                if let Err(error) =
                    store.transition_conversation_membership(&id, true, Some(generation), || {
                        let _ = sender.send(BackendEvent::ChannelLeft {
                            workspace_id,
                            conversation_id: id.clone(),
                        });
                    })
                {
                    let _ = sender.send(BackendEvent::CommandFailed {
                        command: Box::new(command),
                        error: format!(
                            "Channel left, but local membership could not be saved: {error}"
                        ),
                    });
                }
            }
            Err(error) => {
                let _ = sender.send(BackendEvent::CommandFailed {
                    command: Box::new(command),
                    error: format!("Could not leave channel: {error}"),
                });
            }
        }
        outcome.complete();
    });
}

async fn leave_rpc(client: &mut KeybaseRpcClient, raw_id: &[u8]) -> io::Result<()> {
    let response = client
        .call(
            "chat.1.local.leaveConversationLocal",
            vec![Value::Map(vec![(
                "convID".into(),
                Value::Binary(raw_id.to_vec()),
            )])],
        )
        .await?;
    ensure_online(&response)
}

pub(super) fn ensure_online(response: &Value) -> io::Result<()> {
    match map_get_any(response, &["offline", "Offline"]).and_then(Value::as_bool) {
        Some(false) => Ok(()),
        Some(true) => Err(io::Error::other(
            "Keybase is offline. Reconnect and try again.",
        )),
        None => Err(io::Error::other(
            "Keybase returned an invalid membership response.",
        )),
    }
}

/// A join notification may omit `conv`. Restore known metadata immediately;
/// the normal metadata notification can replace it with fresher details later.
pub(super) fn cached_join_event(store: &LocalStore, id: &ConversationId) -> Option<BackendEvent> {
    let summary = store.get_conversation(id).ok().flatten()?;
    let provider_ref = store.get_conversation_binding(id).ok().flatten()?;
    let (channels, direct_messages) = if summary.kind == ConversationKind::Channel {
        (vec![summary], vec![])
    } else {
        (vec![], vec![summary])
    };
    Some(BackendEvent::WorkspaceConversationsExtended {
        workspace_id: WorkspaceId::new(WORKSPACE_ID),
        channels,
        direct_messages,
        conversation_bindings: vec![ConversationBinding {
            conversation_id: id.clone(),
            backend_id: BackendId::new(KEYBASE_BACKEND_ID),
            account_id: AccountId::new(DEFAULT_ACCOUNT_ID),
            provider_conversation_ref: provider_ref,
        }],
    })
}

pub(super) fn handle_notify(
    event: &KeybaseNotifyEvent,
    sender: &Sender<BackendEvent>,
    store: &Arc<LocalStore>,
) {
    let left = match event.method_name() {
        "chat.1.NotifyChat.ChatLeftConversation" => true,
        "chat.1.NotifyChat.ChatJoinedConversation" => false,
        _ => return,
    };
    let params = match event {
        KeybaseNotifyEvent::Known { raw_params, .. }
        | KeybaseNotifyEvent::Unknown { raw_params, .. } => raw_params,
    };
    let Some(raw_id) =
        find_value_for_keys(params, &["convID"], 0).and_then(value_to_conversation_id_bytes)
    else {
        return;
    };
    let conversation_id = ConversationId::new(format!("kb_conv:{}", hex_encode(&raw_id)));
    if let Err(error) =
        store.transition_conversation_membership(&conversation_id, left, None, || {
            let workspace_id = WorkspaceId::new(WORKSPACE_ID);
            let _ = sender.send(if left {
                BackendEvent::ChannelLeft {
                    workspace_id,
                    conversation_id: conversation_id.clone(),
                }
            } else {
                BackendEvent::ChannelJoined {
                    workspace_id,
                    conversation_id: conversation_id.clone(),
                }
            });
            if !left && let Some(event) = cached_join_event(store, &conversation_id) {
                let _ = sender.send(event);
            }
        })
    {
        tracing::warn!("Could not persist channel membership notification: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leave_rpc_sends_binary_id_and_requires_online_confirmation() {
        Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                for offline in [false, true] {
                    let (client_stream, server_stream) = tokio::net::UnixStream::pair().unwrap();
                    let server = tokio::spawn(async move {
                        let mut transport = FramedMsgpackTransport::from_stream(server_stream);
                        let request = transport.read_value().await.unwrap();
                        let request = request.as_array().unwrap();
                        assert_eq!(
                            request[2].as_str(),
                            Some("chat.1.local.leaveConversationLocal")
                        );
                        let params = request[3].as_array().unwrap();
                        assert_eq!(
                            map_get(&params[0], "convID"),
                            Some(&Value::Binary(vec![0xab, 0xcd]))
                        );
                        transport
                            .write_value(&Value::Array(vec![
                                1.into(),
                                request[1].clone(),
                                Value::Nil,
                                Value::Map(vec![("offline".into(), offline.into())]),
                            ]))
                            .await
                            .unwrap();
                    });
                    let mut client =
                        KeybaseRpcClient::new(FramedMsgpackTransport::from_stream(client_stream));
                    assert_eq!(
                        leave_rpc(&mut client, &[0xab, 0xcd]).await.is_ok(),
                        !offline
                    );
                    server.await.unwrap();
                }
            });
    }

    #[test]
    fn notifications_persist_membership_and_restore_cached_rows_on_rejoin() {
        let path = super::super::tests::temp_emoji_asset_dir("channel-membership");
        let store = Arc::new(LocalStore::open_at(path.clone()).unwrap());
        let id = ConversationId::new("kb_conv:abcd");
        let summary = ConversationSummary {
            id: id.clone(),
            title: "dev".into(),
            kind: ConversationKind::Channel,
            topic: "dev".into(),
            group: None,
            unread_count: 0,
            mention_count: 0,
            muted: false,
            last_activity_ms: 1,
            last_message: None,
        };
        store.persist_conversation(&summary, 1).unwrap();
        store
            .persist_conversation_binding(&id, &ProviderConversationRef::new("kb_conv:abcd"))
            .unwrap();
        let (sender, receiver) = mpsc::channel();
        let leave_generation = store.conversation_membership_generation(&id).unwrap();
        for (method, left) in [
            ("chat.1.NotifyChat.ChatLeftConversation", true),
            ("chat.1.NotifyChat.ChatJoinedConversation", false),
        ] {
            let event = KeybaseNotifyEvent::from_method(
                method,
                Value::Array(vec![Value::Map(vec![(
                    "convID".into(),
                    Value::Binary(vec![0xab, 0xcd]),
                )])]),
            );
            handle_notify(&event, &sender, &store);
            assert_eq!(store.is_conversation_left(&id).unwrap(), left);
            let event = receiver.try_recv().unwrap();
            assert!(if left {
                matches!(event, BackendEvent::ChannelLeft { .. })
            } else {
                matches!(event, BackendEvent::ChannelJoined { .. })
            });
        }
        assert!(
            matches!(receiver.try_recv().unwrap(), BackendEvent::WorkspaceConversationsExtended { channels, .. } if channels.len() == 1)
        );
        // Deterministic interleaving: an older leave RPC completes after the
        // notification pair above. It must neither persist nor publish a leave.
        assert!(
            !store
                .transition_conversation_membership(&id, true, Some(leave_generation), || {
                    sender
                        .send(BackendEvent::ChannelLeft {
                            workspace_id: WorkspaceId::new(WORKSPACE_ID),
                            conversation_id: id.clone(),
                        })
                        .unwrap();
                })
                .unwrap()
        );
        assert!(!store.is_conversation_left(&id).unwrap());
        assert!(receiver.try_recv().is_err());
        let generation = store.conversation_membership_generation(&id).unwrap();
        assert!(
            store
                .transition_conversation_membership(&id, true, Some(generation), || {
                    sender
                        .send(BackendEvent::ChannelLeft {
                            workspace_id: WorkspaceId::new(WORKSPACE_ID),
                            conversation_id: id.clone(),
                        })
                        .unwrap();
                })
                .unwrap()
        );
        assert!(store.is_conversation_left(&id).unwrap());
        assert!(matches!(
            receiver.try_recv().unwrap(),
            BackendEvent::ChannelLeft { .. }
        ));
        drop(store);
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn offline_leave_is_not_success() {
        assert!(ensure_online(&Value::Map(vec![("offline".into(), true.into())])).is_err());
        assert!(ensure_online(&Value::Map(vec![("offline".into(), false.into())])).is_ok());
        assert!(ensure_online(&Value::Nil).is_err());
    }
}
