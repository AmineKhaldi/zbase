use crate::domain::{ids::SidebarSectionId, route::Route};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarRow {
    pub label: String,
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
