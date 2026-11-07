use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use crate::domain::backend::ProviderConversationRef;

#[derive(Default)]
struct PendingSync {
    running: bool,
    conversations: HashMap<String, ProviderConversationRef>,
}

/// Each backend owns its queue. Requests during a reconnect are coalesced, not
/// discarded just because an older sync task is still running.
#[derive(Default)]
pub struct SyncService {
    pending: Mutex<PendingSync>,
    cancelled: AtomicBool,
}

impl SyncService {
    pub fn request(&self, references: &[ProviderConversationRef]) -> bool {
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if self.is_cancelled() {
            return false;
        }
        for reference in references {
            pending
                .conversations
                .insert(reference.0.clone(), reference.clone());
        }
        if pending.running || pending.conversations.is_empty() {
            return false;
        }
        pending.running = true;
        true
    }

    pub fn take_pending(&self) -> Option<Vec<ProviderConversationRef>> {
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if self.is_cancelled() || pending.conversations.is_empty() {
            pending.running = false;
            pending.conversations.clear();
            return None;
        }
        Some(
            pending
                .conversations
                .drain()
                .map(|(_, reference)| reference)
                .collect(),
        )
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalesces_requests_while_running_and_restarts_after_completion() {
        let sync = SyncService::default();
        let a = ProviderConversationRef::new("a");
        let b = ProviderConversationRef::new("b");
        assert!(sync.request(std::slice::from_ref(&a)));
        assert_eq!(sync.take_pending().unwrap(), vec![a.clone()]);
        assert!(!sync.request(&[b.clone(), b.clone()]));
        assert_eq!(sync.take_pending().unwrap(), vec![b]);
        assert!(sync.take_pending().is_none());
        assert!(sync.request(&[a]));
    }

    #[test]
    fn cancellation_stops_existing_and_future_work() {
        let sync = SyncService::default();
        let a = ProviderConversationRef::new("a");
        assert!(sync.request(std::slice::from_ref(&a)));
        sync.cancel();
        assert!(sync.take_pending().is_none());
        assert!(!sync.request(&[a]));
    }
}
