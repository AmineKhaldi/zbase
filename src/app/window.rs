use crate::{
    domain::{
        backend::{
            AccountId, BackendCapabilities, BackendId, ProviderConversationRef, ProviderMessageRef,
            ProviderWorkspaceRef,
        },
        conversation::ConversationSummary,
    },
    models::AppModels,
    services::{
        backends::{keybase::KeybaseBackend, router::BackendRouter},
        local_store::LocalStore,
        settings_store::SettingsStore,
        window_preferences::WindowPreferences,
    },
    state::{
        AccountState, AppStore, ConnectionState, ConversationBinding, MessageBinding, UiState,
        WorkspaceBinding,
    },
    views::{WINDOW_MIN_HEIGHT_PX, WINDOW_MIN_WIDTH_PX, app_window::AppWindow, input::TextField},
};
use gpui::{
    App, AppContext, Bounds, Focusable, TitlebarOptions, WindowBackgroundAppearance, WindowBounds,
    WindowOptions, point, px, size,
};
use std::env;
use std::sync::Arc;

const WINDOW_DEFAULT_HEIGHT_PX: f32 = 880.;

const ENV_BENCH_USE_DEMO: &str = "ZBASE_BENCH_USE_DEMO";
const ENV_BENCH_SKIP_BACKEND: &str = "ZBASE_BENCH_SKIP_BACKEND";

pub fn open_main_window(cx: &mut App) {
    let default = Bounds::centered(
        None,
        size(px(WINDOW_MIN_WIDTH_PX), px(WINDOW_DEFAULT_HEIGHT_PX)),
        cx,
    );
    let preferred = WindowPreferences::load().unwrap_or(WindowPreferences {
        x: default.origin.x.into(),
        y: default.origin.y.into(),
        width: WINDOW_MIN_WIDTH_PX,
        height: WINDOW_DEFAULT_HEIGHT_PX,
    });
    let mut displays = cx.displays();
    if let Some(primary) = cx.primary_display() {
        displays.sort_by_key(|display| display.id() != primary.id());
    }
    let displays = displays
        .iter()
        .map(|display| {
            let bounds = display.bounds();
            WindowPreferences {
                x: bounds.origin.x.into(),
                y: bounds.origin.y.into(),
                width: bounds.size.width.into(),
                height: bounds.size.height.into(),
            }
        })
        .collect::<Vec<_>>();
    let preferred = preferred.constrained(&displays, (WINDOW_MIN_WIDTH_PX, WINDOW_MIN_HEIGHT_PX));
    let bounds = Bounds::new(
        point(px(preferred.x), px(preferred.y)),
        size(px(preferred.width), px(preferred.height)),
    );
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(WINDOW_MIN_WIDTH_PX), px(WINDOW_MIN_HEIGHT_PX))),
        titlebar: Some(TitlebarOptions {
            title: None,
            appears_transparent: cfg!(target_os = "macos"),
            traffic_light_position: cfg!(target_os = "macos").then_some(point(px(18.), px(18.))),
        }),
        window_background: if cfg!(target_os = "macos") {
            WindowBackgroundAppearance::Blurred
        } else {
            WindowBackgroundAppearance::Opaque
        },
        ..Default::default()
    };

    cx.open_window(options, |window, cx| {
        let settings = SettingsStore::load_from_disk().unwrap_or_default();
        let models = if env_flag(ENV_BENCH_USE_DEMO) {
            #[cfg(any(test, feature = "demo-fixtures"))]
            {
                assert!(env_flag(ENV_BENCH_SKIP_BACKEND) && std::env::var_os("ZBASE_DATA_DIR").is_some(),
                    "Demo fixtures require ZBASE_BENCH_SKIP_BACKEND=1 and an isolated ZBASE_DATA_DIR");
                AppModels::demo_with_settings(settings)
            }
            #[cfg(not(any(test, feature = "demo-fixtures")))]
            panic!("Demo fixtures are excluded from this build; explicitly build with --features demo-fixtures for isolated benchmarks");
        } else {
            AppModels::empty_with_settings(settings)
        };
        let app_store = build_app_store(&models);
        let local_store =
            Arc::new(LocalStore::open().unwrap_or_else(|error| {
                panic!("failed to initialize local RocksDB store: {error}")
            }));
        let backend_router = build_backend_router(&models, Arc::clone(&local_store));
        let search_text = models.search.query.clone();
        let emoji_picker_text = models.emoji_picker.query.clone();
        let find_in_chat_text = models.find_in_chat.query.clone();
        let composer_text = models.composer.draft_text.clone();
        let thread_text = models.thread_pane.reply_draft.clone();
        let sidebar_filter_text = models.sidebar.filter.clone();

        let root = cx.new(|cx| {
            let search_input = cx.new(|cx| {
                TextField::new(
                    cx.focus_handle(),
                    "Search messages, people, channels",
                    search_text.clone(),
                )
            });
            let quick_switcher_input = cx.new(|cx| {
                TextField::new_with_key_context(
                    cx.focus_handle(),
                    "Jump to channel, DM, or message",
                    String::new(),
                    "QuickSwitcherTextField",
                )
            });
            let new_chat_input = cx.new(|cx| {
                TextField::new(
                    cx.focus_handle(),
                    "Search people by username",
                    String::new(),
                )
            });
            let emoji_picker_input = cx.new(|cx| {
                TextField::new(cx.focus_handle(), "Search emoji", emoji_picker_text.clone())
            });
            let file_upload_caption_input = cx.new(|cx| {
                TextField::new_multiline(cx.focus_handle(), "Add a caption...", String::new(), 2)
            });
            let find_in_chat_input = cx.new(|cx| {
                TextField::new_with_key_context(
                    cx.focus_handle(),
                    "Find in this conversation",
                    find_in_chat_text.clone(),
                    "FindInChatTextField",
                )
            });
            let composer_input = cx.new(|cx| {
                TextField::new_auto_grow(
                    cx.focus_handle(),
                    "Message this conversation",
                    composer_text.clone(),
                    8,
                )
            });
            let thread_input = cx.new(|cx| {
                TextField::new_auto_grow(
                    cx.focus_handle(),
                    "Reply in thread",
                    thread_text.clone(),
                    6,
                )
            });
            let sidebar_filter_input = cx.new(|cx| {
                TextField::new(
                    cx.focus_handle(),
                    "Filter channels and DMs",
                    sidebar_filter_text,
                )
            });

            let mut root = AppWindow::new(
                models,
                app_store,
                backend_router,
                local_store,
                cx.focus_handle(),
                quick_switcher_input,
                new_chat_input,
                emoji_picker_input,
                file_upload_caption_input,
                find_in_chat_input,
                search_input,
                composer_input,
                thread_input,
                sidebar_filter_input,
            );
            root.init(window, cx);
            root
        });
        window.focus(&root.focus_handle(cx));
        root
    })
    .expect("failed to open zbase window");
}

fn env_flag(name: &str) -> bool {
    env::var(name)
        .ok()
        .map(|raw| {
            matches!(
                raw.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn build_app_store(models: &AppModels) -> AppStore {
    let mut store = AppStore::new(UiState::with_route(models.navigation.current.clone()));
    let account_id = AccountId::new("account_demo_keybase");
    let backend_id = BackendId::new("keybase");

    store.register_account(AccountState {
        account_id: account_id.clone(),
        backend_id: backend_id.clone(),
        display_name: "Keybase".to_string(),
        avatar: None,
        connection_state: ConnectionState::Disconnected,
        capabilities: BackendCapabilities::keybase_defaults(),
    });

    for ws_id in &models.app.open_workspaces {
        store.register_workspace_binding(WorkspaceBinding {
            workspace_id: ws_id.clone(),
            backend_id: backend_id.clone(),
            account_id: account_id.clone(),
            provider_workspace_ref: ProviderWorkspaceRef::new(ws_id.0.clone()),
        });
    }

    for conversation in models
        .workspace
        .channels
        .iter()
        .chain(models.workspace.direct_messages.iter())
    {
        register_conversation_binding(&mut store, &backend_id, &account_id, conversation);
    }

    for row in &models.timeline.rows {
        let crate::models::timeline_model::TimelineRow::Message(message_row) = row else {
            continue;
        };

        let message = &message_row.message;
        store.register_message_binding(MessageBinding {
            message_id: message.id.clone(),
            backend_id: backend_id.clone(),
            account_id: account_id.clone(),
            provider_message_ref: ProviderMessageRef::new(message.id.0.clone()),
        });
    }

    if env_flag("ZBASE_BENCH_USE_DEMO") && env_flag(ENV_BENCH_SKIP_BACKEND) {
        let mut state = store.snapshot().clone();
        state.app.boot_phase = crate::state::state::BootPhase::Ready;
        state.workspace.active_workspace_id = Some(models.app.active_workspace_id.clone());
        state.workspace.workspace_name = models.workspace.workspace_name.clone();
        state.workspace.channels = models.workspace.channels.clone();
        state.workspace.direct_messages = models.workspace.direct_messages.clone();
        state.sidebar.sections = models
            .sidebar
            .sections
            .iter()
            .map(|section| crate::state::state::UiSidebarSectionState {
                id: Some(section.id.clone()),
                title: section.title.clone(),
                collapsed: section.collapsed,
                rows: section
                    .rows
                    .iter()
                    .map(|row| crate::state::state::UiSidebarRowState {
                        label: row.label.clone(),
                        unread_count: row.unread_count,
                        mention_count: row.mention_count,
                        route: Some(row.route.clone()),
                    })
                    .collect(),
            })
            .collect();
        state.sidebar.highlighted_route = models.sidebar.highlighted_route.clone();
        state.timeline.conversation_id = Some(models.timeline.conversation_id.clone());
        state.timeline.typing_text = models.timeline.typing_text.clone();
        state.timeline.messages = models
            .timeline
            .rows
            .iter()
            .filter_map(|row| match row {
                crate::models::timeline_model::TimelineRow::Message(row) => {
                    Some(row.message.clone())
                }
                _ => None,
            })
            .collect();
        store = AppStore::new(state);
    }

    store
}

fn build_backend_router(models: &AppModels, local_store: Arc<LocalStore>) -> BackendRouter {
    if env_flag(ENV_BENCH_SKIP_BACKEND) {
        tracing::warn!("bench.backend.skip enabled at router setup");
        return BackendRouter::default();
    }

    let mut router = BackendRouter::default();
    let account_id = AccountId::new("account_demo_keybase");
    let backend_id = BackendId::new("keybase");

    router.register_backend(Box::new(KeybaseBackend::new(local_store)));
    router.register_account(account_id.clone(), backend_id.clone());

    for ws_id in &models.app.open_workspaces {
        router.register_workspace_binding(WorkspaceBinding {
            workspace_id: ws_id.clone(),
            backend_id: backend_id.clone(),
            account_id: account_id.clone(),
            provider_workspace_ref: ProviderWorkspaceRef::new(ws_id.0.clone()),
        });
    }

    for conversation in models
        .workspace
        .channels
        .iter()
        .chain(models.workspace.direct_messages.iter())
    {
        router.register_conversation_binding(ConversationBinding {
            conversation_id: conversation.id.clone(),
            backend_id: backend_id.clone(),
            account_id: account_id.clone(),
            provider_conversation_ref: ProviderConversationRef::new(conversation.id.0.clone()),
        });
    }

    for row in &models.timeline.rows {
        let crate::models::timeline_model::TimelineRow::Message(message_row) = row else {
            continue;
        };

        let message = &message_row.message;
        router.register_message_binding(MessageBinding {
            message_id: message.id.clone(),
            backend_id: backend_id.clone(),
            account_id: account_id.clone(),
            provider_message_ref: ProviderMessageRef::new(message.id.0.clone()),
        });
    }

    router
}

fn register_conversation_binding(
    store: &mut AppStore,
    backend_id: &BackendId,
    account_id: &AccountId,
    conversation: &ConversationSummary,
) {
    store.register_conversation_binding(ConversationBinding {
        conversation_id: conversation.id.clone(),
        backend_id: backend_id.clone(),
        account_id: account_id.clone(),
        provider_conversation_ref: ProviderConversationRef::new(conversation.id.0.clone()),
    });
}
