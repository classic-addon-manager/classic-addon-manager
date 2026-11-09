//! Running blocking `cam_core` work off the UI thread, and keeping a busy indicator up long enough to be seen.
//!
//! [`run_blocking`] starts `work` on the background executor and returns its task. Await it inside `cx.spawn` or
//! `cx.spawn_in` to continue on the UI thread with the result:
//! ```ignore
//! let backend = state::backend(cx);
//! cx.spawn(async move |this, cx| {
//!     let ui = state::run_blocking(cx, move || backend.config.ui()).await;
//!     this.update(cx, |this, cx| {
//!         this.ui = ui;
//!         cx.notify();
//!     })
//! })
//! .detach();
//! ```
//! Dropping the task cancels the work if it has not started yet.
use std::time::{Duration, Instant};

use gpui_kit::{AppContext, AsyncApp, Task};

/// Shortest time a busy indicator stays up, so a click reads as feedback even when the work takes milliseconds.
pub const MIN_SPINNER: Duration = Duration::from_millis(500);

/// Runs `work` on the background executor. Works with every gpui context, sync and async.
pub fn run_blocking<R: Send + 'static>(cx: &impl AppContext, work: impl FnOnce() -> R + Send + 'static) -> Task<R> {
    cx.background_spawn(async move { work() })
}

/// Waits until [`MIN_SPINNER`] has passed since `shown_at`, the moment a busy indicator appeared. Hide the indicator
/// after this returns.
pub async fn hold_spinner(shown_at: Instant, cx: &AsyncApp) {
    let left = MIN_SPINNER.saturating_sub(shown_at.elapsed());
    if !left.is_zero() {
        cx.background_executor().timer(left).await;
    }
}

/// Converts an `anyhow::Error` into the sentence-cased message shown to users.
pub fn error_message(err: &anyhow::Error) -> String {
    sentence_case(&format!("{err:#}"))
}

pub fn sentence_case(message: &str) -> String {
    let message = message.trim();
    let mut chars = message.chars();
    match chars.next() {
        Some(c) if !c.is_uppercase() => c.to_uppercase().collect::<String>() + chars.as_str(),
        _ => message.to_string(),
    }
}
