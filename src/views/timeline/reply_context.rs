use super::TimelineList;
use crate::{
    domain::message::MessageRecord,
    models::timeline_model::{TimelineModel, TimelineRow},
    views::attachment_display_label,
};

pub(super) fn cached_reply_context(
    message: &MessageRecord,
    timeline: &TimelineModel,
) -> Option<String> {
    use unicode_segmentation::UnicodeSegmentation;
    let reply_to = message.reply_to.as_ref()?;
    let parent_id = message
        .thread_root_id
        .as_ref()
        .filter(|id| **id != message.id)
        .unwrap_or(reply_to);
    let find_parent = |id: &crate::domain::ids::MessageId| {
        timeline.rows.iter().find_map(|row| match row {
            TimelineRow::Message(row)
                if row.message.id == *id
                    && row.message.conversation_id == message.conversation_id =>
            {
                Some(row)
            }
            _ => None,
        })
    };
    let parent = find_parent(parent_id).or_else(|| {
        (parent_id != reply_to)
            .then(|| find_parent(reply_to))
            .flatten()
    })?;
    if matches!(
        parent.message.event,
        Some(crate::domain::message::ChatEvent::MessageDeleted { .. })
    ) {
        return None;
    }
    let text = TimelineList::message_caption_text(&parent.message)
        .or_else(|| {
            parent
                .message
                .attachments
                .first()
                .map(|attachment| attachment_display_label(attachment))
        })
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut graphemes = text.graphemes(true);
    let mut preview = graphemes.by_ref().take(100).collect::<String>();
    if graphemes.next().is_some() {
        preview.push('…');
    }
    Some(if preview.is_empty() {
        format!("Reply to {}", parent.author.display_name)
    } else {
        format!("Reply to {}: {}", parent.author.display_name, preview)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            affinity::Affinity,
            ids::{ConversationId, MessageId, UserId},
            message::{MessageRecord, MessageFragment, MessageSendState},
            presence::{Availability, Presence},
            user::UserSummary,
        },
        models::timeline_model::MessageRow,
    };
    use unicode_segmentation::UnicodeSegmentation;

    fn row(id: &str, text: &str) -> MessageRow {
        MessageRow {
            author: UserSummary {
                id: UserId::new("alice"),
                display_name: "Alice".into(),
                title: String::new(),
                avatar_asset: None,
                presence: Presence {
                    availability: Availability::Unknown,
                    status_text: None,
                },
                affinity: Affinity::None,
            },
            message: MessageRecord {
                id: MessageId::new(id),
                conversation_id: ConversationId::new("conversation"),
                author_id: UserId::new("alice"),
                reply_to: None,
                thread_root_id: None,
                timestamp_ms: None,
                event: None,
                link_previews: vec![],
                permalink: String::new(),
                fragments: vec![MessageFragment::Text(text.into())],
                source_text: None,
                attachments: vec![],
                reactions: vec![],
                thread_reply_count: 0,
                send_state: MessageSendState::Sent,
                edited: None,
            },
            show_header: true,
        }
    }

    #[test]
    fn reply_context_uses_cached_parent_text_and_never_leaks_another_conversation() {
        let parent = row("1", "What are the limits?\n  Any details?");
        let mut reply = row("2", "Here is the answer").message;
        reply.reply_to = Some(MessageId::new("1"));
        reply.thread_root_id = Some(MessageId::new("1"));
        let mut timeline = TimelineModel {
            rows: vec![TimelineRow::Message(parent.clone())],
            ..Default::default()
        };
        assert_eq!(
            cached_reply_context(&reply, &timeline).as_deref(),
            Some("Reply to Alice: What are the limits? Any details?")
        );
        assert_eq!(cached_reply_context(&parent.message, &timeline), None);
        reply.thread_root_id = Some(MessageId::new("uncached-root"));
        assert!(
            cached_reply_context(&reply, &timeline).is_some(),
            "a cached immediate parent supplies context when the root is outside the loaded window"
        );
        reply.conversation_id = ConversationId::new("other");
        assert_eq!(cached_reply_context(&reply, &timeline), None);
        reply.conversation_id = ConversationId::new("conversation");
        timeline.rows.clear();
        assert_eq!(cached_reply_context(&reply, &timeline), None);
    }

    #[test]
    fn reply_preview_truncation_preserves_complete_emoji_graphemes() {
        let parent = row("1", &"👩🏽‍💻".repeat(101));
        let mut reply = row("2", "reply").message;
        reply.reply_to = Some(MessageId::new("1"));
        let timeline = TimelineModel {
            rows: vec![TimelineRow::Message(parent)],
            ..Default::default()
        };
        let preview = cached_reply_context(&reply, &timeline).unwrap();
        let text = preview.strip_prefix("Reply to Alice: ").unwrap();
        assert_eq!(text.graphemes(true).count(), 101);
        assert!(text.ends_with('…'));
    }
}
