use gpui::{Pixels, px};

/// Decide before the list remeasures its rows, using the previous layout's
/// bottom distance. Content (previews, images, highlighting) can grow even when
/// the viewport stays the same size. Reading history and explicit jumps win.
pub(crate) fn should_follow_latest(
    previous_height: Pixels,
    was_near_bottom: bool,
    explicit_scroll_pending: bool,
) -> bool {
    previous_height > px(0.) && was_near_bottom && !explicit_scroll_pending
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_content_growth_without_requiring_a_viewport_resize() {
        // Preview arrival, image decoding, or code highlighting can change row
        // heights across several layouts while the viewport remains 700px tall.
        assert!(should_follow_latest(px(700.), true, false));
        // Composer growth and collapse use the same policy.
        assert!(should_follow_latest(px(540.), true, false));
        assert!(!should_follow_latest(px(0.), true, false));
    }

    #[test]
    fn preserves_reading_and_explicit_navigation_positions() {
        assert!(!should_follow_latest(px(700.), false, false));
        assert!(!should_follow_latest(px(540.), false, false));
        assert!(!should_follow_latest(px(700.), true, true));
        assert!(!should_follow_latest(px(540.), false, true));
    }
}
