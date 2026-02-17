use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::watch;

/// Tracks which repos have background indexing in progress.
pub struct IndexingTracker {
    entries: Mutex<HashMap<String, watch::Receiver<bool>>>,
}

/// Handle returned by `start_indexing()`. Call `complete()` when done.
/// If dropped without calling `complete()`, the sender drops and waiters see an error.
pub struct IndexingHandle {
    tx: Option<watch::Sender<bool>>,
}

impl IndexingHandle {
    pub fn complete(mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(true);
        }
    }
}

impl Drop for IndexingHandle {
    fn drop(&mut self) {
        // If complete() wasn't called, the sender is dropped here,
        // causing waiters to see a RecvError and break out.
    }
}

impl IndexingTracker {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn is_indexing(&self, path: &str) -> bool {
        let entries = self.entries.lock().unwrap();
        if let Some(rx) = entries.get(path) {
            return !*rx.borrow();
        }
        false
    }

    /// Start tracking indexing for a repo. Returns `None` if already indexing.
    pub fn start_indexing(&self, path: &str) -> Option<IndexingHandle> {
        let mut entries = self.entries.lock().unwrap();

        // Clean up completed or dropped entries
        entries.retain(|_, rx| !*rx.borrow() && rx.has_changed().is_ok());

        if entries.contains_key(path) {
            return None;
        }

        let (tx, rx) = watch::channel(false);
        entries.insert(path.to_string(), rx);
        Some(IndexingHandle { tx: Some(tx) })
    }

    /// Get a receiver to wait for completion. Returns `None` if not indexing.
    pub fn wait_for_completion(&self, path: &str) -> Option<watch::Receiver<bool>> {
        let mut entries = self.entries.lock().unwrap();

        // Clean up completed or dropped entries
        entries.retain(|_, rx| !*rx.borrow() && rx.has_changed().is_ok());

        entries.get(path).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_and_complete() {
        let tracker = IndexingTracker::new();

        assert!(!tracker.is_indexing("/repo"));

        let handle = tracker.start_indexing("/repo").expect("should start");
        assert!(tracker.is_indexing("/repo"));

        // Can't start again while in progress
        assert!(tracker.start_indexing("/repo").is_none());

        handle.complete();
        assert!(!tracker.is_indexing("/repo"));

        // Can start again after completion
        let handle2 = tracker.start_indexing("/repo").expect("should start again");
        assert!(tracker.is_indexing("/repo"));
        handle2.complete();
    }

    #[test]
    fn test_drop_without_complete() {
        let tracker = IndexingTracker::new();

        let handle = tracker.start_indexing("/repo").expect("should start");
        assert!(tracker.is_indexing("/repo"));

        drop(handle);

        // After drop, the sender is gone — entry gets cleaned up on next call
        assert!(tracker.start_indexing("/repo").is_some());
    }

    #[tokio::test]
    async fn test_wait_for_completion() {
        let tracker = IndexingTracker::new();

        // No receiver when not indexing
        assert!(tracker.wait_for_completion("/repo").is_none());

        let handle = tracker.start_indexing("/repo").expect("should start");
        let mut rx = tracker
            .wait_for_completion("/repo")
            .expect("should get receiver");

        // Spawn completion in background
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            handle.complete();
        });

        // Wait for the value to become true
        while !*rx.borrow() {
            if rx.changed().await.is_err() {
                break;
            }
        }
        assert!(*rx.borrow(), "should have received completion signal");
    }
}
