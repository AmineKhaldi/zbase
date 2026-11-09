//! Files pane state and attachment interactions.
use super::*;

impl AppWindow {
    pub(crate) fn files_pane_state(
        &self,
        conversation: &ConversationId,
    ) -> crate::views::right_pane::files::FilesPaneState {
        self.files_panes
            .get(conversation)
            .cloned()
            .unwrap_or_default()
    }

    pub(crate) fn select_files_tab(
        &mut self,
        tab: crate::views::right_pane::files::FilesTab,
        cx: &mut Context<Self>,
    ) {
        self.files_panes
            .entry(self.models.timeline.conversation_id.clone())
            .or_default()
            .tab = tab;
        self.refresh(cx);
    }

    pub(crate) fn open_files_pane(&mut self, cx: &mut Context<Self>) {
        self.files_panes
            .entry(self.models.timeline.conversation_id.clone())
            .or_default();
        self.toggle_simple_pane(RightPaneMode::Files, cx);
    }

    pub(crate) fn load_older_files(&mut self, cx: &mut Context<Self>) {
        self.request_timeline_older_page_if_needed();
        self.refresh(cx);
    }

    pub(crate) fn show_file_in_conversation(
        &mut self,
        message_id: MessageId,
        cx: &mut Context<Self>,
    ) {
        self.jump_to_find_match(message_id, cx);
    }

    pub(crate) fn open_files_attachment(
        &mut self,
        attachment: &crate::domain::attachment::AttachmentSummary,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use crate::domain::attachment::AttachmentKind;
        if attachment.kind == AttachmentKind::Image
            && let Some(source) = crate::views::attachment_lightbox_source(attachment)
        {
            self.open_image_lightbox(
                source,
                Some(attachment.name.clone()),
                attachment.width,
                attachment.height,
                cx,
            );
        } else if let Some(path) = crate::views::attachment_local_path(attachment) {
            if attachment.kind == AttachmentKind::Video || attachment.kind == AttachmentKind::Audio
            {
                Self::open_video_in_native_player(&path);
            } else {
                self.save_attachment_copy(&path, &attachment.name, cx);
            }
        } else if let Some(target) = crate::views::attachment_open_target(attachment) {
            if target.starts_with("https://") || target.starts_with("http://") {
                self.download_attachment_and_open(&target, &attachment.name, cx);
            } else {
                self.open_url_or_deep_link(&target, window, cx);
            }
        } else {
            self.notify_attachment_not_ready(cx);
        }
    }
}
