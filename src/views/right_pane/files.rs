//! Conversation attachments, using the same pane shell and media openers as chat.
use super::{conversation_location, pane_header, pane_with_header};
use crate::{
    domain::{
        attachment::{AttachmentKind, AttachmentSummary},
        ids::MessageId,
    },
    models::{
        conversation_model::ConversationModel,
        timeline_model::{TimelineModel, TimelineRow},
    },
    views::{self, app_window::AppWindow},
};
use chrono::{DateTime, Local};
use gpui::{
    AnyElement, Context, CursorStyle, IntoElement, ObjectFit, ScrollHandle, SharedString, div, img,
    prelude::*, px, rgb,
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum FilesTab {
    #[default]
    Media,
    Documents,
}

#[derive(Clone, Default)]
pub(crate) struct FilesPaneState {
    pub tab: FilesTab,
    pub media_scroll: ScrollHandle,
    pub documents_scroll: ScrollHandle,
}

struct FileItem<'a> {
    attachment: &'a AttachmentSummary,
    message_id: &'a MessageId,
    author: &'a str,
    timestamp: Option<i64>,
    index: usize,
}

fn is_media(attachment: &AttachmentSummary) -> bool {
    matches!(
        attachment.kind,
        AttachmentKind::Image | AttachmentKind::Video
    )
}

fn items(timeline: &TimelineModel, tab: FilesTab) -> Vec<FileItem<'_>> {
    let mut items = timeline
        .rows
        .iter()
        .filter_map(|row| match row {
            TimelineRow::Message(row) => Some(row),
            _ => None,
        })
        .flat_map(|row| {
            row.message
                .attachments
                .iter()
                .enumerate()
                .filter_map(move |(index, attachment)| {
                    (is_media(attachment) == (tab == FilesTab::Media)).then_some(FileItem {
                        attachment,
                        index,
                        message_id: &row.message.id,
                        author: &row.author.display_name,
                        timestamp: row.message.timestamp_ms,
                    })
                })
        })
        .collect::<Vec<_>>();
    items.sort_by_key(|item| std::cmp::Reverse(item.timestamp));
    items
}

fn date(timestamp: Option<i64>, format: &str) -> String {
    timestamp
        .and_then(DateTime::from_timestamp_millis)
        .map(|date| date.with_timezone(&Local).format(format).to_string())
        .unwrap_or_else(|| "Date unknown".into())
}

/// Preserve the extension when a long name needs ellipsizing; the flex label
/// handles the remaining clipping at the current pane width.
fn filename(name: &str) -> String {
    let name = if name.trim().is_empty() {
        "Untitled file"
    } else {
        name
    };
    if name.chars().count() <= 34 {
        return name.into();
    }
    let (stem, extension) = name
        .rsplit_once('.')
        .filter(|(_, ext)| ext.len() <= 10)
        .map(|(stem, ext)| (stem, format!(".{ext}")))
        .unwrap_or((name, String::new()));
    format!(
        "{}…{}",
        stem.chars().take(27).collect::<String>(),
        extension
    )
}

pub(super) fn render_files_panel(
    timeline: &TimelineModel,
    conversation: &ConversationModel,
    width: f32,
    state: FilesPaneState,
    cx: &mut Context<AppWindow>,
) -> AnyElement {
    let selected = items(timeline, state.tab);
    let count = selected.len();
    let tabs = div().px_4().py_3().flex().gap_1().children(
        [
            (FilesTab::Media, "Media"),
            (FilesTab::Documents, "Documents"),
        ]
        .into_iter()
        .map(|(tab, label)| {
            div()
                .id(label)
                .border_b_2()
                .border_color(if tab == state.tab {
                    rgb(views::accent()).into()
                } else {
                    gpui::transparent_black()
                })
                .flex_1()
                .py_1p5()
                .rounded_md()
                .text_sm()
                .text_center()
                .cursor(CursorStyle::PointingHand)
                .bg(if tab == state.tab {
                    views::panel_alt_surface()
                } else {
                    gpui::transparent_black()
                })
                .text_color(rgb(if tab == state.tab {
                    views::text_primary()
                } else {
                    views::text_secondary()
                }))
                .hover(|s| s.bg(views::subtle_surface()))
                .on_click(cx.listener(move |this, _, _, cx| this.select_files_tab(tab, cx)))
                .child(label)
        }),
    );
    let usable_width = (width - views::RIGHT_PANE_RESIZE_HANDLE_WIDTH_PX - 32. - 8.).max(100.);
    let columns = (usable_width / 94.).floor().max(2.) as usize;
    let tile_size = ((usable_width - (columns - 1) as f32 * 6.) / columns as f32).floor();
    let mut groups: Vec<(String, Vec<FileItem<'_>>)> = Vec::new();
    for item in selected {
        let month = date(item.timestamp, "%B %Y");
        if groups.last().is_none_or(|(label, _)| *label != month) {
            groups.push((month, Vec::new()));
        }
        groups.last_mut().unwrap().1.push(item);
    }
    let scroll = if state.tab == FilesTab::Media {
        &state.media_scroll
    } else {
        &state.documents_scroll
    };
    let body = div()
        .flex_1()
        .min_h(px(0.))
        .id("files-scroll")
        .overflow_y_scroll()
        .track_scroll(scroll)
        .scrollbar_width(px(8.))
        .px_4()
        .pb_4()
        .flex()
        .flex_col()
        .gap_4()
        .when(count == 0, |body| {
            body.child(views::empty_state(
                "assets/icons/files.svg",
                if state.tab == FilesTab::Media {
                    "No media yet"
                } else {
                    "No documents yet"
                },
                Some(
                    if state.tab == FilesTab::Media {
                        "Images and videos shared in this conversation appear here."
                    } else {
                        "Documents and audio shared in this conversation appear here."
                    }
                    .into(),
                ),
            ))
        })
        .children(groups.into_iter().map(|(month, group)| {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(views::section_label(month))
                .child(
                    div()
                        .flex()
                        .when(state.tab == FilesTab::Media, |s| s.flex_wrap().gap(px(6.)))
                        .when(state.tab == FilesTab::Documents, |s| s.flex_col())
                        .children(
                            group
                                .into_iter()
                                .map(|item| render_item(item, state.tab, tile_size, cx)),
                        ),
                )
        }))
        .when(
            timeline
                .older_cursor
                .as_ref()
                .is_some_and(|s| !s.trim().is_empty()),
            |body| {
                body.child(
                    div()
                        .text_xs()
                        .text_color(rgb(views::text_secondary()))
                        .child("Showing attachments from loaded messages."),
                )
                .child(
                    views::secondary_button(
                        "files-load-older",
                        if timeline.loading_older {
                            "Loading older messages…"
                        } else {
                            "Load older messages"
                        },
                    )
                    .when(!timeline.loading_older, |button| {
                        button.on_click(cx.listener(|this, _, _, cx| this.load_older_files(cx)))
                    }),
                )
            },
        );
    pane_with_header(
        pane_header(
            "Files",
            conversation_location(conversation),
            "files-pane-close",
            cx,
        ),
        div()
            .flex_1()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .child(tabs)
            .child(body)
            .into_any_element(),
    )
}

fn render_item(
    item: FileItem<'_>,
    tab: FilesTab,
    size: f32,
    cx: &mut Context<AppWindow>,
) -> AnyElement {
    let download_control = crate::views::app_window::downloads::control(
        item.attachment,
        format!("files-download-{}-{}", item.message_id.0, item.index),
        cx,
    );
    let attachment = item.attachment.clone();
    let name = filename(&attachment.name);
    let full_name = attachment.name.clone();
    let metadata = format!(
        "{} · {} · {}",
        views::attachment_size_text(attachment.size_bytes).unwrap_or_else(|| "Size unknown".into()),
        item.author,
        date(item.timestamp, "%b %-d")
    );
    let id = SharedString::from(format!("file-{}-{}", item.message_id.0, item.index));
    let message_id = item.message_id.clone();
    let mut content = div()
        .id(id)
        .group("files-item")
        .relative()
        .min_w(px(0.))
        .rounded_md()
        .cursor(CursorStyle::PointingHand)
        .hover(|s| s.bg(views::subtle_surface()))
        .tooltip(views::tooltip::TextTooltip::single_line(format!(
            "{full_name}\n{metadata}"
        )))
        .on_click(cx.listener(move |this, _, window, cx| {
            this.open_files_attachment(&attachment, window, cx)
        }));
    if tab == FilesTab::Media {
        content = content
            .w(px(size))
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .w(px(size))
                    .h(px(size))
                    .rounded_md()
                    .overflow_hidden()
                    .relative()
                    .bg(views::panel_alt_surface())
                    .when_some(
                        views::attachment_image_source(item.attachment),
                        |el, source| el.child(img(source).size_full().object_fit(ObjectFit::Cover)),
                    )
                    .when(
                        views::attachment_image_source(item.attachment).is_none(),
                        |el| {
                            el.flex()
                                .items_center()
                                .justify_center()
                                .child(views::icon_asset(
                                    "assets/icons/files.svg",
                                    24.,
                                    views::text_secondary(),
                                ))
                        },
                    )
                    .when(item.attachment.kind == AttachmentKind::Video, |el| {
                        el.child(
                            div()
                                .absolute()
                                .bottom_1()
                                .right_1()
                                .rounded_sm()
                                .px_1()
                                .bg(views::tint(0x000000, 0.7))
                                .text_color(rgb(0xffffff))
                                .text_xs()
                                .child(format!(
                                    "▶ {}",
                                    item.attachment
                                        .duration_ms
                                        .map(views::format_duration_ms)
                                        .unwrap_or_default()
                                )),
                        )
                    }),
            )
            .child(
                div()
                    .absolute()
                    .top_1()
                    .right_1()
                    .rounded_sm()
                    .bg(views::right_pane_surface())
                    .opacity(0.)
                    .group_hover("files-item", |s| s.opacity(1.))
                    .child(jump_button(message_id, item.index, cx)),
            );
        content = content.child(div().flex().justify_end().child(download_control));
    } else {
        content = content
            .w_full()
            .py_2()
            .px_1()
            .flex()
            .items_center()
            .gap_2()
            .child(views::icon_asset(
                "assets/icons/files.svg",
                24.,
                views::text_secondary(),
            ))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(filename_row(&name))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(views::text_secondary()))
                            .truncate()
                            .child(metadata),
                    ),
            )
            .child(download_control)
            .child(jump_button(message_id, item.index, cx));
    }
    content.into_any_element()
}

fn filename_row(name: &str) -> AnyElement {
    let (stem, extension) = name
        .rsplit_once('.')
        .filter(|(_, ext)| ext.len() <= 10)
        .map(|(stem, ext)| (stem, format!(".{ext}")))
        .unwrap_or((name, String::new()));
    div()
        .min_w(px(0.))
        .flex()
        .text_sm()
        .child(div().min_w(px(0.)).truncate().child(stem.to_owned()))
        .child(div().flex_shrink_0().child(extension))
        .into_any_element()
}

fn jump_button(message_id: MessageId, index: usize, cx: &mut Context<AppWindow>) -> AnyElement {
    div()
        .id(SharedString::from(format!(
            "file-jump-{}-{index}",
            message_id.0
        )))
        .text_xs()
        .text_color(rgb(views::text_secondary()))
        .rounded_sm()
        .px_1()
        .py_1()
        .hover(|s| {
            s.bg(views::panel_alt_surface())
                .text_color(rgb(views::text_primary()))
        })
        .tooltip(views::tooltip::TextTooltip::single_line(
            "Show in conversation".into(),
        ))
        .on_click(cx.listener(move |this, _, _, cx| {
            cx.stop_propagation();
            this.show_file_in_conversation(message_id.clone(), cx);
        }))
        .child("↗")
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filenames_keep_extensions_and_unicode_boundaries() {
        assert_eq!(filename("report.pdf"), "report.pdf");
        assert!(filename(&format!("{}.pdf", "界".repeat(60))).ends_with("….pdf"));
        assert_eq!(filename(""), "Untitled file");
    }
    #[test]
    fn gallery_sorts_newest_first_and_separates_documents() {
        let mut timeline = TimelineModel {
            rows: crate::models::demo::demo_files_rows(),
            ..Default::default()
        };
        timeline.rows.reverse();
        let media = items(&timeline, FilesTab::Media);
        assert_eq!(media.len(), 6);
        assert!(
            media
                .windows(2)
                .all(|pair| pair[0].timestamp >= pair[1].timestamp)
        );
        assert_eq!(items(&timeline, FilesTab::Documents).len(), 4);
    }
}
