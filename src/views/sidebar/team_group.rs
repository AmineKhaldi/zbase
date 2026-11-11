//! A team's header and its channels in the grouped list.
use super::{
    DragPreview, DraggedSection, RowContext, RowLook, Sidebar, SidebarHost, disclosure, team_asset,
};
use crate::{
    domain::{conversation::ConversationGroup, ids::SidebarSectionId},
    models::sidebar_model::SidebarRow,
    views::{accent, avatar::Avatar, shell_border_strong, subtle_surface, text_secondary},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyElement, AppContext, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div, px, rgb,
};

pub(super) struct TeamGroup<'a> {
    pub team: &'a ConversationGroup,
    pub section_id: &'a SidebarSectionId,
    pub rows: &'a [SidebarRow],
    pub collapsed: bool,
    pub context: &'a RowContext<'a>,
}

impl TeamGroup<'_> {
    pub fn render<H: SidebarHost>(&self, cx: &mut Context<H>) -> AnyElement {
        let collapsed = self.collapsed;
        let rows = self
            .rows
            .iter()
            .map(|row| Sidebar::render_row(self.section_id, row, self.context, cx))
            .collect::<Vec<_>>();
        div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(self.header(collapsed, cx))
            .when(!collapsed, |group| {
                group.child(match self.context.look {
                    // Compact rows nest under a guide, like a Finder outline.
                    RowLook::Compact => div()
                        .ml(px(16.))
                        .pl(px(4.))
                        .border_l_1()
                        .border_color(shell_border_strong())
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .children(rows),
                    // Previews already carry a channel tile; the header groups them,
                    // so they keep the list's single inset and rhythm.
                    RowLook::Previews { .. } => div().flex().flex_col().gap(px(2.)).children(rows),
                })
            })
            .into_any_element()
    }

    fn header<H: SidebarHost>(&self, collapsed: bool, cx: &mut Context<H>) -> AnyElement {
        let group = SharedString::from(format!(
            "sidebar-team-header-{}-{}",
            self.section_id.0, self.team.id
        ));
        let toggle_id = self.section_id.clone();
        let drop_id = self.section_id.clone();
        let drag_id = self.section_id.clone();
        let drag_title = self.team.display_name.clone();
        div()
            .id(group.clone())
            .group(group.clone())
            .h(px(24.))
            .flex()
            .items_center()
            .gap(px(6.))
            .px_2()
            .rounded_md()
            .text_xs()
            .text_color(rgb(text_secondary()))
            .cursor_pointer()
            .hover(|style| style.bg(subtle_surface()))
            .on_click(
                cx.listener(move |this, _, _, cx| {
                    this.sidebar_toggle_section(toggle_id.clone(), cx)
                }),
            )
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
            .child(Avatar::render_team(
                &self.team.display_name,
                team_asset(self.context.avatars, &self.team.id),
                16.,
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
            .child(disclosure(collapsed, group))
            .into_any_element()
    }
}
