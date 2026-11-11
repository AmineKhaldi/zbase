use super::{
    DragPreview, DraggedSection, LEAD_GAP_PX, ROW_AVATAR_SIZE_PX, ROW_INSET_PX, RowContext,
    SidebarHost, disclosure, team_asset,
};
use crate::{
    domain::{conversation::ConversationGroup, ids::SidebarSectionId},
    views::{accent, avatar::Avatar, sidebar_hover_bg, text_primary},
};
use gpui_kit::{
    AnyElement, AppContext, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div, px, rgb,
};

const TEAM_TEXT_PX: f32 = 14.;
pub(super) const TEAM_HEADER_PX: f32 = 28.;

pub(super) struct TeamHeader<'a> {
    pub team: &'a ConversationGroup,
    pub section_id: &'a SidebarSectionId,
    pub collapsed: bool,
    pub context: &'a RowContext<'a>,
}

impl TeamHeader<'_> {
    pub fn render<H: SidebarHost>(&self, cx: &mut Context<H>) -> AnyElement {
        let collapsed = self.collapsed;
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
            .h(px(TEAM_HEADER_PX))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(px(LEAD_GAP_PX))
            .px(px(ROW_INSET_PX))
            .rounded(px(6.))
            .text_size(px(TEAM_TEXT_PX))
            .text_color(rgb(text_primary()))
            .cursor_pointer()
            .hover(|style| style.bg(rgb(sidebar_hover_bg())))
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
            .child(
                div()
                    .w(px(ROW_AVATAR_SIZE_PX))
                    .flex_shrink_0()
                    .flex()
                    .justify_center()
                    .child(Avatar::render_team(
                        &self.team.display_name,
                        team_asset(self.context.avatars, &self.team.id),
                        ROW_AVATAR_SIZE_PX,
                    )),
            )
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
