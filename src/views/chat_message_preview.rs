//! The quote shown in place of a link preview for `keybase://chat/...` links.

use crate::domain::message::ChatMessagePreview;
use crate::util::formatting::message_timestamp_label;
use crate::views::avatar::{Avatar, default_avatar_background};
use crate::views::truncated_line::{Segment, TruncatedLine};
use crate::views::{accent, text_primary, text_secondary, tint, typography};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    Div, FontWeight, InteractiveElement, ParentElement, SharedString, Styled, div, px, rgb,
};

const MAX_WIDTH: f32 = 520.;
const AVATAR_SIZE: f32 = 16.;

/// `group` must be unique per preview; callers add the element id, click and
/// hover handlers.
pub fn render_chat_message_preview(preview: &ChatMessagePreview, group: SharedString) -> Div {
    let time = preview
        .timestamp_ms
        .map(|timestamp| message_timestamp_label(Some(timestamp)));
    let details = [Some(preview.location.clone()), time]
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    let details = if details.is_empty() {
        details
    } else {
        format!("\u{2002}{details}")
    };

    let header = div()
        .flex()
        .items_center()
        .gap_1p5()
        .min_w(px(0.))
        .line_height(typography::font_px(18.))
        .text_size(typography::caption())
        .child(div().flex_shrink_0().child(Avatar::render(
            &preview.author,
            preview.author_avatar.as_deref(),
            AVATAR_SIZE,
            default_avatar_background(&preview.author),
            text_primary(),
        )))
        .child(div().flex_1().min_w(px(0.)).child(TruncatedLine::new(vec![
            Segment {
                text: preview.author.clone(),
                color: rgb(text_primary()).into(),
                weight: FontWeight::SEMIBOLD,
            },
            Segment {
                text: details,
                color: rgb(text_secondary()).into(),
                weight: FontWeight::NORMAL,
            },
        ])));

    div()
        .group(group.clone())
        .max_w(px(MAX_WIDTH))
        .min_w(px(0.))
        .border_l_2()
        .border_color(tint(text_secondary(), 0.55))
        .group_hover(group, |style| style.border_color(rgb(accent())))
        .pl_2()
        .py_0p5()
        .flex()
        .flex_col()
        .gap_0p5()
        .child(header)
        .when(!preview.excerpt.is_empty(), |card| {
            card.child(
                div()
                    .min_w(px(0.))
                    .whitespace_normal()
                    .text_size(typography::body())
                    .text_color(rgb(text_primary()))
                    .child(preview.excerpt.clone()),
            )
        })
}
