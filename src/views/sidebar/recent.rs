use super::{Sidebar, SidebarHost, SidebarTooltip};
use crate::{
    domain::route::Route,
    models::{
        recent_sidebar::RecentSidebarRow,
        sidebar_model::{SidebarAvatar, SidebarModel, team_avatar_key},
    },
    views::{
        accent, avatar::Avatar, is_dark_theme, subtle_surface, text_primary, text_secondary,
        unread_indicator,
    },
};
use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, Context, FontWeight, HighlightStyle, InteractiveElement, IntoElement,
    ParentElement, SharedString, StatefulInteractiveElement, Styled, StyledText,
    UniformListScrollHandle, div, px, rgb, uniform_list,
};
use std::collections::HashMap;

pub(super) fn render<H: SidebarHost>(
    sidebar: &SidebarModel,
    avatars: &HashMap<String, SidebarAvatar>,
    current: &Route,
    hovered: Option<&str>,
    scroll: &UniformListScrollHandle,
    cx: &mut Context<H>,
) -> AnyElement {
    let filter = sidebar.filter.trim().to_lowercase();
    let rows = sidebar.visible_recent_rows().cloned().collect::<Vec<_>>();
    let theme = crate::views::current_theme();
    let avatars = avatars.clone();
    let current = current.clone();
    let hovered = hovered.map(str::to_owned);
    let highlighted = sidebar.highlighted_route.clone();
    let owner = cx.entity().downgrade();
    if rows.is_empty() {
        return div()
            .flex_1()
            .px_2()
            .py_2()
            .text_xs()
            .text_color(rgb(text_secondary()))
            .child(if filter.is_empty() {
                "No conversations yet"
            } else {
                "No matching conversations"
            })
            .into_any_element();
    }
    uniform_list("sidebar-recent-list", rows.len(), move |range, _, app| {
        owner
            .update(app, |_, cx| {
                crate::views::with_theme(theme, || {
                    rows[range]
                        .iter()
                        .map(|recent| {
                            render_row(
                                recent,
                                &avatars,
                                &current,
                                hovered.as_deref(),
                                highlighted.as_ref(),
                                cx,
                            )
                        })
                        .collect::<Vec<_>>()
                })
            })
            .unwrap_or_default()
    })
    .track_scroll(scroll.clone())
    .flex_1()
    .min_h(px(0.))
    .into_any_element()
}

fn render_row<H: SidebarHost>(
    recent: &RecentSidebarRow,
    avatars: &HashMap<String, SidebarAvatar>,
    current: &Route,
    hovered: Option<&str>,
    highlighted: Option<&Route>,
    cx: &mut Context<H>,
) -> AnyElement {
    let row = &recent.row;
    let selected = &row.route == current;
    let unread = row.unread_count > 0 || row.mention_count > 0;
    let route_key = row.route.label();
    let is_hovered = hovered == Some(route_key.as_str()) || highlighted == Some(&row.route);
    let route = row.route.clone();
    let tooltip = row.full_label();
    let primary = if selected && !is_dark_theme() {
        crate::views::text_inverted()
    } else if selected {
        accent()
    } else {
        text_primary()
    };
    let secondary = if selected && !is_dark_theme() {
        crate::views::text_inverted()
    } else {
        text_secondary()
    };
    let mut label = if recent.is_channel {
        format!("#{}", row.label)
    } else {
        row.label.clone()
    };
    let team_start = label.len();
    let show_team = recent.is_channel && row.team.is_some() && !recent.secondary.is_empty();
    if show_team {
        label.push_str(" · ");
        label.push_str(&recent.secondary);
    }
    let team_end = label.len();
    let label = StyledText::new(label).with_highlights(show_team.then_some((
        team_start..team_end,
        HighlightStyle {
            color: Some(rgb(secondary).into()),
            font_weight: Some(FontWeight::NORMAL),
            ..Default::default()
        },
    )));
    let avatar = if let Some(team) = &row.team {
        let asset = match avatars.get(&team_avatar_key(&team.id)) {
            Some(SidebarAvatar::Team(asset)) => Some(asset.as_str()),
            _ => None,
        };
        Avatar::render_square(
            &team.display_name,
            asset,
            20.,
            crate::views::accent_soft(),
            accent(),
        )
    } else {
        Sidebar::leading_element(row, &route_key, avatars, secondary)
    };
    div()
        .id(SharedString::from(format!("sidebar-recent-{route_key}")))
        .h(px(30.))
        .w_full()
        .px_2()
        .flex()
        .items_center()
        .gap_1p5()
        .rounded_md()
        .cursor_pointer()
        .when(selected && !is_dark_theme(), |row| row.bg(rgb(accent())))
        .when(is_hovered && !selected, |row| row.bg(subtle_surface()))
        .tooltip(SidebarTooltip::builder(tooltip))
        .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
            this.sidebar_set_hovered_row(hovered.then(|| route_key.clone()), cx);
        }))
        .on_click(cx.listener(move |this, _, window, cx| {
            this.sidebar_navigate_to(route.clone(), window, cx)
        }))
        .child(div().w(px(20.)).flex_shrink_0().child(avatar))
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .text_xs()
                .whitespace_normal()
                .text_ellipsis()
                .text_color(rgb(primary))
                .when(unread || selected, |label| {
                    label.font_weight(FontWeight::BOLD)
                })
                .child(label),
        )
        .when(unread, |row| {
            row.child(
                div()
                    .w(px(6.))
                    .h(px(6.))
                    .flex_shrink_0()
                    .rounded_full()
                    .bg(rgb(unread_indicator())),
            )
        })
        .into_any_element()
}
