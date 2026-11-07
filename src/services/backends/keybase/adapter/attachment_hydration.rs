//! Attachment availability is checked only at the media hydration boundary.
use super::{
    KeybaseRpcClient, benchmark_read_only, download_attachment_to_cache,
    extract_video_thumbnail_to_file, is_unrenderable_keybase_asset_url,
};
use crate::domain::{
    attachment::{AttachmentKind, AttachmentPreview, AttachmentSource},
    ids::MessageId,
    message::MessageRecord,
};
use std::path::Path;

pub(super) fn message_needs_image_attachment_hydration(message: &MessageRecord) -> bool {
    let (source, preview) = image_attachment_hydration_flags(message);
    source || preview
}

pub(super) fn message_needs_video_attachment_hydration(message: &MessageRecord) -> bool {
    message.attachments.iter().any(|attachment| {
        attachment.kind == AttachmentKind::Video
            && !attachment
                .source
                .as_ref()
                .is_some_and(attachment_source_is_existing_local_file)
    })
}

pub(super) fn message_needs_file_attachment_hydration(message: &MessageRecord) -> bool {
    message.attachments.iter().any(|attachment| {
        attachment.kind == AttachmentKind::File
            && !attachment
                .source
                .as_ref()
                .is_some_and(attachment_source_is_existing_local_file)
    })
}

pub(super) fn image_attachment_hydration_flags(message: &MessageRecord) -> (bool, bool) {
    let needs_source_hydration = message.attachments.iter().any(|attachment| {
        if attachment.kind != AttachmentKind::Image {
            return false;
        }
        let source_renderable = attachment
            .source
            .as_ref()
            .is_some_and(attachment_source_renderable_for_image);
        if !source_renderable {
            return true;
        }
        // Source and preview point to the same file — source is just a copy
        // of the preview thumbnail and needs to be replaced with the full file.
        if let (Some(source), Some(preview)) = (&attachment.source, &attachment.preview) {
            return source == &preview.source;
        }
        false
    });
    let needs_preview_hydration = message.attachments.iter().any(|attachment| {
        attachment.kind == AttachmentKind::Image
            && !attachment
                .source
                .as_ref()
                .is_some_and(attachment_source_renderable_for_image)
            && !attachment
                .preview
                .as_ref()
                .is_some_and(|preview| attachment_source_renderable_for_image(&preview.source))
    });
    (needs_source_hydration, needs_preview_hydration)
}

struct AttachmentAvailability {
    source: bool,
    preview: bool,
}

pub(super) async fn hydrate_attachment_paths_for_messages(
    client: &mut KeybaseRpcClient,
    raw_conversation_id: &[u8],
    messages: &mut [MessageRecord],
    message_limit: usize,
) -> Vec<MessageId> {
    if benchmark_read_only() {
        return Vec::new();
    }
    let mut hydrated = 0usize;
    let mut updated_ids = Vec::new();
    for message in messages {
        if hydrated >= message_limit {
            break;
        }
        let (needs_source_hydration, needs_preview_hydration) =
            image_attachment_hydration_flags(message);
        let needs_video_hydration = message_needs_video_attachment_hydration(message);
        let needs_file_hydration = message_needs_file_attachment_hydration(message);
        if !needs_source_hydration
            && !needs_preview_hydration
            && !needs_video_hydration
            && !needs_file_hydration
        {
            continue;
        }
        let Ok(message_id) = message.id.0.parse::<i64>() else {
            continue;
        };

        // Capture availability before downloads: a download may recreate the
        // same cached path. Rechecking afterwards would suppress the UI update.
        let availability = message
            .attachments
            .iter()
            .map(|attachment| {
                let source = attachment.source.as_ref().is_some_and(|source| {
                    if attachment.kind == AttachmentKind::Image {
                        attachment_source_renderable_for_image(source)
                    } else {
                        attachment_source_is_existing_local_file(source)
                    }
                });
                let preview = attachment
                    .preview
                    .as_ref()
                    .is_some_and(|preview| attachment_source_renderable_for_image(&preview.source));
                AttachmentAvailability { source, preview }
            })
            .collect::<Vec<_>>();
        let needs_image_download = needs_source_hydration || needs_preview_hydration;
        let full_file_path =
            if needs_source_hydration || needs_video_hydration || needs_file_hydration {
                download_attachment_to_cache(client, raw_conversation_id, message_id, false).await
            } else {
                None
            };
        let preview_file_path = if needs_preview_hydration {
            download_attachment_to_cache(client, raw_conversation_id, message_id, true).await
        } else {
            None
        };
        if full_file_path.is_none() && preview_file_path.is_none() {
            continue;
        }

        let mut attachment_updated = false;
        hydrated = hydrated.saturating_add(1);
        for (attachment, available) in message.attachments.iter_mut().zip(availability) {
            if attachment.kind == AttachmentKind::Image && needs_image_download {
                let preview_renderable = available.preview;
                if !preview_renderable {
                    let Some(file_path) =
                        preview_file_path.clone().or_else(|| full_file_path.clone())
                    else {
                        continue;
                    };
                    attachment.preview = Some(AttachmentPreview {
                        source: AttachmentSource::LocalPath(file_path),
                        width: attachment.width,
                        height: attachment.height,
                    });
                    attachment_updated = true;
                }
                let source_is_same_as_preview = attachment
                    .source
                    .as_ref()
                    .is_some_and(|s| attachment.preview.as_ref().is_some_and(|p| s == &p.source));
                let source_renderable = !source_is_same_as_preview && available.source;
                if !source_renderable && let Some(file_path) = full_file_path.clone() {
                    attachment.source = Some(AttachmentSource::LocalPath(file_path));
                    attachment_updated = true;
                }
            } else if attachment.kind == AttachmentKind::Video && needs_video_hydration {
                let already_has_local_source = available.source;
                if already_has_local_source {
                    continue;
                }
                let Some(video_path) = full_file_path.clone() else {
                    continue;
                };
                attachment.source = Some(AttachmentSource::LocalPath(video_path.clone()));
                attachment_updated = true;

                let thumb_path = format!("{video_path}.thumb.png");
                if let Some((thumb_w, thumb_h)) =
                    extract_video_thumbnail_to_file(Path::new(&video_path), Path::new(&thumb_path))
                {
                    attachment.preview = Some(AttachmentPreview {
                        source: AttachmentSource::LocalPath(thumb_path),
                        width: Some(thumb_w),
                        height: Some(thumb_h),
                    });
                }
            } else if attachment.kind == AttachmentKind::File && needs_file_hydration {
                let already_has_local_source = available.source;
                if already_has_local_source {
                    continue;
                }
                let Some(file_path) = full_file_path.clone().or_else(|| preview_file_path.clone())
                else {
                    continue;
                };
                attachment.source = Some(AttachmentSource::LocalPath(file_path));
                attachment_updated = true;
            }
        }
        if attachment_updated {
            updated_ids.push(message.id.clone());
        }
    }
    updated_ids
}

fn attachment_source_renderable_for_image(source: &AttachmentSource) -> bool {
    match source {
        AttachmentSource::Url(url) => !is_unrenderable_keybase_asset_url(url),
        AttachmentSource::LocalPath(_) => attachment_source_is_existing_local_file(source),
    }
}

// Filesystem validation belongs to explicit media hydration, never cache decoding.
fn attachment_source_is_existing_local_file(source: &AttachmentSource) -> bool {
    matches!(source, AttachmentSource::LocalPath(path) if Path::new(path).is_file())
}
