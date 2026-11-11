//! Shared typography. Text and leading use rems; windows apply the user's scale.
//! Pixel dimensions of images, icons and windows remain independent.
use std::cell::Cell;

use gpui_kit::{Rems, rems};

use crate::models::settings_model::TextSize;

pub fn font_px(default_px: f32) -> Rems {
    rems(default_px / 16.)
}

pub fn label() -> Rems {
    font_px(13.)
}

pub fn code() -> Rems {
    font_px(12.5)
}

pub fn inline_code() -> Rems {
    font_px(12.)
}

pub fn caption() -> Rems {
    font_px(12.)
}
pub fn body() -> Rems {
    font_px(14.)
}
pub fn large_body() -> Rems {
    font_px(16.)
}
pub fn heading() -> Rems {
    font_px(18.)
}

thread_local! {
    static SIDEBAR_TEXT_SIZE: Cell<TextSize> = const { Cell::new(TextSize::BASE) };
}

/// Sidebar fitting needs resolved pixels as well as rem-based text styles.
/// Scope this to its render, including the grouped pane's height calculations.
pub fn with_sidebar_size<T>(size: TextSize, render: impl FnOnce() -> T) -> T {
    struct Restore(TextSize);
    impl Drop for Restore {
        fn drop(&mut self) {
            SIDEBAR_TEXT_SIZE.set(self.0);
        }
    }
    let _restore = Restore(SIDEBAR_TEXT_SIZE.replace(size));
    render()
}

pub fn sidebar_text_size() -> TextSize {
    SIDEBAR_TEXT_SIZE.get()
}

pub fn sidebar_px(default_px: f32) -> f32 {
    SIDEBAR_TEXT_SIZE.get().scale() * default_px
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidebar_measurements_follow_text_size_and_restore_scope() {
        assert_eq!(sidebar_px(30.), 30.);
        with_sidebar_size(TextSize::from_percent(200).unwrap(), || {
            assert_eq!(sidebar_px(30.), 60.);
            with_sidebar_size(TextSize::from_percent(100).unwrap(), || {
                assert_eq!(sidebar_px(30.), 30.)
            });
            assert_eq!(sidebar_px(30.), 60.);
        });
        assert_eq!(sidebar_px(30.), 30.);
    }
}
