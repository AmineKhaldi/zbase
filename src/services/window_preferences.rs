use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Logical window bounds, excluding width temporarily added for a side pane.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowPreferences {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl WindowPreferences {
    pub(super) fn path() -> PathBuf {
        super::local_store::paths::data_root().join("window-preferences.json")
    }

    pub fn load() -> Option<Self> {
        let value: Self = serde_json::from_slice(&fs::read(Self::path()).ok()?).ok()?;
        value.valid().then_some(value)
    }

    fn valid(self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|v| v.is_finite())
            && self.width > 0.
            && self.height > 0.
    }

    /// Choose the display with greatest overlap; disconnected displays fall
    /// back to the first (primary) display. Keep the whole restored window visible.
    pub fn constrained(self, displays: &[Self], minimum: (f32, f32)) -> Self {
        let Some(mut display) = displays.first().copied() else {
            return self;
        };
        let mut best_overlap = 0.;
        for candidate in displays {
            let overlap = ((self.x + self.width).min(candidate.x + candidate.width)
                - self.x.max(candidate.x))
            .max(0.)
                * ((self.y + self.height).min(candidate.y + candidate.height)
                    - self.y.max(candidate.y))
                .max(0.);
            if overlap > best_overlap {
                display = *candidate;
                best_overlap = overlap;
            }
        }
        let width = self.width.max(minimum.0).min(display.width);
        let height = self.height.max(minimum.1).min(display.height);
        Self {
            x: self.x.clamp(display.x, display.x + display.width - width),
            y: self.y.clamp(display.y, display.y + display.height - height),
            width,
            height,
        }
    }

    pub(super) fn save_at(self, path: &Path) -> io::Result<()> {
        if !self.valid() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid window bounds",
            ));
        }
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::other("missing preferences directory"))?;
        fs::create_dir_all(parent)?;
        let temporary = parent.join(format!(
            ".window-preferences-{}-{}.tmp",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let result = (|| {
            use std::io::Write;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(&serde_json::to_vec(&self)?)?;
            drop(file);
            fs::rename(&temporary, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(temporary);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_geometry_is_atomic_and_survives_round_trip() {
        let dir = std::env::temp_dir().join(format!(
            "zbase-window-prefs-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("window.json");
        let value = WindowPreferences {
            x: -900.,
            y: 100.,
            width: 900.,
            height: 850.,
        };
        value.save_at(&path).unwrap();
        assert_eq!(
            serde_json::from_slice::<WindowPreferences>(&fs::read(path).unwrap()).unwrap(),
            value
        );
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(&dir).unwrap();
        assert!(
            !WindowPreferences {
                width: f32::NAN,
                ..value
            }
            .valid()
        );
    }
    #[test]
    fn restore_handles_removed_displays_and_smaller_screens() {
        let screen = WindowPreferences {
            x: 0.,
            y: 30.,
            width: 1200.,
            height: 800.,
        };
        let saved = WindowPreferences {
            x: -1800.,
            y: -200.,
            width: 1500.,
            height: 1000.,
        };
        assert_eq!(saved.constrained(&[screen], (780., 680.)), screen);
        let second = WindowPreferences {
            x: -1200.,
            ..screen
        };
        let saved = WindowPreferences {
            x: -1100.,
            y: 100.,
            width: 800.,
            height: 700.,
        };
        assert_eq!(saved.constrained(&[screen, second], (780., 680.)), saved);
    }
}
