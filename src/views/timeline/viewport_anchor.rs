use gpui::{Pixels, px};

/// Decide against the old viewport before the list lays out at its new height.
/// Explicit navigation and reading older messages always take precedence.
pub(crate) fn should_follow_latest(
    previous_height: Pixels,
    next_height: Pixels,
    was_near_bottom: bool,
    explicit_scroll_pending: bool,
) -> bool {
    previous_height > px(0.)
        && (previous_height - next_height).abs() > px(0.5)
        && was_near_bottom
        && !explicit_scroll_pending
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_latest_through_composer_growth_and_collapse() {
        assert!(should_follow_latest(px(700.), px(540.), true, false));
        assert!(should_follow_latest(px(540.), px(700.), true, false));
        assert!(!should_follow_latest(px(700.), px(700.), true, false));
        assert!(!should_follow_latest(px(0.), px(700.), true, false));
    }

    #[test]
    fn preserves_reading_and_explicit_navigation_positions() {
        assert!(!should_follow_latest(px(700.), px(540.), false, false));
        assert!(!should_follow_latest(px(540.), px(700.), false, false));
        assert!(!should_follow_latest(px(700.), px(540.), true, true));
    }
}
