//! Sidebar rows in the Previews style: a 32px lead beside two lines, the
//! conversation and its time on top, the newest message (or what is happening
//! now: typing, a draft, a failed send) and the unread badge below. Selection,
//! hover and unread use the compact rows' `RowColors` and `RowActivity`, so
//! both styles read alike.
use super::{RowActivity, RowColors, SidebarHost};
use crate::{
    domain::route::Route,
    models::{sidebar_model::SidebarRow, sidebar_preview::PreviewLive},
    util::formatting::{sidebar_time_label, sidebar_time_label_compact},
    views::{
        accent,
        avatar::Avatar,
        danger,
        truncated_line::{Segment, TruncatedLine},
    },
};
use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, Context, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, div, px, rgb, svg,
};

/// Two text lines of 18 and 16px, 1px apart, centered beside the 32px lead.
pub(super) const PREVIEW_ROW_HEIGHT_PX: f32 = 54.;
pub(super) const PREVIEW_AVATAR_PX: f32 = 32.;
const TITLE_TEXT_PX: f32 = 13.;
const TITLE_LINE_PX: f32 = 18.;
const PREVIEW_TEXT_PX: f32 = 12.;
const PREVIEW_LINE_PX: f32 = 16.;
const TIME_TEXT_PX: f32 = 11.;

pub(super) struct PreviewRow<'a> {
    pub row: &'a SidebarRow,
    /// Element id; also the hover key handed to the host.
    pub id: String,
    /// The title: name first, then anything that may truncate before it.
    pub title: Vec<Segment>,
    /// 32px avatar or mark, already rendered on the row's surface.
    pub lead: AnyElement,
    pub lead_circular: bool,
    pub colors: &'a RowColors,
    pub selected: bool,
    /// Shown instead of the newest message.
    pub live: Option<&'a PreviewLive>,
    /// Short times ("10:30", "Thu") for a sidebar narrower than Previews prefer.
    pub compact_time: bool,
}

impl PreviewRow<'_> {
    pub fn render<H: SidebarHost>(self, cx: &mut Context<H>) -> AnyElement {
        let Self {
            row,
            id,
            title,
            lead,
            lead_circular,
            colors,
            selected,
            live,
            compact_time,
        } = self;
        let activity = RowActivity::of(row);
        let unread = activity.is_unread() && !selected;
        let time = row
            .preview
            .as_ref()
            .and_then(|preview| preview.timestamp_ms)
            .and_then(if compact_time {
                sidebar_time_label_compact
            } else {
                sidebar_time_label
            });
        let (preview_line, italic) = preview_line(row, live, colors, unread, selected);
        // On the selection fill a thin light ring keeps colored marks off the accent.
        let lead = if selected {
            Avatar::with_selection_ring(lead, PREVIEW_AVATAR_PX, lead_circular)
        } else {
            lead
        };
        let hover_id = id.clone();
        let route: Route = row.route.clone();
        div()
            .id(SharedString::from(id))
            .h(px(PREVIEW_ROW_HEIGHT_PX))
            .w_full()
            .flex_shrink_0()
            .px(px(8.))
            .flex()
            .items_center()
            .gap(px(10.))
            .rounded(px(8.))
            .cursor_pointer()
            .when_some(colors.background, |row, background| row.bg(background))
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                this.sidebar_set_row_hovered(hover_id.clone(), *hovered, cx);
            }))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.sidebar_hide_hover_tooltip(cx);
                this.sidebar_navigate_to(route.clone(), window, cx);
            }))
            .child(
                div()
                    .size(px(PREVIEW_AVATAR_PX))
                    .flex_shrink_0()
                    .child(lead),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .child(
                        div()
                            .h(px(TITLE_LINE_PX))
                            .flex()
                            .items_center()
                            // Tight, so the time gives way before the name does.
                            .gap(px(4.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .text_size(px(TITLE_TEXT_PX))
                                    .line_height(px(TITLE_LINE_PX))
                                    .child(TruncatedLine::new(title)),
                            )
                            .when_some(time, |line, time| {
                                line.child(
                                    div()
                                        .flex_shrink_0()
                                        .text_size(px(TIME_TEXT_PX))
                                        .line_height(px(TITLE_LINE_PX))
                                        .text_color(colors.secondary)
                                        .child(time),
                                )
                            }),
                    )
                    .child(
                        div()
                            .h(px(PREVIEW_LINE_PX))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .flex()
                                    .items_center()
                                    .gap(px(3.))
                                    .text_size(px(PREVIEW_TEXT_PX))
                                    .line_height(px(PREVIEW_LINE_PX))
                                    .when(italic, |line| line.italic())
                                    .when(matches!(live, Some(PreviewLive::NotSent)), |line| {
                                        line.child(
                                            svg()
                                                .path("assets/icons/alert-triangle.svg")
                                                .size(px(12.))
                                                .flex_shrink_0()
                                                .text_color(not_sent_color(colors, selected)),
                                        )
                                    })
                                    .child(div().flex_1().min_w(px(0.)).children(preview_line)),
                            )
                            .when_some(activity.indicator(selected), |line, indicator| {
                                line.child(indicator)
                            }),
                    ),
            )
            .into_any_element()
    }
}

/// The second line and whether it is set in italics (typing). Live states
/// replace the newest message; on the selection fill they keep the row's
/// light text so nothing colored sits on the accent.
fn preview_line(
    row: &SidebarRow,
    live: Option<&PreviewLive>,
    colors: &RowColors,
    unread: bool,
    selected: bool,
) -> (Option<TruncatedLine>, bool) {
    let segment = |text: String, color: Hsla, weight: FontWeight| Segment {
        text,
        color,
        weight,
    };
    match live {
        Some(PreviewLive::Typing(text)) => (
            Some(TruncatedLine::new(vec![segment(
                text.clone(),
                colors.secondary,
                FontWeight::NORMAL,
            )])),
            true,
        ),
        Some(PreviewLive::Draft(text)) => (
            Some(TruncatedLine::new(vec![
                segment(
                    "Draft: ".to_string(),
                    if selected {
                        colors.primary
                    } else {
                        rgb(accent()).into()
                    },
                    FontWeight::MEDIUM,
                ),
                segment(text.clone(), colors.secondary, FontWeight::NORMAL),
            ])),
            false,
        ),
        Some(PreviewLive::NotSent) => (
            Some(TruncatedLine::new(vec![segment(
                "Not sent".to_string(),
                not_sent_color(colors, selected),
                FontWeight::MEDIUM,
            )])),
            false,
        ),
        None => {
            // Unread messages read in full contrast, like the title above them.
            let body_color = if unread {
                colors.primary
            } else {
                colors.secondary
            };
            let line = row.preview.as_ref().map(|preview| {
                TruncatedLine::new(
                    preview
                        .prefix
                        .iter()
                        .map(|prefix| segment(prefix.clone(), body_color, FontWeight::MEDIUM))
                        .chain(std::iter::once(segment(
                            preview.text.clone(),
                            body_color,
                            FontWeight::NORMAL,
                        )))
                        .collect(),
                )
            });
            (line, false)
        }
    }
}

fn not_sent_color(colors: &RowColors, selected: bool) -> Hsla {
    if selected {
        colors.primary
    } else {
        rgb(danger()).into()
    }
}

/// Title segments: the name, semibold while unread or selected, then a
/// secondary suffix (the team of an ambiguous channel) that truncates first.
pub(super) fn title_segments(
    label: String,
    suffix: Option<String>,
    colors: &RowColors,
    emphasized: bool,
) -> Vec<Segment> {
    std::iter::once(Segment {
        text: label,
        color: colors.primary,
        weight: if emphasized {
            FontWeight::SEMIBOLD
        } else {
            FontWeight::NORMAL
        },
    })
    .chain(suffix.map(|suffix| Segment {
        text: format!("  {suffix}"),
        color: colors.secondary,
        weight: FontWeight::NORMAL,
    }))
    .collect()
}
