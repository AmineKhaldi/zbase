//! Shared geometry for the timeline, thread replies, and right-click strip.
//! Never clamp a right-hand candidate back across the pointer; choose another
//! side instead. Hidden/unplaceable strips must not contribute any hit boxes.
use gpui::{Bounds, Point, Size, point, size};

pub(super) const BUTTON_SIZE: f32 = 26.0;
const GAP: f32 = 14.0;

pub(super) fn toolbar_size(stock_buttons: usize, recent: bool, own_message: bool) -> Size<f32> {
    let buttons = stock_buttons + 4 + usize::from(recent) + usize::from(own_message);
    let separators = 1 + usize::from(recent);
    let gaps = buttons + separators - 1;
    size(
        buttons as f32 * BUTTON_SIZE + separators as f32 + gaps as f32 * 2.0 + 6.0,
        BUTTON_SIZE + 6.0,
    )
}

#[derive(Clone, Copy)]
pub(super) struct HoverGeometry {
    pub pointer: Point<f32>,
    pub container_origin: Point<f32>,
    pub container_width: f32,
    pub viewport: Size<f32>,
}

impl HoverGeometry {
    pub fn local_placement(self, toolbar: Size<f32>) -> Option<Point<f32>> {
        // Include the avatar gutter so a full strip can fit inside a thread
        // column, while leaving the window/header/composer edges unobstructed.
        let right =
            (self.container_origin.x + self.container_width + 16.).min(self.viewport.width - 8.);
        let left = (self.container_origin.x - 48.)
            .max(8.)
            .min((right - toolbar.width).max(8.));
        let available = Bounds::new(
            point(left, 56.),
            size(
                (right - left).max(0.),
                (self.viewport.height - 128.).max(0.),
            ),
        );
        place_toolbar(self.pointer, available, toolbar).map(|position| {
            point(
                position.x - self.container_origin.x,
                position.y - self.container_origin.y,
            )
        })
    }
}

fn place_toolbar(
    pointer: Point<f32>,
    available: Bounds<f32>,
    toolbar: Size<f32>,
) -> Option<Point<f32>> {
    if available.size.width < toolbar.width || available.size.height < toolbar.height {
        return None;
    }
    let left = available.origin.x;
    let top = available.origin.y;
    let right = left + available.size.width;
    let bottom = top + available.size.height;
    let centered_x = (pointer.x - toolbar.width / 2.0).clamp(left, right - toolbar.width);
    let centered_y = (pointer.y - toolbar.height / 2.0).clamp(top, bottom - toolbar.height);
    let candidates = [
        point(pointer.x + GAP, centered_y),
        point(centered_x, pointer.y - GAP - toolbar.height),
        point(centered_x, pointer.y + GAP),
    ];
    candidates.into_iter().find(|position| {
        let x = position.x;
        let y = position.y;
        x >= left
            && x + toolbar.width <= right
            && y >= top
            && y + toolbar.height <= bottom
            && (x >= pointer.x + GAP
                || x + toolbar.width <= pointer.x - GAP
                || y >= pointer.y + GAP
                || y + toolbar.height <= pointer.y - GAP)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stock_recent_and_own_message_strips_never_cover_the_pointer_at_edges() {
        for (left, width) in [(200., 560.), (1080., 340.)] {
            let area = Bounds::new(point(left, 56.), size(width, 700.));
            for recent in [false, true] {
                for own in [false, true] {
                    let toolbar = toolbar_size(5, recent, own);
                    for x in [left + 1., left + width / 2., left + width - 1.] {
                        for y in [57., 400., 755.] {
                            let pointer = point(x, y);
                            let position =
                                place_toolbar(pointer, area, toolbar).expect("a safe side exists");
                            assert!(
                                position.x >= left && position.x + toolbar.width <= left + width
                            );
                            assert!(position.y >= 56. && position.y + toolbar.height <= 756.);
                            assert!(
                                position.x >= x + GAP
                                    || position.x + toolbar.width <= x - GAP
                                    || position.y >= y + GAP
                                    || position.y + toolbar.height <= y - GAP
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn thread_geometry_translates_safe_window_placement_back_to_message_coordinates() {
        let geometry = HoverGeometry {
            pointer: point(1300., 75.),
            container_origin: point(1160., 60.),
            container_width: 244.,
            viewport: size(1440., 920.),
        };
        let toolbar = toolbar_size(5, true, true);
        let local = geometry.local_placement(toolbar).unwrap();
        let global = geometry.container_origin + local;
        assert!(global.x >= 8. && global.x + toolbar.width <= 1432.);
        assert!(
            global.y > geometry.pointer.y,
            "top-edge hover should use the space below"
        );
    }

    #[test]
    fn constrained_right_edge_uses_above_or_below_instead_of_clamping_under_cursor() {
        let area = Bounds::new(point(0., 0.), size(340., 600.));
        let toolbar = toolbar_size(5, true, true);
        let above = place_toolbar(point(160., 300.), area, toolbar).unwrap();
        assert!(above.y + toolbar.height < 300.);
        let below = place_toolbar(point(160., 5.), area, toolbar).unwrap();
        assert!(below.y > 5.);
        assert!(
            toolbar.width > 260.,
            "recent/delete buttons must be included in placement width"
        );
        assert!(
            place_toolbar(
                point(160., 16.),
                Bounds::new(point(0., 0.), size(340., 32.)),
                toolbar
            )
            .is_none()
        );
    }
}
