//! Remember only window expansion owned by opening a pane. Explicit user
//! resizing is respected rather than being undone when the pane closes.
#[derive(Default)]
pub(super) struct PaneWindowSizing {
    original_width: Option<f32>,
    expanded_width: Option<f32>,
    pane_width: Option<f32>,
    pending_open: bool,
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
                let expanded = self.expanded_width.take().unwrap_or(original);
                // Keep a deliberate manual resize. An unchanged expansion (or
                // a close before the native resize completes) restores exactly.
                if (width - expanded).abs() <= 1.0 || (width - original).abs() <= 1.0 {
                    Some(original)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn wide_windows_and_manual_resizes_are_left_alone() {
        let mut sizing = PaneWindowSizing::default();
        assert_eq!(sizing.sync(1400., Some(360.), 1124., 1600., true), None);
        assert_eq!(sizing.sync(1400., None, 764., 1600., true), None);
        assert_eq!(
            sizing.sync(780., Some(360.), 1124., 1600., true),
            Some(1140.)
        );
        assert_eq!(sizing.sync(1300., None, 764., 1600., true), None);
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
