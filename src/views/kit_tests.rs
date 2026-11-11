//! Exercise the migrated platform, Kit Root, wheel dispatch and our custom input
//! against both selectable GPUI engines. No backend service or user data is used.
use super::input::TextField;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AppContext, Context, Entity, Focusable, InputEvent, IntoElement, ListAlignment, ListState,
    ParentElement, Render, ScrollDelta, Styled, TestAppContext, div, list, point, px,
};

struct Transcript {
    list: ListState,
    scroll_events: usize,
}

impl Render for Transcript {
    fn render(&mut self, _: &mut gpui_kit::Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.weak_entity();
        gpui_kit::base::ScrollBounce::new(
            "transcript-bounce",
            &self.list,
            list(self.list.clone(), |_, _, _| {
                div().h(px(40.)).w_full().into_any_element()
            })
            .w(px(240.))
            .h(px(200.)),
        )
        .enabled(true)
        .on_scroll(move |_, cx| {
            let _ = owner.update(cx, |this, _| this.scroll_events += 1);
        })
    }
}

#[gpui_kit::test]
fn kit_root_applies_each_wheel_delta_once(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let list = ListState::new(40, ListAlignment::Top, px(0.)).measure_all();
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(Default::default(), cx, |_, cx| {
            cx.new(|_| Transcript {
                list: list.clone(),
                scroll_events: 0,
            })
        })
        .unwrap()
    });
    cx.update_window(window, |_, window, cx| window.render_frame(cx))
        .unwrap();
    cx.update_window(window, |_, window, cx| {
        window.dispatch_event(
            gpui_kit::ScrollEvent {
                position: point(px(100.), px(100.)),
                delta: ScrollDelta::Pixels(point(px(0.), px(-30.))),
                touch_phase: gpui_kit::TouchPhase::Started,
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
        window.render_frame(cx);
        assert_eq!(list.scroll_px_offset_for_scrollbar().y, px(-30.));
        assert_eq!(view.read(cx).scroll_events, 1);
    })
    .unwrap();
}

struct InputFixture(Entity<TextField>);
impl Render for InputFixture {
    fn render(&mut self, _: &mut gpui_kit::Window, _: &mut Context<Self>) -> impl IntoElement {
        div().p_4().w(px(320.)).child(self.0.clone())
    }
}

#[gpui_kit::test]
fn custom_composer_keeps_unicode_input_and_focus_through_redraw(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::app::key_bindings::install(cx);
    });
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(Default::default(), cx, |window, cx| {
            let input = cx
                .new(|cx| TextField::new_auto_grow(cx.focus_handle(), "Message", String::new(), 8));
            window.focus(&input.focus_handle(cx), cx);
            cx.new(|_| InputFixture(input))
        })
        .unwrap()
    });
    cx.update_window(window, |_, window, cx| {
        window.render_frame(cx);
        window.input("hello 🦀界", cx);
        let input = view.read(cx).0.clone();
        assert_eq!(input.read(cx).text(), "hello 🦀界");
        window.press("backspace", cx);
        assert_eq!(input.read(cx).text(), "hello 🦀");
        view.update(cx, |_, cx| cx.notify());
        window.render_frame(cx);
        window.input("!", cx);
        assert_eq!(input.read(cx).text(), "hello 🦀!");
        assert!(input.focus_handle(cx).is_focused(window));
    })
    .unwrap();
}

#[cfg(target_os = "macos")]
mod sidebar_bounce {
    use super::*;
    use gpui_kit::test::TestSupportExt;
    use gpui_kit::{
        InteractiveElement, ScrollHandle, StatefulInteractiveElement, UniformListScrollHandle,
        base::ScrollbarHandle,
    };

    #[derive(Clone)]
    enum Handle {
        Grouped(ScrollHandle),
        Recent(UniformListScrollHandle),
    }
    impl Handle {
        fn scroll(&self) -> &dyn ScrollbarHandle {
            match self {
                Self::Grouped(handle) => handle,
                Self::Recent(handle) => handle,
            }
        }
    }
    struct SidebarFixture {
        handle: Handle,
        reduced_motion: bool,
    }
    impl Render for SidebarFixture {
        fn render(&mut self, _: &mut gpui_kit::Window, _: &mut Context<Self>) -> impl IntoElement {
            let viewport = match &self.handle {
                Handle::Grouped(handle) => super::super::sidebar::scroll_viewport(
                    "test-grouped-bounce",
                    handle,
                    div()
                        .id("grouped")
                        .flex_1()
                        .min_h(px(0.))
                        .overflow_y_scroll()
                        .track_scroll(handle)
                        .child(div().h(px(1600.)).flex_shrink_0()),
                    self.reduced_motion,
                ),
                Handle::Recent(handle) => super::super::sidebar::scroll_viewport(
                    "test-recent-bounce",
                    handle,
                    gpui_kit::uniform_list("recent", 40, |range, _, _| {
                        range
                            .map(|_| div().h(px(40.)).w_full().into_any_element())
                            .collect()
                    })
                    .track_scroll(handle)
                    .flex_1()
                    .min_h(px(0.)),
                    self.reduced_motion,
                ),
            };
            div()
                .w(px(240.))
                .h(px(300.))
                .flex()
                .flex_col()
                .child(div().id("header").test_support().h(px(40.)).flex_shrink_0())
                .child(viewport)
                .child(div().id("footer").test_support().h(px(40.)).flex_shrink_0())
        }
    }

    fn check(cx: &mut TestAppContext, reduced_motion: bool) {
        for recent in [false, true] {
            for bottom in [false, true] {
                let handle = if recent {
                    Handle::Recent(UniformListScrollHandle::new())
                } else {
                    Handle::Grouped(ScrollHandle::new())
                };
                let mut app = cx.new_app();
                app.update(gpui_kit::init);
                let (window, _) = app.update(|cx| {
                    gpui_kit::open_window(Default::default(), cx, |_, cx| {
                        cx.new(|_| SidebarFixture {
                            handle: handle.clone(),
                            reduced_motion,
                        })
                    })
                    .unwrap()
                });
                app.update_window(window, |_, window, cx| {
                    window.render_frame(cx);
                    let max = handle.scroll().content_size().height
                        - handle.scroll().viewport_bounds().size.height;
                    assert!(max > px(0.));
                    let offset = if bottom { -max } else { px(0.) };
                    handle.scroll().set_offset(point(px(0.), offset));
                    window.refresh();
                    window.render_frame(cx);
                    let header = window.find("header").bounds();
                    let footer = window.find("footer").bounds();
                    let origin = handle.scroll().viewport_bounds().origin;
                    let direction = if bottom { -1. } else { 1. };
                    for (phase, dy) in [
                        (gpui_kit::TouchPhase::Started, 0.),
                        (gpui_kit::TouchPhase::Moved, 100. * direction),
                    ] {
                        window.dispatch_event(
                            gpui_kit::ScrollEvent {
                                position: point(px(100.), px(100.)),
                                delta: ScrollDelta::Pixels(point(px(0.), px(dy))),
                                touch_phase: phase,
                                ..Default::default()
                            }
                            .to_platform_input(),
                            cx,
                        );
                        window.render_frame(cx);
                    }
                    let pulled = handle.scroll().viewport_bounds().origin;
                    if reduced_motion {
                        assert_eq!(pulled, origin);
                    } else {
                        assert!(
                            (pulled.y - origin.y) * direction > px(0.),
                            "sidebar should stretch at either edge"
                        );
                    }
                    assert_eq!(
                        handle.scroll().offset().y,
                        offset,
                        "logical scrolling stays clamped"
                    );
                    assert_eq!(window.find("header").bounds(), header);
                    assert_eq!(window.find("footer").bounds(), footer);
                })
                .unwrap();
            }
        }
    }

    #[gpui_kit::test]
    fn both_sidebar_modes_bounce_without_moving_chrome(cx: &mut TestAppContext) {
        check(cx, false);
    }

    #[gpui_kit::test]
    fn sidebar_reduce_motion_disables_stretch(cx: &mut TestAppContext) {
        check(cx, true);
    }

    #[gpui_kit::test]
    fn dragging_the_sidebar_scrollbar_scrolls_both_modes(cx: &mut TestAppContext) {
        for recent in [false, true] {
            let handle = if recent {
                Handle::Recent(UniformListScrollHandle::new())
            } else {
                Handle::Grouped(ScrollHandle::new())
            };
            let mut app = cx.new_app();
            app.update(gpui_kit::init);
            let (window, _) = app.update(|cx| {
                gpui_kit::open_window(Default::default(), cx, |_, cx| {
                    cx.new(|_| SidebarFixture {
                        handle: handle.clone(),
                        reduced_motion: false,
                    })
                })
                .unwrap()
            });
            app.update_window(window, |_, window, cx| {
                window.render_frame(cx);
                // The thumb starts at the top of the gutter on the right edge.
                let viewport = handle.scroll().viewport_bounds();
                let grab = point(viewport.right() - px(7.), viewport.top() + px(10.));
                window.simulate_mouse_move(grab, cx);
                window.render_frame(cx);
                window.dispatch_event(
                    gpui_kit::MouseDownEvent {
                        button: gpui_kit::MouseButton::Left,
                        position: grab,
                        click_count: 1,
                        ..Default::default()
                    }
                    .to_platform_input(),
                    cx,
                );
                let dropped = grab + point(px(0.), px(100.));
                window.dispatch_event(
                    gpui_kit::MouseMoveEvent {
                        position: dropped,
                        pressed_button: Some(gpui_kit::MouseButton::Left),
                        ..Default::default()
                    }
                    .to_platform_input(),
                    cx,
                );
                window.dispatch_event(
                    gpui_kit::MouseUpEvent {
                        button: gpui_kit::MouseButton::Left,
                        position: dropped,
                        click_count: 1,
                        ..Default::default()
                    }
                    .to_platform_input(),
                    cx,
                );
                window.render_frame(cx);
                assert!(
                    handle.scroll().offset().y < px(0.),
                    "dragging the thumb down scrolls the {} list",
                    if recent { "Recent" } else { "Grouped" }
                );
            })
            .unwrap();
        }
    }
}

mod slow_scroll {
    use super::*;
    use crate::views::timeline::layout_anchor;
    use gpui_kit::InteractiveElement;
    use std::{cell::Cell, rc::Rc};

    struct PinnedTranscript {
        list: ListState,
        bottom_distance: Rc<Cell<gpui_kit::Pixels>>,
    }
    impl Render for PinnedTranscript {
        fn render(&mut self, _: &mut gpui_kit::Window, _: &mut Context<Self>) -> impl IntoElement {
            let list_state = self.list.clone();
            let distance = self.bottom_distance.clone();
            let sampled = self.bottom_distance.clone();
            let scroll_state = self.list.clone();
            div()
                .relative()
                .w(px(400.))
                .h(px(200.))
                .child(
                    gpui_kit::canvas(
                        move |_, _, _| {
                            if layout_anchor::should_follow_latest(
                                list_state.viewport_bounds().size.height,
                                distance.get(),
                                false,
                            ) {
                                list_state.scroll_to_end();
                            }
                        },
                        |_, _, _, _| (),
                    )
                    .absolute()
                    .inset_0(),
                )
                .child(
                    gpui_kit::base::ScrollBounce::new(
                        "pinned-transcript",
                        &self.list,
                        list(self.list.clone(), |_, _, _| {
                            div().h(px(40.)).into_any_element()
                        })
                        .w(px(400.))
                        .h(px(200.)),
                    )
                    .on_scroll(move |_, _| {
                        sampled.set(layout_anchor::bottom_distance(&scroll_state))
                    }),
                )
                .child(
                    gpui_kit::deferred(
                        div()
                            .absolute()
                            .left(px(40.))
                            .top(px(40.))
                            .w(px(260.))
                            .h(px(32.))
                            .block_mouse_except_scroll()
                            .on_mouse_move(|_, _, cx| cx.stop_propagation())
                            .child("😀 👍 ❤️"),
                    )
                    .priority(10),
                )
        }
    }

    #[gpui_kit::test]
    fn slow_upward_scrolling_escapes_bottom_pinning(cx: &mut TestAppContext) {
        for delta in [
            ScrollDelta::Pixels(point(px(0.), px(0.25))),
            ScrollDelta::Pixels(point(px(0.), px(1.))),
            ScrollDelta::Lines(point(0., 1.)),
        ] {
            let mut app = cx.new_app();
            app.update(gpui_kit::init);
            let state = ListState::new(40, ListAlignment::Top, px(0.)).measure_all();
            state.scroll_to_end();
            let (window, _) = app.update(|cx| {
                gpui_kit::open_window(Default::default(), cx, |_, cx| {
                    cx.new(|_| PinnedTranscript {
                        list: state.clone(),
                        bottom_distance: Rc::new(Cell::new(px(0.))),
                    })
                })
                .unwrap()
            });
            app.update_window(window, |_, window, cx| {
                window.render_frame(cx);
                assert_eq!(layout_anchor::bottom_distance(&state), px(0.));
                let step = delta.pixel_delta(window.line_height()).y;
                for packet in 0..=16 {
                    window.dispatch_event(gpui_kit::ScrollEvent {
                        position: point(px(100.), px(100.)),
                        delta: if packet == 0 { ScrollDelta::Pixels(point(px(0.), px(0.))) } else { delta },
                        touch_phase: if packet == 0 { gpui_kit::TouchPhase::Started } else { gpui_kit::TouchPhase::Moved },
                        ..Default::default()
                    }.to_platform_input(), cx);
                    window.render_frame(cx);
                    assert!((layout_anchor::bottom_distance(&state) - step * packet as f32).abs() < px(0.01),
                        "small deltas must accumulate across layouts: {delta:?}, packet {packet}, actual {:?}", layout_anchor::bottom_distance(&state));
                }
            }).unwrap();
        }
    }
}

mod window_exit {
    use super::*;
    use gpui_kit::InteractiveElement;

    struct HoverWindow {
        hovered: bool,
        exits: usize,
    }
    impl Render for HoverWindow {
        fn render(&mut self, _: &mut gpui_kit::Window, cx: &mut Context<Self>) -> impl IntoElement {
            use gpui_kit::prelude::FluentBuilder;
            div()
                .relative()
                .w(px(400.))
                .h(px(200.))
                .child(super::super::app_window::window_exit_observer(cx.listener(
                    |this, _, _, cx| {
                        this.hovered = false;
                        this.exits += 1;
                        cx.notify();
                    },
                )))
                .when(self.hovered, |root| {
                    root.child(
                        gpui_kit::deferred(
                            div()
                                .absolute()
                                .inset_0()
                                .block_mouse_except_scroll()
                                .on_mouse_move(|_, _, cx| cx.stop_propagation())
                                .on_mouse_exit(|_, _, cx| cx.stop_propagation())
                                .child("😀 👍 ❤️"),
                        )
                        .priority(10),
                    )
                })
        }
    }

    #[gpui_kit::test]
    fn window_exit_clears_hover_immediately_even_over_a_floating_strip(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (window, view) = cx.update(|cx| {
            gpui_kit::open_window(Default::default(), cx, |_, cx| {
                cx.new(|_| HoverWindow {
                    hovered: true,
                    exits: 0,
                })
            })
            .unwrap()
        });
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            let pointer = point(px(100.), px(100.));
            window.simulate_mouse_move(pointer, cx);
            assert!(view.read(cx).hovered);
            window.dispatch_event(
                gpui_kit::MouseExitEvent {
                    position: pointer,
                    ..Default::default()
                }
                .to_platform_input(),
                cx,
            );
            // No timer advance or another pointer move is needed.
            assert!(!view.read(cx).hovered);
            assert_eq!(view.read(cx).exits, 1);
            window.render_frame(cx);
            assert!(!view.read(cx).hovered);
        })
        .unwrap();
    }
}
