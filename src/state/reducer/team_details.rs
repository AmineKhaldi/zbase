use super::*;
use crate::domain::{channel_details::TeamChannel, conversation::ConversationGroup};
use crate::state::bindings::ConversationBinding;

pub(super) fn load(state: &mut UiState, team: ConversationGroup) -> ReducerOutput {
    let request_id = state.team_details.request_id.wrapping_add(1);
    let team_name = team.id.clone();
    state.team_details = super::super::state::TeamDetailsState {
        team: Some(team),
        request_id,
        loading: true,
        ..Default::default()
    };
    ReducerOutput {
        effects: vec![Effect::Backend(BackendCommand::LoadTeamChannels {
            workspace_id: state
                .workspace
                .active_workspace_id
                .clone()
                .unwrap_or_else(|| WorkspaceId::new("ws_primary")),
            team_name,
            request_id,
        })],
    }
}

pub(super) fn loaded(
    state: &mut UiState,
    request_id: u64,
    mut channels: Vec<TeamChannel>,
    bindings: Vec<ConversationBinding>,
) {
    if state.team_details.request_id != request_id {
        return;
    }
    for binding in bindings {
        state
            .backend
            .conversation_bindings
            .insert(binding.conversation_id.clone(), binding);
    }
    for channel in &mut channels {
        if let Some(joined) = state
            .team_details
            .membership_changes
            .get(&channel.conversation.id)
        {
            channel.joined = *joined;
        }
    }
    let joined = channels
        .iter()
        .filter(|c| c.joined)
        .map(|c| c.conversation.clone())
        .collect();
    merge_workspace_conversations(state, joined, Vec::new());
    refresh_sidebar(state);
    channels.sort_by(|a, b| {
        (!a.conversation.title.eq_ignore_ascii_case("general"))
            .cmp(&(!b.conversation.title.eq_ignore_ascii_case("general")))
            .then_with(|| {
                a.conversation
                    .title
                    .to_lowercase()
                    .cmp(&b.conversation.title.to_lowercase())
            })
    });
    state.team_details.channels = channels;
    state.team_details.membership_changes.clear();
    state.team_details.loading = false;
    state.team_details.error = None;
}

pub(super) fn join(state: &mut UiState, conversation: ConversationSummary) -> ReducerOutput {
    let id = conversation.id.clone();
    if state.workspace.joining_channels.contains_key(&id) {
        return ReducerOutput::default();
    }
    state
        .workspace
        .joining_channels
        .insert(id.clone(), conversation.clone());
    state.workspace.join_channel_errors.remove(&id);
    ReducerOutput {
        effects: vec![Effect::Backend(BackendCommand::JoinChannel {
            workspace_id: state
                .workspace
                .active_workspace_id
                .clone()
                .unwrap_or_else(|| WorkspaceId::new("ws_primary")),
            conversation_id: id,
            summary: Box::new(conversation),
        })],
    }
}

pub(super) fn joined(state: &mut UiState, id: &ConversationId) {
    if state.team_details.loading {
        state
            .team_details
            .membership_changes
            .insert(id.clone(), true);
    }
    state.workspace.join_channel_errors.remove(id);
    if let Some(channel) = state
        .team_details
        .channels
        .iter_mut()
        .find(|c| c.conversation.id == *id)
    {
        channel.joined = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::conversation::ConversationTeamType;

    fn team(name: &str) -> ConversationGroup {
        ConversationGroup {
            id: name.into(),
            display_name: name.into(),
            team_type: ConversationTeamType::Complex,
        }
    }

    fn channel(name: &str, joined: bool) -> TeamChannel {
        TeamChannel {
            joined,
            conversation: ConversationSummary {
                id: ConversationId::new(format!("channel:{name}")),
                title: name.into(),
                kind: ConversationKind::Channel,
                topic: name.into(),
                group: Some(team("acme")),
                unread_count: 0,
                mention_count: 0,
                muted: false,
                last_activity_ms: 1,
                last_message: None,
            },
        }
    }

    #[test]
    fn discovery_sorts_general_first_and_only_adds_joined_channels_to_sidebar() {
        let mut state = UiState::default();
        load(&mut state, team("acme"));
        loaded(
            &mut state,
            1,
            vec![
                channel("zebra", false),
                channel("general", true),
                channel("design", false),
            ],
            vec![],
        );
        assert_eq!(
            state
                .team_details
                .channels
                .iter()
                .map(|c| c.conversation.title.as_str())
                .collect::<Vec<_>>(),
            vec!["general", "design", "zebra"]
        );
        assert_eq!(state.workspace.channels.len(), 1);
        assert_eq!(state.workspace.channels[0].title, "general");
        assert!(!state.team_details.loading);
    }

    #[test]
    fn switching_teams_ignores_old_directory_results_and_failures() {
        let mut state = UiState::default();
        load(&mut state, team("acme"));
        load(&mut state, team("other"));
        loaded(&mut state, 1, vec![channel("general", true)], vec![]);
        reduce_command_failure(
            &mut state,
            BackendCommand::LoadTeamChannels {
                workspace_id: WorkspaceId::new("ws"),
                team_name: "acme".into(),
                request_id: 1,
            },
            "old failure".into(),
        );
        assert_eq!(state.team_details.team.as_ref().unwrap().id, "other");
        assert!(state.team_details.channels.is_empty());
        assert!(state.team_details.loading);
        assert!(state.team_details.error.is_none());
        assert!(state.notifications.toasts.is_empty());
    }

    #[test]
    fn join_waits_for_confirmation_and_survives_switching_team_panels() {
        let mut state = UiState::default();
        let summary = channel("design", false).conversation;
        let id = summary.id.clone();
        load(&mut state, team("acme"));
        assert_eq!(join(&mut state, summary.clone()).effects.len(), 1);
        assert!(join(&mut state, summary).effects.is_empty());
        assert!(state.workspace.channels.is_empty());
        load(&mut state, team("other"));
        reduce_backend_event(
            &mut state,
            BackendEvent::ChannelJoined {
                workspace_id: WorkspaceId::new("ws"),
                conversation_id: id.clone(),
            },
        );
        assert!(!state.workspace.joining_channels.contains_key(&id));
        assert_eq!(state.workspace.channels[0].id, id);
        assert!(state.team_details.channels.is_empty());
    }

    #[test]
    fn failed_join_is_retryable_and_late_failure_does_not_undo_confirmation() {
        let mut state = UiState::default();
        let summary = channel("design", false).conversation;
        let id = summary.id.clone();
        let output = join(&mut state, summary.clone());
        let Effect::Backend(command) = output.effects[0].clone() else {
            panic!("expected join command");
        };
        reduce_command_failure(&mut state, command.clone(), "offline".into());
        assert!(state.workspace.channels.is_empty());
        assert_eq!(
            state
                .workspace
                .join_channel_errors
                .get(&id)
                .map(String::as_str),
            Some("offline")
        );
        assert_eq!(join(&mut state, summary).effects.len(), 1);
        assert!(!state.workspace.join_channel_errors.contains_key(&id));
        reduce_backend_event(
            &mut state,
            BackendEvent::ChannelJoined {
                workspace_id: WorkspaceId::new("ws"),
                conversation_id: id.clone(),
            },
        );
        reduce_command_failure(&mut state, command, "late failure".into());
        assert!(!state.workspace.join_channel_errors.contains_key(&id));
        assert_eq!(state.workspace.channels[0].id, id);
    }

    #[test]
    fn membership_notifications_win_over_an_inflight_directory_snapshot() {
        let mut state = UiState::default();
        load(&mut state, team("acme"));
        let design = channel("design", false);
        let dev = channel("dev", true);
        reduce_backend_event(
            &mut state,
            BackendEvent::ChannelJoined {
                workspace_id: WorkspaceId::new("ws"),
                conversation_id: design.conversation.id.clone(),
            },
        );
        channel_membership::channel_left(
            &mut state,
            WorkspaceId::new("ws"),
            dev.conversation.id.clone(),
        );
        loaded(&mut state, 1, vec![design, dev], vec![]);
        assert!(
            state
                .team_details
                .channels
                .iter()
                .find(|c| c.conversation.title == "design")
                .unwrap()
                .joined
        );
        assert!(
            !state
                .team_details
                .channels
                .iter()
                .find(|c| c.conversation.title == "dev")
                .unwrap()
                .joined
        );
        assert_eq!(state.workspace.channels.len(), 1);
        assert_eq!(state.workspace.channels[0].title, "design");
    }
}
