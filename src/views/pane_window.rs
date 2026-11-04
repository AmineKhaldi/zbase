//! Remember only window expansion owned by opening a pane. Explicit user
//! resizing updates the base chat width independently of temporary pane space.
#[derive(Default)]
pub(super) struct PaneWindowSizing {
    original_width: Option<f32>,
    expanded_width: Option<f32>,
    pane_width: Option<f32>,
    pending_open: bool,
    pending_resize: Option<(f32, f32)>,
    resize_acknowledged: bool,
}

impl PaneWindowSizing {
    pub fn sync(
        &mut self,
        width: f32,
        pane: Option<f32>,
        required: f32,
        maximum: f32,
        resizable: bool,
    ) -> Option<f32> {
        self.resize_acknowledged = false;
        if let Some((start, target)) = self.pending_resize {
            if (width - target).abs() <= 1.0 || (width - start).abs() > 1.0 {
                self.resize_acknowledged = (width - target).abs() <= 1.0;
                self.pending_resize = None;
            }
        }
        if resizable && self.pending_resize.is_none() && !self.resize_acknowledged {
            if let (Some(base), Some(expanded)) = (self.original_width, self.expanded_width) {
                // A manual resize changes the base by the same amount. Keep
                // only the actual app-added delta, including screen caps.
                self.original_width =
                    Some((base + width - expanded).max(crate::views::WINDOW_MIN_WIDTH_PX));
                self.expanded_width = Some(width);
            }
        }
        let result = self.sync_size(width, pane, required, maximum, resizable);
        if let Some(target) = result {
            self.pending_resize = ((target - width).abs() > 1.0).then_some((width, target));
        }
        result
    }

    pub fn automatic_resize(&self) -> bool {
        self.pending_resize.is_some() || self.resize_acknowledged
    }

    pub fn base_width(&self, current: f32) -> f32 {
        self.original_width
            .or_else(|| self.closing_width())
            .unwrap_or(current)
    }

    /// Keep the current chat intact until the native content-size callback.
    pub fn expanding(&self) -> bool {
        self.pending_resize
            .is_some_and(|(start, target)| target > start)
    }

    pub fn closing_width(&self) -> Option<f32> {
        self.pending_resize
            .filter(|(start, target)| target < start)
            .map(|(_, target)| target)
    }

    fn sync_size(
        &mut self,
        width: f32,
        pane: Option<f32>,
        required: f32,
        maximum: f32,
        resizable: bool,
    ) -> Option<f32> {
        let opening = pane.is_some() && (self.pane_width.is_none() || self.pending_open);
        let pane_changed = pane != self.pane_width;
        self.pane_width = pane;
        if !resizable {
            self.pending_open = opening && pane.is_some();
            return None;
        }
        self.pending_open = false;
        match pane {
            Some(pane_width) if opening || (pane_changed && self.original_width.is_some()) => {
                if opening && width >= required {
                    return None;
                }
                let original = *self.original_width.get_or_insert(width);
                let expanded = (original + pane_width)
                    .max(required)
                    .min(maximum.max(original));
                self.expanded_width = Some(expanded);
                ((expanded - width).abs() > 1.0).then_some(expanded)
            }
            None => {
                let original = self.original_width.take()?;
                self.expanded_width = None;
                Some(original)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_resize_transition_retains_chat_layout_until_size_is_observed() {
        let mut sizing = PaneWindowSizing::default();
        sizing.sync(780., Some(360.), 1124., 1600., true);
        assert!(sizing.expanding());
        sizing.sync(780., Some(360.), 1124., 1600., true);
        assert!(
            sizing.expanding(),
            "extra renders must not reveal an overlay"
        );
        sizing.sync(1140., Some(360.), 1124., 1600., true);
        assert!(!sizing.expanding());
        sizing.sync(1140., None, 764., 1600., true);
        assert_eq!(sizing.closing_width(), Some(780.));
        sizing.sync(1140., None, 764., 1600., true);
        assert_eq!(sizing.closing_width(), Some(780.));
        sizing.sync(780., None, 764., 1600., true);
        assert_eq!(sizing.closing_width(), None);
    }

    #[test]
    fn narrow_window_expands_once_and_restores_after_switching_pane_content() {
        let mut sizing = PaneWindowSizing::default();
        assert_eq!(
            sizing.sync(780., Some(360.), 1124., 1600., true),
            Some(1140.)
        );
        assert_eq!(sizing.sync(1140., Some(360.), 1124., 1600., true), None);
        assert_eq!(sizing.sync(1140., Some(360.), 1124., 1600., true), None);
        assert_eq!(sizing.sync(1140., None, 764., 1600., true), Some(780.));
        assert_eq!(sizing.sync(780., None, 764., 1600., true), None);
    }

    #[test]
    fn wide_windows_use_existing_space_and_manual_resize_updates_the_base() {
        let mut sizing = PaneWindowSizing::default();
        assert_eq!(sizing.sync(1400., Some(360.), 1124., 1600., true), None);
        assert_eq!(sizing.sync(1400., None, 764., 1600., true), None);
        assert_eq!(
            sizing.sync(780., Some(360.), 1124., 1600., true),
            Some(1140.)
        );
        assert_eq!(sizing.sync(1300., Some(360.), 1124., 1600., true), None);
        assert_eq!(sizing.base_width(1300.), 940.);
        assert_eq!(sizing.sync(1300., None, 764., 1600., true), Some(940.));
    }

    #[test]
    fn pane_resize_uses_the_original_chat_width_and_expansion_is_screen_bounded() {
        let mut sizing = PaneWindowSizing::default();
        sizing.sync(780., Some(360.), 1124., 1600., true);
        assert_eq!(
            sizing.sync(1140., Some(520.), 1284., 1600., true),
            Some(1300.)
        );
        assert_eq!(sizing.sync(1300., None, 764., 1600., true), Some(780.));
        assert_eq!(
            sizing.sync(780., Some(360.), 1124., 1000., true),
            Some(1000.)
        );
        assert_eq!(sizing.sync(1000., None, 764., 1000., true), Some(780.));
    }

    #[test]
    fn saved_width_excludes_only_app_added_space_even_with_a_screen_cap() {
        let mut sizing = PaneWindowSizing::default();
        sizing.sync(780., Some(360.), 1124., 1000., true);
        assert_eq!(sizing.base_width(780.), 780.);
        sizing.sync(1000., Some(360.), 1124., 1000., true);
        assert_eq!(sizing.base_width(1000.), 780.);
        sizing.sync(1200., Some(360.), 1124., 1600., true);
        assert_eq!(
            sizing.base_width(1200.),
            980.,
            "exclude the actual 220-point expansion"
        );
        assert_eq!(sizing.sync(1200., None, 764., 1600., true), Some(980.));
        assert_eq!(
            sizing.base_width(1200.),
            980.,
            "save the base even before collapse finishes"
        );
        sizing.sync(980., None, 764., 1600., true);
        assert_eq!(sizing.base_width(980.), 980.);
        sizing.sync(1400., Some(360.), 1124., 1600., true);
        assert_eq!(
            sizing.base_width(1400.),
            1400.,
            "a pane borrowing existing space adds nothing"
        );
    }

    #[test]
    fn fullscreen_defers_native_changes() {
        let mut sizing = PaneWindowSizing::default();
        assert_eq!(sizing.sync(1400., Some(360.), 1124., 1600., false), None);
        assert_eq!(
            sizing.sync(780., Some(360.), 1124., 1600., true),
            Some(1140.)
        );
        assert_eq!(sizing.sync(1600., None, 764., 1600., false), None);
        assert_eq!(sizing.sync(1140., None, 764., 1600., true), Some(780.));
    }
}
