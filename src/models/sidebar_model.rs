use crate::domain::{conversation::ConversationGroup, ids::SidebarSectionId, route::Route};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarRow {
    pub label: String,
    pub team: Option<ConversationGroup>,
    pub unread_count: u32,
    pub mention_count: u32,
    pub route: Route,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarSection {
    pub id: SidebarSectionId,
    pub title: String,
    pub rows: Vec<SidebarRow>,
    pub collapsed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SidebarModel {
    pub sections: Vec<SidebarSection>,
    pub recent_rows: Vec<super::recent_sidebar::RecentSidebarRow>,
    pub mode: super::settings_model::SidebarMode,
    pub filter: String,
    pub highlighted_route: Option<Route>,
    pub width_px: f32,
}

/// Conversation rows may represent people or a simple team's general channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SidebarAvatar {
    Participants(Vec<super::conversation_model::AvatarParticipant>),
    Team(String),
}

impl SidebarAvatar {
    pub fn participants(&self) -> &[super::conversation_model::AvatarParticipant] {
        match self {
            Self::Participants(people) => people,
            Self::Team(_) => &[],
        }
    }
}

impl SidebarRow {
    pub fn full_label(&self) -> String {
        self.team.as_ref().map_or_else(
            || self.label.clone(),
            |team| format!("{} #{}", team.display_name, self.label),
        )
    }
}

/// Separate from conversation-route keys: complex teams use this image in group headers.
pub fn team_avatar_key(team_id: &str) -> String {
    format!("sidebar-team:{team_id}")
}

impl SidebarModel {
    pub fn visible_recent_rows(
        &self,
    ) -> impl Iterator<Item = &super::recent_sidebar::RecentSidebarRow> {
        let filter = self.filter.trim().to_lowercase();
        self.recent_rows.iter().filter(move |recent| {
            filter.is_empty()
                || recent.row.full_label().to_lowercase().contains(&filter)
                || recent.secondary.to_lowercase().contains(&filter)
                || recent.row.route.label().to_lowercase().contains(&filter)
        })
    }

    /// Keep the flat navigation order identical to the grouped visual order. Stable
    /// sorting preserves the reducer's recency order within each team and for DMs.
    pub fn order_unread_groups(&mut self) {
        let team_order = self
            .sections
            .iter()
            .enumerate()
            .filter_map(|(index, section)| {
                section
                    .id
                    .0
                    .strip_prefix("group_")
                    .map(|id| (id.to_string(), index))
            })
            .collect::<HashMap<_, _>>();
        let Some(unread) = self
            .sections
            .iter_mut()
            .find(|section| section.id.0 == "unread")
        else {
            return;
        };
        unread.rows.sort_by_key(|row| match &row.team {
            Some(team) => (
                team_order.get(&team.id).copied().unwrap_or(usize::MAX - 1),
                team.id.clone(),
            ),
            None => (usize::MAX, String::new()),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        conversation::ConversationTeamType,
        ids::{ChannelId, WorkspaceId},
    };

    fn row(id: &str, team: Option<&str>) -> SidebarRow {
        SidebarRow {
            label: id.into(),
            team: team.map(|name| ConversationGroup {
                id: name.into(),
                display_name: name.into(),
                team_type: ConversationTeamType::Complex,
            }),
            unread_count: 1,
            mention_count: 0,
            route: Route::Channel {
                workspace_id: WorkspaceId::new("ws"),
                channel_id: ChannelId::new(id),
            },
        }
    }

    fn team_section(id: &str) -> SidebarSection {
        SidebarSection {
            id: SidebarSectionId::new(format!("group_{id}")),
            title: id.into(),
            rows: vec![],
            collapsed: true,
        }
    }

    #[test]
    fn unread_groups_follow_saved_team_order_and_preserve_recency_within_groups() {
        let mut retained = row("alpha-retained", Some("alpha"));
        retained.unread_count = 0;
        let mut sidebar = SidebarModel {
            mode: Default::default(),
            recent_rows: Vec::new(),
            sections: vec![
                SidebarSection {
                    id: SidebarSectionId::new("unread"),
                    title: "Unread".into(),
                    rows: vec![
                        row("dm-new", None),
                        row("alpha-new", Some("alpha")),
                        row("beta", Some("beta")),
                        retained,
                        row("simple-team", None),
                    ],
                    collapsed: false,
                },
                team_section("beta"),
                team_section("alpha"),
            ],
            filter: String::new(),
            highlighted_route: None,
            width_px: 200.,
        };
        sidebar.order_unread_groups();
        assert_eq!(
            sidebar.sections[0]
                .rows
                .iter()
                .map(|row| row.label.as_str())
                .collect::<Vec<_>>(),
            [
                "beta",
                "alpha-new",
                "alpha-retained",
                "dm-new",
                "simple-team"
            ]
        );
        assert_eq!(
            sidebar.sections[0].rows[2].unread_count, 0,
            "retaining a row must not restore its unread badge"
        );
        sidebar.sections.swap(1, 2);
        sidebar.order_unread_groups();
        assert_eq!(
            sidebar.sections[0]
                .rows
                .iter()
                .map(|row| row.label.as_str())
                .collect::<Vec<_>>(),
            [
                "alpha-new",
                "alpha-retained",
                "beta",
                "dm-new",
                "simple-team"
            ]
        );
        assert!(
            sidebar.sections[1].collapsed,
            "Unread must not depend on normal team expansion"
        );
    }

    #[test]
    fn channel_identity_is_available_without_consuming_its_visible_label() {
        let channel = row("alerts-prod-mainnet", Some("chia_network.ent_wallet"));
        assert_eq!(channel.label, "alerts-prod-mainnet");
        assert_eq!(
            channel.full_label(),
            "chia_network.ent_wallet #alerts-prod-mainnet"
        );
        assert_eq!(row("alice", None).full_label(), "alice");
    }
}
