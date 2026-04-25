//! Progress reporting abstraction shared by the `core` layer.
//!
//! The `core` layer stays intentionally unaware of how progress is rendered.
//! It only talks to a [`ProgressReporter`] trait, and any UI concern
//! (indicatif spinners, log lines, test counters, GUI indicators, …) lives
//! entirely in the consumer.
//!
//! This keeps `core` free of any terminal-rendering dependency such as
//! `indicatif`, and makes the library usable from non-CLI contexts (LSP, MCP
//! servers, GUIs) without forcing a UI crate onto them.

/// A sink for progress signals emitted by long-running `core` operations.
///
/// All methods have a no-op default body so that consumers can implement only
/// the signals they care about. Use [`NoopProgress`] when you want to silence
/// progress entirely.
///
/// The `Sync` super-trait bound lets a `&dyn ProgressReporter` be shared
/// across `rayon` worker threads (see [`crate::find_references`]).
pub trait ProgressReporter: Sync {
    /// Attach a short, human-readable phase label (e.g. `"Scanning references..."`).
    fn set_message(&self, _message: &str) {}

    /// Declare the total amount of work the current phase is about to do.
    fn set_total(&self, _total: u64) {}

    /// Report that `delta` additional units of work have been completed.
    fn inc(&self, _delta: u64) {}
}

/// A zero-cost no-op reporter used when the caller doesn't care about progress.
///
/// Pass `&NoopProgress` to any `core` function that takes `&dyn ProgressReporter`.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopProgress;

impl ProgressReporter for NoopProgress {}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    #[test]
    fn test_noop_progress_does_nothing() {
        // Should not panic and should not require any side effect.
        let reporter = NoopProgress;
        reporter.set_message("hello");
        reporter.set_total(42);
        reporter.inc(1);
    }

    #[test]
    fn test_noop_progress_works_behind_trait_object() {
        // Guards the intended usage: callers pass `&NoopProgress` as
        // `&dyn ProgressReporter` into `core` functions.
        let reporter: &dyn ProgressReporter = &NoopProgress;
        reporter.set_message("phase");
        reporter.set_total(10);
        reporter.inc(3);
    }

    /// A counting reporter used to verify that custom implementations
    /// receive the events they care about.
    #[derive(Default)]
    struct CountingReporter {
        total: AtomicU64,
        ticks: AtomicU64,
    }

    impl ProgressReporter for CountingReporter {
        fn set_total(&self, total: u64) {
            self.total.store(total, Ordering::SeqCst);
        }

        fn inc(&self, delta: u64) {
            self.ticks.fetch_add(delta, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_custom_reporter_receives_set_total_and_inc() {
        let reporter = CountingReporter::default();

        let dyn_reporter: &dyn ProgressReporter = &reporter;
        dyn_reporter.set_total(5);
        dyn_reporter.inc(2);
        dyn_reporter.inc(3);

        assert_eq!(reporter.total.load(Ordering::SeqCst), 5);
        assert_eq!(reporter.ticks.load(Ordering::SeqCst), 5);
    }

    #[test]
    fn test_custom_reporter_unimplemented_methods_are_noop() {
        // A reporter may legitimately skip `set_message`; the default body
        // guarantees that call sites don't have to special-case it.
        let reporter = CountingReporter::default();
        let dyn_reporter: &dyn ProgressReporter = &reporter;

        dyn_reporter.set_message("ignored");

        assert_eq!(reporter.total.load(Ordering::SeqCst), 0);
        assert_eq!(reporter.ticks.load(Ordering::SeqCst), 0);
    }
}
