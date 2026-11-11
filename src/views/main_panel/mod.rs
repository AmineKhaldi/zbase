use crate::{
    domain::route::Route,
    models::AppModels,
    views::{
        MAIN_PANEL_MIN_WIDTH_PX, app_window::AppWindow, calls::CallWindow, content_surface,
        conversation::ConversationView, home::WorkspaceHomeView, inbox::InboxView,
        input::TextField, search::SearchView, text_primary,
    },
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyElement, Context, Entity, IntoElement, ListState, ParentElement, RenderImage, Styled, div,
    px, rgb,
};
use std::{collections::HashMap, sync::Arc};

#[derive(Default)]
pub struct MainPanelHost;

impl MainPanelHost {
    #[expect(
        clippy::too_many_arguments,
        reason = "Keep the explicit inputs of this existing view API."
    )]
    pub fn render(
        &self,
        models: &AppModels,
        fixed_width: Option<f32>,
        video_render_cache: &HashMap<String, Arc<RenderImage>>,
        search_input: &Entity<TextField>,
        search_contexts: &[crate::views::search::SearchResultContext],
        search_focused: bool,
        find_in_chat_input: &Entity<TextField>,
        composer_input: &Entity<TextField>,
        composer_state: crate::views::composer::ComposerFieldState,
        timeline_list_state: &ListState,
        timeline_unseen_count: usize,
        show_timeline_jump_to_bottom: bool,
        cx: &mut Context<AppWindow>,
    ) -> AnyElement {
        let content = match &models.navigation.current {
            Route::WorkspaceHome { .. } => {
                WorkspaceHomeView.render(&models.app, &models.workspace, &models.notifications, cx)
            }
            Route::Channel { .. } | Route::DirectMessage { .. } => ConversationView.render(
                &models.conversation,
                &models.timeline,
                &models.find_in_chat,
                find_in_chat_input,
                &models.composer,
                composer_input,
                composer_state,
                &models.navigation.right_pane,
                timeline_list_state,
                timeline_unseen_count,
                show_timeline_jump_to_bottom,
                &models.settings,
                &models.app.connectivity,
                cx,
            ),
            Route::Search { .. } => SearchView.render(
                models,
                search_contexts,
                search_focused,
                video_render_cache,
                search_input,
                cx,
            ),
            Route::Activity { .. } => InboxView.render(&models.notifications, cx),
            // Settings open in their own window; this route never stays current.
            Route::Preferences => crate::views::page_header("Settings", None, None),
            Route::ActiveCall { .. } => CallWindow.render(&models.call),
        };

        div()
            .flex_1()
            .when_some(fixed_width, |panel, width| {
                // GPUI's flex_none leaves flex_1's zero basis in place. Set
                // the basis too, or this frame collapses to the minimum width
                // and briefly rewraps long messages before native resizing.
                panel.flex_none().flex_basis(px(width)).w(px(width))
            })
            .min_w(px(MAIN_PANEL_MIN_WIDTH_PX))
            .h_full()
            .flex()
            .flex_col()
            .bg(content_surface())
            .overflow_hidden()
            .text_color(rgb(text_primary()))
            .child(content)
            .into_any_element()
    }
}
