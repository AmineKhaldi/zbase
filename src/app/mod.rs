pub mod actions;
pub mod assets;
pub mod bootstrap;
pub mod commands;
#[cfg(any(test, feature = "demo-fixtures"))]
pub mod demo;
pub mod dock_badge;
pub mod menu_state;
pub mod platform;
pub mod theme;
pub mod window;

mod key_bindings;
pub mod shortcuts;
