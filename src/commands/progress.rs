use indicatif::{ProgressBar, ProgressStyle};

const SPINNER_TEMPLATE: &str = "{spinner:.green} [{pos}/{len}] {msg}";

/// Create an optional spinner progress bar.
///
/// Returns `Some(ProgressBar)` when `enabled` is `true`, `None` otherwise.
/// All command modules share the same spinner style, so this centralises the
/// template in one place.
pub fn create_spinner(enabled: bool) -> Option<ProgressBar> {
    if !enabled {
        return None;
    }

    let progress_bar = ProgressBar::new_spinner();
    progress_bar.set_style(ProgressStyle::with_template(SPINNER_TEMPLATE).expect("valid template"));
    Some(progress_bar)
}

/// Finish and clear the progress bar, if present.
pub fn finish(progress: &Option<ProgressBar>) {
    if let Some(progress_bar) = progress {
        progress_bar.finish_and_clear();
    }
}
