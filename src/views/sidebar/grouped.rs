use super::{
    DragPreview, DraggedSection, GroupedSplit, ROW_GAP_PX, ROW_INSET_PX, RowContext, RowLook,
    SCROLLBAR_GUTTER_PX, SECTION_HEADER_PX, SIDEBAR_PADDING_PX, Sidebar, SidebarHandles,
    SidebarHost, disclosure, scroll_viewport,
    team_group::{TEAM_HEADER_PX, TeamHeader},
};
use crate::views::typography;
use crate::{
    domain::conversation::ConversationGroup,
    models::sidebar_model::{SidebarModel, SidebarRow, SidebarSection, TeamsItem},
    views::{
        accent, accent_strong, scroll_edge_fade, scroll_edge_fade_bottom, sidebar_card_shadow,
        sidebar_hover_bg, sidebar_splitter_bg, sidebar_surface, text_secondary, tint,
    },
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyElement, App, AppContext, Bounds, Canvas, ClickEvent, Context, CursorStyle, EntityId,
    FontWeight, InteractiveElement, IntoElement, MouseButton, MouseUpEvent, ParentElement, Pixels,
    Render, ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Window, canvas, div,
    px, relative, rgb, svg,
};
use std::rc::Rc;

const PANE_TOP_PX: f32 = 6.;
const PANE_BOTTOM_PX: f32 = 8.;
const CONVERSATIONS_SHARE: f32 = 1. / 3.;
const CONVERSATIONS_MAX_SHARE: f32 = 1. / 2.;
const CONVERSATIONS_MIN_ROWS: f32 = 4.;
const COMPACT_PEEK_PX: f32 = 24.;
const PREVIEW_PEEK_PX: f32 = 30.;
const TEAM_GAP_PX: f32 = 8.;
const SPLITTER_PX: f32 = 9.;
const TEAMS_MIN_PX: f32 = 96.;
const UNREAD_PILL_PX: f32 = 20.;

pub(super) fn render<H: SidebarHost>(
    sidebar: &SidebarModel,
    context: &RowContext<'_>,
    handles: &SidebarHandles,
    reduced_motion: bool,
    cx: &mut Context<H>,
) -> AnyElement {
    let split = &handles.split;
    let current = context.current_route;
    let conversations = sidebar
        .sections
        .iter()
        .find(|section| section.is_conversations());
    let has_teams = sidebar.teams_items(current).next().is_some();
    let fit = match (has_teams, split.height.get()) {
        (false, _) => PaneFit::Whole,
        (true, None) => PaneFit::Unmeasured,
        (true, Some(list)) => PaneFit::Beside(PaneRoom {
            list,
            teams: SPLITTER_PX
                + PANE_TOP_PX
                + list_height(
                    sidebar
                        .teams_items(current)
                        .enumerate()
                        .map(|(index, item)| teams_item_px(index, item)),
                ),
            share: split.share.get(),
        }),
    };
    let measure = {
        let split = split.clone();
        rerender_after_layout(cx.entity_id(), move |bounds| {
            let measured = Some(f32::from(bounds.size.height));
            split.height.replace(measured) != measured
        })
        .absolute()
        .size_full()
    };
    div()
        .relative()
        .flex_1()
        .min_h(px(0.))
        .flex()
        .flex_col()
        .child(measure)
        .when_some(conversations, |body, section| {
            body.child(conversations_pane(
                section,
                context,
                &handles.conversations,
                fit,
                cx,
            ))
        })
        .when(conversations.is_some() && has_teams, |body| {
            body.child(splitter(split, cx))
                .on_drag_move::<DraggedSplitter>({
                    let split = split.clone();
                    let view = cx.entity_id();
                    move |event, _, cx| {
                        let Some(list) = split.height.get() else {
                            return;
                        };
                        let pointer = f32::from(event.event.position.y - event.bounds.top());
                        split.share.set(Some(splitter_share(pointer, list)));
                        cx.notify(view);
                    }
                })
                .on_mouse_up(MouseButton::Left, save_after_drag(split, cx))
                .on_mouse_up_out(MouseButton::Left, save_after_drag(split, cx))
        })
        .when(has_teams, |body| {
            body.child(teams_pane(
                sidebar,
                context,
                &handles.teams,
                reduced_motion,
                cx,
            ))
        })
        .into_any_element()
}

/// Saves the splitter's position once a drag of it ends. A drop handler
/// would miss a release outside the list; GPUI ends the drag only after
/// mouse-up listeners run.
fn save_after_drag<H: SidebarHost>(
    split: &Rc<GroupedSplit>,
    cx: &mut Context<H>,
) -> impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static {
    let split = split.clone();
    cx.listener(move |this, _: &MouseUpEvent, _, cx| {
        if cx.has_active_drag() {
            this.sidebar_save_conversations_share(split.share.get(), cx);
        }
    })
}

/// How the Conversations pane is sized.
#[derive(Clone, Copy)]
enum PaneFit {
    /// No teams below: it takes the whole list.
    Whole,
    /// Teams below, before the list's first layout.
    Unmeasured,
    Beside(PaneRoom),
}

#[derive(Clone, Copy)]
struct PaneRoom {
    list: f32,
    teams: f32,
    share: Option<f32>,
}

#[derive(Clone, Copy)]
struct DraggedSplitter;

struct SplitterGhost;

impl Render for SplitterGhost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

fn splitter<H: SidebarHost>(split: &Rc<GroupedSplit>, cx: &mut Context<H>) -> AnyElement {
    let split = split.clone();
    div()
        .id("sidebar-splitter")
        .group("sidebar-splitter")
        .flex_shrink_0()
        .mx(px(-SIDEBAR_PADDING_PX))
        .h(px(SPLITTER_PX))
        .bg(rgb(sidebar_splitter_bg()))
        .border_t_1()
        .border_b_1()
        .border_color(rgb(crate::views::border()))
        .flex()
        .items_center()
        .justify_center()
        .cursor(CursorStyle::ResizeRow)
        .on_drag(DraggedSplitter, |_, _, _, cx| cx.new(|_| SplitterGhost))
        .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
            if event.click_count() >= 2 {
                split.share.set(None);
                cx.notify();
                this.sidebar_save_conversations_share(None, cx);
            }
        }))
        .child(
            div()
                .w(px(28.))
                .h(px(3.))
                .rounded_full()
                .bg(tint(text_secondary(), 0.35))
                .group_hover("sidebar-splitter", |grip| {
                    grip.bg(tint(text_secondary(), 0.8))
                }),
        )
        .into_any_element()
}

fn splitter_share(pointer: f32, list: f32) -> f32 {
    ((pointer - SPLITTER_PX / 2.) / list).clamp(0., 1.)
}

/// The height of a scroll area's list holding items of these heights.
fn list_height(items: impl IntoIterator<Item = f32>) -> f32 {
    let (sum, count) = items
        .into_iter()
        .fold((0., 0.), |(sum, count), height| (sum + height, count + 1.));
    ROW_GAP_PX + sum + (count - 1_f32).max(0.) * ROW_GAP_PX + PANE_BOTTOM_PX
}

fn conversations_pane<H: SidebarHost>(
    section: &SidebarSection,
    context: &RowContext<'_>,
    scroll_handle: &ScrollHandle,
    fit: PaneFit,
    cx: &mut Context<H>,
) -> AnyElement {
    let pane = div().flex().flex_col().pt(px(PANE_TOP_PX));
    let row_px = context.look.row_px();
    let content_px = PANE_TOP_PX + list_height(section.rows.iter().map(|_| row_px));
    let peek_px = match context.look {
        RowLook::Compact | RowLook::Channel => COMPACT_PEEK_PX,
        RowLook::Previews { .. } => PREVIEW_PEEK_PX,
    };
    let rows = section
        .rows
        .iter()
        .map(|row| ListItem {
            element: Sidebar::render_row(&section.id, row, context, cx),
            unread: row_wants_attention(row, context),
        })
        .collect();
    pane.map(|pane| match fit {
        PaneFit::Whole => pane.flex_1().min_h(px(0.)),
        PaneFit::Unmeasured => pane
            .flex_shrink_0()
            .h(px(content_px))
            .max_h(relative(CONVERSATIONS_MAX_SHARE)),
        PaneFit::Beside(room) => pane
            .flex_shrink_0()
            .h(px(pane_height(content_px, row_px, peek_px, room))),
    })
    .child(scroll_area(
        "sidebar-conversations",
        scroll_handle,
        rows,
        false,
        cx,
    ))
    .into_any_element()
}

fn pane_height(content_px: f32, row_px: f32, peek: f32, room: PaneRoom) -> f32 {
    let chrome_px = PANE_TOP_PX + ROW_GAP_PX;
    let pitch = row_px + ROW_GAP_PX;
    if let Some(share) = room.share {
        let most = (room.list - SPLITTER_PX - TEAMS_MIN_PX).max(chrome_px + pitch);
        return (room.list * share)
            .clamp(chrome_px + pitch, most)
            .min(content_px);
    }
    let floor = chrome_px + CONVERSATIONS_MIN_ROWS * pitch + peek;
    let share = (room.list * CONVERSATIONS_SHARE)
        .max(floor)
        .min(room.list * CONVERSATIONS_MAX_SHARE);
    let cap = share.max(room.list - room.teams);
    if content_px <= cap {
        return content_px;
    }
    let whole_rows = ((cap - chrome_px - peek) / pitch).floor().max(0.);
    chrome_px + whole_rows * pitch + peek
}

fn team_of(section: &SidebarSection) -> Option<&ConversationGroup> {
    section
        .rows
        .iter()
        .find_map(|row| row.team.as_ref())
        .filter(|_| section.is_team())
}

fn teams_pane<H: SidebarHost>(
    sidebar: &SidebarModel,
    context: &RowContext<'_>,
    scroll_handle: &ScrollHandle,
    reduced_motion: bool,
    cx: &mut Context<H>,
) -> AnyElement {
    let context = RowContext {
        look: RowLook::Channel,
        ..*context
    };
    let items = sidebar
        .teams_items(context.current_route)
        .enumerate()
        .map(|(index, item)| match item {
            TeamsItem::Header(section) => ListItem {
                element: div()
                    .pt(px(header_gap(index)))
                    .child(header(section, &context, cx))
                    .into_any_element(),
                unread: false,
            },
            TeamsItem::Row(section, row) => ListItem {
                element: Sidebar::render_row(&section.id, row, &context, cx),
                unread: row_wants_attention(row, &context),
            },
        })
        .collect();
    div()
        .flex_1()
        .min_h(px(0.))
        .flex()
        .flex_col()
        .pt(px(PANE_TOP_PX))
        .child(scroll_area(
            "sidebar-teams",
            scroll_handle,
            items,
            !reduced_motion,
            cx,
        ))
        .into_any_element()
}

/// The height of the teams list's item at `index`.
fn teams_item_px(index: usize, item: TeamsItem<'_>) -> f32 {
    match item {
        TeamsItem::Header(section) => {
            header_gap(index)
                + match team_of(section) {
                    Some(_) => typography::sidebar_px(TEAM_HEADER_PX),
                    None => typography::sidebar_px(SECTION_HEADER_PX),
                }
        }
        TeamsItem::Row(..) => RowLook::Channel.row_px(),
    }
}

/// Space above a section's header, setting it apart from the rows before it.
fn header_gap(index: usize) -> f32 {
    if index == 0 {
        0.
    } else {
        TEAM_GAP_PX - ROW_GAP_PX
    }
}

fn header<H: SidebarHost>(
    section: &SidebarSection,
    context: &RowContext<'_>,
    cx: &mut Context<H>,
) -> AnyElement {
    match team_of(section) {
        Some(team) => TeamHeader {
            team,
            section_id: &section.id,
            collapsed: section.collapsed,
            context,
        }
        .render(cx),
        None => section_header(section, cx),
    }
}

/// One entry of a scroll area's list. Unread entries scrolled out of view
/// earn an Unread pill.
struct ListItem {
    element: AnyElement,
    unread: bool,
}

fn section_header<H: SidebarHost>(section: &SidebarSection, cx: &mut Context<H>) -> AnyElement {
    let group = SharedString::from(format!("sidebar-section-{}", section.id.0));
    let section_id = section.id.clone();
    let section_id_for_drop = section.id.clone();
    let drag_title = section.title.clone();
    div()
        .id(group.clone())
        .group(group.clone())
        .h(px(typography::sidebar_px(SECTION_HEADER_PX)))
        .flex()
        .items_center()
        .justify_between()
        .gap_1()
        .text_size(typography::caption())
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(text_secondary()))
        .px(px(ROW_INSET_PX))
        .rounded(px(6.))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(sidebar_hover_bg())))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.sidebar_toggle_section(section_id.clone(), cx);
        }))
        .on_drag(DraggedSection(section.id.clone()), move |_, _, _, cx| {
            cx.new(|_| DragPreview {
                title: drag_title.clone(),
            })
        })
        .drag_over::<DraggedSection>(|style, _, _, _| {
            style.border_t_2().border_color(rgb(accent()))
        })
        .on_drop(cx.listener(move |this, dragged: &DraggedSection, _, cx| {
            this.sidebar_reorder_section(dragged.0.clone(), section_id_for_drop.clone(), cx);
        }))
        .child(section.title.clone())
        .child(disclosure(section.collapsed, group))
        .into_any_element()
}

fn row_wants_attention(row: &SidebarRow, context: &RowContext<'_>) -> bool {
    row.is_unread() && &row.route != context.current_route
}

fn scroll_area<H: SidebarHost>(
    id: &'static str,
    handle: &ScrollHandle,
    items: Vec<ListItem>,
    bounce: bool,
    cx: &mut Context<H>,
) -> AnyElement {
    let (elements, unread): (Vec<_>, Vec<_>) = items
        .into_iter()
        .map(|item| (item.element, item.unread))
        .unzip();
    let extent = ScrollExtent::of(handle, &unread);
    let settle = {
        let handle = handle.clone();
        rerender_after_layout(cx.entity_id(), move |_| {
            ScrollExtent::of(&handle, &unread) != extent
        })
        .absolute()
        .size_0()
    };
    let list = div()
        .id(SharedString::from(format!("{id}-scroll")))
        .flex_1()
        .min_h(px(0.))
        .overflow_y_scroll()
        .scrollbar_width(px(SCROLLBAR_GUTTER_PX))
        .track_scroll(handle)
        .on_drag_move::<DraggedSection>({
            let handle = handle.clone();
            move |event, _, _| Sidebar::auto_scroll_during_drag(event, &handle)
        })
        .pt(px(ROW_GAP_PX))
        .pb(px(PANE_BOTTOM_PX))
        .flex()
        .flex_col()
        .gap(px(ROW_GAP_PX))
        .children(elements);
    div()
        .relative()
        .flex_1()
        .min_h(px(0.))
        .flex()
        .flex_col()
        .child(scroll_viewport(
            SharedString::from(format!("{id}-bounce")),
            handle,
            list,
            !bounce,
        ))
        .when(extent.above && extent.unread_above.is_none(), |area| {
            area.child(scroll_edge_fade(sidebar_surface()))
        })
        .when(extent.below && extent.unread_below.is_none(), |area| {
            area.child(scroll_edge_fade_bottom(sidebar_surface()))
        })
        .when_some(extent.unread_above, |area, index| {
            area.child(unread_pill(id, true, index, handle, cx))
        })
        .when_some(extent.unread_below, |area, index| {
            area.child(unread_pill(id, false, index, handle, cx))
        })
        .child(settle)
        .into_any_element()
}

/// Renders `view` again after layout when `changed` says the layout moved
/// something the last render read.
fn rerender_after_layout(
    view: EntityId,
    changed: impl FnOnce(Bounds<Pixels>) -> bool + 'static,
) -> Canvas<()> {
    canvas(
        move |bounds, _, cx| {
            if changed(bounds) {
                cx.notify(view);
            }
        },
        |_, _, _, _| {},
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ScrollExtent {
    above: bool,
    below: bool,
    unread_above: Option<usize>,
    unread_below: Option<usize>,
}

impl ScrollExtent {
    fn of(handle: &ScrollHandle, unread: &[bool]) -> Self {
        let bounds = handle.bounds();
        let offset = handle.offset().y;
        let max = handle.max_offset().y;
        let top = bounds.top() - offset;
        let bottom = bounds.bottom() - offset;
        let unread_items = || {
            unread
                .iter()
                .enumerate()
                .filter(|(_, unread)| **unread)
                .filter_map(|(index, _)| Some((index, handle.bounds_for_item(index)?)))
        };
        Self {
            above: offset < px(-1.),
            below: max > px(1.) && offset > -max + px(1.),
            unread_above: unread_items()
                .filter(|(_, item)| item.top() < top)
                .map(|(index, _)| index)
                .next_back(),
            unread_below: unread_items()
                .find(|(_, item)| item.bottom() > bottom)
                .map(|(index, _)| index),
        }
    }
}

fn unread_pill<H: SidebarHost>(
    list_id: &'static str,
    up: bool,
    index: usize,
    handle: &ScrollHandle,
    cx: &mut Context<H>,
) -> AnyElement {
    let handle = handle.clone();
    div()
        .id(SharedString::from(format!(
            "{list_id}-unread-{}",
            if up { "up" } else { "down" }
        )))
        .absolute()
        .right(px(SCROLLBAR_GUTTER_PX + ROW_INSET_PX / 2.))
        .when(up, |pill| pill.top(px(4.)))
        .when(!up, |pill| pill.bottom(px(4.)))
        .h(typography::font_px(UNREAD_PILL_PX))
        .pl(px(7.))
        .pr(px(9.))
        .flex()
        .items_center()
        .gap(px(3.))
        .rounded_full()
        .bg(rgb(accent_strong()))
        .shadow(sidebar_card_shadow())
        .text_size(typography::font_px(11.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0xffffff))
        .cursor_pointer()
        .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
            handle.scroll_to_item(index);
            cx.notify();
        }))
        .child(
            svg()
                .path(if up {
                    "assets/icons/chevron-up.svg"
                } else {
                    "assets/icons/chevron-down.svg"
                })
                .size(px(12.))
                .text_color(rgb(0xffffff)),
        )
        .child("Unread")
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::sidebar::preview_row::PREVIEW_ROW_HEIGHT_PX;

    const CHROME: f32 = PANE_TOP_PX + ROW_GAP_PX;
    const ROW: f32 = 30.;
    const PEEK: f32 = 24.;

    fn room(list: f32, teams: f32) -> PaneRoom {
        PaneRoom {
            list,
            teams,
            share: None,
        }
    }

    #[test]
    fn a_dragged_splitter_holds_its_share_within_bounds() {
        let dragged = |share| PaneRoom {
            list: 900.,
            teams: 300.,
            share: Some(share),
        };
        assert_eq!(pane_height(2000., ROW, PEEK, dragged(0.4)), 360.);
        assert_eq!(pane_height(200., ROW, PEEK, dragged(0.4)), 200.);
        let pitch = ROW + ROW_GAP_PX;
        assert_eq!(pane_height(2000., ROW, PEEK, dragged(0.)), CHROME + pitch);
        assert_eq!(
            pane_height(2000., ROW, PEEK, dragged(1.)),
            900. - SPLITTER_PX - TEAMS_MIN_PX
        );
        assert!((splitter_share(364.5, 900.) - 0.4).abs() < 1e-6);
    }

    #[test]
    fn conversations_that_fit_take_only_their_height() {
        assert_eq!(pane_height(200., ROW, PEEK, room(900., 900.)), 200.);
    }

    #[test]
    fn overflowing_conversations_end_on_a_peeking_row() {
        let height = pane_height(2000., ROW, PEEK, room(900., 900.));
        assert!(height <= 300.);
        let pitch = ROW + ROW_GAP_PX;
        let rows = (height - CHROME) / pitch;
        assert!((rows.fract() * pitch - PEEK).abs() < 0.01, "{height}");
    }

    #[test]
    fn conversations_take_what_short_teams_leave() {
        assert_eq!(pane_height(400., ROW, PEEK, room(900., 300.)), 400.);
        let height = pane_height(2000., ROW, PEEK, room(900., 300.));
        assert!(height <= 600. && height > 560., "{height}");
    }

    #[test]
    fn a_short_window_keeps_four_rows_up_to_half_the_list() {
        let (row, peek) = (PREVIEW_ROW_HEIGHT_PX, PREVIEW_PEEK_PX);
        let pitch = row + ROW_GAP_PX;
        let height = pane_height(2000., row, peek, room(700., 700.));
        assert!(height >= CHROME + 4. * pitch, "{height}");
        assert!(height <= 350.);
        assert!(pane_height(2000., row, peek, room(300., 300.)) <= 150.);
    }

    #[test]
    fn list_height_counts_gaps_between_items_only() {
        assert_eq!(list_height([]), ROW_GAP_PX + PANE_BOTTOM_PX);
        assert_eq!(
            list_height([30., 30., 30.]),
            ROW_GAP_PX + 90. + 2. * ROW_GAP_PX + PANE_BOTTOM_PX
        );
    }
}
