use super::{details::details_card, pane_header, pane_with_header};
use crate::{
    domain::{channel_details::TeamChannel, ids::ChannelId, route::Route},
    state::state::{TeamDetailsState, UiWorkspaceState},
    views::{
        accent, app_window::AppWindow, avatar::Avatar, danger, subtle_surface, text_primary,
        text_secondary,
    },
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyElement, Context, CursorStyle, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, px, rgb,
};

pub(super) fn render_team_panel(
    state: &TeamDetailsState,
    workspace: &UiWorkspaceState,
    avatar: Option<&str>,
    cx: &mut Context<AppWindow>,
) -> AnyElement {
    let Some(team) = state.team.as_ref() else {
        return div().into_any_element();
    };
    let retry_team = team.clone();
    let identity = details_card()
        .items_center()
        .gap_3()
        .child(Avatar::render_team(&team.display_name, avatar, 64.))
        .child(
            div()
                .w_full()
                .text_lg()
                .text_center()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(text_primary()))
                .child(team.display_name.clone()),
        )
        .child(
            div()
                .text_sm()
                .text_color(rgb(text_secondary()))
                .child("Team"),
        );
    let channels = details_card()
        .gap_3()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(text_secondary()))
                .child("Channels"),
        )
        .child(
            div()
                .text_sm()
                .text_color(rgb(text_secondary()))
                .child("Everyone on this team can read its channels. Join a channel to add it to your sidebar."),
        )
        .when(state.loading, |card| {
            card.child(div().text_sm().child("Loading channels…"))
        })
        .when_some(state.error.as_ref(), |card, error| {
            card.child(div().text_sm().text_color(rgb(danger())).child(error.clone()))
                .child(
                    div()
                        .id("team-channels-retry")
                        .text_sm()
                        .text_color(rgb(accent()))
                        .cursor(CursorStyle::PointingHand)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.open_team_details(retry_team.clone(), cx);
                        }))
                        .child("Try again"),
                )
        })
        .when(!state.loading && state.error.is_none() && state.channels.is_empty(), |card| {
            card.child(div().text_sm().child("No channels available."))
        })
        .children(state.channels.iter().enumerate().map(|(index, channel)| {
            channel_row(index, channel, workspace, cx)
        }));
    let body = div()
        .id("team-details-scroll")
        .flex_1()
        .min_h(px(0.))
        .overflow_y_scroll()
        .scrollbar_width(px(8.))
        .p_4()
        .flex()
        .flex_col()
        .gap_3()
        .child(identity)
        .child(channels);
    pane_with_header(
        pane_header("Team details", None, "team-details-close", cx),
        body.into_any_element(),
    )
}

fn channel_row(
    index: usize,
    channel: &TeamChannel,
    workspace: &UiWorkspaceState,
    cx: &mut Context<AppWindow>,
) -> AnyElement {
    let conversation = channel.conversation.clone();
    let id = conversation.id.clone();
    let joining = workspace.joining_channels.contains_key(&id);
    let joined = channel.joined;
    let workspace_id = workspace.active_workspace_id.clone();
    let label = if joining {
        "Joining…"
    } else if joined {
        "Open"
    } else {
        "Join"
    };
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .min_w(px(0.))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .text_sm()
                        .text_color(rgb(text_primary()))
                        .child(format!("#{}", conversation.title)),
                )
                .child(
                    div()
                        .id(("team-channel-action", index))
                        .flex_shrink_0()
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .text_sm()
                        .text_color(rgb(if joining { text_secondary() } else { accent() }))
                        .cursor(if joining {
                            CursorStyle::Arrow
                        } else {
                            CursorStyle::PointingHand
                        })
                        .when(!joining, |button| {
                            button
                                .hover(|style| style.bg(subtle_surface()))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    if joined {
                                        if let Some(workspace_id) = workspace_id.clone() {
                                            this.navigate_to(
                                                Route::Channel {
                                                    workspace_id,
                                                    channel_id: ChannelId::new(
                                                        conversation.id.0.clone(),
                                                    ),
                                                },
                                                window,
                                                cx,
                                            );
                                        }
                                    } else {
                                        this.join_team_channel(conversation.clone(), cx);
                                    }
                                }))
                        })
                        .child(label),
                ),
        )
        .when_some(workspace.join_channel_errors.get(&id), |row, error| {
            row.child(
                div()
                    .text_sm()
                    .text_color(rgb(danger()))
                    .child(error.clone()),
            )
        })
        .into_any_element()
}
