use super::{details::details_card, pane_header, pane_with_header};
use crate::{
    domain::channel_details::TeamChannel,
    state::state::{TeamDetailsState, UiWorkspaceState},
    views::{
        accent, app_window::AppWindow, avatar::Avatar, danger, secondary_button, text_primary,
        text_secondary, tint, typography,
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
                .text_size(typography::heading())
                .text_center()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(text_primary()))
                .child(team.display_name.clone()),
        )
        .child(
            div()
                .text_size(typography::body())
                .text_color(rgb(text_secondary()))
                .child("Team"),
        );
    let channels = details_card()
        .gap_3()
        .child(
            div()
                .text_size(typography::body())
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(text_secondary()))
                .child("Channels"),
        )
        .when(state.loading, |card| {
            card.child(
                div()
                    .text_size(typography::body())
                    .child("Loading channels…"),
            )
        })
        .when_some(state.error.as_ref(), |card, error| {
            card.child(
                div()
                    .text_size(typography::body())
                    .text_color(rgb(danger()))
                    .child(error.clone()),
            )
            .child(
                div()
                    .id("team-channels-retry")
                    .text_size(typography::body())
                    .text_color(rgb(accent()))
                    .cursor(CursorStyle::PointingHand)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.open_team_details(retry_team.clone(), cx);
                    }))
                    .child("Try again"),
            )
        })
        .when(
            !state.loading && state.error.is_none() && state.channels.is_empty(),
            |card| {
                card.child(
                    div()
                        .text_size(typography::body())
                        .child("No channels available."),
                )
            },
        )
        .children(
            state
                .channels
                .iter()
                .enumerate()
                .map(|(index, channel)| channel_row(index, channel, workspace, cx)),
        );
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
    let join_conversation = conversation.clone();
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .id(("team-channel-row", index))
                .min_h(px(40.))
                .px(px(8.))
                .py(px(6.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .gap(px(10.))
                .min_w(px(0.))
                .cursor(CursorStyle::PointingHand)
                .hover(|style| style.bg(tint(text_primary(), 0.05)))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_team_channel(conversation.clone(), joined, window, cx);
                }))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .text_size(typography::label())
                        .text_color(rgb(text_primary()))
                        .child(format!("#{}", channel.conversation.title)),
                )
                .when(!joined, |row| {
                    row.child(
                        secondary_button(
                            ("team-channel-join", index),
                            if joining { "Joining…" } else { "Join" },
                        )
                        .flex_shrink_0()
                        .when(joining, |button| {
                            button.cursor(CursorStyle::Arrow).opacity(0.6)
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            if !joining {
                                this.join_team_channel(join_conversation.clone(), cx);
                            }
                        })),
                    )
                }),
        )
        .when_some(workspace.join_channel_errors.get(&id), |row, error| {
            row.child(
                div()
                    .text_size(typography::body())
                    .text_color(rgb(danger()))
                    .child(error.clone()),
            )
        })
        .into_any_element()
}
