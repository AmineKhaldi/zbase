use crate::domain::presence::Availability;
use crate::models::conversation_model::AvatarParticipant;
use crate::views::{accent, is_dark_theme, panel_surface};
use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, FontWeight, Hsla, ImageSource, IntoElement, ObjectFit, ParentElement, Styled,
    StyledImage, div, img, px, rgb,
};
use std::path::PathBuf;

pub const ME_AVATAR_ASSET: &str = "assets/avatars/me.svg";

pub struct Avatar;

enum GroupAvatarStyle {
    Faces,
    Count,
}

impl Avatar {
    pub fn render(
        name: &str,
        asset: Option<&str>,
        size: f32,
        background: u32,
        foreground: u32,
    ) -> AnyElement {
        Self::render_shaped(name, asset, size, background, foreground, true)
    }

    pub fn render_square(
        name: &str,
        asset: Option<&str>,
        size: f32,
        background: u32,
        foreground: u32,
    ) -> AnyElement {
        Self::render_shaped(name, asset, size, background, foreground, false)
    }

    /// Team or channel mark: a rounded square with the team image, or its uppercase
    /// initial on a stable per-team hue that reads in both themes.
    pub fn render_team(name: &str, asset: Option<&str>, size: f32) -> AnyElement {
        let (background, foreground) = team_colors(name);
        Self::render_shaped(name, asset, size, background, foreground, false)
    }

    pub fn render_participants(
        participants: &[AvatarParticipant],
        fallback_name: &str,
        size: f32,
    ) -> AnyElement {
        Self::render_participants_on(
            participants,
            fallback_name,
            size,
            rgb(crate::views::panel_bg()).into(),
        )
    }

    /// Like [`Self::render_participants`], with `surface` as the ring that separates
    /// the overlapping pair of a group conversation and the presence dot of a 1:1.
    pub fn render_participants_on(
        participants: &[AvatarParticipant],
        fallback_name: &str,
        size: f32,
        surface: Hsla,
    ) -> AnyElement {
        Self::render_participants_with_style(
            participants,
            fallback_name,
            size,
            surface,
            GroupAvatarStyle::Faces,
        )
    }

    /// Preserve a pair of peer faces; use a count only for larger groups.
    pub fn render_sidebar_participants_on(
        participants: &[AvatarParticipant],
        fallback_name: &str,
        size: f32,
        surface: Hsla,
    ) -> AnyElement {
        Self::render_participants_with_style(
            participants,
            fallback_name,
            size,
            surface,
            GroupAvatarStyle::Count,
        )
    }

    // Header and sidebar use the same unique peer projection (excluding you),
    // including the same fallback and 1:1 presence/avatar behavior.
    fn render_participants_with_style(
        participants: &[AvatarParticipant],
        fallback_name: &str,
        size: f32,
        surface: Hsla,
        group_style: GroupAvatarStyle,
    ) -> AnyElement {
        if let [participant] = participants {
            let avatar = Self::render(
                &participant.display_name,
                participant.asset.as_deref(),
                size,
                default_avatar_background(&participant.display_name),
                crate::views::text_primary(),
            );
            return Self::with_presence(
                avatar,
                size,
                participant.presence.as_ref().map(|presence| &presence.availability),
                surface,
            );
        }
        if participants.is_empty() {
            return Self::render(
                fallback_name,
                None,
                size,
                default_avatar_background(fallback_name),
                crate::views::text_primary(),
            );
        }
        if participants.len() > 2 && matches!(group_style, GroupAvatarStyle::Count) {
            return Self::render_group_count(participants, size);
        }
        let avatars = participants
            .iter()
            .take(2)
            .map(|participant| {
                (
                    participant.display_name.as_str(),
                    participant.asset.as_deref(),
                    default_avatar_background(&participant.display_name),
                    crate::views::text_primary(),
                )
            })
            .collect::<Vec<_>>();
        Self::render_group_on(&avatars, size, surface)
    }

    fn peer_count_label(participants: &[AvatarParticipant]) -> String {
        let people = participants.len();
        if people > 99 {
            "99+".to_string()
        } else {
            people.to_string()
        }
    }

    /// At sidebar size, count the same peers the header represents with faces.
    fn render_group_count(participants: &[AvatarParticipant], size: f32) -> AnyElement {
        let label = Self::peer_count_label(participants);
        div()
            .flex_shrink_0()
            .w(px(size))
            .h(px(size))
            .rounded_full()
            .bg(rgb(default_avatar_background("")))
            .text_color(rgb(crate::views::text_primary()))
            .flex()
            .items_center()
            .justify_center()
            // 11px at sidebar size, growing with the disc so it never looks lost.
            .text_size(px((size * 0.4).round().clamp(11., 14.)))
            .font_weight(FontWeight::SEMIBOLD)
            .line_height(px(size))
            .child(label)
            .into_any_element()
    }

    /// A channel: a neutral tile with `#`, so channels of one team differ from
    /// its team mark. `team` adds the team's square as a badge where a DM's
    /// presence dot sits, ringed in `surface`; omit it under the team's header.
    pub fn render_channel(team: Option<(&str, Option<&str>)>, size: f32, surface: Hsla) -> AnyElement {
        let tile = div()
            .flex_shrink_0()
            .w(px(size))
            .h(px(size))
            .rounded(px(square_radius(size)))
            .bg(rgb(if is_dark_theme() { 0x2f2f2f } else { 0xe6e8ec }))
            .flex()
            .items_center()
            .justify_center()
            .child(crate::views::hash_icon(crate::views::text_secondary()));
        let Some((name, asset)) = team else {
            return tile.into_any_element();
        };
        let badge = (size * 0.44).round();
        let ring = 2.;
        div()
            .relative()
            .flex_shrink_0()
            .w(px(size))
            .h(px(size))
            .child(tile)
            .child(
                div()
                    .absolute()
                    .right(px(-ring - 1.))
                    .bottom(px(-ring - 1.))
                    .p(px(ring))
                    .rounded(px(square_radius(badge) + ring))
                    .bg(surface)
                    .child(Self::render_team(name, asset, badge)),
            )
            .into_any_element()
    }

    /// Draws a thin light ring around a lead (avatar or mark) on a selected
    /// row, so colored marks don't sit directly on the selection fill.
    pub fn with_selection_ring(lead: AnyElement, size: f32, circular: bool) -> AnyElement {
        let ring = 1.5;
        div()
            .relative()
            .flex_shrink_0()
            .w(px(size))
            .h(px(size))
            // Painted first: the lead covers the ring's inner half, and a
            // presence dot or team badge stays on top of it.
            .child(
                div()
                    .absolute()
                    .top(px(-ring))
                    .left(px(-ring))
                    .w(px(size + ring * 2.))
                    .h(px(size + ring * 2.))
                    .when(circular, |ring| ring.rounded_full())
                    .when(!circular, |frame| frame.rounded(px(square_radius(size) + ring)))
                    .border(px(ring))
                    .border_color(if is_dark_theme() {
                        crate::views::tint(0xffffff, 0.2)
                    } else {
                        crate::views::tint(0xffffff, 1.)
                    }),
            )
            .child(lead)
            .into_any_element()
    }

    pub fn render_group(names: &[(&str, Option<&str>, u32, u32)], size: f32) -> AnyElement {
        Self::render_group_on(names, size, rgb(crate::views::panel_bg()).into())
    }

    fn render_group_on(
        names: &[(&str, Option<&str>, u32, u32)],
        size: f32,
        surface: Hsla,
    ) -> AnyElement {
        if names.len() < 2 {
            if let Some((name, asset, bg, fg)) = names.first() {
                return Self::render(name, *asset, size, *bg, *fg);
            }
            return div().w(px(size)).h(px(size)).into_any_element();
        }

        // Two faces on a diagonal; the front one carries a surface-colored ring so
        // the pair never reads as one blob.
        let mini = (size * 0.7).round();
        let offset = size - mini;
        let ring = (size / 12.).clamp(1.5, 2.5);

        let (name1, asset1, bg1, fg1) = &names[0];
        let (name2, asset2, bg2, fg2) = &names[1];

        div()
            .w(px(size))
            .h(px(size))
            .flex_shrink_0()
            .relative()
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .child(Self::render_shaped(name1, *asset1, mini, *bg1, *fg1, true)),
            )
            .child(
                div()
                    .absolute()
                    .top(px(offset - ring))
                    .left(px(offset - ring))
                    .w(px(mini + ring * 2.))
                    .h(px(mini + ring * 2.))
                    .rounded_full()
                    .bg(surface)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(Self::render_shaped(name2, *asset2, mini, *bg2, *fg2, true)),
            )
            .into_any_element()
    }

    /// Overlays a presence dot on the bottom-right of an avatar. `surface` is the
    /// color behind the avatar, used as the dot's ring. Offline is a hollow ring;
    /// unknown presence draws nothing.
    pub fn with_presence(
        avatar: AnyElement,
        size: f32,
        availability: Option<&Availability>,
        surface: Hsla,
    ) -> AnyElement {
        let Some(availability) = availability else {
            return avatar;
        };
        let Some(dot) = presence_dot(availability, size, surface) else {
            return avatar;
        };
        let dot_size = presence_dot_outer_size(size);
        div()
            .relative()
            .flex_shrink_0()
            .w(px(size))
            .h(px(size))
            .child(avatar)
            .child(
                div()
                    .absolute()
                    .right(px(-dot_size * 0.2))
                    .bottom(px(-dot_size * 0.2))
                    .child(dot),
            )
            .into_any_element()
    }

    /// [`Self::with_presence`] with a hover tooltip naming the state ("Away"),
    /// so presence never depends on reading the dot alone.
    pub fn with_presence_tooltip(
        id: impl Into<gpui::ElementId>,
        avatar: AnyElement,
        size: f32,
        availability: Option<&Availability>,
        surface: Hsla,
    ) -> AnyElement {
        use gpui::{InteractiveElement, StatefulInteractiveElement};
        let label = availability.and_then(presence_label);
        let avatar = Self::with_presence(avatar, size, availability, surface);
        match label {
            Some(label) => div()
                .id(id)
                .flex_shrink_0()
                .child(avatar)
                .tooltip(crate::views::tooltip::TextTooltip::single_line(
                    label.to_string(),
                ))
                .into_any_element(),
            None => avatar,
        }
    }

    fn render_shaped(
        name: &str,
        asset: Option<&str>,
        size: f32,
        background: u32,
        foreground: u32,
        circular: bool,
    ) -> AnyElement {
        match asset {
            Some(asset_path) => image_source_for_asset(asset_path)
                .map(|source| Self::image(name, source, size, background, foreground, circular))
                .unwrap_or_else(|| Self::fallback(name, size, background, foreground, circular)),
            None => Self::fallback(name, size, background, foreground, circular),
        }
    }

    pub fn fallback(
        name: &str,
        size: f32,
        background: u32,
        foreground: u32,
        circular: bool,
    ) -> AnyElement {
        // Small marks (sidebar, team squares) have room for one letter only.
        let initials = if size < 22. || !circular {
            initials_for_name(name).chars().take(1).collect()
        } else {
            initials_for_name(name)
        };

        div()
            .flex_shrink_0()
            .w(px(size))
            .h(px(size))
            .when(circular, |d| d.rounded_full())
            .when(!circular, |d| d.rounded(px(square_radius(size))))
            .bg(rgb(background))
            .text_color(rgb(foreground))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px((size * if circular { 0.42 } else { 0.55 }).clamp(7., 15.)))
            .font_weight(FontWeight::SEMIBOLD)
            .line_height(px(size))
            .child(initials)
            .into_any_element()
    }

    fn image(
        name: &str,
        image_source: ImageSource,
        size: f32,
        background: u32,
        foreground: u32,
        circular: bool,
    ) -> AnyElement {
        let fallback_name = name.to_string();

        div()
            .w(px(size))
            .h(px(size))
            .flex_shrink_0()
            .when(circular, |d| d.rounded_full())
            .when(!circular, |d| d.rounded(px(square_radius(size))))
            .overflow_hidden()
            .bg(panel_surface())
            .child(
                img(image_source)
                    .w(px(size))
                    .h(px(size))
                    .when(circular, |d| d.rounded_full())
                    .when(!circular, |d| d.rounded(px(square_radius(size))))
                    .object_fit(ObjectFit::Cover)
                    .with_fallback(move || {
                        Self::fallback(&fallback_name, size, background, foreground, circular)
                    }),
            )
            .into_any_element()
    }
}

pub fn default_avatar_background(name: &str) -> u32 {
    match name {
        "You" => accent(),
        _ if is_dark_theme() => 0x484848,
        _ => 0xd8dde3,
    }
}

/// Corner radius for square marks: 4px at sidebar size, 6px at header size.
fn square_radius(size: f32) -> f32 {
    (size * 0.22).round().clamp(3., 12.)
}

/// Fill and initial color for a team mark without an image. The hue is derived
/// from the name so a team keeps its color everywhere; every fill carries white
/// text and stands out from both the light and the dark sidebar.
pub fn team_colors(name: &str) -> (u32, u32) {
    const FILLS: [u32; 8] = [
        0x3f73d9, // blue
        0x7a55d6, // violet
        0x0f8a7e, // teal
        0xc0522a, // rust
        0xb8437c, // magenta
        0x3d8a3f, // green
        0x5a6b85, // slate
        0xa86512, // amber
    ];
    let hash = name
        .trim()
        .to_lowercase()
        .bytes()
        .fold(0x811c9dc5u32, |hash, byte| {
            (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
        });
    (FILLS[hash as usize % FILLS.len()], 0xffffff)
}

pub fn presence_color(availability: &Availability) -> Option<u32> {
    match availability {
        Availability::Active => Some(crate::views::success()),
        Availability::Away => Some(0xf2a516),
        Availability::DoNotDisturb => Some(crate::views::danger()),
        Availability::Offline => Some(crate::views::text_secondary()),
        Availability::Unknown => None,
    }
}

pub fn presence_label(availability: &Availability) -> Option<&'static str> {
    match availability {
        Availability::Active => Some("Active"),
        Availability::Away => Some("Away"),
        Availability::DoNotDisturb => Some("Do not disturb"),
        Availability::Offline => Some("Offline"),
        Availability::Unknown => None,
    }
}

fn presence_dot_inner_size(avatar_size: f32) -> f32 {
    if avatar_size <= 20. {
        7.
    } else if avatar_size <= 28. {
        8.
    } else {
        10.
    }
}

fn presence_dot_outer_size(avatar_size: f32) -> f32 {
    presence_dot_inner_size(avatar_size) + 4.
}

/// A presence dot with a 2px ring in `surface`, sized for an avatar of
/// `avatar_size`. Every state has its own shape so it survives without color:
/// Active is solid, Away carries clock hands, Do not disturb a minus bar, and
/// Offline is a hollow ring.
pub fn presence_dot(availability: &Availability, avatar_size: f32, surface: Hsla) -> Option<AnyElement> {
    let color = presence_color(availability)?;
    let inner = presence_dot_inner_size(avatar_size);
    let outer = presence_dot_outer_size(avatar_size);
    let glyph = presence_glyph(availability);
    // Strokes stay at least 1.5px so they hold up at sidebar size.
    let stroke = (inner * 0.2).max(1.5);
    let mark = div()
        .w(px(inner))
        .h(px(inner))
        .rounded_full()
        .relative()
        .when(glyph == PresenceGlyph::Ring, |dot| {
            dot.border(px(1.5)).border_color(rgb(color))
        })
        .when(glyph != PresenceGlyph::Ring, |dot| dot.bg(rgb(color)))
        .when(glyph == PresenceGlyph::Minus, |dot| {
            let bar = (inner * 0.6).round();
            dot.child(
                div()
                    .absolute()
                    .left(px((inner - bar) / 2.))
                    .top(px((inner - stroke) / 2.))
                    .w(px(bar))
                    .h(px(stroke))
                    .rounded(px(stroke / 2.))
                    .bg(rgb(0xffffff)),
            )
        })
        .when(glyph == PresenceGlyph::Clock, |dot| {
            // Hands from the center: one up to 12, one across to 3.
            let center = inner / 2.;
            let hand = (inner * 0.32).max(2.);
            dot.child(
                div()
                    .absolute()
                    .left(px(center - stroke / 2.))
                    .top(px(center - hand))
                    .w(px(stroke))
                    .h(px(hand + stroke / 2.))
                    .rounded(px(stroke / 2.))
                    .bg(rgb(0xffffff)),
            )
            .child(
                div()
                    .absolute()
                    .left(px(center - stroke / 2.))
                    .top(px(center - stroke / 2.))
                    .w(px(hand + stroke / 2.))
                    .h(px(stroke))
                    .rounded(px(stroke / 2.))
                    .bg(rgb(0xffffff)),
            )
        });
    Some(
        div()
            .w(px(outer))
            .h(px(outer))
            .rounded_full()
            .bg(surface)
            .flex()
            .items_center()
            .justify_center()
            .child(mark)
            .into_any_element(),
    )
}

/// The shape inside a presence dot, so states differ by more than hue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PresenceGlyph {
    Solid,
    Clock,
    Minus,
    Ring,
}

pub fn presence_glyph(availability: &Availability) -> PresenceGlyph {
    match availability {
        Availability::Active | Availability::Unknown => PresenceGlyph::Solid,
        Availability::Away => PresenceGlyph::Clock,
        Availability::DoNotDisturb => PresenceGlyph::Minus,
        Availability::Offline => PresenceGlyph::Ring,
    }
}

fn initials_for_name(name: &str) -> String {
    let mut parts = name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .flat_map(char::to_uppercase)
        .take(2)
        .collect::<String>();

    if parts.is_empty() {
        parts.push('?');
    }

    parts
}

pub(crate) fn image_source_for_asset(asset_path: &str) -> Option<ImageSource> {
    let trimmed = asset_path.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("assets/")
    {
        return Some(ImageSource::from(trimmed.to_string()));
    }
    let normalized = crate::views::normalize_local_source_path(trimmed);
    if normalized.is_empty() {
        return None;
    }
    Some(ImageSource::from(PathBuf::from(normalized)))
}

#[cfg(test)]
mod presence_tests {
    use super::*;

    #[test]
    fn group_avatar_counts_only_the_projected_peers() {
        let peer = |name: &str| AvatarParticipant {
            display_name: name.into(),
            asset: None,
            presence: None,
        };
        let peers = vec![peer("Alexander Mey"), peer("Craig Corning, Jr.")];
        assert_eq!(Avatar::peer_count_label(&peers), "2");
        let mut larger = peers.clone();
        larger.push(peer("Gene Hoffman, Jr."));
        assert_eq!(Avatar::peer_count_label(&larger), "3");
        assert_eq!(Avatar::peer_count_label(&vec![peer("Guest"); 100]), "99+");
    }

    #[test]
    fn every_known_state_has_a_distinct_shape_and_label() {
        let states = [
            Availability::Active,
            Availability::Away,
            Availability::DoNotDisturb,
            Availability::Offline,
        ];
        let glyphs: std::collections::HashSet<_> = states.iter().map(presence_glyph).collect();
        assert_eq!(glyphs.len(), states.len());
        for state in &states {
            assert!(presence_label(state).is_some());
        }
    }
}
