//! Explicit save/open actions shared by attachment surfaces.
use super::*;
use crate::domain::attachment::AttachmentSummary;
use gpui::{Animation, AnimationExt, Global, SharedString, Transformation};

#[derive(Clone)]
enum DownloadState {
    Downloading,
    Saved(PathBuf),
    Failed(String),
    Reveal(PathBuf),
}
#[derive(Default)]
struct Downloads(HashMap<String, DownloadState>);
impl Global for Downloads {}

fn source(attachment: &AttachmentSummary) -> Option<String> {
    crate::views::attachment_local_path(attachment)
        .or_else(|| crate::views::attachment_open_target(attachment))
}
fn key(source: &str, name: &str) -> String {
    format!("{source}\n{name}")
}
fn state(source: &str, name: &str, cx: &App) -> Option<DownloadState> {
    cx.try_global::<Downloads>()?
        .0
        .get(&key(source, name))
        .cloned()
        .filter(|s| !matches!(s, DownloadState::Saved(p) | DownloadState::Reveal(p) if !p.exists()))
}

pub(crate) fn control(
    attachment: &AttachmentSummary,
    id: String,
    cx: &mut Context<AppWindow>,
) -> AnyElement {
    let available = source(attachment).is_some();
    let current = source(attachment).and_then(|s| state(&s, &attachment.name, cx));
    let (label, mut tooltip) = match &current {
        Some(DownloadState::Downloading) => ("", "Downloading…".to_string()),
        Some(DownloadState::Saved(_)) => ("Open", "Open downloaded file".into()),
        Some(DownloadState::Reveal(_)) => (
            "Show",
            "Show in Finder — no application could open this file".into(),
        ),
        Some(DownloadState::Failed(error)) => ("Retry", format!("Download failed: {error}")),
        None => ("↓", "Download file to Downloads".into()),
    };
    if !available {
        tooltip = "File is not available yet".into();
    }
    let attachment = attachment.clone();
    div()
        .id(SharedString::from(id))
        .w(px(48.))
        .h(px(26.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .text_xs()
        .text_color(rgb(crate::views::accent()))
        .cursor(if available {
            CursorStyle::PointingHand
        } else {
            CursorStyle::Arrow
        })
        .when(!available, |el| el.opacity(0.4))
        .hover(|s| s.bg(subtle_surface()))
        .tooltip(crate::views::tooltip::TextTooltip::single_line(tooltip))
        .child(if matches!(current, Some(DownloadState::Downloading)) {
            gpui::svg()
                .path("assets/icons/download-spinner.svg")
                .size(px(14.))
                .text_color(rgb(crate::views::accent()))
                .with_animation(
                    "download-spinner",
                    Animation::new(Duration::from_millis(800)).repeat(),
                    |el, delta| {
                        el.with_transformation(Transformation::rotate(gpui::Radians(
                            delta * std::f32::consts::TAU,
                        )))
                    },
                )
                .into_any_element()
        } else if current.is_none() {
            gpui::svg()
                .path("assets/icons/download.svg")
                .size(px(16.))
                .text_color(rgb(crate::views::accent()))
                .into_any_element()
        } else {
            div().child(label).into_any_element()
        })
        .on_click(cx.listener(move |this, _, _, cx| {
            cx.stop_propagation();
            this.activate_attachment_download(&attachment, cx);
        }))
        .into_any_element()
}

impl AppWindow {
    pub(crate) fn activate_attachment_download(
        &mut self,
        attachment: &AttachmentSummary,
        cx: &mut Context<Self>,
    ) {
        if let Some(source) = source(attachment) {
            self.save_or_open_attachment(&source, &attachment.name, cx);
        }
    }

    pub(super) fn save_or_open_attachment(
        &mut self,
        source: &str,
        name: &str,
        cx: &mut Context<Self>,
    ) {
        let download_key = key(source, name);
        match state(source, name, cx) {
            Some(DownloadState::Downloading) => return,
            Some(DownloadState::Saved(path)) => {
                cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                    let background = cx.background_executor().clone();
                    let mut app = cx.clone();
                    async move {
                        let target = path.clone();
                        let opened = background
                            .spawn(async move { crate::app::platform::open_file(&target) })
                            .await;
                        if !opened {
                            let _ = this.update(&mut app, |this, cx| {
                                cx.default_global::<Downloads>()
                                    .0
                                    .insert(download_key, DownloadState::Reveal(path));
                                this.refresh(cx);
                            });
                        }
                    }
                })
                .detach();
                return;
            }
            Some(DownloadState::Reveal(path)) => {
                cx.reveal_path(&path);
                return;
            }
            _ => {}
        }
        cx.default_global::<Downloads>()
            .0
            .insert(download_key.clone(), DownloadState::Downloading);
        self.refresh(cx);
        let source = source.to_string();
        let name = name.to_string();
        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let background = cx.background_executor().clone();
            let mut app = cx.clone();
            async move {
                let started = Instant::now();
                let result = background
                    .spawn(async move { save_copy(&source, &name) })
                    .await;
                background
                    .timer(Duration::from_millis(500).saturating_sub(started.elapsed()))
                    .await;
                let _ = this.update(&mut app, |this, cx| {
                    cx.default_global::<Downloads>().0.insert(
                        download_key,
                        match result {
                            Ok(path) => DownloadState::Saved(path),
                            Err(error) => DownloadState::Failed(error),
                        },
                    );
                    this.refresh(cx);
                });
            }
        })
        .detach();
    }
}

fn save_copy(source: &str, name: &str) -> Result<PathBuf, String> {
    let home = home_dir().ok_or("Unable to locate Downloads")?;
    let directory = PathBuf::from(home).join("Downloads");
    save_copy_to(source, name, &directory)
}

fn save_copy_to(source: &str, name: &str, directory: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let name = Path::new(name)
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("attachment");
    let mut attempts = 0;
    let destination = loop {
        attempts += 1;
        if attempts > 500 {
            return Err("Too many copies of this filename in Downloads".into());
        }
        let path = unique_copy_destination(directory, name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => break path,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        }
    };
    let result = if source.starts_with("https://") || source.starts_with("http://") {
        std::process::Command::new("curl")
            .args([
                "-L",
                "--fail",
                "--silent",
                "--show-error",
                "--connect-timeout",
                "30",
                "--max-time",
                "1800",
                "--output",
            ])
            .arg(&destination)
            .arg(source)
            .output()
            .map_err(|e| e.to_string())
            .and_then(|output| {
                if output.status.success() {
                    Ok(())
                } else {
                    Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
                }
            })
    } else {
        std::fs::copy(
            crate::views::normalize_local_source_path(source),
            &destination,
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
    };
    if let Err(error) = result {
        let _ = std::fs::remove_file(&destination);
        return Err(error);
    }
    Ok(destination)
}

pub(crate) fn row(
    attachment: &AttachmentSummary,
    id: String,
    header: AnyElement,
    cx: &mut Context<AppWindow>,
) -> AnyElement {
    let generic = attachment.kind == crate::domain::attachment::AttachmentKind::File;
    let owned = attachment.clone();
    let header = if generic {
        crate::views::attachment_header_row(attachment)
    } else {
        header
    };
    div()
        .id(SharedString::from(format!("{id}-row")))
        .flex()
        .items_center()
        .gap_1()
        .max_w(px(420.))
        .min_w(px(0.))
        .child(div().flex_1().min_w(px(0.)).overflow_hidden().child(header))
        .child(control(attachment, id, cx))
        .when(generic, |el| {
            el.h(px(32.))
                .pl_2()
                .pr_1()
                .rounded(px(8.))
                .border_1()
                .border_color(rgb(crate::views::field_border()))
                .bg(rgb(crate::views::field_bg()))
                .cursor(CursorStyle::PointingHand)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.activate_attachment_download(&owned, cx);
                }))
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saving_preserves_existing_files_and_cleans_up_failed_copies() {
        let directory = env::temp_dir().join(format!(
            "zbase-download-{}-{}",
            std::process::id(),
            crate::util::formatting::now_unix_ms()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let source = directory.join("source");
        std::fs::write(&source, b"new file").unwrap();
        std::fs::write(directory.join("example.zip"), b"existing file").unwrap();
        let saved = save_copy_to(source.to_str().unwrap(), "../example.zip", &directory).unwrap();
        assert_eq!(saved, directory.join("example (1).zip"));
        assert_eq!(std::fs::read(saved).unwrap(), b"new file");
        assert_eq!(
            std::fs::read(directory.join("example.zip")).unwrap(),
            b"existing file"
        );
        assert!(save_copy_to("/nonexistent/zbase-fixture", "failed.zip", &directory).is_err());
        assert!(!directory.join("failed.zip").exists());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
