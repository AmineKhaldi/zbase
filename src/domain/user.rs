use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::{affinity::Affinity, ids::UserId, presence::Presence};

#[derive(Clone, Debug)]
pub struct UserSummary {
    pub id: UserId,
    pub display_name: String,
    pub title: String,
    pub avatar_asset: Option<String>,
    pub presence: Presence,
    pub affinity: Affinity,
}

impl UserSummary {
    /// The name the app shows for this person (see [`NameStyle`]).
    pub fn shown_name(&self) -> &str {
        shown_name(&self.id.0, &self.display_name)
    }

    /// The name [`Self::shown_name`] leaves out, for tooltips and secondary lines.
    pub fn other_name(&self) -> Option<String> {
        other_name(&self.id.0, &self.display_name)
    }
}

/// Which name the app shows for people: the name on their profile, or always
/// their username. People without a profile name show their username either way.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NameStyle {
    #[default]
    FullName,
    Username,
}

impl NameStyle {
    /// `full_name` is the profile name; an empty one falls back to the username.
    pub fn shown<'a>(self, username: &'a str, full_name: &'a str) -> &'a str {
        let full_name = full_name.trim();
        match self {
            NameStyle::FullName if !full_name.is_empty() => full_name,
            _ => username.trim(),
        }
    }

    /// The other name, when the person has both: "@alice" next to a shown
    /// "Alice Example", or "Alice Example" next to a shown "alice".
    pub fn other(self, username: &str, full_name: &str) -> Option<String> {
        let full_name = distinct_full_name(username, full_name)?;
        Some(match self {
            NameStyle::FullName => format!("@{}", username.trim()),
            NameStyle::Username => full_name.to_string(),
        })
    }
}

/// The profile name, unless it is empty or only the username again.
fn distinct_full_name<'a>(username: &str, full_name: &'a str) -> Option<&'a str> {
    let full_name = full_name.trim();
    (!full_name.is_empty() && !full_name.eq_ignore_ascii_case(username.trim())).then_some(full_name)
}

/// The app-wide choice, set from Settings. Names are resolved in many places,
/// from the store's reducer to tooltips, so it is a process-wide value.
static SHOW_USERNAMES: AtomicBool = AtomicBool::new(false);

pub fn name_style() -> NameStyle {
    if SHOW_USERNAMES.load(Ordering::Relaxed) {
        NameStyle::Username
    } else {
        NameStyle::FullName
    }
}

pub fn set_name_style(style: NameStyle) {
    SHOW_USERNAMES.store(style == NameStyle::Username, Ordering::Relaxed);
}

/// The name to show for a person under the current [`NameStyle`].
pub fn shown_name<'a>(username: &'a str, full_name: &'a str) -> &'a str {
    name_style().shown(username, full_name)
}

/// The name [`shown_name`] leaves out, under the current [`NameStyle`].
pub fn other_name(username: &str, full_name: &str) -> Option<String> {
    name_style().other(username, full_name)
}

#[cfg(test)]
mod tests {
    use super::NameStyle;

    #[test]
    fn full_name_style_prefers_the_profile_name() {
        let style = NameStyle::FullName;
        assert_eq!(style.shown("alice", "Alice Example"), "Alice Example");
        assert_eq!(
            style.other("alice", "Alice Example").as_deref(),
            Some("@alice")
        );
        assert_eq!(style.shown("bob", " "), "bob");
        assert_eq!(style.other("bob", ""), None);
        assert_eq!(style.shown("carol", "Carol"), "Carol");
        assert_eq!(style.other("carol", "Carol"), None);
    }

    #[test]
    fn username_style_shows_the_username_and_keeps_the_name_for_tooltips() {
        let style = NameStyle::Username;
        assert_eq!(style.shown("alice", "Alice Example"), "alice");
        assert_eq!(
            style.other("alice", "Alice Example").as_deref(),
            Some("Alice Example")
        );
        assert_eq!(style.shown("bob", ""), "bob");
        assert_eq!(style.other("bob", ""), None);
    }
}
