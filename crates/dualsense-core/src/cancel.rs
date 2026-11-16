use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Thread-safe cooperative cancellation token shared between the UI and background workers.
#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Creates a new uncancelled token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Signals cancellation to any operation observing this token.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// Clears the cancellation flag so the token can be reused.
    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::Release);
    }

    /// Returns `true` if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::CancellationToken;

    #[test]
    fn token_starts_uncancelled_and_propagates_across_clones() {
        let token = CancellationToken::new();
        let clone = token.clone();
        assert!(!token.is_cancelled());
        assert!(!clone.is_cancelled());

        token.cancel();
        assert!(token.is_cancelled());
        assert!(clone.is_cancelled());

        clone.reset();
        assert!(!token.is_cancelled());
        assert!(!clone.is_cancelled());
    }
}
