use crate::{
    domain::user::UserSummary,
    models::new_chat_model::{NewChatModel, start_chat_label},
    views::{
        accent_strong, affinity_broken, avatar::Avatar, avatar::default_avatar_background,
        card_shadow, close_icon, empty_state, icon_asset, input::TextField, modal_surface,
        primary_button, search_field_shell, secondary_button, section_label, shell_border,
        shell_border_strong, sidebar_selected_bg, sidebar_selected_fg, sidebar_selected_secondary,
        text_primary, text_secondary, tint, tooltip::shortcut_label,
    },
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyElement, Context, Entity, FontWeight, HighlightStyle, Hsla, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, StyledText, div, px, rgb,
};

use crate::views::app_window::AppWindow;

fn display_name(user: &UserSummary) -> String {
    user.shown_name().to_string()
}

/// New conversation: people already added as removable chips, a search field,
/// then matching people. Clicking a person adds or removes them.
pub fn render_new_chat_modal(
    new_chat: &NewChatModel,
    new_chat_input: &Entity<TextField>,
    input_focused: bool,
    cx: &mut Context<AppWindow>,
) -> AnyElement {
    let chips = div().flex().flex_wrap().gap(px(6.)).children(
        new_chat
            .selected_participants
            .iter()
            .enumerate()
            .map(|(index, user)| {
                let user_id = user.id.clone();
                let name = display_name(user);
                div()
                    .h(px(26.))
                    .pl(px(3.))
                    .pr(px(1.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .rounded_full()
                    .bg(tint(accent_strong(), 0.14))
                    .child(Avatar::render(
                        &name,
                        user.avatar_asset.as_deref(),
                        20.,
                        default_avatar_background(&name),
                        text_primary(),
                    ))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(text_primary()))
                            .child(name.clone()),
                    )
                    .child(
                        div()
                            .id(("new-chat-remove", index))
                            .size(px(24.))
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|style| style.bg(tint(text_primary(), 0.10)))
                            .tooltip(crate::views::tooltip::TextTooltip::single_line(format!(
                                "Remove {name}"
                            )))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.new_chat_remove_participant(user_id.clone(), cx);
                            }))
                            .child(close_icon(text_secondary())),
                    )
                    .into_any_element()
            }),
    );

    let query = new_chat.search_query.trim();
    let highlighted_index = new_chat
        .highlighted_index
        .min(new_chat.search_results.len().saturating_sub(1));
    let results: AnyElement = if query.is_empty() {
        empty_state(
            "assets/icons/dm.svg",
            "Start a conversation",
            Some(
                "Search for people by name or Keybase username. Add more than one for a group chat."
                    .to_string(),
            ),
        )
        .into_any_element()
    } else if new_chat.search_results.is_empty() && new_chat.is_searching {
        empty_state("assets/icons/search.svg", "Searching Keybase…", None).into_any_element()
    } else if new_chat.search_results.is_empty() {
        empty_state(
            "assets/icons/search.svg",
            format!("No people match “{query}”"),
            Some("Check the spelling, or search by Keybase username.".to_string()),
        )
        .into_any_element()
    } else {
        div()
            .flex()
            .flex_col()
            .gap(px(1.))
            .child(
                section_label(if new_chat.is_searching {
                    "People · Searching Keybase…"
                } else {
                    "People"
                })
                .px(px(8.))
                .pb(px(4.)),
            )
            .children(
                new_chat
                    .search_results
                    .iter()
                    .enumerate()
                    .map(|(index, user)| {
                        let added = new_chat
                            .selected_participants
                            .iter()
                            .any(|selected_user| selected_user.id == user.id);
                        let highlighted = index == highlighted_index;
                        person_row(index, user, query, added, highlighted, cx)
                    }),
            )
            .into_any_element()
    };

    let can_start = !new_chat.selected_participants.is_empty() && !new_chat.creating;
    let start_label = start_chat_label(&new_chat.selected_participants, new_chat.creating);
    let keyboard_hint = new_chat_keyboard_hint(
        !query.is_empty() && !new_chat.search_results.is_empty(),
        can_start,
    );

    div()
        .mt(px(super::OVERLAY_TOP_PX))
        .w(px(480.))
        .max_h(px(560.))
        .rounded_xl()
        .border_1()
        .border_color(shell_border_strong())
        .bg(modal_surface())
        .shadow(card_shadow())
        .flex()
        .flex_col()
        .overflow_hidden()
        .id("overlay-new-chat")
        .on_click(cx.listener(|_, _, _, cx| {
            cx.stop_propagation();
        }))
        .child(
            div()
                .h(px(48.))
                .flex_shrink_0()
                .px_4()
                .flex()
                .items_center()
                .border_b_1()
                .border_color(shell_border())
                .text_size(px(15.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(text_primary()))
                .child("New conversation"),
        )
        .child(
            div()
                .flex_shrink_0()
                .px_4()
                .pt(px(12.))
                .pb(px(8.))
                .flex()
                .flex_col()
                .gap(px(10.))
                .when(!new_chat.selected_participants.is_empty(), |container| {
                    container.child(
                        div()
                            .flex()
                            .items_start()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .h(px(26.))
                                    .flex()
                                    .items_center()
                                    .text_size(px(13.))
                                    .text_color(rgb(text_secondary()))
                                    .child("To"),
                            )
                            .child(chips.flex_1().min_w(px(0.))),
                    )
                })
                .child(
                    search_field_shell(
                        "new-chat-input",
                        new_chat_input.clone().into_any_element(),
                        input_focused,
                        36.,
                    )
                    .on_click(cx.listener(AppWindow::focus_new_chat_input)),
                )
                .when_some(new_chat.error.as_ref(), |container, error| {
                    container.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .text_color(rgb(affinity_broken()))
                            .child(icon_asset(
                                "assets/icons/alert-triangle.svg",
                                13.,
                                affinity_broken(),
                            ))
                            .child(error.clone()),
                    )
                }),
        )
        .child(
            div()
                .flex_1()
                .min_h(px(0.))
                .px_2()
                .pb_2()
                .id("new-chat-results-scroll")
                .overflow_y_scroll()
                .on_scroll(cx.listener(AppWindow::consume_scroll_wheel))
                .child(results),
        )
        .child(
            div()
                .h(px(52.))
                .flex_shrink_0()
                .px_4()
                .border_t_1()
                .border_color(shell_border())
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .truncate()
                        .text_size(px(12.))
                        .text_color(rgb(text_secondary()))
                        .when_some(keyboard_hint, |hint, text| {
                            hint.child(text.replace("⌘↵", &shortcut_label("⌘↵")))
                        }),
                )
                .child(
                    secondary_button("new-chat-cancel", "Cancel").on_click(cx.listener(
                        |this, _, _, cx| {
                            this.dismiss_overlays(cx);
                        },
                    )),
                )
                .child(
                    primary_button("new-chat-start", start_label)
                        .when(can_start, |button| {
                            button.on_click(cx.listener(|this, _, window, cx| {
                                this.new_chat_create(window, cx);
                            }))
                        })
                        .when(!can_start, |button| button.opacity(0.45).cursor_default()),
                ),
        )
        .into_any_element()
}

/// 44px person row: avatar, name, the other name and title, and a check once added.
/// The highlighted row (Return's target) takes the accent fill, like the
/// quick switcher; letters matching the query are bold.
fn person_row(
    index: usize,
    user: &UserSummary,
    query: &str,
    added: bool,
    highlighted: bool,
    cx: &mut Context<AppWindow>,
) -> AnyElement {
    let name = display_name(user);
    let username = user.id.0.clone();
    let detail = [
        user.other_name(),
        Some(user.title.trim().to_string()).filter(|title| !title.is_empty()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");
    let (primary, secondary): (Hsla, Hsla) = if highlighted {
        (
            rgb(sidebar_selected_fg()).into(),
            sidebar_selected_secondary(),
        )
    } else {
        (rgb(text_primary()).into(), rgb(text_secondary()).into())
    };
    let surface: Hsla = if highlighted {
        rgb(sidebar_selected_bg()).into()
    } else {
        modal_surface()
    };
    // Matches are bold; on the accent-filled row, where weight alone is faint,
    // they are underlined too (never a yellow box on blue).
    let bold = HighlightStyle {
        font_weight: Some(FontWeight::BOLD),
        underline: highlighted.then(|| gpui_kit::UnderlineStyle {
            thickness: px(1.),
            color: None,
            wavy: false,
        }),
        ..Default::default()
    };
    let name_matches = matched_ranges(&name, query);
    let detail_matches = matched_ranges(&detail, query);
    let user_for_toggle = user.clone();
    div()
        .id(("new-chat-result", index))
        .h(px(44.))
        .px(px(8.))
        .rounded(px(6.))
        .flex()
        .items_center()
        .gap(px(10.))
        .cursor_pointer()
        .when(highlighted, |row| row.bg(surface))
        .when(!highlighted && added, |row| {
            row.bg(tint(accent_strong(), 0.10))
        })
        .when(!highlighted && !added, |row| {
            row.hover(|style| style.bg(tint(text_primary(), 0.06)))
        })
        .on_click(cx.listener(move |this, _, _, cx| {
            if added {
                this.new_chat_remove_participant(user_for_toggle.id.clone(), cx);
            } else {
                this.new_chat_add_participant(user_for_toggle.clone(), cx);
            }
        }))
        .child(Avatar::with_presence_tooltip(
            ("new-chat-presence", index),
            Avatar::render(
                &name,
                user.avatar_asset.as_deref(),
                28.,
                default_avatar_background(&name),
                text_primary(),
            ),
            28.,
            Some(&user.presence.availability),
            surface,
        ))
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .child(div().text_size(px(13.)).text_color(primary).child(
                            StyledText::new(name.clone()).with_highlights(
                                name_matches.into_iter().map(|range| (range, bold)),
                            ),
                        ))
                        .children(crate::views::identity_marker_tinted(
                            gpui_kit::SharedString::from(format!("new-chat-identity-{username}")),
                            user.affinity,
                            &name,
                            true,
                            highlighted.then(sidebar_selected_fg),
                        )),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(px(12.))
                        .text_color(secondary)
                        .child(StyledText::new(detail.clone()).with_highlights(
                            detail_matches.into_iter().map(|range| (range, bold)),
                        )),
                ),
        )
        .when(added, |row| {
            row.child(icon_asset(
                "assets/icons/check-circle.svg",
                16.,
                if highlighted {
                    sidebar_selected_fg()
                } else {
                    accent_strong()
                },
            ))
        })
        .into_any_element()
}

/// What Return and ⌘Return do right now: with people listed, Return adds the
/// highlighted one; ⌘Return starts the chat from anywhere.
fn new_chat_keyboard_hint(has_candidates: bool, can_start: bool) -> Option<&'static str> {
    match (has_candidates, can_start) {
        (true, _) => Some("↵ Add person · ⌘↵ Start chat"),
        (false, true) => Some("↵ Start chat"),
        (false, false) => None,
    }
}

/// Byte ranges in `text` where `query` occurs, ignoring case. Compares char by
/// char so ranges stay on `text`'s own boundaries even when case folding
/// changes byte lengths.
fn matched_ranges(text: &str, query: &str) -> Vec<std::ops::Range<usize>> {
    let needle: Vec<char> = query.trim().chars().flat_map(char::to_lowercase).collect();
    if needle.is_empty() {
        return Vec::new();
    }
    let haystack: Vec<(usize, char)> = text.char_indices().collect();
    let mut ranges = Vec::new();
    let mut start = 0;
    while start < haystack.len() {
        let mut matched = 0;
        let mut end = start;
        while end < haystack.len() && matched < needle.len() {
            let folded: Vec<char> = haystack[end].1.to_lowercase().collect();
            if needle[matched..].starts_with(&folded) {
                matched += folded.len();
                end += 1;
            } else {
                break;
            }
        }
        if matched == needle.len() {
            let byte_end = haystack.get(end).map_or(text.len(), |(offset, _)| *offset);
            ranges.push(haystack[start].0..byte_end);
            start = end;
        } else {
            start += 1;
        }
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::{matched_ranges, new_chat_keyboard_hint};

    #[test]
    fn keyboard_hint_matches_what_return_does() {
        assert_eq!(
            new_chat_keyboard_hint(true, false),
            Some("↵ Add person · ⌘↵ Start chat")
        );
        assert_eq!(new_chat_keyboard_hint(false, true), Some("↵ Start chat"));
        assert_eq!(new_chat_keyboard_hint(false, false), None);
    }

    #[test]
    fn finds_every_case_insensitive_match() {
        let text = "Alice Bellini";
        let ranges = matched_ranges(text, "LI");
        let found: Vec<&str> = ranges.iter().map(|range| &text[range.clone()]).collect();
        assert_eq!(found, ["li", "li"]);
        assert!(matched_ranges(text, "  ").is_empty());
        assert!(matched_ranges(text, "zz").is_empty());
    }

    #[test]
    fn ranges_stay_on_char_boundaries() {
        let text = "Zoë Ölund";
        let ranges = matched_ranges(text, "öl");
        assert_eq!(ranges.len(), 1);
        assert_eq!(&text[ranges[0].clone()], "Öl");
    }
}
