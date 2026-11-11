use super::{member_avatar, member_markers, pane_header, pane_with_header, truncate_with_ellipsis};
use crate::{
    domain::{
        backend::BackendCapabilities, channel_details::ChannelMemberPreview,
        conversation::ConversationKind,
    },
    models::{
        conversation_model::ConversationModel,
        timeline_model::{TimelineModel, TimelineRow},
    },
    views::{
        accent, app_window::AppWindow, avatar::Avatar, badge, border, danger, panel_alt_bg,
        panel_alt_surface, panel_surface, subtle_surface, text_primary, text_secondary,
    },
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyElement, Context, CursorStyle, Div, FontWeight, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, div, px, rgb,
};

fn details_card() -> Div {
    div()
        .flex_shrink_0()
        .min_w(px(0.))
        .rounded_lg()
        .bg(panel_surface())
        .border_1()
        .border_color(rgb(border()))
        .p_4()
        .flex()
        .flex_col()
}

fn section_heading(title: &str) -> AnyElement {
    div()
        .text_sm()
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(text_secondary()))
        .child(title.to_owned())
        .into_any_element()
}

fn member_section(
    title: &str,
    row_id: &'static str,
    members: Vec<ChannelMemberPreview>,
    cx: &mut Context<AppWindow>,
) -> AnyElement {
    details_card()
        .gap_2()
        .child(section_heading(title))
        .children(members.into_iter().enumerate().map(|(index, member)| {
            let user_id = member.user_id.clone();
            div()
                .id((row_id, index))
                .w_full()
                .rounded_md()
                .px_2()
                .py_2()
                .flex()
                .items_center()
                .gap_2()
                .min_w(px(0.))
                .cursor(CursorStyle::PointingHand)
                .hover(|style| style.bg(subtle_surface()))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_user_profile_panel(user_id.clone(), window, cx);
                }))
                .child(member_avatar("details", &member, 32., panel_surface()))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .whitespace_nowrap()
                        .text_sm()
                        .text_ellipsis()
                        .text_color(rgb(text_primary()))
                        .child(member.display_name.clone()),
                )
                .children(member_markers(&member, row_id))
                .into_any_element()
        }))
        .into_any_element()
}

pub(super) fn render_details_panel(
    conversation: &ConversationModel,
    timeline: &TimelineModel,
    _capabilities: &BackendCapabilities,
    cx: &mut Context<AppWindow>,
) -> AnyElement {
    let details = conversation.details.as_ref();
    let shared_attachments = timeline
        .rows
        .iter()
        .filter_map(|row| match row {
            TimelineRow::Message(message) => Some(message.message.attachments.iter().cloned()),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();
    let files_count = shared_attachments.len();
    let member_count = details
        .map(|details| details.member_count)
        .unwrap_or(conversation.member_count);
    let members = details
        .map(|details| details.members.clone())
        .unwrap_or_default();
    let (bots, people): (Vec<_>, Vec<_>) = members
        .into_iter()
        .partition(|member| member.user_id.0.to_ascii_lowercase().ends_with("bot"));
    let has_members = !people.is_empty();
    let has_bots = !bots.is_empty();
    let member_count = if member_count == 0 {
        (people.len() + bots.len()) as u32
    } else {
        member_count
    };
    let group_name = details
        .and_then(|details| details.group.as_ref())
        .map(|group| group.display_name.clone())
        .or_else(|| {
            conversation
                .summary
                .group
                .as_ref()
                .map(|group| group.display_name.clone())
        });

    let title = if conversation.summary.kind == ConversationKind::Channel {
        format!("#{}", conversation.summary.title)
    } else {
        conversation.summary.title.clone()
    };
    let topic = details.map(|details| details.topic.trim()).filter(|topic| {
        !topic.is_empty()
            && *topic != conversation.summary.title
            && group_name.as_deref() != Some(*topic)
    });
    let kind_label = match conversation.summary.kind {
        ConversationKind::Channel => "Channel",
        ConversationKind::DirectMessage => "Direct message",
        ConversationKind::GroupDirectMessage => "Group conversation",
    };

    let body =
        div()
            .flex_1()
            .min_h(px(0.))
            .id("right-pane-details-scroll")
            .overflow_y_scroll()
            .scrollbar_width(px(8.))
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                details_card()
                    .items_center()
                    .gap_3()
                    .child(if conversation.summary.kind == ConversationKind::Channel {
                        Avatar::render_team(
                            group_name.as_deref().unwrap_or(&title),
                            conversation.avatar_asset.as_deref(),
                            64.,
                        )
                    } else {
                        Avatar::render_participants_on(
                            &conversation.participant_avatars,
                            &title,
                            64.,
                            panel_surface(),
                        )
                    })
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(text_primary()))
                            .text_center()
                            .child(title),
                    )
                    .when_some(group_name, |card, group| {
                        card.child(
                            div()
                                .text_sm()
                                .text_center()
                                .text_color(rgb(text_secondary()))
                                .child(group),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .justify_center()
                            .gap_2()
                            .child(badge(kind_label, panel_alt_bg(), text_secondary()))
                            .when(member_count > 0, |row| {
                                row.child(badge(
                                    format!("{member_count} members"),
                                    panel_alt_bg(),
                                    text_secondary(),
                                ))
                            })
                            .when(conversation.summary.muted, |row| {
                                row.child(badge("Muted", panel_alt_bg(), text_secondary()))
                            }),
                    ),
            )
            .when_some(topic, |panel, topic| {
                panel.child(
                    details_card()
                        .gap_2()
                        .child(section_heading("About"))
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(text_primary()))
                                .child(topic.to_owned()),
                        ),
                )
            })
            .when(has_members, |panel| {
                panel.child(member_section(
                    &format!("Members ({})", people.len()),
                    "details-member-entry",
                    people,
                    cx,
                ))
            })
            .when(has_bots, |panel| {
                panel.child(member_section(
                    &format!("Bots ({})", bots.len()),
                    "details-bot-entry",
                    bots,
                    cx,
                ))
            })
            .when(!has_members && !has_bots, |panel| {
                panel.child(
                    details_card()
                        .gap_2()
                        .child(section_heading("Members"))
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(text_secondary()))
                                .child("Member list is not available yet."),
                        ),
                )
            })
            .child(
                details_card()
                    .gap_2()
                    .child(section_heading(&format!("Shared files ({files_count})")))
                    .when(files_count == 0, |section| {
                        section.child(
                            div()
                                .rounded_md()
                                .bg(rgb(panel_alt_bg()))
                                .px_2()
                                .py_1p5()
                                .text_xs()
                                .text_color(rgb(text_secondary()))
                                .child("No files in the loaded messages."),
                        )
                    })
                    .children(shared_attachments.into_iter().enumerate().map(
                        |(index, attachment)| {
                            let download_control = crate::views::app_window::downloads::control(
                                &attachment,
                                format!("details-download-{index}"),
                                cx,
                            );
                            let file_name = if attachment.name.trim().is_empty() {
                                "attachment".to_string()
                            } else {
                                attachment.name.clone()
                            };
                            let truncated_name = truncate_with_ellipsis(&file_name, 56);
                            div()
                                .id(("details-shared-file", index))
                                .rounded_md()
                                .bg(rgb(panel_alt_bg()))
                                .px_2()
                                .py_1p5()
                                .flex()
                                .items_center()
                                .min_w(px(0.))
                                .cursor(CursorStyle::PointingHand)
                                .hover(|s| s.bg(subtle_surface()))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.open_files_attachment(&attachment, window, cx);
                                }))
                                .child(
                                    div()
                                        .id(("details-shared-file-open", index))
                                        .flex_1()
                                        .min_w(px(0.))
                                        .overflow_hidden()
                                        .text_xs()
                                        .text_color(rgb(accent()))
                                        .child(truncated_name),
                                )
                                .child(download_control)
                                .into_any_element()
                        },
                    )),
            )
            .when_some(
                details.and_then(|details| details.leave_error.clone()),
                |panel, error| panel.child(div().text_sm().text_color(rgb(danger())).child(error)),
            )
            .when(details.is_some_and(|details| details.can_leave), |panel| {
                let leaving = details.is_some_and(|details| details.is_leaving);
                panel.child(
                    div()
                        .id("leave-channel")
                        .flex_shrink_0()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(border()))
                        .bg(panel_alt_surface())
                        .p_3()
                        .text_sm()
                        .text_center()
                        .text_color(rgb(danger()))
                        .cursor(if leaving {
                            CursorStyle::Arrow
                        } else {
                            CursorStyle::PointingHand
                        })
                        .when(!leaving, |button| {
                            button.hover(|style| style.bg(subtle_surface())).on_click(
                                cx.listener(|this, _, _, cx| this.leave_current_channel(cx)),
                            )
                        })
                        .child(if leaving {
                            "Leaving…"
                        } else {
                            "Leave Channel"
                        }),
                )
            })
            .into_any_element();
    pane_with_header(pane_header("Details", None, "details-pane-close", cx), body)
}
