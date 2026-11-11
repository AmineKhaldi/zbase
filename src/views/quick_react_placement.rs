//! Shared geometry for the message toolbar in the timeline and thread replies.
//! The strip straddles the hovered message's bottom edge at its right end,
//! half over this message and half over the next, so it appears in the same
//! place on every message; it never depends on where the pointer is. The last message has the composer below it, so its
//! strip moves to the top instead.
use gpui_kit::{Bounds, Point, Size, point, size};

pub(super) const BUTTON_SIZE: f32 = 26.0;
/// Where the window's top chrome ends; the strip stays below it.
const TOP_INSET: f32 = 56.0;
/// Room kept for the composer at the bottom of the window, so a tall message
/// whose bottom is scrolled out of view still shows its strip.
const BOTTOM_INSET: f32 = 96.0;
/// The row's horizontal padding: the message column ends this far from its edge.
const ROW_PADDING: f32 = 16.0;
/// How far the strip stays in from the row's right edge.
const RIGHT_INSET: f32 = 12.0;
/// Gap between the strip and the body when it sits on the author line.
const HEADER_GAP: f32 = 2.0;

/// `own_buttons` counts the buttons only your own messages get (edit, delete).
pub(super) fn toolbar_size(stock_buttons: usize, recent: bool, own_buttons: usize) -> Size<f32> {
    let buttons = stock_buttons + 4 + usize::from(recent) + own_buttons;
    let separators = 1 + usize::from(recent);
    let gaps = buttons + separators - 1;
    size(
        buttons as f32 * BUTTON_SIZE + separators as f32 + gaps as f32 * 2.0 + 6.0,
        BUTTON_SIZE + 6.0,
    )
}

/// The hovered message body's window bounds and the window size.
#[derive(Clone, Copy)]
pub(super) struct HoverGeometry {
    pub container: Bounds<f32>,
    pub viewport: Size<f32>,
    /// The message opens with its author line, above the body.
    pub has_header: bool,
    /// The newest message, right above the composer.
    pub is_last_message: bool,
}

impl HoverGeometry {
    /// The strip's position relative to the message body, or `None` when the
    /// window is too small to show it whole.
    pub fn local_placement(self, toolbar: Size<f32>) -> Option<Point<f32>> {
        let origin = self.container.origin;
        let row_right =
            (origin.x + self.container.size.width + ROW_PADDING).min(self.viewport.width - 8.);
        // A narrow thread column lets the strip reach past its avatar gutter.
        let x = (row_right - RIGHT_INSET - toolbar.width).max(8.);
        let lowest = self.viewport.height - BOTTOM_INSET - toolbar.height;
        if x + toolbar.width > row_right || lowest < TOP_INSET {
            return None;
        }
        let y = if !self.is_last_message {
            (origin.y + self.container.size.height - toolbar.height / 2.).min(lowest)
        } else if self.has_header {
            // On the author line's empty right end.
            origin.y - toolbar.height - HEADER_GAP
        } else {
            origin.y - toolbar.height / 2.
        };
        Some(point(x - origin.x, y.max(TOP_INSET) - origin.y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry(top: f32, height: f32, is_last_message: bool) -> HoverGeometry {
        HoverGeometry {
            container: Bounds::new(point(300., top), size(800., height)),
            viewport: size(1440., 920.),
            has_header: true,
            is_last_message,
        }
    }

    #[test]
    fn strip_straddles_the_bottom_right_corner_of_every_message() {
        let toolbar = toolbar_size(5, true, 2);
        for (top, height) in [(200., 22.), (420., 140.), (610., 44.)] {
            let local = geometry(top, height, false)
                .local_placement(toolbar)
                .expect("room to show");
            assert_eq!(local.x + toolbar.width, 800. + ROW_PADDING - RIGHT_INSET);
            assert_eq!(local.y, height - toolbar.height / 2.);
        }
    }

    #[test]
    fn last_message_puts_the_strip_on_top_away_from_the_composer() {
        let toolbar = toolbar_size(5, true, 1);
        // Close above the composer: the strip still sits right on the message.
        let last = geometry(800., 22., true);
        assert_eq!(
            last.local_placement(toolbar).unwrap().y,
            -toolbar.height - HEADER_GAP
        );
        let follow_up = HoverGeometry {
            has_header: false,
            ..last
        };
        assert_eq!(
            follow_up.local_placement(toolbar).unwrap().y,
            -toolbar.height / 2.
        );
    }

    #[test]
    fn strip_stays_between_the_window_chrome_and_the_composer() {
        let toolbar = toolbar_size(5, true, 1);
        let scrolled_off = geometry(-400., 300., false);
        let global = scrolled_off.container.origin + scrolled_off.local_placement(toolbar).unwrap();
        assert_eq!(global.y, TOP_INSET);
        let tall = geometry(600., 400., false);
        let global = tall.container.origin + tall.local_placement(toolbar).unwrap();
        assert_eq!(global.y + toolbar.height, 920. - BOTTOM_INSET);
    }

    #[test]
    fn narrow_thread_column_keeps_the_strip_right_aligned_in_the_window() {
        let toolbar = toolbar_size(5, true, 1);
        let thread = HoverGeometry {
            container: Bounds::new(point(1160., 300.), size(244., 60.)),
            ..geometry(0., 0., false)
        };
        let global = thread.container.origin + thread.local_placement(toolbar).unwrap();
        assert_eq!(
            global.x + toolbar.width,
            1160. + 244. + ROW_PADDING - RIGHT_INSET
        );
        let cramped = HoverGeometry {
            container: Bounds::new(point(20., 300.), size(120., 60.)),
            viewport: size(200., 920.),
            ..thread
        };
        assert!(cramped.local_placement(toolbar).is_none());
    }
}
