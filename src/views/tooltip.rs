//! Plain-text hover tooltip shared by sidebar rows, toolbar buttons and badges.
use crate::views::{panel_bg, shell_border_strong, text_primary, text_secondary};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AppContext, Context, IntoElement, ParentElement, Render, Styled, Window, div, px, rgb,
};

/// GPUI supplies the hover delay and removes this tooltip on pointer exit.
pub struct TextTooltip {
    text: String,
    shortcut: Option<String>,
    theme: crate::app::theme::ThemeVariant,
    single_line: bool,
}

impl TextTooltip {
    /// Wrapping tooltip for long labels such as full conversation names.
    pub fn builder(
        text: String,
    ) -> impl Fn(&mut Window, &mut gpui_kit::App) -> gpui_kit::AnyView + 'static {
        Self::build(text, None, false)
    }

    /// One-line tooltip for icon buttons.
    pub fn single_line(
        text: String,
    ) -> impl Fn(&mut Window, &mut gpui_kit::App) -> gpui_kit::AnyView + 'static {
        Self::build(text, None, true)
    }

    /// One-line tooltip with a secondary keyboard shortcut, e.g. "Members  ⇧⌘M".
    pub fn with_shortcut(
        text: String,
        shortcut: String,
    ) -> impl Fn(&mut Window, &mut gpui_kit::App) -> gpui_kit::AnyView + 'static {
        Self::build(text, Some(shortcut), true)
    }

    fn build(
        text: String,
        shortcut: Option<String>,
        single_line: bool,
    ) -> impl Fn(&mut Window, &mut gpui_kit::App) -> gpui_kit::AnyView + 'static {
        let theme = crate::views::current_theme();
        move |_, cx| {
            cx.new(|_| Self {
                text: text.clone(),
                shortcut: shortcut.clone(),
                theme,
                single_line,
            })
            .into()
        }
    }
}

impl Render for TextTooltip {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        crate::views::with_theme(self.theme, || {
            div()
                .max_w(px(360.))
                .px_2()
                .py_1()
                .rounded_md()
                .bg(rgb(panel_bg()))
                .border_1()
                .border_color(shell_border_strong())
                .text_xs()
                .text_color(rgb(text_primary()))
                .flex()
                .items_center()
                .gap_2()
                .when(self.single_line, |tooltip| tooltip.whitespace_nowrap())
                .child(self.text.clone())
                .when_some(self.shortcut.clone(), |tooltip, shortcut| {
                    tooltip.child(div().text_color(rgb(text_secondary())).child(shortcut))
                })
        })
    }
}

/// Platform label for a command shortcut: "⇧⌘D" on macOS, "Ctrl+Shift+D"
/// elsewhere. Other text passes through unchanged.
pub fn shortcut_label(keys: &str) -> String {
    if cfg!(target_os = "macos") {
        return keys.to_string();
    }
    let mut label = String::new();
    for (symbol, name) in [
        ('⌘', crate::app::shortcuts::command_label()),
        ('⌥', "Alt+"),
        ('⇧', "Shift+"),
    ] {
        if keys.contains(symbol) {
            label.push_str(name);
        }
    }
    label.extend(keys.chars().filter(|c| !matches!(c, '⌘' | '⌥' | '⇧')));
    label
}
