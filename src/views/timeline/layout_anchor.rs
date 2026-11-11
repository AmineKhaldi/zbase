use gpui_kit::{ListState, Pixels, px};

pub(crate) fn bottom_distance(state: &ListState) -> Pixels {
    (state.max_offset_for_scrollbar().y - state.scroll_px_offset_for_scrollbar().y.abs())
        .max(px(0.))
}

/// Decide before the list remeasures its rows, using the previous layout's
/// bottom distance. Content (previews, images, highlighting) can grow even when
/// the viewport stays the same size. Only the exact end is pinned: a proximity
/// threshold here would undo slow upward scrolls on every frame.
pub(crate) fn should_follow_latest(
    previous_height: Pixels,
    bottom_distance: Pixels,
    explicit_scroll_pending: bool,
) -> bool {
    previous_height > px(0.) && bottom_distance <= px(0.) && !explicit_scroll_pending
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_content_growth_without_requiring_a_viewport_resize() {
        // Preview arrival, image decoding, or code highlighting can change row
        // heights across several layouts while the viewport remains 700px tall.
        assert!(should_follow_latest(px(700.), px(0.), false));
        // Composer growth and collapse use the same policy.
        assert!(should_follow_latest(px(540.), px(0.), false));
        assert!(!should_follow_latest(px(0.), px(0.), false));
    }

    #[test]
    fn preserves_reading_and_explicit_navigation_positions() {
        assert!(!should_follow_latest(px(700.), px(0.25), false));
        assert!(!should_follow_latest(px(700.), px(1.), false));
        assert!(!should_follow_latest(px(700.), px(25.), false));
        assert!(!should_follow_latest(px(540.), px(25.), false));
        assert!(!should_follow_latest(px(700.), px(0.), true));
        assert!(!should_follow_latest(px(540.), px(25.), true));
    }
}
