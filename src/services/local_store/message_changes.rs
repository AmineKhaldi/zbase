//! Bounded invalidations, not a second in-memory copy of message history.
//! Overflow invalidates the subscription: the consumer must drop it and replay
//! a fresh snapshot. Writers never wait for an index consumer.
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError},
};
use std::time::Duration;

use crate::domain::ids::{ConversationId, MessageId};

const CAPACITY: usize = 1024;

#[derive(Clone, Debug)]
pub enum MessageChange {
    Dirty {
        conversation_id: ConversationId,
        message_id: MessageId,
    },
    ReplayRequired,
}

struct Subscriber {
    sender: SyncSender<MessageChange>,
    overflowed: Arc<AtomicBool>,
}

pub struct MessageChanges {
    receiver: Receiver<MessageChange>,
    overflowed: Arc<AtomicBool>,
}

impl MessageChanges {
    pub fn recv_timeout(&self, timeout: Duration) -> Result<MessageChange, RecvTimeoutError> {
        if self.overflowed.load(Ordering::Acquire) {
            return Ok(MessageChange::ReplayRequired);
        }
        let result = self.receiver.recv_timeout(timeout);
        // A reset/overflow during the wait takes precedence over queued events.
        if self.overflowed.load(Ordering::Acquire) {
            return Ok(MessageChange::ReplayRequired);
        }
        result
    }
}

#[derive(Default)]
pub(super) struct MessageWrites {
    subscribers: Vec<Subscriber>,
}

impl MessageWrites {
    pub fn subscribe(&mut self) -> MessageChanges {
        let (sender, receiver) = mpsc::sync_channel(CAPACITY);
        let overflowed = Arc::new(AtomicBool::new(false));
        self.subscribers.push(Subscriber {
            sender,
            overflowed: Arc::clone(&overflowed),
        });
        MessageChanges {
            receiver,
            overflowed,
        }
    }

    pub fn changed(&mut self, conversation_id: &ConversationId, message_id: &MessageId) {
        let change = MessageChange::Dirty {
            conversation_id: conversation_id.clone(),
            message_id: message_id.clone(),
        };
        self.subscribers.retain(
            |subscriber| match subscriber.sender.try_send(change.clone()) {
                Ok(()) => true,
                Err(TrySendError::Full(_)) => {
                    subscriber.overflowed.store(true, Ordering::Release);
                    true
                }
                Err(TrySendError::Disconnected(_)) => false,
            },
        );
    }

    pub fn reset(&mut self) {
        for subscriber in &self.subscribers {
            subscriber.overflowed.store(true, Ordering::Release);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slow_consumer_is_bounded_and_must_replay_after_overflow() {
        let mut writes = MessageWrites::default();
        let changes = writes.subscribe();
        let conversation = ConversationId::new("conversation");
        for id in 0..CAPACITY * 10 {
            writes.changed(&conversation, &MessageId::new(id.to_string()));
        }
        assert_eq!(changes.receiver.try_iter().count(), CAPACITY);
        assert!(matches!(
            changes.recv_timeout(Duration::ZERO).unwrap(),
            MessageChange::ReplayRequired
        ));
        drop(changes);
        let fresh = writes.subscribe();
        writes.changed(&conversation, &MessageId::new("latest"));
        assert!(
            matches!(fresh.recv_timeout(Duration::ZERO).unwrap(), MessageChange::Dirty { message_id, .. } if message_id.0 == "latest")
        );
        assert_eq!(writes.subscribers.len(), 1);
        writes.reset();
        assert!(matches!(
            fresh.recv_timeout(Duration::ZERO).unwrap(),
            MessageChange::ReplayRequired
        ));
    }
}
