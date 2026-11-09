//! Limits how often downloads and extraction report progress.
use std::time::{Duration, Instant};

/// Minimum time between two progress reports.
const INTERVAL: Duration = Duration::from_millis(50);

pub(crate) struct Throttle {
    last: Instant,
}

impl Throttle {
    pub(crate) fn new() -> Self {
        Self { last: Instant::now() }
    }

    /// True at most once per interval. Callers report the start and the end themselves.
    pub(crate) fn ready(&mut self) -> bool {
        if self.last.elapsed() < INTERVAL {
            return false;
        }
        self.last = Instant::now();
        true
    }
}
