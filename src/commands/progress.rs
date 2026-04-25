use indicatif::{ProgressBar, ProgressStyle};
use mdref::ProgressReporter;

const SPINNER_TEMPLATE: &str = "{spinner:.green} [{pos}/{len}] {msg}";

/// A CLI-side progress handle that renders an `indicatif` spinner and exposes
/// itself as a generic [`ProgressReporter`] to the `core` layer.
///
/// The type doubles as a newtype adapter: `core` never sees `indicatif` at
/// all, and Rust's orphan rule is satisfied because `Spinner` is defined in
/// this crate. When `enabled` is `false`, the spinner is absent and all
/// reporter methods become no-ops.
///
/// Use [`Spinner::as_reporter`] when calling into `core`, and drop the
/// [`Spinner`] (or call [`Spinner::finish`]) to clear the terminal line when
/// done.
pub struct Spinner {
    bar: Option<ProgressBar>,
}

impl Spinner {
    /// Create a spinner that renders when `enabled` is `true`, or a silent
    /// no-op reporter otherwise. Both variants share the same `ProgressReporter`
    /// contract, so the caller code stays branch-free.
    pub fn new(enabled: bool) -> Self {
        if !enabled {
            return Self { bar: None };
        }

        let progress_bar = ProgressBar::new_spinner();
        progress_bar
            .set_style(ProgressStyle::with_template(SPINNER_TEMPLATE).expect("valid template"));
        Self {
            bar: Some(progress_bar),
        }
    }

    /// Borrow this spinner as a `&dyn ProgressReporter` for `core` APIs.
    pub fn as_reporter(&self) -> &dyn ProgressReporter {
        self
    }

    /// Finish and clear the underlying spinner, if any.
    pub fn finish(&self) {
        if let Some(bar) = &self.bar {
            bar.finish_and_clear();
        }
    }
}

impl ProgressReporter for Spinner {
    fn set_message(&self, message: &str) {
        if let Some(bar) = &self.bar {
            // indicatif requires an owned `Cow<'static, str>` for messages, so
            // we allocate here. This only runs a handful of times per command.
            bar.set_message(message.to_owned());
        }
    }

    fn set_total(&self, total: u64) {
        if let Some(bar) = &self.bar {
            bar.set_length(total);
        }
    }

    fn inc(&self, delta: u64) {
        if let Some(bar) = &self.bar {
            bar.inc(delta);
        }
    }
}
