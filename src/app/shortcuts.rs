//! Command modifier policy shared by key registration and shortcut hints.
#[cfg(target_os = "macos")]
const COMMAND_MODIFIER: (&str, &str) = ("cmd-", "⌘");
#[cfg(not(target_os = "macos"))]
const COMMAND_MODIFIER: (&str, &str) = ("ctrl-", "Ctrl+");

pub fn command_keys(keys: &str) -> String {
    keys.replace("cmd-", COMMAND_MODIFIER.0)
}

pub fn command_label() -> &'static str {
    COMMAND_MODIFIER.1
}
