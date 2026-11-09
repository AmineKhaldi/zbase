//! Opening a downloaded file needs a launch result to offer Reveal on failure.
use std::path::Path;

#[cfg(target_os = "windows")]
pub fn open_file(path: &Path) -> bool {
    use windows::{
        Win32::{
            System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize},
            UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWDEFAULT},
        },
        core::{HSTRING, w},
    };
    // Called on a background worker. Shell extensions can require COM; balance
    // only our successful initialization, preserving any existing apartment.
    unsafe {
        let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
        let result = ShellExecuteW(
            None,
            w!("open"),
            &HSTRING::from(path.as_os_str()),
            None,
            None,
            SW_SHOWDEFAULT,
        );
        if initialized {
            CoUninitialize();
        }
        result.0 as isize > 32
    }
}

#[cfg(not(target_os = "windows"))]
pub fn open_file(path: &Path) -> bool {
    std::process::Command::new("open")
        .arg(path)
        .status()
        .is_ok_and(|status| status.success())
}
