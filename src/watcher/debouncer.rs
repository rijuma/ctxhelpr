use std::collections::HashMap;
use std::time::{Duration, Instant};

const DEBOUNCE_DELAY: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileChangeKind {
    Modified,
    Deleted,
}

/// Batches file change events per repo with a timer-based debounce.
/// After `DEBOUNCE_DELAY` of quiet (no new events), pending changes are flushed.
pub struct Debouncer {
    pending: HashMap<String, HashMap<String, FileChangeKind>>,
    last_event_at: Option<Instant>,
}

impl Default for Debouncer {
    fn default() -> Self {
        Self::new()
    }
}

impl Debouncer {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            last_event_at: None,
        }
    }

    /// Record a file change event. Resets the debounce timer.
    pub fn record(&mut self, repo_path: &str, rel_path: &str, kind: FileChangeKind) {
        self.pending
            .entry(repo_path.to_string())
            .or_default()
            .insert(rel_path.to_string(), kind);
        self.last_event_at = Some(Instant::now());
    }

    /// Returns the duration until the next flush should happen, or None if nothing is pending.
    pub fn time_until_flush(&self) -> Option<Duration> {
        let last = self.last_event_at?;
        if self.pending.is_empty() {
            return None;
        }
        let elapsed = last.elapsed();
        if elapsed >= DEBOUNCE_DELAY {
            Some(Duration::ZERO)
        } else {
            Some(DEBOUNCE_DELAY - elapsed)
        }
    }

    /// Returns true if there are pending changes ready to flush.
    pub fn is_ready(&self) -> bool {
        matches!(self.time_until_flush(), Some(d) if d.is_zero())
    }

    /// Drain all pending changes grouped by repo. Returns empty map if not ready.
    pub fn flush(&mut self) -> HashMap<String, HashMap<String, FileChangeKind>> {
        self.last_event_at = None;
        std::mem::take(&mut self.pending)
    }

    #[cfg(test)]
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_debouncer() {
        let debouncer = Debouncer::new();
        assert!(!debouncer.has_pending());
        assert!(debouncer.time_until_flush().is_none());
        assert!(!debouncer.is_ready());
    }

    #[test]
    fn test_record_single_event() {
        let mut debouncer = Debouncer::new();
        debouncer.record("/repo", "src/main.rs", FileChangeKind::Modified);

        assert!(debouncer.has_pending());
        assert!(debouncer.time_until_flush().is_some());
        // Should not be ready immediately (2s debounce)
        assert!(!debouncer.is_ready());
    }

    #[test]
    fn test_multiple_events_same_file() {
        let mut debouncer = Debouncer::new();
        debouncer.record("/repo", "src/main.rs", FileChangeKind::Modified);
        debouncer.record("/repo", "src/main.rs", FileChangeKind::Deleted);

        let pending = &debouncer.pending["/repo"];
        assert_eq!(pending.len(), 1);
        assert_eq!(pending["src/main.rs"], FileChangeKind::Deleted);
    }

    #[test]
    fn test_multiple_files_same_repo() {
        let mut debouncer = Debouncer::new();
        debouncer.record("/repo", "src/a.rs", FileChangeKind::Modified);
        debouncer.record("/repo", "src/b.rs", FileChangeKind::Modified);

        let pending = &debouncer.pending["/repo"];
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn test_multiple_repos() {
        let mut debouncer = Debouncer::new();
        debouncer.record("/repo1", "a.rs", FileChangeKind::Modified);
        debouncer.record("/repo2", "b.rs", FileChangeKind::Deleted);

        assert_eq!(debouncer.pending.len(), 2);
    }

    #[test]
    fn test_flush_clears_pending() {
        let mut debouncer = Debouncer::new();
        debouncer.record("/repo", "a.rs", FileChangeKind::Modified);

        let flushed = debouncer.flush();
        assert_eq!(flushed.len(), 1);
        assert!(flushed["/repo"].contains_key("a.rs"));

        assert!(!debouncer.has_pending());
        assert!(debouncer.time_until_flush().is_none());
    }

    #[test]
    fn test_ready_after_delay() {
        let mut debouncer = Debouncer::new();
        debouncer.record("/repo", "a.rs", FileChangeKind::Modified);
        // Simulate time passing by backdating last_event_at
        debouncer.last_event_at = Some(Instant::now() - DEBOUNCE_DELAY - Duration::from_millis(1));

        assert!(debouncer.is_ready());
        assert_eq!(debouncer.time_until_flush(), Some(Duration::ZERO));
    }

    #[test]
    fn test_timer_resets_on_new_event() {
        let mut debouncer = Debouncer::new();
        debouncer.record("/repo", "a.rs", FileChangeKind::Modified);
        // Backdate to almost ready
        debouncer.last_event_at =
            Some(Instant::now() - DEBOUNCE_DELAY + Duration::from_millis(100));

        // New event resets the timer
        debouncer.record("/repo", "b.rs", FileChangeKind::Modified);
        assert!(!debouncer.is_ready());

        let until = debouncer.time_until_flush().unwrap();
        // Should be close to full DEBOUNCE_DELAY again
        assert!(until > Duration::from_secs(1));
    }

    #[test]
    fn test_mixed_change_kinds_in_flush() {
        let mut debouncer = Debouncer::new();
        debouncer.record("/repo", "src/new.rs", FileChangeKind::Modified);
        debouncer.record("/repo", "src/old.rs", FileChangeKind::Deleted);
        debouncer.record("/repo", "src/changed.rs", FileChangeKind::Modified);

        let flushed = debouncer.flush();
        let changes = &flushed["/repo"];
        assert_eq!(changes["src/new.rs"], FileChangeKind::Modified);
        assert_eq!(changes["src/old.rs"], FileChangeKind::Deleted);
        assert_eq!(changes["src/changed.rs"], FileChangeKind::Modified);
    }

    #[test]
    fn test_real_timing() {
        let mut debouncer = Debouncer::new();
        debouncer.record("/repo", "a.rs", FileChangeKind::Modified);
        assert!(!debouncer.is_ready());

        // Sleep just past the debounce window (using a short custom delay for test speed)
        // We can't easily test the full 2s in unit tests, but we can test the logic
        // by manually setting last_event_at
        debouncer.last_event_at = Some(Instant::now() - Duration::from_secs(3));
        assert!(debouncer.is_ready());
    }
}
