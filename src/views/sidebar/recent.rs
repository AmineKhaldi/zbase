use super::{
    CompactRow, ROW_AVATAR_SIZE_PX, RowActivity, RowColors, RowLook, RowSelection, Sidebar,
    SidebarHost,
    preview_row::{PREVIEW_AVATAR_PX, PreviewRow, title_segments},
    team_asset,
};
use crate::{
    domain::route::Route,
    models::{
        recent_sidebar::{RecentSidebarRow, ambiguous_channel_labels},
        sidebar_model::{SidebarAvatar, SidebarModel},
        sidebar_preview::PreviewLive,
    },
    views::{avatar::Avatar, text_secondary},
};
use gpui::{
    AnyElement, Context, IntoElement, ParentElement, Styled, UniformListScrollHandle, div, px, rgb,
    uniform_list,
};
use std::collections::{HashMap, HashSet};

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
    let ambiguous = ambiguous_channel_labels(&rows);
    let theme = crate::views::current_theme();
    let window_active = crate::views::window_is_active();
    let avatars = avatars.clone();
    let current = current.clone();
    let hovered = hovered.map(str::to_owned);
    let highlighted = sidebar.highlighted_route.clone();
    let look = RowLook::new(sidebar.style, sidebar.width_px);
    let live = sidebar.live_previews.clone();
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
                    crate::views::with_window_active(window_active, || {
                        rows[range]
                            .iter()
                            .map(|recent| {
                                render_row(
                                    recent,
                                    &ambiguous,
                                    &avatars,
                                    RowState {
                                        look,
                                        current: &current,
                                        hovered: hovered.as_deref(),
                                        highlighted: highlighted.as_ref(),
                                        live: &live,
                                    },
                                    cx,
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                })
            })
            .unwrap_or_default()
    })
    .track_scroll(scroll.clone())
    .flex_1()
    .min_h(px(0.))
    .into_any_element()
}

/// What decides a row's look besides the row itself.
struct RowState<'a> {
    look: RowLook,
    current: &'a Route,
    hovered: Option<&'a str>,
    /// Keyboard highlight, drawn like hover.
    highlighted: Option<&'a Route>,
    live: &'a HashMap<String, PreviewLive>,
}

fn render_row<H: SidebarHost>(
    recent: &RecentSidebarRow,
    ambiguous: &HashSet<String>,
    avatars: &HashMap<String, SidebarAvatar>,
    state: RowState<'_>,
    cx: &mut Context<H>,
) -> AnyElement {
    let RowState {
        look,
        current,
        hovered,
        highlighted,
        live,
    } = state;
    let row = &recent.row;
    // Recent lists every conversation once, so its row is always the selection.
    let selection = RowSelection::of(row, current, true);
    let activity = RowActivity::of(row);
    let route_key = row.route.label();
    let row_id = format!("sidebar-recent-{route_key}");
    let is_hovered = hovered == Some(row_id.as_str()) || highlighted == Some(&row.route);
    let colors = RowColors::new(selection.is_full(), is_hovered, row.muted);
    let team_label = recent.team_label(ambiguous).map(str::to_owned);
    let emphasized = activity.is_unread() && !selection.is_full();
    match look {
        RowLook::Previews { compact_time } => {
            // A channel shows its team's logo; a simple team keeps its mark.
            let lead = match &row.team {
                Some(team) => Avatar::render_team(
                    &team.display_name,
                    team_asset(avatars, &team.id),
                    PREVIEW_AVATAR_PX,
                ),
                None => {
                    Sidebar::leading_element(row, &route_key, avatars, &colors, PREVIEW_AVATAR_PX)
                }
            };
            PreviewRow {
                row,
                id: row_id,
                title: title_segments(row.label.clone(), team_label, &colors, emphasized),
                lead,
                colors: &colors,
                selected: selection.is_full(),
                live: row.route.conversation_key().and_then(|key| live.get(key)),
                compact_time,
            }
            .render(cx)
        }
        RowLook::Compact => {
            let label = if recent.is_channel {
                format!("#{}", row.label)
            } else {
                row.label.clone()
            };
            let leading = match &row.team {
                Some(team) => Avatar::render_team(
                    &team.display_name,
                    team_asset(avatars, &team.id),
                    ROW_AVATAR_SIZE_PX,
                ),
                None => {
                    Sidebar::leading_element(row, &route_key, avatars, &colors, ROW_AVATAR_SIZE_PX)
                }
            };
            CompactRow {
                id: row_id,
                row,
                title: title_segments(label, team_label, &colors, emphasized),
                leading,
                colors: &colors,
                selection,
            }
            .render(cx)
        }
    }
}
