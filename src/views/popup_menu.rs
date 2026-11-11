//! A small macOS-style menu that pops up from a control: opaque, rounded,
//! 24px items that fill with a soft grey on hover, an optional check-mark
//! column, right-aligned shortcuts and separators. The owner keeps the open
//! state, places the menu with `anchored` and closes it from `on_dismiss`.
use crate::views::typography;
use crate::views::{is_dark_theme, text_primary, text_secondary, tint};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyElement, App, BoxShadow, ClickEvent, FontWeight, Hsla, InteractiveElement, IntoElement,
    MouseDownEvent, ParentElement, SharedString, StatefulInteractiveElement, Styled, Window, div,
    point, px, rgb, svg,
};

const ITEM_HEIGHT_PX: f32 = 24.;
const CHECK_COLUMN_PX: f32 = 18.;

type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

pub enum PopupMenuEntry {
    Item {
        label: SharedString,
        /// `Some` puts the item in a group of choices; `true` draws the check.
        checked: Option<bool>,
        shortcut: Option<String>,
        on_click: ClickHandler,
    },
    Separator,
}

impl PopupMenuEntry {
    pub fn item(
        label: impl Into<SharedString>,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self::Item {
            label: label.into(),
            checked: None,
            shortcut: None,
            on_click: Box::new(on_click),
        }
    }

    /// One of a set of choices, checked when it is the current one.
    pub fn choice(
        label: impl Into<SharedString>,
        checked: bool,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self::Item {
            label: label.into(),
            checked: Some(checked),
            shortcut: None,
            on_click: Box::new(on_click),
        }
    }

    pub fn shortcut(mut self, keys: String) -> Self {
        if let Self::Item { shortcut, .. } = &mut self {
            *shortcut = Some(keys);
        }
        self
    }
}

/// The menu panel. `on_dismiss` runs on a press anywhere outside it.
pub fn popup_menu(
    id: &'static str,
    min_width: f32,
    entries: Vec<PopupMenuEntry>,
    on_dismiss: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let has_checks = entries.iter().any(|entry| {
        matches!(
            entry,
            PopupMenuEntry::Item {
                checked: Some(_),
                ..
            }
        )
    });
    div()
        .id(id)
        .occlude()
        .min_w(px(min_width))
        .p(px(5.))
        .flex()
        .flex_col()
        .rounded(px(8.))
        .bg(rgb(menu_bg()))
        .border_1()
        .border_color(menu_border())
        .shadow(menu_shadow())
        .text_size(typography::label())
        .text_color(rgb(text_primary()))
        .on_mouse_down_out(on_dismiss)
        .children(entries.into_iter().enumerate().map(|(index, entry)| {
            match entry {
                PopupMenuEntry::Separator => div()
                    .mx(px(10.))
                    .my(px(5.))
                    .h(px(1.))
                    .bg(menu_border())
                    .into_any_element(),
                PopupMenuEntry::Item {
                    label,
                    checked,
                    shortcut,
                    on_click,
                } => {
                    div()
                        .id(SharedString::from(format!("{id}-item-{index}")))
                        .min_h(typography::font_px(ITEM_HEIGHT_PX))
                        .pl(px(if has_checks { 4. } else { 10. }))
                        .pr(px(10.))
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .rounded(px(4.))
                        .cursor_pointer()
                        .whitespace_nowrap()
                        // Grey with the normal text colors, like the sidebar's
                        // selection: no accent.
                        .hover(|style| style.bg(tint(text_primary(), 0.08)))
                        .on_click(on_click)
                        .when(has_checks, |item| {
                            item.child(
                                div()
                                    .w(px(CHECK_COLUMN_PX))
                                    .flex_shrink_0()
                                    .flex()
                                    .justify_center()
                                    .when(checked == Some(true), |column| {
                                        column.child(
                                            svg()
                                                .path("assets/icons/check.svg")
                                                .size(px(12.))
                                                .text_color(rgb(text_primary())),
                                        )
                                    }),
                            )
                        })
                        .child(div().flex_1().child(label))
                        .when_some(shortcut, |item, shortcut| {
                            item.child(
                                div()
                                    .pl(px(24.))
                                    .text_color(rgb(text_secondary()))
                                    .font_weight(FontWeight::NORMAL)
                                    .child(shortcut),
                            )
                        })
                        .into_any_element()
                }
            }
        }))
        .into_any_element()
}

/// Opaque menu fill, a step above the window in dark like AppKit menus.
fn menu_bg() -> u32 {
    if is_dark_theme() { 0x2c2c2c } else { 0xffffff }
}

fn menu_border() -> Hsla {
    if is_dark_theme() {
        tint(0xffffff, 0.12)
    } else {
        tint(0x000000, 0.12)
    }
}

fn menu_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        inset: false,
        color: tint(0x000000, if is_dark_theme() { 0.45 } else { 0.16 }),
        offset: point(px(0.), px(6.)),
        blur_radius: px(18.),
        spread_radius: px(0.),
    }]
}
