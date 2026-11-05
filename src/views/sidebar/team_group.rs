//! Shared team header and nested channels for Unread and the full team list.
use super::{DragPreview, DraggedSection, Sidebar, SidebarHost, SidebarTooltip};
use crate::{
    domain::{conversation::ConversationGroup, ids::SidebarSectionId, route::Route},
    models::sidebar_model::{SidebarAvatar, SidebarRow, team_avatar_key},
    views::{
        accent, avatar::Avatar, chevron_down_icon, chevron_right_icon, shell_border_strong,
        subtle_surface, text_secondary,
    },
};
use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, AppContext, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div, px, rgb,
};
use std::collections::HashMap;

pub(super) enum TeamGroupMode {
    Unread,
    Collapsible { collapsed: bool },
}

pub(super) struct TeamGroup<'a> {
    pub team: &'a ConversationGroup,
    pub section_id: &'a SidebarSectionId,
    pub rows: &'a [SidebarRow],
    pub mode: TeamGroupMode,
    pub avatars: &'a HashMap<String, SidebarAvatar>,
    pub current_route: &'a Route,
    pub hovered_row: Option<&'a str>,
}

impl TeamGroup<'_> {
    pub fn render<H: SidebarHost>(&self, cx: &mut Context<H>) -> AnyElement {
        let collapsed = matches!(self.mode, TeamGroupMode::Collapsible { collapsed: true });
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(self.header(cx))
            .when(!collapsed, |group| {
                group.child(
                    div()
                        .ml(px(16.))
                        .pl(px(4.))
                        .border_l_1()
                        .border_color(shell_border_strong())
                        .flex()
                        .flex_col()
                        .gap_1()
                        .children(self.rows.iter().map(|row| {
                            Sidebar::render_row(
                                self.section_id,
                                row,
                                self.avatars,
                                self.current_route,
                                self.hovered_row,
                                cx,
                            )
                        })),
                )
            })
            .into_any_element()
    }

    fn header<H: SidebarHost>(&self, cx: &mut Context<H>) -> AnyElement {
        let asset = match self.avatars.get(&team_avatar_key(&self.team.id)) {
            Some(SidebarAvatar::Team(asset)) => Some(asset.as_str()),
            _ => None,
        };
        let toggle_id = self.section_id.clone();
        let drop_id = self.section_id.clone();
        let drag_id = self.section_id.clone();
        let drag_title = self.team.display_name.clone();
        div()
            .id(SharedString::from(format!(
                "sidebar-team-header-{}-{}",
                self.section_id.0, self.team.id
            )))
            .flex()
            .items_center()
            .gap_1p5()
            .px_2()
            .pt_1()
            .pb_0p5()
            .rounded_md()
            .text_xs()
            .text_color(rgb(text_secondary()))
            .tooltip(SidebarTooltip::builder(self.team.display_name.clone()))
            .when(
                matches!(self.mode, TeamGroupMode::Collapsible { .. }),
                |header| {
                    header
                        .cursor_pointer()
                        .hover(|style| style.bg(subtle_surface()))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.sidebar_toggle_section(toggle_id.clone(), cx)
                        }))
                        .on_drag(DraggedSection(drag_id), move |_, _, _, cx| {
                            cx.new(|_| DragPreview {
                                title: drag_title.clone(),
                            })
                        })
                        .drag_over::<DraggedSection>(|style, _, _, _| {
                            style.border_t_2().border_color(rgb(accent()))
                        })
                        .on_drop(cx.listener(move |this, dragged: &DraggedSection, _, cx| {
                            this.sidebar_reorder_section(dragged.0.clone(), drop_id.clone(), cx)
                        }))
                },
            )
            .child(Avatar::render_square(
                &self.team.display_name,
                asset,
                16.,
                crate::views::accent_soft(),
                accent(),
            ))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .whitespace_normal()
                    .line_clamp(1)
                    .text_ellipsis()
                    .child(self.team.display_name.clone()),
            )
            .when(
                matches!(self.mode, TeamGroupMode::Collapsible { .. }),
                |header| {
                    header.child(
                        if matches!(self.mode, TeamGroupMode::Collapsible { collapsed: true }) {
                            chevron_right_icon(text_secondary())
                        } else {
                            chevron_down_icon(text_secondary())
                        },
                    )
                },
            )
            .into_any_element()
    }
}
