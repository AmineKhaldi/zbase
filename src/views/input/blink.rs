//! Caret blinking: solid while the user types or moves the cursor, then on and
//! off like a native field. The timer runs only while the field is focused in
//! the active window, so idle fields cause no redraws.
use super::TextField;
use gpui_kit::{AnyWindowHandle, Context};
use std::{ops::Range, time::Duration};

/// The macOS caret's half-period.
const BLINK_INTERVAL: Duration = Duration::from_millis(530);

pub(super) struct CaretBlink {
    visible: bool,
    running: bool,
    /// Bumped to retire a running timer.
    epoch: usize,
    task: Option<gpui_kit::Task<()>>,
    /// Selection and text length at the last paint; a change restarts the blink solid.
    last_state: Option<(Range<usize>, usize)>,
}

impl Default for CaretBlink {
    fn default() -> Self {
        Self {
            visible: true,
            running: false,
            epoch: 0,
            task: None,
            last_state: None,
        }
    }
}

impl TextField {
    /// Called while painting. Keeps the blink running while `active` (focused in
    /// the active window), restarting it solid after any edit or cursor move, and
    /// returns whether the caret shows this frame.
    pub(super) fn sync_caret_blink(
        &mut self,
        active: bool,
        window_handle: AnyWindowHandle,
        cx: &mut Context<Self>,
    ) -> bool {
        if !active {
            if self.blink.running {
                self.blink.epoch = self.blink.epoch.wrapping_add(1);
                self.blink.running = false;
                self.blink.task = None;
            }
            self.blink.visible = true;
            self.blink.last_state = None;
            return false;
        }
        let state = (self.selected_range.clone(), self.content.len());
        if !self.blink.running || self.blink.last_state.as_ref() != Some(&state) {
            self.blink.last_state = Some(state);
            self.blink.visible = true;
            self.start_caret_blink(window_handle, cx);
        }
        self.blink.visible
    }

    fn start_caret_blink(&mut self, window_handle: AnyWindowHandle, cx: &mut Context<Self>) {
        self.blink.epoch = self.blink.epoch.wrapping_add(1);
        self.blink.running = true;
        let epoch = self.blink.epoch;
        let focus = self.focus_handle.clone();
        self.blink.task = Some(cx.spawn(
            move |this: gpui_kit::WeakEntity<Self>, cx: &mut gpui_kit::AsyncApp| {
                let mut cx = cx.clone();
                let background = cx.background_executor().clone();
                async move {
                    loop {
                        background.timer(BLINK_INTERVAL).await;
                        let active = window_handle
                            .update(&mut cx, |_, window, _| {
                                window.is_window_active() && focus.is_focused(window)
                            })
                            .unwrap_or(false);
                        let keep_running = this
                            .update(&mut cx, |input, cx| {
                                if input.blink.epoch != epoch {
                                    return false;
                                }
                                if !active {
                                    input.blink.running = false;
                                    input.blink.visible = true;
                                    return false;
                                }
                                input.blink.visible = !input.blink.visible;
                                cx.notify();
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
