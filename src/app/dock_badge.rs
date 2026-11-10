//! The count on the Dock icon. GPUI has no API for it, so on macOS this sets
//! `NSApp.dockTile.badgeLabel` directly; elsewhere it does nothing.

/// Shows `count` on the Dock icon, or removes the badge when it is zero.
pub fn set_count(count: u32) {
    #[cfg(target_os = "macos")]
    mac::set_count(count);
    #[cfg(not(target_os = "macos"))]
    let _ = count;
}

#[cfg(target_os = "macos")]
// objc 0.2's `msg_send!` expands to a `cfg(feature = "cargo-clippy")` check.
#[allow(unexpected_cfgs)]
mod mac {
    use objc::{class, msg_send, runtime::Object, sel, sel_impl};
    use std::ffi::CString;

    pub fn set_count(count: u32) {
        // An empty label removes the badge.
        let label = if count == 0 {
            String::new()
        } else {
            count.to_string()
        };
        let Ok(label) = CString::new(label) else {
            return;
        };
        // SAFETY: AppKit calls on the main thread (GPUI runs app code there),
        // each result checked for nil before use; the string is autoreleased.
        unsafe {
            let is_main: bool = msg_send![class!(NSThread), isMainThread];
            if !is_main {
                return;
            }
            let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
            let tile: *mut Object = msg_send![app, dockTile];
            if tile.is_null() {
                return;
            }
            let label: *mut Object =
                msg_send![class!(NSString), stringWithUTF8String: label.as_ptr()];
            let _: () = msg_send![tile, setBadgeLabel: label];
        }
    }
}
