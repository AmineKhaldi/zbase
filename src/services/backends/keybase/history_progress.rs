//! Catch-up checkpoints describe server operations, independently of which
//! operations can be projected as visible messages.
use std::io;

#[derive(Clone, Copy, Debug)]
pub(super) enum EnvelopeDisposition {
    Available(u64),
    PermanentlyUnavailable(u64),
    Retry,
}

#[derive(Clone, Debug, Default)]
pub(super) struct PageProgress {
    max_id: Option<u64>,
    retry: bool,
}

impl PageProgress {
    pub fn observe(&mut self, disposition: EnvelopeDisposition) {
        match disposition {
            EnvelopeDisposition::Available(id)
            | EnvelopeDisposition::PermanentlyUnavailable(id) => {
                self.max_id = Some(self.max_id.map_or(id, |previous| previous.max(id)));
            }
            EnvelopeDisposition::Retry => self.retry = true,
        }
    }

    pub fn next_position(&self, position: u64, last: bool) -> io::Result<Option<u64>> {
        if self.retry {
            return Err(io::Error::other(
                "History contains an unavailable operation; retry without advancing the checkpoint",
            ));
        }
        match self.max_id {
            Some(id) if id > position => Ok(Some(id)),
            Some(_) if !last => Err(io::Error::other("History page made no forward progress")),
            // An empty response or an explicitly terminal anchor-only page ends
            // catch-up. A nonempty nonterminal page must never report success.
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_operations_advance_but_transient_errors_never_checkpoint() {
        let mut page = PageProgress::default();
        for id in 1..=200 {
            page.observe(EnvelopeDisposition::PermanentlyUnavailable(id));
        }
        assert_eq!(page.next_position(0, false).unwrap(), Some(200));
        page.observe(EnvelopeDisposition::Available(201));
        page.observe(EnvelopeDisposition::Retry);
        assert!(page.next_position(0, true).is_err());
    }

    #[test]
    fn nonterminal_stalls_are_failures_and_empty_or_terminal_pages_complete() {
        let mut page = PageProgress::default();
        assert_eq!(page.next_position(100, false).unwrap(), None);
        page.observe(EnvelopeDisposition::Available(100));
        assert!(page.next_position(100, false).is_err());
        assert_eq!(page.next_position(100, true).unwrap(), None);
    }
}
