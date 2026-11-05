// Explicit test/benchmark fixtures. This module is excluded from normal builds.
use super::*;
use super::{
    notifications_model::{ActivityItem, ActivityKind},
    sidebar_model::{SidebarRow, SidebarSection},
    timeline_model::MessageRow,
};
use crate::domain::{
    call::CallSessionSummary,
    conversation::{ConversationGroup, ConversationTeamType},
    ids::CallId,
    presence::{Availability, Presence},
    user::UserSummary,
};

impl AppModels {
    pub fn demo() -> Self {
        Self::demo_with_settings(SettingsModel::default())
    }

    pub fn demo_with_settings(settings: SettingsModel) -> Self {
        let acme_id = WorkspaceId::new("ws_acme");
        let friends_id = WorkspaceId::new("ws_friends");
        let oss_id = WorkspaceId::new("ws_oss");

        let general_channel_id = ChannelId::new("general");
        let design_channel_id = ChannelId::new("design");
        let hangout_channel_id = ChannelId::new("hangout");
        let travel_channel_id = ChannelId::new("travel");
        let zbase_channel_id = ChannelId::new("zbase");
        let gpui_channel_id = ChannelId::new("gpui");
        let alice_dm_id = DmId::new("alice");

        let general_conversation_id = ConversationId::new("conv_general");
        let design_conversation_id = ConversationId::new("conv_design");
        let hangout_conversation_id = ConversationId::new("conv_hangout");
        let travel_conversation_id = ConversationId::new("conv_travel");
        let zbase_conversation_id = ConversationId::new("conv_zbase");
        let gpui_conversation_id = ConversationId::new("conv_gpui");
        let alice_conversation_id = ConversationId::new("conv_alice");
        let alice_id = UserId::new("user_alice");
        let search_query = "gpui".to_string();
        let route = Route::Channel {
            workspace_id: acme_id.clone(),
            channel_id: general_channel_id.clone(),
        };

        let acme_group = ConversationGroup {
            id: "acme".to_string(),
            display_name: "Acme Product".to_string(),
            team_type: ConversationTeamType::Complex,
        };
        let friends_group = ConversationGroup {
            id: "friends".to_string(),
            display_name: "Friends".to_string(),
            team_type: ConversationTeamType::Complex,
        };
        let oss_group = ConversationGroup {
            id: "oss".to_string(),
            display_name: "Open Source".to_string(),
            team_type: ConversationTeamType::Complex,
        };

        let general = ConversationSummary {
            id: general_conversation_id.clone(),
            title: "general".to_string(),
            kind: ConversationKind::Channel,
            topic: "Company-wide announcements".to_string(),
            group: Some(acme_group.clone()),
            unread_count: 3,
            mention_count: 1,
            muted: false,
            last_activity_ms: 1_711_000_000_000,
        };
        let design = ConversationSummary {
            id: design_conversation_id.clone(),
            title: "design".to_string(),
            kind: ConversationKind::Channel,
            topic: "Product design crit and UI system work".to_string(),
            group: Some(acme_group.clone()),
            unread_count: 0,
            mention_count: 0,
            muted: false,
            last_activity_ms: 1_710_000_000_000,
        };
        let hangout = ConversationSummary {
            id: hangout_conversation_id.clone(),
            title: "hangout".to_string(),
            kind: ConversationKind::Channel,
            topic: "Off-topic chat".to_string(),
            group: Some(friends_group.clone()),
            unread_count: 5,
            mention_count: 0,
            muted: false,
            last_activity_ms: 1_709_000_000_000,
        };
        let travel = ConversationSummary {
            id: travel_conversation_id.clone(),
            title: "travel".to_string(),
            kind: ConversationKind::Channel,
            topic: "Trip planning".to_string(),
            group: Some(friends_group.clone()),
            unread_count: 0,
            mention_count: 0,
            muted: false,
            last_activity_ms: 1_708_000_000_000,
        };
        let zbase_conv = ConversationSummary {
            id: zbase_conversation_id.clone(),
            title: "zbase".to_string(),
            kind: ConversationKind::Channel,
            topic: "Keybase UI client".to_string(),
            group: Some(oss_group.clone()),
            unread_count: 2,
            mention_count: 1,
            muted: false,
            last_activity_ms: 1_712_000_000_000,
        };
        let gpui_conv = ConversationSummary {
            id: gpui_conversation_id.clone(),
            title: "gpui".to_string(),
            kind: ConversationKind::Channel,
            topic: "GPUI framework discussion".to_string(),
            group: Some(oss_group.clone()),
            unread_count: 0,
            mention_count: 0,
            muted: false,
            last_activity_ms: 1_707_000_000_000,
        };
        let alice_dm = ConversationSummary {
            id: alice_conversation_id.clone(),
            title: "Alice Johnson".to_string(),
            kind: ConversationKind::DirectMessage,
            topic: "Product design".to_string(),
            group: None,
            unread_count: 1,
            mention_count: 0,
            muted: false,
            last_activity_ms: 1_713_000_000_000,
        };

        let general_message = MessageRecord {
            id: MessageId::new("msg_001"),
            conversation_id: general_conversation_id.clone(),
            author_id: alice_id,
            reply_to: None,
            thread_root_id: None,
            timestamp_ms: Some(1_712_000_000_000),
            event: None,
            link_previews: Vec::new(),
            permalink: "slack://acme/general/p/msg_001".to_string(),
            fragments: vec![
                MessageFragment::Text("Here is the shell layout for the GPUI port.".to_string()),
                MessageFragment::Code {
                    text: "AppWindow -> BodySplit -> TimelineList".to_string(),
                    lang: None,
                },
            ],
            source_text: None,
            attachments: vec![AttachmentSummary {
                name: "slack-shell.png".to_string(),
                kind: AttachmentKind::Image,
                size_bytes: 128_000,
                ..AttachmentSummary::default()
            }],
            reactions: Vec::new(),
            thread_reply_count: 4,
            send_state: MessageSendState::Sent,
            edited: None,
        };

        let composer = ComposerModel {
            conversation_id: general_conversation_id.clone(),
            mode: ComposerMode::Compose,
            draft_text: String::new(),
            attachments: Vec::new(),
            autocomplete: Some(AutocompleteState {
                trigger: '@',
                query: "ali".to_string(),
                trigger_offset: 0,
                selected_index: 0,
                candidates: Vec::new(),
            }),
        };
        let thread_pane = ThreadPaneModel {
            open: true,
            root_message_id: Some(MessageId::new("msg_001")),
            width_px: 360.0,
            following: true,
            replies: Vec::new(),
            reply_draft: String::new(),
            reply_attachments: Vec::new(),
            reply_autocomplete: None,
            loading: false,
        };

        let mut models = Self {
            fixture_mode: true,
            app: AppModel {
                open_workspaces: vec![acme_id.clone(), friends_id.clone(), oss_id.clone()],
                active_workspace_id: acme_id.clone(),
                connectivity: Connectivity::Online,
                global_unread_count: 4,
                current_user_id: Some(UserId::new("cameron")),
                current_user_display_name: "cameron".to_string(),
                current_user_avatar_asset: Some("assets/avatars/me.svg".to_string()),
            },
            workspace: WorkspaceModel {
                workspace_id: acme_id.clone(),
                workspace_name: "cameron".to_string(),
                channels: vec![
                    general.clone(),
                    design.clone(),
                    hangout.clone(),
                    travel.clone(),
                    zbase_conv.clone(),
                    gpui_conv.clone(),
                ],
                direct_messages: vec![alice_dm.clone()],
            },
            navigation: NavigationModel {
                current: route.clone(),
                back_stack: Vec::new(),
                forward_stack: Vec::new(),
                right_pane: RightPaneMode::Thread,
            },
            sidebar: SidebarModel {
                mode: Default::default(),
                recent_rows: Vec::new(),
                sections: vec![
                    SidebarSection {
                        id: SidebarSectionId::new("team_acme"),
                        title: "Acme Product".to_string(),
                        rows: vec![
                            SidebarRow {
                                team: None,
                                label: "general".to_string(),
                                unread_count: 3,
                                mention_count: 1,
                                route: Route::Channel {
                                    workspace_id: acme_id.clone(),
                                    channel_id: general_channel_id.clone(),
                                },
                            },
                            SidebarRow {
                                team: None,
                                label: "design".to_string(),
                                unread_count: 0,
                                mention_count: 0,
                                route: Route::Channel {
                                    workspace_id: acme_id.clone(),
                                    channel_id: design_channel_id.clone(),
                                },
                            },
                        ],
                        collapsed: false,
                    },
                    SidebarSection {
                        id: SidebarSectionId::new("team_friends"),
                        title: "Friends".to_string(),
                        rows: vec![
                            SidebarRow {
                                team: None,
                                label: "hangout".to_string(),
                                unread_count: 5,
                                mention_count: 0,
                                route: Route::Channel {
                                    workspace_id: friends_id.clone(),
                                    channel_id: hangout_channel_id.clone(),
                                },
                            },
                            SidebarRow {
                                team: None,
                                label: "travel".to_string(),
                                unread_count: 0,
                                mention_count: 0,
                                route: Route::Channel {
                                    workspace_id: friends_id.clone(),
                                    channel_id: travel_channel_id.clone(),
                                },
                            },
                        ],
                        collapsed: false,
                    },
                    SidebarSection {
                        id: SidebarSectionId::new("team_oss"),
                        title: "Open Source".to_string(),
                        rows: vec![
                            SidebarRow {
                                team: None,
                                label: "zbase".to_string(),
                                unread_count: 2,
                                mention_count: 1,
                                route: Route::Channel {
                                    workspace_id: oss_id.clone(),
                                    channel_id: zbase_channel_id.clone(),
                                },
                            },
                            SidebarRow {
                                team: None,
                                label: "gpui".to_string(),
                                unread_count: 0,
                                mention_count: 0,
                                route: Route::Channel {
                                    workspace_id: oss_id.clone(),
                                    channel_id: gpui_channel_id.clone(),
                                },
                            },
                        ],
                        collapsed: false,
                    },
                    SidebarSection {
                        id: SidebarSectionId::new("dms"),
                        title: "Conversations".to_string(),
                        rows: vec![SidebarRow {
                            team: None,
                            label: "Alice Johnson".to_string(),
                            unread_count: 1,
                            mention_count: 0,
                            route: Route::DirectMessage {
                                workspace_id: acme_id.clone(),
                                dm_id: alice_dm_id.clone(),
                            },
                        }],
                        collapsed: false,
                    },
                ],
                filter: String::new(),
                highlighted_route: Some(route.clone()),
                width_px: crate::views::SIDEBAR_WIDTH_PX,
            },
            conversation: ConversationModel {
                participant_avatars: Vec::new(),
                summary: general.clone(),
                pinned_message: None,
                details: None,
                avatar_asset: None,
                member_count: 24,
                can_post: true,
                is_archived: false,
            },
            timeline: TimelineModel {
                conversation_id: general_conversation_id.clone(),
                current_user_id: Some(UserId::new("user_me")),
                rows: vec![
                    TimelineRow::DateDivider("Today".to_string()),
                    TimelineRow::UnreadDivider("3 new messages".to_string()),
                    TimelineRow::Message(MessageRow {
                        author: demo_alice(),
                        message: general_message,
                        show_header: true,
                    }),
                ],
                highlighted_message_id: Some(MessageId::new("msg_001")),
                editing_message_id: None,
                unread_marker: Some(MessageId::new("msg_001")),
                affinity_index: HashMap::new(),
                emoji_index: HashMap::new(),
                emoji_source_index: HashMap::new(),
                reaction_index: HashMap::new(),
                author_role_index: HashMap::new(),
                pending_scroll_target: None,
                older_cursor: Some("cursor_older".to_string()),
                newer_cursor: None,
                loading_older: false,
                hovered_message_id: None,
                hovered_message_is_thread: None,
                hovered_message_anchor_x: None,
                hovered_message_anchor_y: None,
                hovered_message_window_left: None,
                hovered_message_window_top: None,
                hovered_message_window_width: None,
                hovered_message_viewport_size: None,
                hover_toolbar_settled: false,
                typing_text: Some("Sam is typing…".to_string()),
                quick_react_recent: None,
            },
            composer,
            quick_switcher: QuickSwitcherModel::default(),
            emoji_picker: EmojiPickerModel {
                query: String::new(),
                selected_skin_tone: skin_tone_from_setting_value(
                    settings.emoji_skin_tone.as_deref(),
                ),
                active_group: None,
                hovered: None,
                recent_aliases: settings.emoji_recents.clone(),
                skin_tone_expanded: false,
            },
            find_in_chat: FindInChatModel::default(),
            search: SearchModel {
                query: search_query.clone(),
                filters: Vec::new(),
                results: demo_search_results(&search_query, &[], &acme_id),
                highlighted_index: Some(0),
                is_loading: false,
            },
            new_chat: NewChatModel::default(),
            thread_pane,
            profile_panel: ProfilePanelModel::default(),
            overlay: OverlayModel {
                new_chat_open: false,
                quick_switcher_open: false,
                command_palette_open: false,
                emoji_picker_open: false,
                reaction_target_message_id: None,
                fullscreen_image: None,
                file_upload_lightbox: None,
                active_modal: None,
                profile_card_user_id: None,
                profile_card_position: None,
                sidebar_hover_tooltip: None,
                reaction_hover_tooltip: None,
            },
            call: CallModel {
                active_call: None,
                is_muted: false,
                is_sharing_screen: false,
            },
            notifications: NotificationsModel {
                toasts: Vec::new(),
                notification_center_count: 2,
                activity_items: demo_activity_items(&acme_id),
                highlighted_index: Some(0),
            },
            settings,
            quick_switcher_profile_names: HashMap::new(),
            quick_switcher_search_corpus: Arc::new(QuickSwitcherSearchCorpus {
                revision: 0,
                entries: Vec::new(),
                channel_entry_indices: Vec::new(),
                dm_entry_indices: Vec::new(),
            }),
            quick_switcher_search_corpus_revision: 0,
            quick_switcher_search_corpus_dirty: false,
            composer_drafts: HashMap::new(),
            thread_reply_drafts: HashMap::new(),
            pending_send_outcomes: HashMap::new(),
            send_attempt_count: 0,
        };

        models.store_current_composer_draft();
        models.store_current_thread_draft();
        models.refresh_notification_counts();
        models.apply_saved_sidebar_order();
        models.rebuild_quick_switcher_local_search_corpus();
        models
    }

    pub(super) fn sync_demo_route_models(&mut self, route: &Route) {
        self.find_in_chat.close();
        match route {
            Route::Channel {
                channel_id,
                workspace_id,
                ..
            } => {
                self.app.active_workspace_id = workspace_id.clone();

                let (summary, member_count, timeline, composer_seed) = match channel_id.0.as_str() {
                    "design" => (
                        self.workspace
                            .channels
                            .iter()
                            .find(|c| c.title == "design")
                            .cloned()
                            .expect("missing design demo channel"),
                        12,
                        design_timeline(),
                        String::new(),
                    ),
                    "general" => (
                        self.workspace
                            .channels
                            .iter()
                            .find(|c| c.title == "general")
                            .cloned()
                            .expect("missing general demo channel"),
                        24,
                        general_timeline(),
                        String::new(),
                    ),
                    _ => {
                        let channel_name = channel_id.0.as_str();
                        let summary = self
                            .workspace
                            .channels
                            .iter()
                            .find(|c| c.title == channel_name)
                            .cloned()
                            .unwrap_or_else(|| ConversationSummary {
                                id: ConversationId::new(format!("conv_{channel_name}")),
                                title: channel_name.to_string(),
                                kind: ConversationKind::Channel,
                                topic: format!("#{channel_name}"),
                                group: None,
                                unread_count: 0,
                                mention_count: 0,
                                muted: false,
                                last_activity_ms: 0,
                            });
                        (summary, 8, general_timeline(), String::new())
                    }
                };

                self.conversation = ConversationModel {
                    participant_avatars: Vec::new(),
                    summary: summary.clone(),
                    pinned_message: None,
                    details: None,
                    avatar_asset: None,
                    member_count,
                    can_post: true,
                    is_archived: false,
                };
                self.timeline = timeline;
                self.composer = self.restore_composer_for_conversation(&summary, composer_seed);
            }
            Route::DirectMessage { .. } => {
                let summary = self
                    .workspace
                    .direct_messages
                    .first()
                    .cloned()
                    .expect("missing demo dm");
                self.conversation = ConversationModel {
                    participant_avatars: Vec::new(),
                    summary: summary.clone(),
                    pinned_message: None,
                    details: None,
                    avatar_asset: None,
                    member_count: 2,
                    can_post: true,
                    is_archived: false,
                };
                self.timeline = dm_timeline();
                self.composer = self.restore_composer_for_conversation(&summary, String::new());
            }
            Route::Search {
                query,
                workspace_id,
            } => {
                self.search.query = query.clone();
                self.search.results =
                    demo_search_results(query, &self.search.filters, workspace_id);
                self.search.highlighted_index = (!self.search.results.is_empty()).then_some(0);
                self.set_right_pane(RightPaneMode::Hidden);
            }
            Route::Activity { .. } | Route::Preferences | Route::WorkspaceHome { .. } => {
                self.set_right_pane(RightPaneMode::Hidden);
            }
            Route::ActiveCall { .. } => {
                self.set_right_pane(RightPaneMode::Hidden);
                if self.call.active_call.is_none() {
                    let _ = self.start_or_open_call();
                }
            }
        }

        self.sidebar.highlighted_route = self
            .visible_sidebar_routes()
            .into_iter()
            .find(|candidate| candidate == route)
            .or_else(|| self.sidebar.highlighted_route.clone());

        self.notifications.highlighted_index = if self.notifications.activity_items.is_empty() {
            None
        } else {
            Some(
                self.notifications
                    .highlighted_index
                    .unwrap_or(0)
                    .min(self.notifications.activity_items.len().saturating_sub(1)),
            )
        };

        if self.timeline.highlighted_message_id.is_none() {
            self.timeline.highlighted_message_id = self.default_timeline_highlight();
        }
    }

    pub fn add_composer_attachment(&mut self) {
        if !self.fixture_mode {
            return;
        }
        self.composer.attachments.push(next_demo_attachment(
            self.composer.attachments.len(),
            "composer",
        ));
        self.store_current_composer_draft();
    }

    pub fn add_thread_attachment(&mut self) {
        if !self.fixture_mode {
            return;
        }
        self.thread_pane
            .reply_attachments
            .push(next_demo_attachment(
                self.thread_pane.reply_attachments.len(),
                "thread",
            ));
        self.store_current_thread_draft();
    }

    pub(super) fn start_demo_call(&mut self) -> Route {
        if let Some(route) = self.active_call_route() {
            return route;
        }

        let call_id = CallId::new(format!("call_{}", self.conversation.summary.title));
        self.call.active_call = Some(CallSessionSummary {
            call_id: call_id.clone(),
            status: if matches!(
                self.conversation.summary.kind,
                ConversationKind::DirectMessage
            ) {
                CallStatus::ActiveVideo
            } else {
                CallStatus::ActiveAudio
            },
            participants: active_call_participants(&self.conversation.summary),
        });
        self.call.is_muted = false;
        self.call.is_sharing_screen = false;
        self.push_toast("Call started", Some(ToastAction::OpenActiveCall));
        Route::ActiveCall {
            workspace_id: self.app.active_workspace_id.clone(),
            call_id,
        }
    }
}

fn demo_alice() -> UserSummary {
    UserSummary {
        id: UserId::new("user_alice"),
        display_name: "Alice Johnson".to_string(),
        title: "Staff Product Designer".to_string(),
        avatar_asset: Some("assets/avatars/alice.svg".to_string()),
        presence: Presence {
            availability: Availability::Active,
            status_text: Some("Reviewing the desktop shell".to_string()),
        },
        affinity: Affinity::Positive,
    }
}

fn demo_sam() -> UserSummary {
    UserSummary {
        id: UserId::new("user_sam"),
        display_name: "Sam Rivera".to_string(),
        title: "Rust UI Engineer".to_string(),
        avatar_asset: Some("assets/avatars/sam.svg".to_string()),
        presence: Presence {
            availability: Availability::Active,
            status_text: Some("Profiling the timeline".to_string()),
        },
        affinity: Affinity::None,
    }
}

fn demo_me() -> UserSummary {
    UserSummary {
        id: UserId::new("user_me"),
        display_name: "You".to_string(),
        title: "Desktop Client Engineer".to_string(),
        avatar_asset: Some("assets/avatars/me.svg".to_string()),
        presence: Presence {
            availability: Availability::Active,
            status_text: Some("Driving the GPUI shell".to_string()),
        },
        affinity: Affinity::None,
    }
}

pub(super) fn demo_search_results(
    query: &str,
    filters: &[SearchFilter],
    workspace_id: &WorkspaceId,
) -> Vec<SearchResult> {
    let results = vec![
        SearchResult {
            conversation_id: ConversationId::new("conv_general"),
            route: Route::Channel {
                workspace_id: workspace_id.clone(),
                channel_id: ChannelId::new("general"),
            },
            snippet: "Here is the shell layout for the GPUI port.".to_string(),
            snippet_highlight_ranges: Vec::new(),
            message: MessageRecord {
                id: MessageId::new("msg_001"),
                conversation_id: ConversationId::new("conv_general"),
                author_id: UserId::new("user_alice"),
                reply_to: None,
                thread_root_id: None,
                timestamp_ms: Some(1_712_000_000_000),
                event: None,
                link_previews: Vec::new(),
                permalink: "slack://acme/general/p/msg_001".to_string(),
                fragments: vec![
                    MessageFragment::Text(
                        "Here is the shell layout for the GPUI port.".to_string(),
                    ),
                    MessageFragment::Code {
                        text: "AppWindow -> BodySplit -> TimelineList".to_string(),
                        lang: None,
                    },
                ],
                source_text: None,
                attachments: vec![AttachmentSummary {
                    name: "slack-shell.png".to_string(),
                    kind: AttachmentKind::Image,
                    size_bytes: 128_000,
                    ..AttachmentSummary::default()
                }],
                reactions: Vec::new(),
                thread_reply_count: 4,
                send_state: MessageSendState::Sent,
                edited: None,
            },
        },
        SearchResult {
            conversation_id: ConversationId::new("conv_design"),
            route: Route::Channel {
                workspace_id: workspace_id.clone(),
                channel_id: ChannelId::new("design"),
            },
            snippet: "The shell feels right. Next step is focus routing.".to_string(),
            snippet_highlight_ranges: Vec::new(),
            message: MessageRecord {
                id: MessageId::new("msg_design_001"),
                conversation_id: ConversationId::new("conv_design"),
                author_id: UserId::new("user_sam"),
                reply_to: None,
                thread_root_id: None,
                timestamp_ms: Some(1_711_600_000_000),
                event: None,
                link_previews: Vec::new(),
                permalink: "slack://acme/design/p/msg_design_001".to_string(),
                fragments: vec![
                    MessageFragment::Text(
                        "The shell feels right. Next step is focus routing.".to_string(),
                    ),
                    MessageFragment::Quote(
                        "Keep the root view stateful and the panes presentational.".to_string(),
                    ),
                ],
                source_text: None,
                attachments: Vec::new(),
                reactions: Vec::new(),
                thread_reply_count: 2,
                send_state: MessageSendState::Sent,
                edited: None,
            },
        },
        SearchResult {
            conversation_id: ConversationId::new("conv_alice"),
            route: Route::DirectMessage {
                workspace_id: workspace_id.clone(),
                dm_id: DmId::new("alice"),
            },
            snippet: "Can you make the right pane toggles keyboard accessible too?".to_string(),
            snippet_highlight_ranges: Vec::new(),
            message: MessageRecord {
                id: MessageId::new("msg_dm_001"),
                conversation_id: ConversationId::new("conv_alice"),
                author_id: UserId::new("user_alice"),
                reply_to: None,
                thread_root_id: None,
                timestamp_ms: Some(1_712_200_000_000),
                event: None,
                link_previews: Vec::new(),
                permalink: "slack://acme/dm/alice/p/msg_dm_001".to_string(),
                fragments: vec![
                    MessageFragment::Text(
                        "Can you make the right pane toggles keyboard accessible too?".to_string(),
                    ),
                    MessageFragment::Text(
                        "I also dropped notes in https://docs.acme.dev/ui".to_string(),
                    ),
                ],
                source_text: None,
                attachments: Vec::new(),
                reactions: Vec::new(),
                thread_reply_count: 1,
                send_state: MessageSendState::Sent,
                edited: None,
            },
        },
    ];

    let query = query.trim().to_lowercase();

    results
        .into_iter()
        .filter(|result| {
            query.is_empty()
                || result.snippet.to_lowercase().contains(&query)
                || flatten_fragments(&result.message.fragments)
                    .to_lowercase()
                    .contains(&query)
        })
        .filter(|result| {
            filters
                .iter()
                .all(|filter| search_filter_matches(filter, result))
        })
        .collect()
}

fn demo_activity_items(workspace_id: &WorkspaceId) -> Vec<ActivityItem> {
    vec![
        ActivityItem {
            kind: ActivityKind::Mention,
            title: "Alice mentioned you in #general".to_string(),
            detail: "She asked for the GPUI shell layout so implementation can start.".to_string(),
            route: Route::Channel {
                workspace_id: workspace_id.clone(),
                channel_id: ChannelId::new("general"),
            },
            message_id: Some(MessageId::new("msg_001")),
            unread: true,
        },
        ActivityItem {
            kind: ActivityKind::ThreadReply,
            title: "New replies in the design thread".to_string(),
            detail: "Sam followed up on focus routing and pane ownership.".to_string(),
            route: Route::Channel {
                workspace_id: workspace_id.clone(),
                channel_id: ChannelId::new("design"),
            },
            message_id: Some(MessageId::new("msg_design_001")),
            unread: true,
        },
        ActivityItem {
            kind: ActivityKind::Reminder,
            title: "Preferences review".to_string(),
            detail: "Confirm whether the right pane should stay globally enabled.".to_string(),
            route: Route::Preferences,
            message_id: None,
            unread: false,
        },
        ActivityItem {
            kind: ActivityKind::Reaction,
            title: "Alice reacted in your DM".to_string(),
            detail: "She wants keyboard-accessible thread controls in the same pass.".to_string(),
            route: Route::DirectMessage {
                workspace_id: workspace_id.clone(),
                dm_id: DmId::new("alice"),
            },
            message_id: Some(MessageId::new("msg_dm_001")),
            unread: false,
        },
    ]
}

fn general_timeline() -> TimelineModel {
    let author = demo_alice();
    let conversation_id = ConversationId::new("conv_general");
    TimelineModel {
        conversation_id: conversation_id.clone(),
        current_user_id: Some(UserId::new("user_me")),
        rows: vec![
            TimelineRow::DateDivider("Today".to_string()),
            TimelineRow::UnreadDivider("3 new messages".to_string()),
            TimelineRow::Message(MessageRow {
                author,
                message: MessageRecord {
                    id: MessageId::new("msg_001"),
                    conversation_id,
                    author_id: UserId::new("user_alice"),
                    reply_to: None,
                    thread_root_id: None,
                    timestamp_ms: Some(1_712_000_000_000),
                    event: None,
                    link_previews: Vec::new(),
                    permalink: "slack://acme/general/p/msg_001".to_string(),
                    fragments: vec![
                        MessageFragment::Text(
                            "Here is the shell layout for the GPUI port.".to_string(),
                        ),
                        MessageFragment::Code {
                            text: "AppWindow -> BodySplit -> TimelineList".to_string(),
                            lang: None,
                        },
                    ],
                    source_text: None,
                    attachments: vec![AttachmentSummary {
                        name: "slack-shell.png".to_string(),
                        kind: AttachmentKind::Image,
                        size_bytes: 128_000,
                        ..AttachmentSummary::default()
                    }],
                    reactions: Vec::new(),
                    thread_reply_count: 4,
                    send_state: MessageSendState::Sent,
                    edited: None,
                },
                show_header: true,
            }),
        ],
        highlighted_message_id: Some(MessageId::new("msg_001")),
        editing_message_id: None,
        unread_marker: Some(MessageId::new("msg_001")),
        affinity_index: HashMap::new(),
        emoji_index: HashMap::new(),
        emoji_source_index: HashMap::new(),
        reaction_index: HashMap::new(),
        author_role_index: HashMap::new(),
        pending_scroll_target: None,
        older_cursor: Some("cursor_older".to_string()),
        newer_cursor: None,
        loading_older: false,
        hovered_message_id: None,
        hovered_message_is_thread: None,
        hovered_message_anchor_x: None,
        hovered_message_anchor_y: None,
        hovered_message_window_left: None,
        hovered_message_window_top: None,
        hovered_message_window_width: None,
        hovered_message_viewport_size: None,
        hover_toolbar_settled: false,
        typing_text: Some("Sam is typing…".to_string()),
        quick_react_recent: None,
    }
}

fn design_timeline() -> TimelineModel {
    let conversation_id = ConversationId::new("conv_design");
    TimelineModel {
        conversation_id: conversation_id.clone(),
        current_user_id: Some(UserId::new("user_me")),
        rows: vec![
            TimelineRow::DateDivider("Yesterday".to_string()),
            TimelineRow::Message(MessageRow {
                author: demo_sam(),
                message: MessageRecord {
                    id: MessageId::new("msg_design_001"),
                    conversation_id: conversation_id.clone(),
                    author_id: UserId::new("user_sam"),
                    reply_to: None,
                    thread_root_id: None,
                    timestamp_ms: Some(1_711_600_000_000),
                    event: None,
                    link_previews: Vec::new(),
                    permalink: "slack://acme/design/p/msg_design_001".to_string(),
                    fragments: vec![
                        MessageFragment::Text(
                            "The shell feels right. Next step is focus routing.".to_string(),
                        ),
                        MessageFragment::Quote(
                            "Keep the root view stateful and the panes presentational.".to_string(),
                        ),
                    ],
                    source_text: None,
                    attachments: Vec::new(),
                    reactions: Vec::new(),
                    thread_reply_count: 2,
                    send_state: MessageSendState::Sent,
                    edited: None,
                },
                show_header: true,
            }),
            TimelineRow::Message(MessageRow {
                author: demo_sam(),
                message: MessageRecord {
                    id: MessageId::new("msg_design_002"),
                    conversation_id,
                    author_id: UserId::new("user_sam"),
                    reply_to: None,
                    thread_root_id: None,
                    timestamp_ms: Some(1_711_601_000_000),
                    event: None,
                    link_previews: Vec::new(),
                    permalink: "slack://acme/design/p/msg_design_002".to_string(),
                    fragments: vec![MessageFragment::Text(
                        "We should add a command layer before building the real composer."
                            .to_string(),
                    )],
                    source_text: None,
                    attachments: Vec::new(),
                    reactions: Vec::new(),
                    thread_reply_count: 0,
                    send_state: MessageSendState::Sent,
                    edited: None,
                },
                show_header: false,
            }),
        ],
        highlighted_message_id: Some(MessageId::new("msg_design_002")),
        editing_message_id: None,
        unread_marker: None,
        affinity_index: HashMap::new(),
        emoji_index: HashMap::new(),
        emoji_source_index: HashMap::new(),
        reaction_index: HashMap::new(),
        author_role_index: HashMap::new(),
        pending_scroll_target: None,
        older_cursor: Some("cursor_design_older".to_string()),
        newer_cursor: None,
        loading_older: false,
        hovered_message_id: None,
        hovered_message_is_thread: None,
        hovered_message_anchor_x: None,
        hovered_message_anchor_y: None,
        hovered_message_window_left: None,
        hovered_message_window_top: None,
        hovered_message_window_width: None,
        hovered_message_viewport_size: None,
        hover_toolbar_settled: false,
        typing_text: None,
        quick_react_recent: None,
    }
}

fn dm_timeline() -> TimelineModel {
    let conversation_id = ConversationId::new("conv_alice");
    TimelineModel {
        conversation_id: conversation_id.clone(),
        current_user_id: Some(UserId::new("user_me")),
        rows: vec![
            TimelineRow::DateDivider("Today".to_string()),
            TimelineRow::Message(MessageRow {
                author: demo_alice(),
                message: MessageRecord {
                    id: MessageId::new("msg_dm_001"),
                    conversation_id,
                    author_id: UserId::new("user_alice"),
                    reply_to: None,
                    thread_root_id: None,
                    timestamp_ms: Some(1_712_200_000_000),
                    event: None,
                    link_previews: Vec::new(),
                    permalink: "slack://acme/dm/alice/p/msg_dm_001".to_string(),
                    fragments: vec![MessageFragment::Text(
                        "Can you make the right pane toggles keyboard accessible too?".to_string(),
                    )],
                    source_text: None,
                    attachments: Vec::new(),
                    reactions: Vec::new(),
                    thread_reply_count: 1,
                    send_state: MessageSendState::Sent,
                    edited: None,
                },
                show_header: true,
            }),
        ],
        highlighted_message_id: Some(MessageId::new("msg_dm_001")),
        editing_message_id: None,
        unread_marker: None,
        affinity_index: HashMap::new(),
        emoji_index: HashMap::new(),
        emoji_source_index: HashMap::new(),
        reaction_index: HashMap::new(),
        author_role_index: HashMap::new(),
        pending_scroll_target: None,
        older_cursor: None,
        newer_cursor: None,
        loading_older: false,
        hovered_message_id: None,
        hovered_message_is_thread: None,
        hovered_message_anchor_x: None,
        hovered_message_anchor_y: None,
        hovered_message_window_left: None,
        hovered_message_window_top: None,
        hovered_message_window_width: None,
        hovered_message_viewport_size: None,
        hover_toolbar_settled: false,
        typing_text: None,
        quick_react_recent: None,
    }
}

fn search_filter_matches(filter: &SearchFilter, result: &SearchResult) -> bool {
    match filter {
        SearchFilter::FromUser(user) => {
            let author = match result.message.author_id.0.as_str() {
                "user_alice" => "alice",
                "user_sam" => "sam",
                "user_me" => "you",
                other => other,
            };
            author.contains(&user.to_lowercase())
        }
        SearchFilter::InChannel(channel) => match &result.route {
            Route::Channel { channel_id, .. } => channel_id.0.contains(&channel.to_lowercase()),
            Route::DirectMessage { .. } => channel == "dm",
            _ => false,
        },
        SearchFilter::HasFile => !result.message.attachments.is_empty(),
        SearchFilter::HasLink => flatten_fragments(&result.message.fragments).contains("http"),
        SearchFilter::MentionsMe => flatten_fragments(&result.message.fragments)
            .to_lowercase()
            .contains("you"),
    }
}

fn next_demo_attachment(index: usize, prefix: &str) -> AttachmentSummary {
    let (name, kind, size_bytes) = match index % 3 {
        0 => (format!("{prefix}-notes.md"), AttachmentKind::File, 16_500),
        1 => (format!("{prefix}-mock.png"), AttachmentKind::Image, 248_000),
        _ => (
            format!("{prefix}-clip.mov"),
            AttachmentKind::Video,
            3_200_000,
        ),
    };

    AttachmentSummary {
        name,
        kind,
        size_bytes,
        ..AttachmentSummary::default()
    }
}

fn active_call_participants(conversation: &ConversationSummary) -> Vec<UserSummary> {
    match conversation.kind {
        ConversationKind::DirectMessage => vec![demo_me(), demo_alice()],
        ConversationKind::Channel | ConversationKind::GroupDirectMessage => {
            if conversation.title == "design" {
                vec![demo_me(), demo_sam(), demo_alice()]
            } else {
                vec![demo_me(), demo_alice(), demo_sam()]
            }
        }
    }
}
