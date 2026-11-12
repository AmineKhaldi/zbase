use super::TextField;
use gpui_kit::{Bounds, Context, Pixels, Point, ScrollDelta, ScrollEvent, Window, point, px};
use std::time::Duration;

pub(super) fn resolve_scroll_offset(
    current: Pixels,
    content_height: Pixels,
    visible_height: Pixels,
    cursor_top: Option<Pixels>,
    line_height: Pixels,
    follow_cursor: bool,
) -> Pixels {
    let max_scroll = (content_height - visible_height).max(px(0.));
    let mut scroll = current.min(max_scroll).max(px(0.));
    if follow_cursor && let Some(top) = cursor_top {
        if top < scroll {
            scroll = top;
        } else if top + line_height > scroll + visible_height {
            scroll = top + line_height - visible_height;
        }
    }
    scroll.min(max_scroll).max(px(0.))
}

fn drag_scroll_step(position: Point<Pixels>, bounds: Bounds<Pixels>) -> Pixels {
    let edge = px(16.).min(bounds.size.height / 3.);
    if position.y < bounds.top() + edge {
        ((position.y - bounds.top() - edge) * 0.5).max(px(-36.))
    } else if position.y > bounds.bottom() - edge {
        ((position.y - bounds.bottom() + edge) * 0.5).min(px(36.))
    } else {
        px(0.)
    }
}

impl TextField {
    pub(super) fn stop_selection_drag(&mut self) {
        self.is_selecting = false;
        self.selection_pointer = None;
        self.selection_scroll_task = None;
    }

    pub(super) fn on_scroll(
        &mut self,
        event: &ScrollEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.is_multiline() {
            return;
        }
        let delta = match event.delta {
            ScrollDelta::Pixels(delta) => -delta.y,
            ScrollDelta::Lines(delta) => px(-delta.y * 20.),
        };
        if let Some(bounds) = self.last_bounds {
            self.scroll_y = resolve_scroll_offset(
                self.scroll_y + delta,
                self.last_content_height,
                bounds.size.height,
                None,
                px(0.),
                false,
            );
            self.scroll_to_cursor = false;
            if self.is_selecting
                && let Some(position) = self.selection_pointer
            {
                self.select_to(self.selection_index_for_position(position), cx);
                self.scroll_to_cursor = false;
            }
            cx.notify();
        }
        // A wheel gesture over the editor belongs to it, including at its limits.
        cx.stop_propagation();
    }

    pub(super) fn selection_index_for_position(&self, position: Point<Pixels>) -> usize {
        let position = if self.is_multiline()
            && let Some(bounds) = self.last_bounds
        {
            point(
                position.x,
                position
                    .y
                    .max(bounds.top())
                    .min((bounds.bottom() - px(1.)).max(bounds.top())),
            )
        } else {
            position
        };
        self.index_for_mouse_position(position)
    }

    pub(super) fn scroll_selection_once(&mut self, cx: &mut Context<Self>) {
        let (Some(position), Some(bounds)) = (self.selection_pointer, self.last_bounds) else {
            return;
        };
        let next = resolve_scroll_offset(
            self.scroll_y + drag_scroll_step(position, bounds),
            self.last_content_height,
            bounds.size.height,
            None,
            px(0.),
            false,
        );
        if next != self.scroll_y {
            self.scroll_y = next;
            self.select_to(self.selection_index_for_position(position), cx);
            self.scroll_to_cursor = false;
        }
    }

    pub(super) fn start_selection_scroll(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_multiline() || self.selection_scroll_task.is_some() {
            return;
        }
        let window_handle = window.window_handle();
        let focus = self.focus_handle.clone();
        self.selection_scroll_task = Some(cx.spawn(
            move |this: gpui_kit::WeakEntity<Self>, cx: &mut gpui_kit::AsyncApp| {
                let mut cx = cx.clone();
                let background = cx.background_executor().clone();
                async move {
                    loop {
                        background.timer(Duration::from_millis(30)).await;
                        let focused = window_handle
                            .update(&mut cx, |_, window, _| {
                                window.is_window_active() && focus.is_focused(window)
                            })
                            .unwrap_or(false);
                        let keep_running = this
                            .update(&mut cx, |input, cx| {
                                if !input.is_selecting {
                                    return false;
                                }
                                if !focused {
                                    input.is_selecting = false;
                                    input.selection_pointer = None;
                                    cx.notify();
                                    return false;
                                }
                                input.scroll_selection_once(cx);
                                true
                            })
                            .unwrap_or(false);
                        if !keep_running {
                            break;
                        }
                    }
                }
            },
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::size;

    #[test]
    fn manual_scroll_survives_repaint_until_keyboard_or_edit_reveals_caret() {
        let offset = resolve_scroll_offset(
            px(820.),
            px(1000.),
            px(100.),
            Some(px(978.)),
            px(22.),
            false,
        );
        assert_eq!(offset, px(820.));
        assert_eq!(
            resolve_scroll_offset(offset, px(1000.), px(100.), Some(px(978.)), px(22.), true),
            px(900.)
        );
        assert_eq!(
            resolve_scroll_offset(px(900.), px(50.), px(100.), None, px(22.), false),
            px(0.)
        );
        assert_eq!(
            resolve_scroll_offset(px(-20.), px(1000.), px(100.), None, px(22.), false),
            px(0.)
        );
    }

    #[test]
    fn drag_scroll_has_two_edges_a_dead_zone_and_bounded_speed() {
        let bounds = Bounds::new(point(px(10.), px(100.)), size(px(300.), px(120.)));
        assert!(drag_scroll_step(point(px(50.), px(101.)), bounds) < px(0.));
        assert!(drag_scroll_step(point(px(50.), px(219.)), bounds) > px(0.));
        assert_eq!(drag_scroll_step(point(px(50.), px(160.)), bounds), px(0.));
        assert_eq!(
            drag_scroll_step(point(px(50.), px(-1000.)), bounds),
            px(-36.)
        );
        assert_eq!(drag_scroll_step(point(px(50.), px(1000.)), bounds), px(36.));
    }
}
