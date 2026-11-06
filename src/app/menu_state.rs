//! Check marks on menu-bar items. GPUI builds the menu bar but has no notion
//! of an item's on/off state, so on macOS this sets `NSMenuItem.state` on the
//! built items directly; elsewhere it does nothing.

/// Checks or unchecks the item titled `item` in the top-level menu `menu`.
/// Does nothing when either isn't in the menu bar.
pub fn set_checked(menu: &str, item: &str, checked: bool) {
    #[cfg(target_os = "macos")]
    mac::set_checked(menu, item, checked);
    #[cfg(not(target_os = "macos"))]
    let _ = (menu, item, checked);
}

#[cfg(target_os = "macos")]
// objc 0.2's `msg_send!` expands to a `cfg(feature = "cargo-clippy")` check.
#[allow(unexpected_cfgs)]
mod mac {
    use objc::{
        class, msg_send,
        runtime::Object,
        sel, sel_impl,
    };
    use std::ffi::CString;

    const NS_CONTROL_STATE_VALUE_OFF: isize = 0;
    const NS_CONTROL_STATE_VALUE_ON: isize = 1;

    pub fn set_checked(menu: &str, item: &str, checked: bool) {
        let (Ok(menu), Ok(item)) = (CString::new(menu), CString::new(item)) else {
            return;
        };
        // SAFETY: AppKit calls on the main thread (GPUI runs app code there),
        // each result checked for nil before use; the strings are autoreleased.
        unsafe {
            let is_main: bool = msg_send![class!(NSThread), isMainThread];
            if !is_main {
                return;
            }
            let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
            let bar: *mut Object = msg_send![app, mainMenu];
            if bar.is_null() {
                return;
            }
            let title: *mut Object = msg_send![class!(NSString), stringWithUTF8String: menu.as_ptr()];
            let top: *mut Object = msg_send![bar, itemWithTitle: title];
            if top.is_null() {
                return;
            }
            let submenu: *mut Object = msg_send![top, submenu];
            if submenu.is_null() {
                return;
            }
            let title: *mut Object = msg_send![class!(NSString), stringWithUTF8String: item.as_ptr()];
            let entry: *mut Object = msg_send![submenu, itemWithTitle: title];
            if entry.is_null() {
                return;
            }
            let state = if checked {
                NS_CONTROL_STATE_VALUE_ON
            } else {
                NS_CONTROL_STATE_VALUE_OFF
            };
            let _: () = msg_send![entry, setState: state];
        }
    }
}
