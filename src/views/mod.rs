mod pane_window;
mod quick_react_placement;
pub mod app_window;
pub mod avatar;
pub mod calls;
pub mod code_highlight;
pub mod composer;
pub mod conversation;
pub mod home;
pub mod inbox;
pub mod inline_markdown;
pub mod input;
pub mod main_panel;
pub mod overlays;
pub mod preferences;
pub mod profile;
pub mod right_pane;
pub mod search;
pub mod selectable_text;
pub mod sidebar;
pub mod splash;
pub mod timeline;

use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, Background, BoxShadow, FontWeight, Hsla, ImageSource, IntoElement, ParentElement,
    SharedString, Styled, div, linear_color_stop, linear_gradient, point, px, rgb, svg,
};
use std::{cell::Cell, path::PathBuf};

use crate::{
    app::theme::ThemeVariant,
    domain::attachment::{AttachmentKind, AttachmentSource, AttachmentSummary},
    domain::message::{LinkPreview, MessageFragment},
};

thread_local! {
    static ACTIVE_THEME: Cell<ThemeVariant> = const { Cell::new(ThemeVariant::Light) };
}

#[derive(Clone, Copy)]
struct Palette {
    app_bg: u32,
    titlebar_bg: u32,
    titlebar_surface: u32,
    rail_bg: u32,
    rail_surface: u32,
    sidebar_bg: u32,
    main_bg: u32,
    panel_bg: u32,
    panel_alt_bg: u32,
    border: u32,
    text_primary: u32,
    text_secondary: u32,
    text_inverted: u32,
    text_muted_inverted: u32,
    accent: u32,
    link: u32,
    unread_indicator: u32,
    accent_soft: u32,
    success: u32,
    success_soft: u32,
    warning: u32,
    warning_soft: u32,
    danger: u32,
    danger_soft: u32,
    affinity_positive: u32,
    affinity_positive_soft: u32,
    affinity_broken: u32,
    affinity_broken_soft: u32,
    mention: u32,
    mention_soft: u32,
    mention_you: u32,
    mention_you_soft: u32,
    inline_code_foreground: u32,
    inline_code_background: u32,
    selection: u32,
}

// Light palette follows Keybase shared/styles/colors.tsx: white, blueGrey,
// black_on_white, blue/blueDark, and the matching semantic status colors.
const LIGHT_PALETTE: Palette = Palette {
    app_bg: 0xffffff,
    titlebar_bg: 0xffffff,
    titlebar_surface: 0xe6e6e6,
    rail_bg: 0x3663ea,
    rail_surface: 0x4c8eff,
    sidebar_bg: 0xf2f4f7,
    main_bg: 0xffffff,
    panel_bg: 0xffffff,
    panel_alt_bg: 0xf0f0f0,
    border: 0xe6e6e6,
    text_primary: 0x262626,
    text_secondary: 0x7f7f7f,
    text_inverted: 0xffffff,
    text_muted_inverted: 0xcccccc,
    accent: 0x4c8eff,
    link: 0x3663ea,
    unread_indicator: 0xff6f21,
    accent_soft: 0xebf2fc,
    success: 0x37bd99,
    success_soft: 0xe8faf6,
    warning: 0x75574e,
    warning_soft: 0xfffdcc,
    danger: 0xeb253b,
    danger_soft: 0xfaf2ed,
    affinity_positive: 0x189e7a,
    affinity_positive_soft: 0xe8faf6,
    affinity_broken: 0xeb253b,
    affinity_broken_soft: 0xfaf2ed,
    mention: 0x3663ea,
    mention_soft: 0xebf2fc,
    mention_you: 0x191919,
    mention_you_soft: 0xffffc0,
    inline_code_foreground: 0x3663ea,
    inline_code_background: 0xfaf2ed,
    selection: 0x4c8eff4d,
};

// Keybase desktop dark colors (shared/styles/colors.tsx): white, blueGrey,
// grey, black_on_white, blue, blueDark, greenDark and semantic status colors.
// Soft fills composite those accents over the neutral chat background.
const DARK_PALETTE: Palette = Palette {
    app_bg: 0x191919,
    titlebar_bg: 0x191919,
    titlebar_surface: 0x333333,
    rail_bg: 0x2d2d2d,
    rail_surface: 0x3f3f3f,
    sidebar_bg: 0x202020,
    main_bg: 0x191919,
    panel_bg: 0x202020,
    panel_alt_bg: 0x2d2d2d,
    border: 0x3b3b3b,
    text_primary: 0xd9d9d9,
    text_secondary: 0x8c8c8c,
    text_inverted: 0xffffff,
    text_muted_inverted: 0xcccccc,
    accent: 0x4c8eff,
    link: 0x3663ea,
    unread_indicator: 0xff6f21,
    accent_soft: 0x192344,
    success: 0x37bd99,
    success_soft: 0x1f3a33,
    warning: 0xffb800,
    warning_soft: 0x3b3115,
    danger: 0xff4d61,
    danger_soft: 0x3b2124,
    affinity_positive: 0x189e7a,
    affinity_positive_soft: 0x193b32,
    affinity_broken: 0xeb253b,
    affinity_broken_soft: 0x3b1b20,
    mention: 0x73a6ff,
    mention_soft: 0x192344,
    mention_you: 0x191919,
    mention_you_soft: 0xc3c390,
    inline_code_foreground: 0x73a6ff,
    inline_code_background: 0x2d2d2d,
    selection: 0x4c8eff4d,
};

fn palette() -> Palette {
    ACTIVE_THEME.with(|theme| match theme.get() {
        ThemeVariant::Light => LIGHT_PALETTE,
        ThemeVariant::Dark => DARK_PALETTE,
    })
}

pub fn with_theme<T>(theme: ThemeVariant, f: impl FnOnce() -> T) -> T {
    ACTIVE_THEME.with(|active| {
        let previous = active.replace(theme);
        let result = f();
        active.set(previous);
        result
    })
}

pub fn current_theme() -> ThemeVariant {
    ACTIVE_THEME.with(|theme| theme.get())
}

pub fn is_dark_theme() -> bool {
    current_theme() == ThemeVariant::Dark
}

pub fn app_bg() -> u32 {
    palette().app_bg
}

pub fn titlebar_bg() -> u32 {
    palette().titlebar_bg
}

pub fn titlebar_surface() -> u32 {
    palette().titlebar_surface
}

pub fn rail_bg() -> u32 {
    palette().rail_bg
}

pub fn rail_surface() -> u32 {
    palette().rail_surface
}

pub fn sidebar_bg() -> u32 {
    palette().sidebar_bg
}

pub fn main_bg() -> u32 {
    palette().main_bg
}

pub fn panel_bg() -> u32 {
    palette().panel_bg
}

pub fn panel_alt_bg() -> u32 {
    palette().panel_alt_bg
}

pub fn panel_surface() -> Hsla {
    tint(panel_bg(), if is_dark_theme() { 0.55 } else { 1.0 })
}

pub fn panel_alt_surface() -> Hsla {
    tint(panel_alt_bg(), if is_dark_theme() { 0.50 } else { 1.0 })
}

pub fn border() -> u32 {
    palette().border
}

pub fn text_primary() -> u32 {
    palette().text_primary
}

pub fn mono_font_family() -> &'static str {
    if cfg!(target_os = "macos") {
        "Menlo"
    } else if cfg!(target_os = "windows") {
        "Consolas"
    } else {
        "monospace"
    }
}

pub fn mono_font_size() -> gpui::Pixels {
    px(12.5)
}

pub fn text_secondary() -> u32 {
    palette().text_secondary
}

pub fn text_inverted() -> u32 {
    palette().text_inverted
}

pub fn text_muted_inverted() -> u32 {
    palette().text_muted_inverted
}

pub fn accent() -> u32 {
    palette().accent
}

pub fn link() -> u32 {
    palette().link
}

pub fn unread_indicator() -> u32 {
    palette().unread_indicator
}

pub fn accent_soft() -> u32 {
    palette().accent_soft
}

pub fn success() -> u32 {
    palette().success
}

pub fn success_soft() -> u32 {
    palette().success_soft
}

pub fn warning() -> u32 {
    palette().warning
}

pub fn warning_soft() -> u32 {
    palette().warning_soft
}

pub fn danger() -> u32 {
    palette().danger
}

pub fn danger_soft() -> u32 {
    palette().danger_soft
}

pub fn affinity_positive() -> u32 {
    palette().affinity_positive
}

pub fn affinity_positive_soft() -> u32 {
    palette().affinity_positive_soft
}

pub fn affinity_broken() -> u32 {
    palette().affinity_broken
}

pub fn affinity_broken_soft() -> u32 {
    palette().affinity_broken_soft
}

pub fn mention() -> u32 {
    palette().mention
}

pub fn mention_soft() -> u32 {
    palette().mention_soft
}

pub fn mention_you() -> u32 {
    palette().mention_you
}

pub fn mention_you_soft() -> u32 {
    palette().mention_you_soft
}

pub fn inline_code_font_size() -> gpui::Pixels {
    px(12.)
}

pub fn inline_code_foreground() -> u32 {
    palette().inline_code_foreground
}

pub fn inline_code_background() -> u32 {
    palette().inline_code_background
}

pub fn selection() -> u32 {
    palette().selection
}

pub fn mention_colors_for_user(
    affinity_index: &std::collections::HashMap<
        crate::domain::ids::UserId,
        crate::domain::affinity::Affinity,
    >,
    current_user_id: Option<&crate::domain::ids::UserId>,
    user_id: &crate::domain::ids::UserId,
) -> (u32, u32) {
    if current_user_id.is_some_and(|me| me.0.eq_ignore_ascii_case(&user_id.0)) {
        return (mention_you(), mention_you_soft());
    }
    let affinity = affinity_index
        .get(user_id)
        .copied()
        .or_else(|| {
            let lower = user_id.0.to_ascii_lowercase();
            if lower == user_id.0 {
                None
            } else {
                affinity_index
                    .get(&crate::domain::ids::UserId::new(lower))
                    .copied()
            }
        })
        .or_else(|| {
            // Some backend sources preserve original casing in user IDs, but we treat them
            // case-insensitively in the UI.
            affinity_index.iter().find_map(|(key, affinity)| {
                key.0.eq_ignore_ascii_case(&user_id.0).then_some(*affinity)
            })
        })
        .unwrap_or(crate::domain::affinity::Affinity::None);
    match affinity {
        crate::domain::affinity::Affinity::None => (mention(), mention_soft()),
        crate::domain::affinity::Affinity::Positive => {
            (affinity_positive(), affinity_positive_soft())
        }
        crate::domain::affinity::Affinity::Broken => (affinity_broken(), affinity_broken_soft()),
    }
}

pub const WINDOW_MIN_WIDTH_PX: f32 = 780.0;
pub const WINDOW_MIN_HEIGHT_PX: f32 = 680.0;
pub const SHELL_HORIZONTAL_PADDING_PX: f32 = 0.0;
pub const SHELL_GAP_PX: f32 = 0.0;
pub const SIDEBAR_WIDTH_PX: f32 = 200.0;
pub const MAIN_PANEL_MIN_WIDTH_PX: f32 = 560.0;
pub const RIGHT_PANE_WIDTH_PX: f32 = 320.0;
pub const RIGHT_PANE_RESIZE_HANDLE_WIDTH_PX: f32 = 4.0;
pub const CUSTOM_EMOJI_REACTION_SIZE_PX: f32 = 16.0;
pub const CUSTOM_EMOJI_INLINE_SIZE_PX: f32 = 20.0;
pub const CUSTOM_EMOJI_EMOJI_ONLY_SIZE_PX: f32 = 48.0;

pub fn tint(color: u32, opacity: f32) -> Hsla {
    Hsla::from(rgb(color)).opacity(opacity)
}

pub fn app_backdrop() -> Background {
    linear_gradient(
        135.,
        linear_color_stop(tint(app_bg(), 1.0), 0.0),
        linear_color_stop(tint(main_bg(), 1.0), 1.0),
    )
}

pub fn glass_surface() -> Hsla {
    if is_dark_theme() {
        tint(panel_bg(), 0.98)
    } else {
        tint(panel_bg(), 1.0)
    }
}

pub fn glass_surface_strong() -> Hsla {
    if is_dark_theme() {
        tint(panel_alt_bg(), 0.98)
    } else {
        tint(panel_alt_bg(), 1.0)
    }
}

pub fn glass_surface_dark() -> Hsla {
    if is_dark_theme() {
        tint(sidebar_bg(), 0.98)
    } else {
        tint(sidebar_bg(), 1.0)
    }
}

pub fn modal_surface() -> Hsla {
    if is_dark_theme() {
        tint(panel_bg(), 0.99)
    } else {
        tint(panel_bg(), 1.0)
    }
}

pub fn content_surface() -> Hsla {
    if is_dark_theme() {
        tint(main_bg(), 0.98)
    } else {
        tint(main_bg(), 1.0)
    }
}

pub fn subtle_surface() -> Hsla {
    tint(panel_alt_bg(), if is_dark_theme() { 0.88 } else { 1.0 })
}

pub fn shell_border() -> Hsla {
    if is_dark_theme() {
        tint(0xffffff, 0.10)
    } else {
        tint(0x000000, 0.10)
    }
}

pub fn shell_border_strong() -> Hsla {
    if is_dark_theme() {
        tint(0xffffff, 0.20)
    } else {
        tint(0x000000, 0.20)
    }
}

pub fn floating_shadow() -> Vec<BoxShadow> {
    if is_dark_theme() {
        vec![
            BoxShadow {
                color: tint(0x000000, 0.28),
                offset: point(px(0.), px(18.)),
                blur_radius: px(42.),
                spread_radius: px(0.),
            },
            BoxShadow {
                color: tint(0x000000, 0.14),
                offset: point(px(0.), px(6.)),
                blur_radius: px(18.),
                spread_radius: px(0.),
            },
        ]
    } else {
        vec![
            BoxShadow {
                color: tint(0x2b3440, 0.07),
                offset: point(px(0.), px(18.)),
                blur_radius: px(42.),
                spread_radius: px(0.),
            },
            BoxShadow {
                color: tint(0x2b3440, 0.03),
                offset: point(px(0.), px(6.)),
                blur_radius: px(18.),
                spread_radius: px(0.),
            },
        ]
    }
}

pub fn card_shadow() -> Vec<BoxShadow> {
    if is_dark_theme() {
        vec![
            BoxShadow {
                color: tint(0x000000, 0.20),
                offset: point(px(0.), px(10.)),
                blur_radius: px(24.),
                spread_radius: px(0.),
            },
            BoxShadow {
                color: tint(0x000000, 0.10),
                offset: point(px(0.), px(2.)),
                blur_radius: px(8.),
                spread_radius: px(0.),
            },
        ]
    } else {
        vec![
            BoxShadow {
                color: tint(0x2b3440, 0.05),
                offset: point(px(0.), px(10.)),
                blur_radius: px(24.),
                spread_radius: px(0.),
            },
            BoxShadow {
                color: tint(0x2b3440, 0.02),
                offset: point(px(0.), px(2.)),
                blur_radius: px(8.),
                spread_radius: px(0.),
            },
        ]
    }
}

pub fn badge(label: impl Into<SharedString>, background: u32, foreground: u32) -> AnyElement {
    div()
        .px_2p5()
        .py_1()
        .rounded_full()
        .bg(rgb(background))
        .text_color(rgb(foreground))
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .child(label.into())
        .into_any_element()
}

pub fn attachment_kind_tag(kind: &AttachmentKind) -> &'static str {
    match kind {
        AttachmentKind::Image => "IMG",
        AttachmentKind::Video => "VID",
        AttachmentKind::Audio => "AUD",
        AttachmentKind::File => "FILE",
    }
}

pub fn attachment_size_text(size_bytes: u64) -> Option<String> {
    if size_bytes == 0 {
        return None;
    }
    if size_bytes < 1024 {
        return Some(format!("{size_bytes} B"));
    }
    const UNITS: &[&str] = &["KB", "MB", "GB", "TB"];
    let mut value = size_bytes as f64 / 1024.0;
    let mut unit_index = 0usize;
    while value >= 1024.0 && unit_index + 1 < UNITS.len() {
        value /= 1024.0;
        unit_index += 1;
    }
    let unit = UNITS[unit_index];
    if value >= 10.0 {
        Some(format!("{value:.0} {unit}"))
    } else {
        Some(format!("{value:.1} {unit}"))
    }
}

pub fn attachment_display_label(attachment: &AttachmentSummary) -> String {
    let prefix = attachment_kind_tag(&attachment.kind);
    if let Some(size_label) = attachment_size_text(attachment.size_bytes) {
        format!("[{prefix}] {} ({size_label})", attachment.name)
    } else {
        format!("[{prefix}] {}", attachment.name)
    }
}

pub fn attachment_is_animated_gif(attachment: &AttachmentSummary) -> bool {
    let name_lower = attachment.name.to_ascii_lowercase();
    if name_lower.ends_with(".gif") {
        return true;
    }
    if let Some(ref mime) = attachment.mime_type {
        if mime.eq_ignore_ascii_case("image/gif") {
            return true;
        }
    }
    false
}

pub fn attachment_image_source(attachment: &AttachmentSummary) -> Option<ImageSource> {
    if attachment_is_animated_gif(attachment) {
        attachment
            .source
            .as_ref()
            .cloned()
            .or_else(|| attachment.preview.as_ref().map(|p| p.source.clone()))
            .and_then(|source| image_source_from_attachment_source(&source))
    } else {
        attachment
            .preview
            .as_ref()
            .map(|preview| preview.source.clone())
            .or_else(|| attachment.source.as_ref().cloned())
            .and_then(|source| image_source_from_attachment_source(&source))
    }
}

pub fn attachment_lightbox_source(attachment: &AttachmentSummary) -> Option<AttachmentSource> {
    attachment
        .source
        .as_ref()
        .cloned()
        .or_else(|| {
            attachment
                .preview
                .as_ref()
                .map(|preview| preview.source.clone())
        })
        .and_then(|source| {
            attachment_source_to_image_source(&source)
                .is_some()
                .then_some(source)
        })
}

pub fn image_source_from_attachment_source(source: &AttachmentSource) -> Option<ImageSource> {
    attachment_source_to_image_source(source)
}

pub fn attachment_open_target(attachment: &AttachmentSummary) -> Option<String> {
    attachment
        .source
        .as_ref()
        .or_else(|| attachment.preview.as_ref().map(|preview| &preview.source))
        .and_then(attachment_source_to_open_target)
}

pub fn attachment_local_path(attachment: &AttachmentSummary) -> Option<String> {
    attachment
        .source
        .as_ref()
        .or_else(|| attachment.preview.as_ref().map(|p| &p.source))
        .and_then(|source| match source {
            AttachmentSource::LocalPath(p) if !p.trim().is_empty() => Some(p.clone()),
            _ => None,
        })
}

fn attachment_source_to_image_source(source: &AttachmentSource) -> Option<ImageSource> {
    match source {
        AttachmentSource::Url(url) => {
            (!is_unrenderable_keybase_asset_url(url)).then(|| ImageSource::from(url.clone()))
        }
        AttachmentSource::LocalPath(path) => {
            let normalized = normalize_local_source_path(path);
            (!normalized.is_empty()).then(|| ImageSource::from(PathBuf::from(normalized)))
        }
    }
}

fn attachment_source_to_open_target(source: &AttachmentSource) -> Option<String> {
    match source {
        AttachmentSource::Url(url) => {
            (!is_unrenderable_keybase_asset_url(url)).then(|| url.clone())
        }
        AttachmentSource::LocalPath(_) => None,
    }
}

fn is_unrenderable_keybase_asset_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("http://127.0.0.1:") || lower.starts_with("http://localhost:") {
        return true;
    }
    let prefix = if lower.starts_with("https://s3.amazonaws.com/") {
        "https://s3.amazonaws.com/"
    } else if lower.starts_with("http://s3.amazonaws.com/") {
        "http://s3.amazonaws.com/"
    } else {
        return false;
    };
    if lower.contains("x-amz-signature=") || lower.contains("awsaccesskeyid=") {
        return false;
    }
    let path = &url[prefix.len()..];
    let Some(first_segment) = path.split('/').next() else {
        return false;
    };
    first_segment.len() == 64 && first_segment.chars().all(|ch| ch.is_ascii_hexdigit())
}

pub fn normalize_local_source_path(raw: &str) -> String {
    let value = raw.trim();
    if value.is_empty() {
        return String::new();
    }
    let Some(path) = value.strip_prefix("file://") else {
        return value.to_string();
    };
    let normalized = if let Some(rest) = path.strip_prefix("localhost/") {
        format!("/{rest}")
    } else {
        path.to_string()
    };
    percent_decode(&normalized)
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = String::with_capacity(value.len());
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex(bytes[index + 1]), hex(bytes[index + 2]))
        {
            out.push((hi << 4 | lo) as char);
            index += 3;
            continue;
        }
        out.push(bytes[index] as char);
        index += 1;
    }
    out
}

fn hex(ch: u8) -> Option<u8> {
    match ch {
        b'0'..=b'9' => Some(ch - b'0'),
        b'a'..=b'f' => Some(ch - b'a' + 10),
        b'A'..=b'F' => Some(ch - b'A' + 10),
        _ => None,
    }
}

fn icon_asset(path: &'static str, size: f32, color: u32) -> AnyElement {
    svg()
        .path(path)
        .w(px(size))
        .h(px(size))
        .min_w(px(size))
        .min_h(px(size))
        .flex_shrink_0()
        .text_color(rgb(color))
        .into_any_element()
}

pub fn home_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/home.svg", 18., color)
}

pub fn dm_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/dm.svg", 18., color)
}

pub fn activity_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/activity.svg", 18., color)
}

pub fn history_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/history.svg", 18., color)
}

pub fn search_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/search.svg", 18., color)
}

pub fn sliders_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/sliders.svg", 18., color)
}

pub fn plus_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/plus.svg", 18., color)
}

pub fn emoji_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/emoji.svg", 18., color)
}

pub fn paperclip_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/paperclip.svg", 18., color)
}

pub fn mention_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/mention.svg", 18., color)
}

pub fn format_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/format.svg", 18., color)
}

pub fn link_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/link.svg", 18., color)
}

pub fn copy_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/copy.svg", 18., color)
}

pub fn clipboard_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/clipboard.svg", 18., color)
}

pub fn trash_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/trash.svg", 18., color)
}

pub fn video_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/video.svg", 18., color)
}

pub fn members_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/members.svg", 18., color)
}

pub fn pin_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/pin.svg", 18., color)
}

pub fn files_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/files.svg", 18., color)
}

pub fn thread_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/thread.svg", 18., color)
}

pub fn call_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/call.svg", 18., color)
}

pub fn more_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/more-horizontal.svg", 16., color)
}

pub fn arrow_left_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/arrow-left.svg", 16., color)
}

pub fn arrow_right_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/arrow-right.svg", 16., color)
}

pub fn chevron_right_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/chevron-right.svg", 14., color)
}

pub fn chevron_down_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/chevron-down.svg", 14., color)
}

pub fn hash_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/hash.svg", 16., color)
}

pub fn at_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/at-sign.svg", 16., color)
}

pub fn close_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/x.svg", 14., color)
}

pub fn crown_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/crown.svg", 14., color)
}

pub fn send_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/send.svg", 16., color)
}

pub fn play_icon(color: u32) -> AnyElement {
    icon_asset("assets/icons/play.svg", 24., color)
}

pub fn paperclip_icon_small(color: u32) -> AnyElement {
    icon_asset("assets/icons/paperclip.svg", 14., color)
}

fn trim_url_token(token: &str) -> Option<&str> {
    let mut t = token.trim();
    if t.is_empty() {
        return None;
    }
    while let Some(ch) = t.as_bytes().first().copied()
        && matches!(ch, b'(' | b'[' | b'<')
    {
        t = &t[1..];
    }
    while let Some(ch) = t.as_bytes().last().copied()
        && matches!(ch, b'.' | b',' | b')' | b']' | b'>' | b';')
    {
        t = &t[..t.len().saturating_sub(1)];
    }
    (!t.is_empty()).then_some(t)
}

pub fn is_media_link_only_message(fragments: &[MessageFragment], previews: &[LinkPreview]) -> bool {
    if fragments.is_empty() || previews.is_empty() {
        return false;
    }

    let matches_media_preview = |url: &str| {
        previews.iter().any(|p| {
            if !p.is_media {
                return false;
            }
            crate::views::app_window::urls_match(&p.url, url)
                || p.video_url
                    .as_deref()
                    .is_some_and(|v| crate::views::app_window::urls_match(v, url))
        })
    };

    let mut saw_any_url = false;
    for fragment in fragments {
        match fragment {
            MessageFragment::Link { url, .. } => {
                if !matches_media_preview(url) {
                    return false;
                }
                saw_any_url = true;
            }
            MessageFragment::Text(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let mut local_saw = false;
                for raw in trimmed.split_whitespace() {
                    let Some(token) = trim_url_token(raw) else {
                        continue;
                    };
                    if !(token.starts_with("http://") || token.starts_with("https://")) {
                        return false;
                    }
                    if !matches_media_preview(token) {
                        return false;
                    }
                    local_saw = true;
                }
                if !local_saw {
                    return false;
                }
                saw_any_url = true;
            }
            _ => return false,
        }
    }
    saw_any_url
}

pub fn attachment_header_row(attachment: &AttachmentSummary) -> AnyElement {
    let name = attachment.name.clone();
    let size = attachment_size_text(attachment.size_bytes);
    div()
        .flex()
        .items_center()
        .gap_1()
        .child(paperclip_icon_small(text_secondary()))
        .child(div().text_xs().text_color(rgb(text_primary())).child(name))
        .when_some(size, |row, size_label| {
            row.child(
                div()
                    .text_xs()
                    .text_color(rgb(text_secondary()))
                    .child(size_label),
            )
        })
        .into_any_element()
}

pub fn format_duration_ms(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    if minutes >= 60 {
        let hours = minutes / 60;
        let mins = minutes % 60;
        format!("{hours}:{mins:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

pub fn composer_tool(icon: AnyElement, color: u32) -> AnyElement {
    div()
        .w(px(28.))
        .h(px(28.))
        .rounded_md()
        .bg(subtle_surface())
        .border_1()
        .border_color(shell_border())
        .flex()
        .items_center()
        .justify_center()
        .child(div().text_color(rgb(color)).child(icon))
        .into_any_element()
}

#[cfg(test)]
mod highlight_color_tests {
    use super::*;
    use crate::domain::{affinity::Affinity, ids::UserId};
    use std::collections::HashMap;

    fn contrast(foreground: u32, background: u32) -> f64 {
        let luminance = |color: u32| {
            let channel = |shift: u32| {
                let value = ((color >> shift) & 255u32) as f64 / 255.;
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
        };
        let a = luminance(foreground);
        let b = luminance(background);
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    #[test]
    fn inline_highlights_are_readable_and_self_mentions_are_distinct_in_both_themes() {
        for theme in [ThemeVariant::Light, ThemeVariant::Dark] {
            with_theme(theme, || {
                let me = UserId::new("CameronCooper");
                let mentioned_me = UserId::new("cameroncooper");
                let other = UserId::new("someone_else");
                let following = HashMap::from([(mentioned_me.clone(), Affinity::Positive)]);
                let own = mention_colors_for_user(&following, Some(&me), &mentioned_me);
                let ordinary = mention_colors_for_user(&following, Some(&me), &other);
                assert_ne!(own, ordinary);
                assert_eq!(own, (mention_you(), mention_you_soft()));
                for (foreground, background) in [
                    own,
                    ordinary,
                    (inline_code_foreground(), inline_code_background()),
                ] {
                    assert!(
                        contrast(foreground, background) >= 4.5,
                        "insufficient contrast: {foreground:06x} on {background:06x}"
                    );
                }
                assert_ne!(inline_code_background(), mention_soft());
            });
        }
    }
}
