use crate::domain::profile::SocialListLoadState;
use crate::views::right_pane::{pane_header, pane_with_header};
use crate::{
    domain::{
        affinity::Affinity,
        ids::UserId,
        presence::{Availability, Presence},
        profile::{ProfileSection, ProofState, SocialGraph, SocialGraphListType, UserProfile},
    },
    models::profile_panel_model::{ProfilePanelModel, SocialTab},
    views::{
        accent, affinity_broken, affinity_positive,
        app_window::AppWindow,
        avatar::{Avatar, default_avatar_background},
        border, icon_asset, link, panel_surface, primary_button, secondary_button, section_label,
        shell_border, text_primary, text_secondary, tint,
        tooltip::TextTooltip,
    },
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyElement, Context, CursorStyle, FontWeight, InteractiveElement, IntoElement, ObjectFit,
    ParentElement, ScrollHandle, StatefulInteractiveElement, Styled, StyledImage, div, img, px,
    rgb,
};

const ENV_PROFILE_DISABLE_SOCIAL_VIRTUALIZATION: &str =
    "ZBASE_PROFILE_DISABLE_SOCIAL_VIRTUALIZATION";
const SOCIAL_VIRTUALIZATION_MIN_ENTRIES: usize = 200;

pub fn render_profile_panel(
    profile_panel: &ProfilePanelModel,
    profile_scroll: &ScrollHandle,
    _profile_social_scroll: &ScrollHandle,
    cx: &mut Context<AppWindow>,
) -> AnyElement {
    let Some(user_id) = profile_panel.user_id.as_ref() else {
        return empty_profile_panel(cx);
    };
    let profile = profile_panel.profile.as_ref();
    let display_name = profile
        .map(|value| value.display_name.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or(&user_id.0)
        .to_string();
    let username = profile
        .map(|value| value.username.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or(&user_id.0)
        .to_string();
    let avatar_asset = profile.and_then(|value| value.avatar_asset.as_deref());
    let presence = profile
        .map(|value| value.presence.clone())
        .unwrap_or_else(default_presence);
    let affinity = profile
        .map(|value| value.affinity)
        .unwrap_or(Affinity::None);
    let social_graph = profile.and_then(profile_social_graph);
    let follow = social_graph.is_some_and(|graph| graph.you_are_following);
    let follow_label = if follow { "Unfollow" } else { "Follow" };
    let title = profile
        .and_then(|p| p.title.as_deref())
        .map(str::trim)
        .filter(|t| !t.is_empty());
    let bio = profile
        .and_then(|value| value.bio.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let location = profile
        .and_then(|value| value.location.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let has_bio_block = bio.is_some() || location.is_some();

    let identity_proofs = profile.map(profile_identity_proofs).unwrap_or(&[]);
    let team_showcase = profile.map(profile_team_showcase).unwrap_or(&[]);
    let custom_fields = profile.map(profile_custom_fields).unwrap_or(&[]);
    let social_list_loaded =
        social_graph.is_some_and(|graph| match profile_panel.active_social_tab {
            SocialTab::Followers => graph.followers.is_some(),
            SocialTab::Following => graph.following.is_some(),
        });
    let social_entries = social_graph_entries(social_graph, profile_panel.active_social_tab);
    let use_social_virtualization = social_entries.len() >= SOCIAL_VIRTUALIZATION_MIN_ENTRIES
        && !social_virtualization_disabled();
    let (social_start_index, social_end_index, social_top_spacer_px, social_bottom_spacer_px) =
        if use_social_virtualization {
            social_entry_window(social_entries.len(), profile_scroll)
        } else {
            (0, social_entries.len(), 0.0, 0.0)
        };
    let visible_social_entries = &social_entries[social_start_index..social_end_index];

    let presence_text = presence_label(&presence);
    let hero = div()
        .px_4()
        .pt(px(16.))
        .pb(px(12.))
        .flex()
        .flex_col()
        .items_center()
        .gap(px(12.))
        .child(Avatar::with_presence_tooltip(
            "profile-panel-presence",
            Avatar::render(
                &display_name,
                avatar_asset,
                64.,
                default_avatar_background(&display_name),
                text_primary(),
            ),
            64.,
            Some(&presence.availability),
            crate::views::right_pane_surface(),
        ))
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(2.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(17.))
                                .line_height(px(22.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(text_primary()))
                                .child(display_name.clone()),
                        )
                        .children(crate::views::identity_marker(
                            "profile-panel-identity",
                            affinity,
                            &display_name,
                            true,
                        )),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(rgb(text_secondary()))
                        .child(format!("@{username}")),
                )
                .when_some(title, |container, title| {
                    container.child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(text_secondary()))
                            .child(title.to_string()),
                    )
                })
                .when(!presence_text.is_empty(), |container| {
                    container.child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(text_secondary()))
                            .child(presence_text),
                    )
                }),
        )
        .child(
            div()
                .flex()
                .gap(px(8.))
                .child({
                    let user_id = user_id.clone();
                    primary_button("profile-open-message", "Message").on_click(cx.listener(
                        move |this, _, _, cx| {
                            this.profile_open_message(user_id.clone(), cx);
                        },
                    ))
                })
                .child({
                    let user_id = user_id.clone();
                    secondary_button("profile-follow-toggle", follow_label).on_click(cx.listener(
                        move |this, _, _, cx| {
                            if follow {
                                this.profile_unfollow_user(user_id.clone(), cx);
                            } else {
                                this.profile_follow_user(user_id.clone(), cx);
                            }
                        },
                    ))
                }),
        );

    let about = has_bio_block.then(|| {
        div()
            .flex()
            .flex_col()
            .gap(px(4.))
            .when_some(bio, |about, bio| {
                about.child(
                    div()
                        .text_size(px(13.))
                        .line_height(px(19.))
                        .text_color(rgb(text_primary()))
                        .child(bio.to_string()),
                )
            })
            .when_some(location, |about, location| {
                about.child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(text_secondary()))
                        .child(location.to_string()),
                )
            })
            .into_any_element()
    });

    let social = social_graph.map(|graph| {
        let followers_count = graph.followers_count.unwrap_or(0);
        let following_count = graph.following_count.unwrap_or(0);
        let render_entry = |index: usize, entry: &crate::domain::profile::SocialGraphEntry| {
            let entry_user_id = entry.user_id.clone();
            let entry_name = if entry.display_name.trim().is_empty() {
                entry.user_id.0.clone()
            } else {
                entry.display_name.clone()
            };
            div()
                .id(("profile-social-entry", index))
                .h(px(SOCIAL_ENTRY_ESTIMATED_HEIGHT_PX))
                // Outset so avatars line up with the section text; the hover
                // fill still gets its own padding.
                .px(px(8.))
                .mx(px(-8.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .gap(px(10.))
                .cursor(CursorStyle::PointingHand)
                .hover(|s| s.bg(tint(text_primary(), 0.05)))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_user_profile_panel(entry_user_id.clone(), window, cx);
                }))
                .child(div().flex_shrink_0().child(Avatar::render(
                    &entry_name,
                    entry.avatar_asset.as_deref(),
                    24.,
                    default_avatar_background(&entry_name),
                    text_primary(),
                )))
                .child(
                    div()
                        .min_w(px(0.))
                        .truncate()
                        .text_size(px(13.))
                        .text_color(rgb(text_primary()))
                        .child(entry_name.clone()),
                )
                .children(crate::views::identity_marker(
                    gpui_kit::SharedString::from(format!(
                        "profile-social-identity-{}",
                        entry.user_id.0
                    )),
                    entry.affinity,
                    &entry_name,
                    true,
                ))
                .into_any_element()
        };
        let list = if use_social_virtualization {
            div()
                .id("profile-social-list-scroll")
                .flex()
                .flex_col()
                .when(social_top_spacer_px > 0.0, |list| {
                    list.child(div().h(px(social_top_spacer_px)))
                })
                .children(visible_social_entries.iter().enumerate().map(
                    |(visible_offset, entry)| {
                        render_entry(social_start_index + visible_offset, entry)
                    },
                ))
                .when(social_bottom_spacer_px > 0.0, |list| {
                    list.child(div().h(px(social_bottom_spacer_px)))
                })
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .children(
                    social_entries
                        .iter()
                        .enumerate()
                        .map(|(index, entry)| render_entry(index, entry)),
                )
                .into_any_element()
        };
        let loading = !social_list_loaded
            && matches!(
                profile_panel.social_list_status,
                Some(SocialListLoadState::Loading)
            );
        let failed = !social_list_loaded
            && matches!(
                profile_panel.social_list_status,
                Some(SocialListLoadState::Failed)
            );
        let empty = (social_list_loaded || profile_panel.social_list_status.is_none())
            && social_entries.is_empty();
        let list_type = profile_panel.active_social_tab.as_list_type();
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .gap(px(16.))
                    .border_b_1()
                    .border_color(shell_border())
                    .child(social_tab(
                        "Followers",
                        followers_count,
                        profile_panel.active_social_tab == SocialTab::Followers,
                        "profile-followers-tab",
                        SocialGraphListType::Followers,
                        cx,
                    ))
                    .child(social_tab(
                        "Following",
                        following_count,
                        profile_panel.active_social_tab == SocialTab::Following,
                        "profile-following-tab",
                        SocialGraphListType::Following,
                        cx,
                    )),
            )
            .when(loading, |section| {
                section.child(quiet_note("Loading people\u{2026}"))
            })
            .when(empty, |section| {
                section.child(quiet_note(match profile_panel.active_social_tab {
                    SocialTab::Followers => "No followers yet.",
                    SocialTab::Following => "Not following anyone yet.",
                }))
            })
            .when(failed, |section| {
                section.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(quiet_note("Couldn't load this list from Keybase."))
                        .child(
                            div()
                                .id("profile-social-retry")
                                .cursor_pointer()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(link()))
                                .child("Retry")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.profile_select_social_tab(list_type, cx);
                                })),
                        ),
                )
            })
            .child(list)
            .into_any_element()
    });

    let mut sections: Vec<(&'static str, AnyElement)> = Vec::new();
    if let Some(about) = about {
        sections.push(("About", about));
    }
    if !identity_proofs.is_empty() {
        sections.push((
            "Identity proofs",
            render_proofs_section(identity_proofs, cx),
        ));
    }
    if !team_showcase.is_empty() {
        sections.push(("Teams", render_teams_section(team_showcase)));
    }
    if !custom_fields.is_empty() {
        sections.push(("Details", render_custom_fields_section(custom_fields)));
    }

    let body = div()
        .flex_1()
        .min_h(px(0.))
        .id("right-pane-profile-scroll")
        .overflow_y_scroll()
        .track_scroll(profile_scroll)
        .when(use_social_virtualization, |panel| {
            panel.on_scroll(cx.listener(AppWindow::profile_scrolled))
        })
        .scrollbar_width(px(8.))
        .flex()
        .flex_col()
        .child(hero)
        .when(profile_panel.loading && profile.is_none(), |container| {
            container
                .child(skeleton_section(vec![px(180.), px(120.)]))
                .child(skeleton_section(vec![px(140.), px(100.), px(160.)]))
        })
        .children(
            sections
                .into_iter()
                .map(|(title, content)| profile_section(Some(title), content)),
        )
        .when_some(social, |container, social| {
            container.child(profile_section(None, social))
        })
        // 16px after the last row at the end. A spacer rather than bottom
        // padding, which the scroll extent leaves out.
        .child(div().flex_shrink_0().h(px(16.)));
    // Fade the bottom edge only while more of the profile sits below it, so the
    // last visible row reads as "continues"; a profile that fits, or one
    // scrolled to the end, shows its last row unfaded.
    let more_below = profile_has_more_below(
        f32::from(profile_scroll.max_offset().y),
        f32::from(-profile_scroll.offset().y),
    );
    let body = div()
        .flex_1()
        .min_h(px(0.))
        .relative()
        .flex()
        .flex_col()
        .child(body)
        .when(more_below, |container| {
            container.child(crate::views::scroll_edge_fade_bottom(
                crate::views::right_pane_surface(),
            ))
        })
        .into_any_element();
    pane_with_header(pane_header("Profile", None, "profile-pane-close", cx), body)
}

/// Whether content continues below the visible part of a scroll area that can
/// scroll `max_offset` and has scrolled `scrolled`.
fn profile_has_more_below(max_offset: f32, scrolled: f32) -> bool {
    max_offset > 1. && scrolled < max_offset - 1.
}

/// A profile section: hairline above, 12px semibold title, content below.
fn profile_section(title: Option<&'static str>, content: AnyElement) -> AnyElement {
    div()
        .mx_4()
        .py(px(14.))
        .border_t_1()
        .border_color(shell_border())
        .flex()
        .flex_col()
        .gap(px(8.))
        .when_some(title, |section, title| section.child(section_label(title)))
        .child(content)
        .into_any_element()
}

fn quiet_note(text: &'static str) -> AnyElement {
    div()
        .py(px(4.))
        .text_size(px(13.))
        .text_color(rgb(text_secondary()))
        .child(text)
        .into_any_element()
}

pub fn render_profile_card(
    profile_panel: &ProfilePanelModel,
    user_id: &UserId,
    cx: &mut Context<AppWindow>,
) -> AnyElement {
    let profile = profile_panel
        .profile
        .as_ref()
        .filter(|profile| profile.user_id == *user_id);
    let display_name = profile
        .map(|value| value.display_name.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or(&user_id.0)
        .to_string();
    let username = profile
        .map(|value| value.username.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or(&user_id.0)
        .to_string();
    let avatar_asset = profile.and_then(|value| value.avatar_asset.as_deref());
    let presence = profile
        .map(|value| value.presence.clone())
        .unwrap_or_else(default_presence);
    let affinity = profile
        .map(|value| value.affinity)
        .unwrap_or(Affinity::None);
    let social_graph = profile.and_then(profile_social_graph);
    let follow = social_graph.is_some_and(|graph| graph.you_are_following);

    div()
        .rounded_xl()
        .bg(panel_surface())
        .border_1()
        .border_color(rgb(border()))
        .shadow(crate::views::card_shadow())
        .w(px(320.))
        .p_3()
        .flex()
        .flex_col()
        .gap_2()
        .id("profile-card")
        .on_click(cx.listener(|_, _, _, cx| {
            cx.stop_propagation();
        }))
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(Avatar::with_presence_tooltip(
                    "profile-card-presence",
                    Avatar::render(
                        &display_name,
                        avatar_asset,
                        48.,
                        default_avatar_background(&display_name),
                        text_primary(),
                    ),
                    48.,
                    Some(&presence.availability),
                    panel_surface(),
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(
                                    div()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(text_primary()))
                                        .child(display_name.clone()),
                                )
                                .children(crate::views::identity_marker(
                                    "profile-card-identity",
                                    affinity,
                                    &display_name,
                                    true,
                                )),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(text_secondary()))
                                .child(format!("@{username}")),
                        ),
                ),
        )
        .when_some(
            profile
                .and_then(|value| value.bio.as_deref())
                .map(str::trim)
                .filter(|value| !value.is_empty()),
            |container, bio| {
                container.child(
                    div()
                        .text_sm()
                        .text_color(rgb(text_secondary()))
                        .child(bio.to_string()),
                )
            },
        )
        .when_some(social_graph, |container, graph| {
            container.child(
                div()
                    .text_xs()
                    .text_color(rgb(text_secondary()))
                    .child(format!(
                        "{} followers \u{00B7} {} following",
                        graph.followers_count.unwrap_or(0),
                        graph.following_count.unwrap_or(0)
                    )),
            )
        })
        .child(
            div()
                .pt_1()
                .flex()
                .gap(px(8.))
                .child({
                    let user_id = user_id.clone();
                    primary_button("profile-card-view-full", "View profile").on_click(cx.listener(
                        move |this, _, window, cx| {
                            this.open_user_profile_panel(user_id.clone(), window, cx);
                        },
                    ))
                })
                .child({
                    let user_id = user_id.clone();
                    secondary_button(
                        "profile-card-follow-toggle",
                        if follow { "Unfollow" } else { "Follow" },
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if follow {
                            this.profile_unfollow_user(user_id.clone(), cx);
                        } else {
                            this.profile_follow_user(user_id.clone(), cx);
                        }
                    }))
                })
                .child(div().flex_1())
                .child(
                    secondary_button("profile-card-close", "Close").on_click(cx.listener(
                        |this, _, _, cx| {
                            this.dismiss_overlays(cx);
                        },
                    )),
                ),
        )
        .into_any_element()
}

fn empty_profile_panel(cx: &mut Context<AppWindow>) -> AnyElement {
    let body = div()
        .flex_1()
        .min_h(px(0.))
        .id("right-pane-profile-scroll")
        .overflow_y_scroll()
        .scrollbar_width(px(8.))
        .p_4()
        .flex()
        .flex_col()
        .gap_3()
        .child(crate::views::empty_state(
            "assets/icons/members.svg",
            "No one selected",
            Some("Click a name in a conversation to see their profile.".to_string()),
        ))
        .into_any_element();
    pane_with_header(pane_header("Profile", None, "profile-pane-close", cx), body)
}

fn skeleton_section(widths: Vec<gpui_kit::Pixels>) -> AnyElement {
    div()
        .mx_4()
        .py(px(14.))
        .border_t_1()
        .border_color(shell_border())
        .flex()
        .flex_col()
        .gap(px(10.))
        .children(widths.into_iter().map(|w| {
            div()
                .h(px(10.))
                .w(w)
                .rounded_full()
                .bg(tint(text_primary(), 0.08))
                .into_any_element()
        }))
        .into_any_element()
}

fn social_tab(
    label: &str,
    count: u32,
    selected: bool,
    id: &'static str,
    list_type: SocialGraphListType,
    cx: &mut Context<AppWindow>,
) -> AnyElement {
    div()
        .id(id)
        .pb(px(8.))
        .text_size(px(13.))
        .font_weight(FontWeight::MEDIUM)
        .cursor(CursorStyle::PointingHand)
        .text_color(rgb(if selected {
            text_primary()
        } else {
            text_secondary()
        }))
        .hover(|s| s.text_color(rgb(text_primary())))
        .when(selected, |tab| tab.border_b_2().border_color(rgb(accent())))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.profile_select_social_tab(list_type, cx);
        }))
        .child(format!("{label} · {count}"))
        .into_any_element()
}

/// One row per proof: service icon, handle, service name, and the verification
/// state as an icon with a tooltip. Opens the proof when clicked.
fn render_proofs_section(
    proofs: &[crate::domain::profile::IdentityProof],
    cx: &mut Context<AppWindow>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .children(proofs.iter().enumerate().map(|(index, proof)| {
            let target = proof.proof_url.clone().or_else(|| proof.site_url.clone());
            let status: Option<AnyElement> = match proof.state {
                ProofState::Verified => Some(status_icon(
                    ("profile-proof-status", index),
                    "assets/icons/check-circle.svg",
                    affinity_positive(),
                    "Verified",
                )),
                ProofState::Broken => Some(status_icon(
                    ("profile-proof-status", index),
                    "assets/icons/alert-triangle.svg",
                    affinity_broken(),
                    "Proof is broken",
                )),
                ProofState::Pending => Some(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(text_secondary()))
                        .child("Pending")
                        .into_any_element(),
                ),
                ProofState::Unknown => None,
            };
            let service = proof_service(&proof.service_name);
            let icon = proof
                .icon_asset
                .as_deref()
                .and_then(crate::views::avatar::image_source_for_asset)
                .map(|source| {
                    img(source)
                        .size(px(16.))
                        .flex_shrink_0()
                        .object_fit(ObjectFit::Contain)
                        .into_any_element()
                })
                .unwrap_or_else(|| icon_asset(service.icon, 16., text_primary()));
            div()
                .id(("profile-proof", index))
                .h(px(32.))
                .px(px(8.))
                .mx(px(-8.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .gap(px(10.))
                .when_some(target, |row, target| {
                    row.cursor(CursorStyle::PointingHand)
                        .hover(|s| s.bg(tint(text_primary(), 0.05)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_url_or_deep_link(&target, window, cx);
                        }))
                })
                .child(icon)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .flex()
                        .items_baseline()
                        .gap(px(6.))
                        .child(
                            div()
                                .min_w(px(0.))
                                .truncate()
                                .text_size(px(13.))
                                .text_color(rgb(text_primary()))
                                .child(proof.service_username.clone()),
                        )
                        .child(
                            div()
                                .flex_shrink_0()
                                .text_size(px(12.))
                                .text_color(rgb(text_secondary()))
                                .child(service.label),
                        ),
                )
                .children(status)
                .into_any_element()
        }))
        .into_any_element()
}

/// How a proof's service is shown: a monochrome glyph and a readable name.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ProofService {
    icon: &'static str,
    label: String,
}

/// Keybase service ids ("github", "dns", "mastodon.social") to a glyph and a
/// display name. Unknown services keep their id and a generic link glyph.
fn proof_service(service_name: &str) -> ProofService {
    let id = service_name.trim().to_ascii_lowercase();
    let (icon, label) = match id.as_str() {
        "github" => ("assets/icons/service-github.svg", "GitHub"),
        "twitter" => ("assets/icons/service-twitter.svg", "Twitter"),
        "reddit" => ("assets/icons/service-reddit.svg", "Reddit"),
        "hackernews" => ("assets/icons/service-hackernews.svg", "Hacker News"),
        "facebook" => ("assets/icons/service-facebook.svg", "Facebook"),
        "dns" | "web" | "http" | "https" => ("assets/icons/service-globe.svg", "Website"),
        "pgp" => ("assets/icons/service-key.svg", "PGP key"),
        _ => {
            return ProofService {
                icon: "assets/icons/link.svg",
                label: service_name.trim().to_string(),
            };
        }
    };
    ProofService {
        icon,
        label: label.to_string(),
    }
}

fn status_icon(
    id: impl Into<gpui_kit::ElementId>,
    icon: &'static str,
    color: u32,
    label: &'static str,
) -> AnyElement {
    div()
        .id(id)
        .flex_shrink_0()
        .child(icon_asset(icon, 14., color))
        .tooltip(TextTooltip::single_line(label.to_string()))
        .into_any_element()
}

fn render_teams_section(teams: &[crate::domain::profile::TeamShowcaseEntry]) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(10.))
        .children(teams.iter().map(|team| {
            div()
                .flex()
                .items_start()
                .gap(px(10.))
                .child(Avatar::render_team(&team.name, None, 24.))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .flex()
                                .items_baseline()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .min_w(px(0.))
                                        .truncate()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(rgb(text_primary()))
                                        .child(team.name.clone()),
                                )
                                .child(
                                    div()
                                        .flex_shrink_0()
                                        .text_size(px(12.))
                                        .text_color(rgb(text_secondary()))
                                        .child(format!(
                                            "{} {}",
                                            team.members_count,
                                            if team.members_count == 1 {
                                                "member"
                                            } else {
                                                "members"
                                            }
                                        )),
                                ),
                        )
                        .when(!team.description.trim().is_empty(), |card| {
                            card.child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .text_color(rgb(text_secondary()))
                                    .child(team.description.clone()),
                            )
                        }),
                )
                .into_any_element()
        }))
        .into_any_element()
}

fn render_custom_fields_section(fields: &[crate::domain::profile::CustomField]) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .children(fields.iter().map(|field| {
            div()
                .flex()
                .items_baseline()
                .gap(px(12.))
                .child(
                    div()
                        .w(px(96.))
                        .flex_shrink_0()
                        .text_size(px(12.))
                        .text_color(rgb(text_secondary()))
                        .child(field.label.clone()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .text_size(px(13.))
                        .text_color(rgb(text_primary()))
                        .child(field.value.clone()),
                )
                .into_any_element()
        }))
        .into_any_element()
}

fn profile_social_graph(profile: &UserProfile) -> Option<&SocialGraph> {
    profile.sections.iter().find_map(|section| match section {
        ProfileSection::SocialGraph(graph) => Some(graph),
        _ => None,
    })
}

fn social_graph_entries(
    social_graph: Option<&SocialGraph>,
    tab: SocialTab,
) -> &[crate::domain::profile::SocialGraphEntry] {
    match (social_graph, tab) {
        (Some(graph), SocialTab::Followers) => graph.followers.as_deref().unwrap_or(&[]),
        (Some(graph), SocialTab::Following) => graph.following.as_deref().unwrap_or(&[]),
        (None, _) => &[],
    }
}

const SOCIAL_ENTRY_ESTIMATED_HEIGHT_PX: f32 = 36.0;
const SOCIAL_ENTRY_INITIAL_RENDER_ROWS: usize = 180;
const SOCIAL_ENTRY_PROGRESSIVE_CHUNK_ROWS: usize = 72;
const SOCIAL_ENTRY_MAX_RENDER_ROWS: usize = 260;

fn social_entry_window(
    total_entries: usize,
    profile_scroll: &ScrollHandle,
) -> (usize, usize, f32, f32) {
    if total_entries == 0 {
        return (0, 0, 0.0, 0.0);
    }
    if total_entries <= SOCIAL_ENTRY_INITIAL_RENDER_ROWS {
        return (0, total_entries, 0.0, 0.0);
    }

    let max_offset = profile_scroll.max_offset();
    let progress = if max_offset.y <= px(0.) {
        0.0
    } else {
        let mut scroll_top = profile_scroll.offset().y.abs();
        if scroll_top > max_offset.y {
            scroll_top = max_offset.y;
        }
        (f32::from(scroll_top) / f32::from(max_offset.y)).clamp(0.0, 1.0)
    };
    let remaining = total_entries.saturating_sub(SOCIAL_ENTRY_INITIAL_RENDER_ROWS);
    let revealed = ((remaining as f32) * progress).ceil() as usize;
    let reveal_chunks = revealed.div_ceil(SOCIAL_ENTRY_PROGRESSIVE_CHUNK_ROWS);
    let revealed_rows = reveal_chunks.saturating_mul(SOCIAL_ENTRY_PROGRESSIVE_CHUNK_ROWS);
    let end = (SOCIAL_ENTRY_INITIAL_RENDER_ROWS + revealed_rows).min(total_entries);
    let start = end.saturating_sub(SOCIAL_ENTRY_MAX_RENDER_ROWS);
    let top_spacer_px = (start as f32) * SOCIAL_ENTRY_ESTIMATED_HEIGHT_PX;
    let bottom_spacer_px =
        ((total_entries.saturating_sub(end)) as f32) * SOCIAL_ENTRY_ESTIMATED_HEIGHT_PX;
    (start, end, top_spacer_px, bottom_spacer_px)
}

fn social_virtualization_disabled() -> bool {
    std::env::var(ENV_PROFILE_DISABLE_SOCIAL_VIRTUALIZATION)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn profile_identity_proofs(profile: &UserProfile) -> &[crate::domain::profile::IdentityProof] {
    profile
        .sections
        .iter()
        .find_map(|section| match section {
            ProfileSection::IdentityProofs(items) => Some(items.as_slice()),
            _ => None,
        })
        .unwrap_or(&[])
}

fn profile_team_showcase(profile: &UserProfile) -> &[crate::domain::profile::TeamShowcaseEntry] {
    profile
        .sections
        .iter()
        .find_map(|section| match section {
            ProfileSection::TeamShowcase(items) => Some(items.as_slice()),
            _ => None,
        })
        .unwrap_or(&[])
}

fn profile_custom_fields(profile: &UserProfile) -> &[crate::domain::profile::CustomField] {
    profile
        .sections
        .iter()
        .find_map(|section| match section {
            ProfileSection::CustomFields(items) => Some(items.as_slice()),
            _ => None,
        })
        .unwrap_or(&[])
}

fn presence_label(presence: &Presence) -> String {
    let base = match presence.availability {
        Availability::Active => "Active",
        Availability::Away => "Away",
        Availability::DoNotDisturb => "Do not disturb",
        Availability::Offline => "Offline",
        Availability::Unknown => "",
    };
    let status = presence.status_text.as_deref().map(str::trim).unwrap_or("");
    match (base.is_empty(), status.is_empty()) {
        (true, true) => String::new(),
        (true, false) => status.to_string(),
        (false, true) => base.to_string(),
        (false, false) => format!("{base} \u{00B7} {status}"),
    }
}

fn default_presence() -> Presence {
    Presence {
        availability: Availability::Unknown,
        status_text: None,
    }
}

#[cfg(test)]
mod bottom_fade_tests {
    use super::profile_has_more_below;

    #[test]
    fn fades_only_while_content_remains_below() {
        assert!(!profile_has_more_below(0., 0.), "fits: no fade");
        assert!(profile_has_more_below(200., 0.));
        assert!(profile_has_more_below(200., 150.));
        assert!(!profile_has_more_below(200., 200.), "at the end: no fade");
    }
}

#[cfg(test)]
mod proof_service_tests {
    use super::proof_service;

    #[test]
    fn known_services_get_their_own_glyph_and_name() {
        let github = proof_service("github");
        assert_eq!(github.label, "GitHub");
        assert!(github.icon.ends_with("service-github.svg"));
        for site in ["dns", "web", "https", "HTTP"] {
            let service = proof_service(site);
            assert_eq!(service.label, "Website", "{site}");
            assert!(service.icon.ends_with("service-globe.svg"));
        }
        assert_eq!(proof_service("hackernews").label, "Hacker News");
    }

    #[test]
    fn unknown_services_keep_their_id_and_a_link_glyph() {
        let service = proof_service("mastodon.social");
        assert_eq!(service.label, "mastodon.social");
        assert!(service.icon.ends_with("link.svg"));
    }
}
