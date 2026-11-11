use super::*;
use crate::domain::conversation::ConversationGroup;

impl AppWindow {
    pub(crate) fn open_conversation_team(&mut self, route: &Route, cx: &mut Context<Self>) {
        let team = self
            .models
            .workspace
            .channels
            .iter()
            .find(|c| Some(c.id.0.as_str()) == route.conversation_key())
            .and_then(|c| c.group.clone());
        if let Some(team) = team {
            self.open_team_details(team, cx);
        }
    }

    pub(crate) fn open_team_details(&mut self, team: ConversationGroup, cx: &mut Context<Self>) {
        if !self.models.settings.show_right_pane {
            self.models.push_toast(
                "The side panel is turned off in Settings",
                Some(ToastAction::OpenPreferences),
            );
            self.refresh(cx);
            return;
        }
        self.models.set_right_pane(RightPaneMode::TeamDetails);
        self.dispatch_ui_action(UiAction::LoadTeamDetails { team });
        self.refresh(cx);
    }

    pub(crate) fn open_team_channel(
        &mut self,
        conversation: ConversationSummary,
        joined: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let channel_id = crate::domain::ids::ChannelId::new(conversation.id.0.clone());
        if !joined {
            self.preview_summaries
                .insert(conversation.id.clone(), (conversation, false));
        }
        self.navigate_to(
            Route::Channel {
                workspace_id: self.models.app.active_workspace_id.clone(),
                channel_id,
            },
            window,
            cx,
        );
    }

    pub(crate) fn join_team_channel(
        &mut self,
        conversation: ConversationSummary,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_ui_action(UiAction::JoinChannel {
            conversation: Box::new(conversation),
        });
        self.refresh(cx);
    }
}
