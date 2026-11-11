use crate::domain::{conversation::ConversationGroup, ids::SidebarSectionId, route::Route};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarRow {
    pub label: String,
    pub team: Option<ConversationGroup>,
    pub unread_count: u32,
    pub mention_count: u32,
    pub route: Route,
    /// Newest message, for the Previews style.
    pub preview: Option<super::sidebar_preview::SidebarPreview>,
    /// Notifications are off: the row is drawn dimmed and plain unread stays quiet.
    pub muted: bool,
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
    pub style: super::settings_model::SidebarStyle,
    pub filter: String,
    pub highlighted_route: Option<Route>,
    pub width_px: f32,
    /// What is happening in a conversation right now (typing, an unsent draft, a
    /// failed send), by conversation id. Previews show it instead of the newest message.
    pub live_previews: HashMap<String, super::sidebar_preview::PreviewLive>,
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

pub const CONVERSATIONS_SECTION_ID: &str = "dms";

impl SidebarSection {
    pub fn is_team(&self) -> bool {
        self.id.0.starts_with("group_")
    }

    pub fn is_conversations(&self) -> bool {
        self.id.0 == CONVERSATIONS_SECTION_ID
    }

    /// The rows listed: all of them, or while collapsed only those unread and
    /// the one open (`current`), so collapsing never hides news or the
    /// selection.
    pub fn visible_rows<'a>(&'a self, current: &'a Route) -> impl Iterator<Item = &'a SidebarRow> {
        self.rows
            .iter()
            .filter(move |row| !self.collapsed || row.is_unread() || &row.route == current)
    }
}

impl SidebarRow {
    /// Unread activity the row shows a badge or dot for: a mention, any
    /// direct message, or other messages unless the conversation is muted.
    pub fn is_unread(&self) -> bool {
        let direct = matches!(self.route, Route::DirectMessage { .. });
        self.mention_count > 0 || (self.unread_count > 0 && (direct || !self.muted))
    }

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupedPosition {
    Conversations(usize),
    /// The item at this index of [`SidebarModel::teams_items`].
    Teams(usize),
}

/// One entry of Grouped mode's teams list.
#[derive(Clone, Copy, Debug)]
pub enum TeamsItem<'a> {
    Header(&'a SidebarSection),
    Row(&'a SidebarSection, &'a SidebarRow),
}

impl SidebarModel {
    /// Grouped mode's teams list while `current` is open: every section but
    /// Conversations, as its header followed by its visible rows.
    pub fn teams_items<'a>(&'a self, current: &'a Route) -> impl Iterator<Item = TeamsItem<'a>> {
        self.sections
            .iter()
            .filter(|section| !section.is_conversations())
            .flat_map(move |section| {
                std::iter::once(TeamsItem::Header(section)).chain(
                    section
                        .visible_rows(current)
                        .map(move |row| TeamsItem::Row(section, row)),
                )
            })
    }

    /// Where `route` is listed in Grouped mode while `current` is open;
    /// `None` when it is not listed.
    pub fn grouped_position(&self, route: &Route, current: &Route) -> Option<GroupedPosition> {
        let conversations = self
            .sections
            .iter()
            .filter(|section| section.is_conversations())
            .find_map(|section| {
                section
                    .visible_rows(current)
                    .position(|row| &row.route == route)
            });
        if let Some(index) = conversations {
            return Some(GroupedPosition::Conversations(index));
        }
        self.teams_items(current)
            .position(|item| matches!(item, TeamsItem::Row(_, row) if &row.route == route))
            .map(GroupedPosition::Teams)
    }

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
            preview: None,
            muted: false,
        }
    }

    #[test]
    fn collapsed_sections_keep_unread_rows_and_the_open_one() {
        let mut read = row("read", Some("alpha"));
        read.unread_count = 0;
        let mut muted = row("muted", Some("alpha"));
        muted.muted = true;
        let mut mentioned = row("mentioned", Some("alpha"));
        mentioned.muted = true;
        mentioned.mention_count = 1;
        let mut open = row("open", Some("alpha"));
        open.unread_count = 0;
        let mut section = SidebarSection {
            id: SidebarSectionId::new("group_alpha"),
            title: "alpha".into(),
            rows: vec![
                read,
                row("unread", Some("alpha")),
                muted,
                mentioned,
                open.clone(),
            ],
            collapsed: false,
        };
        let labels = |section: &SidebarSection| {
            section
                .visible_rows(&open.route)
                .map(|row| row.label.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(labels(&section).len(), 5);
        section.collapsed = true;
        assert_eq!(labels(&section), ["unread", "mentioned", "open"]);
    }

    /// Conversations (alice), then alpha collapsed with a1 read and a2
    /// unread, then beta expanded with b1 and b2.
    fn grouped_sidebar() -> SidebarModel {
        let section = |id: &str, rows: Vec<SidebarRow>, collapsed: bool| SidebarSection {
            id: SidebarSectionId::new(id),
            title: id.into(),
            rows,
            collapsed,
        };
        let mut quiet = row("a1", Some("alpha"));
        quiet.unread_count = 0;
        SidebarModel {
            mode: Default::default(),
            style: Default::default(),
            recent_rows: Vec::new(),
            sections: vec![
                section(CONVERSATIONS_SECTION_ID, vec![row("alice", None)], false),
                section("group_alpha", vec![quiet, row("a2", Some("alpha"))], true),
                section(
                    "group_beta",
                    vec![row("b1", Some("beta")), row("b2", Some("beta"))],
                    false,
                ),
            ],
            filter: String::new(),
            highlighted_route: None,
            width_px: 236.,
            live_previews: HashMap::new(),
        }
    }

    #[test]
    fn grouped_positions_count_headers_and_the_rows_shown() {
        let mut sidebar = grouped_sidebar();
        let elsewhere = Route::Preferences;
        let position = |sidebar: &SidebarModel, id: &str, current: &Route| {
            sidebar.grouped_position(&row(id, None).route, current)
        };
        assert_eq!(
            position(&sidebar, "alice", &elsewhere),
            Some(GroupedPosition::Conversations(0))
        );
        assert_eq!(position(&sidebar, "a1", &elsewhere), None);
        assert_eq!(
            position(&sidebar, "a2", &elsewhere),
            Some(GroupedPosition::Teams(1))
        );
        assert_eq!(
            position(&sidebar, "b2", &elsewhere),
            Some(GroupedPosition::Teams(4))
        );
        let a1 = row("a1", None).route;
        assert_eq!(
            position(&sidebar, "a1", &a1),
            Some(GroupedPosition::Teams(1))
        );
        sidebar.sections[1].collapsed = false;
        assert_eq!(
            position(&sidebar, "b2", &elsewhere),
            Some(GroupedPosition::Teams(5))
        );
    }

    #[test]
    fn grouped_positions_index_the_teams_list_as_listed() {
        let mut sidebar = grouped_sidebar();
        let routes = ["a1", "a2", "b1"].map(|id| row(id, None).route);
        for collapsed in [true, false] {
            sidebar.sections[1].collapsed = collapsed;
            for current in routes.iter().chain([&Route::Preferences]) {
                let items = sidebar.teams_items(current).collect::<Vec<_>>();
                assert!(matches!(items[0], TeamsItem::Header(_)));
                for (index, item) in items.iter().enumerate() {
                    if let TeamsItem::Row(_, row) = item {
                        assert_eq!(
                            sidebar.grouped_position(&row.route, current),
                            Some(GroupedPosition::Teams(index)),
                            "{} with {current:?} open",
                            row.label
                        );
                    }
                }
            }
        }
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
