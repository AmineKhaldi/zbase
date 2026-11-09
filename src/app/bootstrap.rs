use crate::app::assets::KbuiAssets;
use crate::app::commands;
use crate::app::window::open_main_window;
use crate::views::input::{
    Backspace, Copy, Cut, Delete, DeleteToBeginningOfLine, DeleteToEndOfLine, DeleteWordBackward,
    Down, End, Home, InsertNewline, Left, MoveToBeginningOfLine, MoveToEndOfLine,
    MoveToNextWordEnd, MoveToPreviousWordStart, Paste, Redo, Right, SelectAll, SelectDown,
    SelectEnd, SelectHome, SelectLeft, SelectRight, SelectToBeginningOfLine, SelectToEndOfLine,
    SelectToNextWordEnd, SelectToPreviousWordStart, SelectUp, ShowCharacterPalette, Undo, Up,
};
use gpui::{Action, Application, KeyBinding, Menu, MenuItem, SystemMenuType};

/// Key bindings are written with macOS keys. Elsewhere Cmd becomes Ctrl, text
/// editing follows the platform (Ctrl moves by word; Home and End move by
/// line), and the Emacs-style Ctrl bindings are left out because Ctrl is the
/// command key there.
fn binding<A: Action>(keys: &str, action: A, context: Option<&str>) -> Option<KeyBinding> {
    if cfg!(target_os = "macos") {
        return Some(KeyBinding::new(keys, action, context));
    }
    let keys = match keys {
        "cmd-left" | "cmd-right" | "cmd-shift-left" | "cmd-shift-right" | "cmd-backspace"
        | "cmd-delete" | "alt-b" | "alt-f" => return None,
        "alt-backspace" | "alt-left" | "alt-right" | "alt-shift-left" | "alt-shift-right" => {
            keys.replacen("alt-", "ctrl-", 1)
        }
        _ if keys.split('-').any(|part| part == "ctrl") => return None,
        _ => keys.replace("cmd-", "ctrl-"),
    };
    Some(KeyBinding::new(&keys, action, context))
}

fn platform_bindings<const N: usize>(
    bindings: [Option<KeyBinding>; N],
) -> impl IntoIterator<Item = KeyBinding> {
    bindings.into_iter().flatten()
}

pub fn run() {
    let application = Application::new().with_assets(KbuiAssets {});

    // On macOS, clicking the Dock icon to re-open an already-running app should restore a window.
    // If we have no windows, create one.
    application.on_reopen(|cx| {
        if cfg!(target_os = "macos") && cx.windows().is_empty() {
            open_main_window(cx);
        }
    });

    application.run(|cx| {
        // On platforms without a Dock/Menu affordance, quit when the last window closes.
        if !cfg!(target_os = "macos") {
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
        }

        cx.bind_keys(platform_bindings([
            binding("cmd-q", commands::QuitApp, Some("Workspace")),
            binding("cmd-w", commands::CloseWindow, Some("SettingsWindow")),
            binding("cmd-q", commands::QuitApp, Some("SettingsWindow")),
            binding("cmd-w", commands::CloseWindow, Some("Workspace")),
            binding("cmd-[", commands::NavigateBack, Some("Workspace")),
            binding("cmd-]", commands::NavigateForward, Some("Workspace")),
            binding("cmd-shift-a", commands::ShowActivity, Some("Workspace")),
            binding("cmd-,", commands::OpenPreferences, Some("Workspace")),
            binding(
                "alt-cmd-1",
                commands::SortSidebarByRecent,
                Some("Workspace"),
            ),
            binding("alt-cmd-2", commands::GroupSidebarByTeam, Some("Workspace")),
            binding(
                "cmd-shift-\\",
                commands::ToggleThreadPane,
                Some("Workspace"),
            ),
            binding(
                "cmd-shift-m",
                commands::ToggleMembersPane,
                Some("Workspace"),
            ),
            binding(
                "cmd-shift-d",
                commands::ToggleDetailsPane,
                Some("Workspace"),
            ),
            binding("cmd-shift-f", commands::OpenFilesPane, Some("Workspace")),
            binding(
                "cmd-shift-s",
                commands::ToggleSplashScreen,
                Some("Workspace"),
            ),
            binding("cmd-1", commands::ShowHome, Some("Workspace")),
            binding(
                "cmd-2",
                commands::OpenQuickSwitcherRecent2,
                Some("Workspace"),
            ),
            binding(
                "cmd-3",
                commands::OpenQuickSwitcherRecent3,
                Some("Workspace"),
            ),
            binding(
                "cmd-4",
                commands::OpenQuickSwitcherRecent4,
                Some("Workspace"),
            ),
            binding(
                "cmd-5",
                commands::OpenQuickSwitcherRecent5,
                Some("Workspace"),
            ),
            binding("cmd-j", commands::OpenSearch, Some("Workspace")),
            binding("cmd-f", commands::ToggleFindInChat, Some("Workspace")),
            binding("enter", commands::ConfirmPrimary, Some("Workspace")),
            binding(
                "tab",
                commands::AcceptAutocomplete,
                Some("MultilineTextField"),
            ),
            binding("up", commands::SelectPrevious, Some("Workspace")),
            binding("down", commands::SelectNext, Some("Workspace")),
            binding("tab", commands::SelectNext, Some("QuickSwitcherTextField")),
            binding(
                "shift-tab",
                commands::SelectPrevious,
                Some("QuickSwitcherTextField"),
            ),
            binding("backspace", Backspace, Some("QuickSwitcherTextField")),
            binding(
                "alt-backspace",
                DeleteWordBackward,
                Some("QuickSwitcherTextField"),
            ),
            binding(
                "cmd-backspace",
                DeleteToBeginningOfLine,
                Some("QuickSwitcherTextField"),
            ),
            binding(
                "cmd-delete",
                DeleteToEndOfLine,
                Some("QuickSwitcherTextField"),
            ),
            binding("ctrl-k", DeleteToEndOfLine, Some("QuickSwitcherTextField")),
            binding("delete", Delete, Some("QuickSwitcherTextField")),
            binding("left", Left, Some("QuickSwitcherTextField")),
            binding("right", Right, Some("QuickSwitcherTextField")),
            binding("ctrl-h", Backspace, Some("QuickSwitcherTextField")),
            binding("ctrl-b", Left, Some("QuickSwitcherTextField")),
            binding("ctrl-f", Right, Some("QuickSwitcherTextField")),
            binding(
                "alt-b",
                MoveToPreviousWordStart,
                Some("QuickSwitcherTextField"),
            ),
            binding("alt-f", MoveToNextWordEnd, Some("QuickSwitcherTextField")),
            binding(
                "alt-left",
                MoveToPreviousWordStart,
                Some("QuickSwitcherTextField"),
            ),
            binding(
                "alt-right",
                MoveToNextWordEnd,
                Some("QuickSwitcherTextField"),
            ),
            binding(
                "cmd-left",
                MoveToBeginningOfLine,
                Some("QuickSwitcherTextField"),
            ),
            binding("cmd-right", MoveToEndOfLine, Some("QuickSwitcherTextField")),
            binding("shift-left", SelectLeft, Some("QuickSwitcherTextField")),
            binding("shift-right", SelectRight, Some("QuickSwitcherTextField")),
            binding(
                "alt-shift-left",
                SelectToPreviousWordStart,
                Some("QuickSwitcherTextField"),
            ),
            binding(
                "alt-shift-right",
                SelectToNextWordEnd,
                Some("QuickSwitcherTextField"),
            ),
            binding(
                "cmd-shift-left",
                SelectToBeginningOfLine,
                Some("QuickSwitcherTextField"),
            ),
            binding(
                "cmd-shift-right",
                SelectToEndOfLine,
                Some("QuickSwitcherTextField"),
            ),
            binding("cmd-a", SelectAll, Some("QuickSwitcherTextField")),
            binding("cmd-v", Paste, Some("QuickSwitcherTextField")),
            binding("cmd-c", Copy, Some("QuickSwitcherTextField")),
            binding("cmd-x", Cut, Some("QuickSwitcherTextField")),
            binding("cmd-z", Undo, Some("QuickSwitcherTextField")),
            binding("ctrl-/", Undo, Some("QuickSwitcherTextField")),
            binding("cmd-shift-z", Redo, Some("QuickSwitcherTextField")),
            binding("home", Home, Some("QuickSwitcherTextField")),
            binding("end", End, Some("QuickSwitcherTextField")),
            binding("shift-home", SelectHome, Some("QuickSwitcherTextField")),
            binding("shift-end", SelectEnd, Some("QuickSwitcherTextField")),
            binding(
                "ctrl-a",
                MoveToBeginningOfLine,
                Some("QuickSwitcherTextField"),
            ),
            binding("ctrl-e", MoveToEndOfLine, Some("QuickSwitcherTextField")),
            binding("ctrl-d", Delete, Some("QuickSwitcherTextField")),
            binding(
                "ctrl-cmd-space",
                ShowCharacterPalette,
                Some("QuickSwitcherTextField"),
            ),
            binding("alt-up", commands::SelectSidebarPrevious, Some("Workspace")),
            binding("alt-down", commands::SelectSidebarNext, Some("Workspace")),
            binding(
                "alt-enter",
                commands::ActivateSidebarSelection,
                Some("Workspace"),
            ),
            binding("cmd-n", commands::OpenNewChat, Some("Workspace")),
            binding("cmd-enter", commands::StartNewChat, Some("Workspace")),
            binding("cmd-b", commands::FormatBold, Some("MultilineTextField")),
            binding("cmd-i", commands::FormatItalic, Some("MultilineTextField")),
            binding("cmd-e", commands::FormatCode, Some("MultilineTextField")),
            binding("cmd-k", commands::ToggleQuickSwitcher, Some("Workspace")),
            binding(
                "cmd-shift-p",
                commands::ToggleCommandPalette,
                Some("Workspace"),
            ),
            binding(
                "cmd-shift-i",
                commands::ToggleKeybaseInspector,
                Some("Workspace"),
            ),
            binding(
                "cmd-shift-b",
                commands::ToggleBenchmarkCapture,
                Some("Workspace"),
            ),
            binding("escape", commands::DismissOverlays, Some("Workspace")),
            binding(
                "enter",
                commands::FindNextMatch,
                Some("FindInChatTextField"),
            ),
            binding(
                "shift-enter",
                commands::FindPrevMatch,
                Some("FindInChatTextField"),
            ),
            binding(
                "escape",
                commands::CloseFindInChat,
                Some("FindInChatTextField"),
            ),
            binding("backspace", Backspace, Some("TextField")),
            binding("alt-backspace", DeleteWordBackward, Some("TextField")),
            binding("cmd-backspace", DeleteToBeginningOfLine, Some("TextField")),
            binding("cmd-delete", DeleteToEndOfLine, Some("TextField")),
            binding("ctrl-k", DeleteToEndOfLine, Some("TextField")),
            binding("delete", Delete, Some("TextField")),
            binding("left", Left, Some("TextField")),
            binding("right", Right, Some("TextField")),
            binding("ctrl-h", Backspace, Some("TextField")),
            binding("ctrl-b", Left, Some("TextField")),
            binding("ctrl-f", Right, Some("TextField")),
            binding("alt-b", MoveToPreviousWordStart, Some("TextField")),
            binding("alt-f", MoveToNextWordEnd, Some("TextField")),
            binding("alt-left", MoveToPreviousWordStart, Some("TextField")),
            binding("alt-right", MoveToNextWordEnd, Some("TextField")),
            binding("cmd-left", MoveToBeginningOfLine, Some("TextField")),
            binding("cmd-right", MoveToEndOfLine, Some("TextField")),
            binding("shift-left", SelectLeft, Some("TextField")),
            binding("shift-right", SelectRight, Some("TextField")),
            binding(
                "alt-shift-left",
                SelectToPreviousWordStart,
                Some("TextField"),
            ),
            binding("alt-shift-right", SelectToNextWordEnd, Some("TextField")),
            binding("cmd-shift-left", SelectToBeginningOfLine, Some("TextField")),
            binding("cmd-shift-right", SelectToEndOfLine, Some("TextField")),
            binding("cmd-a", SelectAll, Some("TextField")),
            binding("cmd-v", Paste, Some("TextField")),
            binding("cmd-c", Copy, Some("TextField")),
            binding("cmd-x", Cut, Some("TextField")),
            binding("cmd-z", Undo, Some("TextField")),
            binding("ctrl-/", Undo, Some("TextField")),
            binding("cmd-shift-z", Redo, Some("TextField")),
            binding("home", Home, Some("TextField")),
            binding("end", End, Some("TextField")),
            binding("shift-home", SelectHome, Some("TextField")),
            binding("shift-end", SelectEnd, Some("TextField")),
            binding("ctrl-a", MoveToBeginningOfLine, Some("TextField")),
            binding("ctrl-e", MoveToEndOfLine, Some("TextField")),
            binding("ctrl-d", Delete, Some("TextField")),
            binding("ctrl-cmd-space", ShowCharacterPalette, Some("TextField")),
            binding("backspace", Backspace, Some("MultilineTextField")),
            binding(
                "alt-backspace",
                DeleteWordBackward,
                Some("MultilineTextField"),
            ),
            binding(
                "cmd-backspace",
                DeleteToBeginningOfLine,
                Some("MultilineTextField"),
            ),
            binding("cmd-delete", DeleteToEndOfLine, Some("MultilineTextField")),
            binding("ctrl-k", DeleteToEndOfLine, Some("MultilineTextField")),
            binding("delete", Delete, Some("MultilineTextField")),
            binding("left", Left, Some("MultilineTextField")),
            binding("right", Right, Some("MultilineTextField")),
            binding("ctrl-h", Backspace, Some("MultilineTextField")),
            binding("ctrl-b", Left, Some("MultilineTextField")),
            binding("ctrl-f", Right, Some("MultilineTextField")),
            binding("alt-b", MoveToPreviousWordStart, Some("MultilineTextField")),
            binding("alt-f", MoveToNextWordEnd, Some("MultilineTextField")),
            binding(
                "alt-left",
                MoveToPreviousWordStart,
                Some("MultilineTextField"),
            ),
            binding("alt-right", MoveToNextWordEnd, Some("MultilineTextField")),
            binding(
                "cmd-left",
                MoveToBeginningOfLine,
                Some("MultilineTextField"),
            ),
            binding("cmd-right", MoveToEndOfLine, Some("MultilineTextField")),
            binding("up", Up, Some("MultilineTextField")),
            binding("down", Down, Some("MultilineTextField")),
            binding("shift-left", SelectLeft, Some("MultilineTextField")),
            binding("shift-right", SelectRight, Some("MultilineTextField")),
            binding(
                "alt-shift-left",
                SelectToPreviousWordStart,
                Some("MultilineTextField"),
            ),
            binding(
                "alt-shift-right",
                SelectToNextWordEnd,
                Some("MultilineTextField"),
            ),
            binding(
                "cmd-shift-left",
                SelectToBeginningOfLine,
                Some("MultilineTextField"),
            ),
            binding(
                "cmd-shift-right",
                SelectToEndOfLine,
                Some("MultilineTextField"),
            ),
            binding("shift-up", SelectUp, Some("MultilineTextField")),
            binding("shift-down", SelectDown, Some("MultilineTextField")),
            binding("cmd-a", SelectAll, Some("MultilineTextField")),
            binding("cmd-v", Paste, Some("MultilineTextField")),
            binding("cmd-c", Copy, Some("MultilineTextField")),
            binding("cmd-x", Cut, Some("MultilineTextField")),
            binding("cmd-z", Undo, Some("MultilineTextField")),
            binding("ctrl-/", Undo, Some("MultilineTextField")),
            binding("cmd-shift-z", Redo, Some("MultilineTextField")),
            binding("home", Home, Some("MultilineTextField")),
            binding("end", End, Some("MultilineTextField")),
            binding("shift-home", SelectHome, Some("MultilineTextField")),
            binding("shift-end", SelectEnd, Some("MultilineTextField")),
            binding("ctrl-a", MoveToBeginningOfLine, Some("MultilineTextField")),
            binding("ctrl-e", MoveToEndOfLine, Some("MultilineTextField")),
            binding("ctrl-d", Delete, Some("MultilineTextField")),
            binding("shift-enter", InsertNewline, Some("MultilineTextField")),
            binding(
                "ctrl-cmd-space",
                ShowCharacterPalette,
                Some("MultilineTextField"),
            ),
            binding("left", Left, Some("SelectableText")),
            binding("right", Right, Some("SelectableText")),
            binding("up", Up, Some("SelectableText")),
            binding("down", Down, Some("SelectableText")),
            binding("shift-left", SelectLeft, Some("SelectableText")),
            binding("shift-right", SelectRight, Some("SelectableText")),
            binding("shift-up", SelectUp, Some("SelectableText")),
            binding("shift-down", SelectDown, Some("SelectableText")),
            binding("cmd-a", SelectAll, Some("SelectableText")),
            binding("cmd-c", Copy, Some("SelectableText")),
            binding("home", Home, Some("SelectableText")),
            binding("end", End, Some("SelectableText")),
            binding("shift-home", SelectHome, Some("SelectableText")),
            binding("shift-end", SelectEnd, Some("SelectableText")),
        ]));
        cx.set_menus(app_menus());
        open_main_window(cx);
    });
}

/// The macOS menu bar: the app menu carries Settings… (⌘,) and Quit, so both are
/// discoverable outside the keyboard. Shortcuts shown come from the key bindings.
fn app_menus() -> Vec<Menu> {
    vec![
        Menu {
            name: "zBase".into(),
            items: vec![
                MenuItem::action("Check for Updates…", commands::CheckForUpdates),
                MenuItem::action("Settings…", commands::OpenPreferences),
                MenuItem::separator(),
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Quit zBase", commands::QuitApp),
            ],
        },
        // The sidebar order; the main window keeps the check mark on the current one.
        Menu {
            name: VIEW_MENU.into(),
            items: vec![
                MenuItem::action(SORT_BY_RECENT_ITEM, commands::SortSidebarByRecent),
                MenuItem::action(GROUP_BY_TEAM_ITEM, commands::GroupSidebarByTeam),
            ],
        },
    ]
}

pub const VIEW_MENU: &str = "View";
pub const SORT_BY_RECENT_ITEM: &str = "Sort by Recent";
pub const GROUP_BY_TEAM_ITEM: &str = "Group by Team";
