//! Styled single-choice control backed by Kit's accessible Select primitive.
use gpui_kit::base::{
    Select,
    actions::{SelectDown, SelectUp},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AccessibleAction, Anchor, App, Bounds, Context, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Pixels, Render, Role, SharedString,
    StatefulInteractiveElement, Styled, Window, anchored, canvas, deferred, div, point, px, rgb,
    svg,
};

use super::{
    current_theme, field_bg, field_border, focus_ring, popup_menu, shell_border, subtle_surface,
    text_primary, text_secondary, typography, with_theme,
};
use crate::app::theme::ThemeVariant;

pub struct SelectChanged {
    pub scope: String,
    pub index: usize,
}

pub struct StyledSelect {
    label: SharedString,
    options: Vec<SharedString>,
    scope: String,
    selected: usize,
    highlighted: usize,
    open: bool,
    trigger: FocusHandle,
    content: FocusHandle,
    bounds: Bounds<Pixels>,
    theme: ThemeVariant,
}

impl EventEmitter<SelectChanged> for StyledSelect {}
impl Focusable for StyledSelect {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.trigger.clone()
    }
}

impl StyledSelect {
    pub fn new(
        label: impl Into<SharedString>,
        options: Vec<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        assert!(!options.is_empty(), "select needs at least one option");
        Self {
            label: label.into(),
            options,
            scope: String::new(),
            selected: 0,
            highlighted: 0,
            open: false,
            trigger: cx.focus_handle(),
            content: cx.focus_handle(),
            bounds: Bounds::default(),
            theme: current_theme(),
        }
    }

    /// Synchronize committed state; rebinding to a different record dismisses the popup.
    pub fn set_value(
        &mut self,
        scope: &str,
        selected: usize,
        theme: ThemeVariant,
        cx: &mut Context<Self>,
    ) {
        let selected = selected.min(self.options.len() - 1);
        if self.scope == scope && self.selected == selected && self.theme == theme {
            return;
        }
        if self.scope != scope || self.selected != selected {
            self.open = false;
            self.highlighted = selected;
        }
        self.scope = scope.to_owned();
        self.selected = selected;
        self.theme = theme;
        cx.notify();
    }

    pub fn set_open(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.open = open;
        self.highlighted = self.selected;
        if open {
            self.content.focus(window, cx);
        }
        cx.notify();
    }

    pub fn close(&mut self, cx: &mut Context<Self>) {
        if self.open {
            self.open = false;
            cx.notify();
        }
    }

    fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        self.trigger.focus(window, cx);
        cx.notify();
    }

    fn commit(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        if index != self.selected {
            cx.emit(SelectChanged {
                scope: self.scope.clone(),
                index,
            });
        }
        let trigger = self.trigger.clone();
        // Kit transfers focus after Confirm; restore the trigger after that transfer.
        window.defer(cx, move |window, cx| trigger.focus(window, cx));
        cx.notify();
    }

    fn move_highlight(&mut self, direction: isize, cx: &mut Context<Self>) {
        self.highlighted = (self.highlighted as isize + direction)
            .rem_euclid(self.options.len() as isize) as usize;
        cx.notify();
    }
}

impl StyledSelect {
    fn render_trigger(&self, window: &Window, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let focused = self.trigger.is_focused(window) || self.content.is_focused(window);
        div()
            .id("select-trigger")
            .w_full()
            .min_h(typography::font_px(34.))
            .px_3()
            .py_1()
            .flex()
            .items_center()
            .gap_2()
            .rounded_md()
            .border_1()
            .border_color(rgb(field_border()))
            .bg(rgb(field_bg()))
            .when(focused, |d| {
                d.border_color(shell_border()).shadow(vec![focus_ring()])
            })
            .text_size(typography::body())
            .text_color(rgb(text_primary()))
            .cursor_pointer()
            .hover(|d| d.bg(subtle_surface()))
            .on_click(cx.listener(|this, _, window, cx| {
                cx.stop_propagation();
                this.set_open(!this.open, window, cx);
                if !this.open {
                    this.trigger.focus(window, cx);
                }
            }))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .whitespace_normal()
                    .child(self.options[self.selected].clone()),
            )
            .child(
                svg()
                    .path("assets/icons/chevron-down.svg")
                    .size(px(14.))
                    .flex_shrink_0()
                    .text_color(rgb(text_secondary())),
            )
            .into_any_element()
    }

    fn render_option(
        &self,
        index: usize,
        option: &SharedString,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let choice = cx.entity();
        div()
            .id(index)
            .role(Role::ListBoxOption)
            .aria_selected(index == self.selected)
            .when(index == self.highlighted, |d| {
                d.aria_active_descendant().bg(subtle_surface())
            })
            .min_h(typography::font_px(32.))
            .px_2()
            .py_1()
            .rounded_md()
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .on_mouse_move(cx.listener(move |this, _, _, cx| {
                if this.highlighted != index {
                    this.highlighted = index;
                    cx.notify();
                }
            }))
            .on_click(cx.listener(move |this, _, window, cx| {
                cx.stop_propagation();
                this.commit(index, window, cx);
            }))
            .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                choice.update(cx, |this, cx| this.commit(index, window, cx));
            })
            .child(
                div()
                    .w(px(14.))
                    .flex_shrink_0()
                    .when(index == self.selected, |d| {
                        d.child(
                            svg()
                                .path("assets/icons/check.svg")
                                .size(px(14.))
                                .text_color(rgb(text_primary())),
                        )
                    }),
            )
            .child(div().flex_1().min_w(px(0.)).child(option.clone()))
            .into_any_element()
    }

    fn render_popup(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        deferred(
            anchored()
                .anchor(Anchor::TopLeft)
                .attach(self.bounds, Anchor::BottomLeft)
                .offset(point(px(0.), px(4.)))
                .child(
                    div()
                        .id("select-options")
                        .role(Role::ListBox)
                        .aria_label(self.label.clone())
                        .track_focus(&self.content)
                        .w(self.bounds.size.width.max(px(180.)))
                        .p(px(5.))
                        .rounded_lg()
                        .border_1()
                        .border_color(popup_menu::menu_border())
                        .bg(rgb(popup_menu::menu_bg()))
                        .shadow(popup_menu::menu_shadow())
                        .occlude()
                        .flex()
                        .flex_col()
                        .text_size(typography::body())
                        .text_color(rgb(text_primary()))
                        .on_action(
                            cx.listener(|this, _: &SelectUp, _, cx| this.move_highlight(-1, cx)),
                        )
                        .on_action(
                            cx.listener(|this, _: &SelectDown, _, cx| this.move_highlight(1, cx)),
                        )
                        .on_mouse_down_out(cx.listener(
                            |this, event: &gpui_kit::MouseDownEvent, window, cx| {
                                if !this.bounds.contains(&event.position) {
                                    this.dismiss(window, cx);
                                }
                            },
                        ))
                        .on_key_down(cx.listener(|this, event: &gpui_kit::KeyDownEvent, _, cx| {
                            if event.keystroke.key == "tab" {
                                this.open = false;
                                cx.notify();
                            }
                        }))
                        .children(
                            self.options
                                .iter()
                                .enumerate()
                                .map(|(index, option)| self.render_option(index, option, cx)),
                        ),
                ),
        )
        .into_any_element()
    }
}

impl Render for StyledSelect {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = cx.entity();
        let confirm = state.clone();
        let owner = cx.weak_entity();
        let measure = canvas(
            move |bounds, window, cx| {
                let visible = window.content_mask().bounds.intersects(&bounds);
                let _ = owner.update(cx, |this, cx| {
                    if !visible {
                        this.close(cx);
                    }
                    if this.bounds != bounds {
                        this.bounds = bounds;
                        cx.notify();
                    }
                });
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();
        with_theme(self.theme, || {
            Select::new("select-root")
                .open(self.open)
                .focus_handle(&self.trigger)
                .content_focus_handle(&self.content)
                .accessibility_label(self.label.clone())
                .accessibility_value(self.options[self.selected].clone())
                .on_open_change(move |open, window, cx| {
                    state.update(cx, |this, cx| this.set_open(open, window, cx))
                })
                .on_confirm(move |window, cx| {
                    confirm.update(cx, |this, cx| this.commit(this.highlighted, window, cx))
                })
                .w_full()
                .child(
                    div()
                        .relative()
                        .w_full()
                        .child(measure)
                        .child(self.render_trigger(window, cx))
                        .when(self.open, |root| root.child(self.render_popup(cx))),
                )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{Entity, TestAppContext, VisualTestContext};
    use std::{cell::RefCell, rc::Rc};

    fn fixture(cx: &mut TestAppContext) -> (Entity<StyledSelect>, &mut VisualTestContext) {
        cx.update(gpui_kit::init);
        let (select, cx) = cx.add_window_view(|_, cx| {
            StyledSelect::new(
                "Notifications",
                vec!["All".into(), "Mentions".into(), "Never".into()],
                cx,
            )
        });
        cx.update(|window, cx| {
            select.update(cx, |select, cx| {
                select.set_value("conversation-a", 1, ThemeVariant::Light, cx)
            });
            select.focus_handle(cx).focus(window, cx);
            window.draw(cx).clear(cx);
        });
        (select, cx)
    }

    #[gpui_kit::test]
    fn keyboard_confirm_emits_scoped_value_and_restores_focus(cx: &mut TestAppContext) {
        let (select, cx) = fixture(cx);
        let events = Rc::new(RefCell::new(Vec::new()));
        let output = events.clone();
        let _subscription = cx.update(|_, cx| {
            cx.subscribe(&select, move |select, event: &SelectChanged, cx| {
                output.borrow_mut().push((event.scope.clone(), event.index));
                select.update(cx, |select, cx| {
                    select.set_value(&event.scope, event.index, ThemeVariant::Light, cx)
                });
            })
        });
        cx.simulate_keystrokes("enter down enter");
        cx.update(|window, cx| {
            assert!(!select.read(cx).open);
            assert_eq!(select.read(cx).selected, 2);
            assert!(select.read(cx).trigger.is_focused(window));
        });
        assert_eq!(*events.borrow(), vec![("conversation-a".to_string(), 2)]);
    }

    #[gpui_kit::test]
    fn escape_discards_highlight_and_rebinding_closes_popup(cx: &mut TestAppContext) {
        let (select, cx) = fixture(cx);
        cx.simulate_keystrokes("space down escape");
        cx.update(|window, cx| {
            assert!(!select.read(cx).open);
            assert_eq!(select.read(cx).selected, 1);
            assert!(select.read(cx).trigger.is_focused(window));
        });
        cx.simulate_keystrokes("enter");
        cx.update(|_, cx| {
            assert!(select.read(cx).open);
            select.update(cx, |select, cx| {
                select.set_value("conversation-b", 0, ThemeVariant::Light, cx)
            });
            assert!(!select.read(cx).open);
            assert_eq!(select.read(cx).scope, "conversation-b");
            assert_eq!(select.read(cx).highlighted, 0);
        });
    }

    #[gpui_kit::test]
    fn mouse_can_choose_an_option_and_click_outside_dismisses(cx: &mut TestAppContext) {
        let (select, cx) = fixture(cx);
        let events = Rc::new(RefCell::new(Vec::new()));
        let output = events.clone();
        let _subscription = cx.update(|_, cx| {
            cx.subscribe(&select, move |_, event: &SelectChanged, _| {
                output.borrow_mut().push(event.index)
            })
        });
        let bounds = cx.update(|_, cx| select.read(cx).bounds);
        cx.simulate_click(bounds.center(), Default::default());
        cx.update(|_, cx| assert!(select.read(cx).open));
        cx.simulate_click(
            point(bounds.left() + px(50.), bounds.bottom() + px(4. + 5. + 16.)),
            Default::default(),
        );
        cx.update(|_, cx| assert!(!select.read(cx).open));
        assert_eq!(*events.borrow(), vec![0]);
        cx.simulate_click(bounds.center(), Default::default());
        cx.simulate_click(
            point(bounds.left() + px(20.), bounds.bottom() + px(250.)),
            Default::default(),
        );
        cx.update(|_, cx| assert!(!select.read(cx).open));
        assert_eq!(*events.borrow(), vec![0]);
    }
}
