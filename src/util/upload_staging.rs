use std::{
    fs::File,
    io,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// Makes dropped files readable by the Keybase service, which opens attachments by path from
/// its own process.
///
/// macOS hands some drops (the screenshot thumbnail, Photos, file promises) to the app as files
/// in a `TemporaryItems/NSIRD_*` directory. A `com.apple.macl` ACL there admits only the app
/// that received the drop, so the service's `open` fails with "operation not permitted". Those
/// files are copied into zbase's temp directory; every other path passes through untouched.
pub fn stage_for_upload(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    stage_for_upload_in(paths, &std::env::temp_dir())
}

fn stage_for_upload_in(paths: Vec<PathBuf>, staging_root: &Path) -> Vec<PathBuf> {
    paths
        .into_iter()
        .enumerate()
        .map(|(index, path)| {
            if !is_drop_receive_path(&path) {
                return path;
            }
            match copy_into_staging(&path, staging_root, index) {
                Ok(staged) => staged,
                Err(error) => {
                    tracing::warn!(
                        "could not stage dropped file {} for upload: {error}",
                        path.display()
                    );
                    path
                }
            }
        })
        .collect()
}

fn is_drop_receive_path(path: &Path) -> bool {
    let mut components = path.components();
    while let Some(component) = components.next() {
        if component == Component::Normal("TemporaryItems".as_ref()) {
            return components.next().is_some_and(|next| {
                next.as_os_str()
                    .to_str()
                    .is_some_and(|name| name.starts_with("NSIRD_"))
            });
        }
    }
    false
}

fn copy_into_staging(source: &Path, staging_root: &Path, index: usize) -> io::Result<PathBuf> {
    let file_name = source
        .file_name()
        .ok_or_else(|| io::Error::other("dropped path has no file name"))?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    // A directory per file keeps the original name, which Keybase shows as the attachment name.
    let dir = staging_root.join(format!(
        "zbase-upload-{}-{nanos}-{index}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir)?;
    let staged = dir.join(file_name);
    // Copy bytes rather than `fs::copy`, which clones extended attributes on macOS and would
    // carry the restrictive ACL along.
    io::copy(&mut File::open(source)?, &mut File::create(&staged)?)?;
    Ok(staged)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "zbase-upload-staging-test-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn recognizes_only_drop_receive_directories() {
        assert!(is_drop_receive_path(Path::new(
            "/var/folders/bl/x/T/TemporaryItems/NSIRD_screencaptureui_LSgKRA/Screenshot 2026-11-11 at 11.07.24.png"
        )));
        assert!(!is_drop_receive_path(Path::new(
            "/var/folders/bl/x/T/TemporaryItems/Screenshot.png"
        )));
        assert!(!is_drop_receive_path(Path::new(
            "/Users/someone/Desktop/Screenshot.png"
        )));
        assert!(!is_drop_receive_path(Path::new(
            "/var/folders/bl/x/T/zbase-paste-1-2.png"
        )));
    }

    #[test]
    fn copies_drop_receive_files_and_keeps_their_names() {
        let root = scratch_dir("copy");
        let receive_dir = root
            .join("TemporaryItems")
            .join("NSIRD_screencaptureui_abc");
        std::fs::create_dir_all(&receive_dir).unwrap();
        let screenshot = receive_dir.join("Screenshot 2026-11-11 at 11.07.24.png");
        std::fs::write(&screenshot, b"png bytes").unwrap();
        let elsewhere = root.join("notes.txt");
        std::fs::write(&elsewhere, b"notes").unwrap();

        let staging_root = root.join("staging");
        let staged =
            stage_for_upload_in(vec![screenshot.clone(), elsewhere.clone()], &staging_root);

        assert_eq!(staged.len(), 2);
        assert!(staged[0].starts_with(&staging_root));
        assert_eq!(staged[0].file_name(), screenshot.file_name());
        assert_eq!(std::fs::read(&staged[0]).unwrap(), b"png bytes");
        assert_eq!(staged[1], elsewhere);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn keeps_the_original_path_when_the_copy_fails() {
        let root = scratch_dir("missing");
        let missing = root
            .join("TemporaryItems")
            .join("NSIRD_screencaptureui_gone")
            .join("Screenshot.png");

        let staged = stage_for_upload_in(vec![missing.clone()], &root.join("staging"));

        assert_eq!(staged, vec![missing]);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
