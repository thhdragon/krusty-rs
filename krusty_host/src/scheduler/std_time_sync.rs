use krusty_shared::TimeInterface;

/// Standard time sync abstraction using std::time.
/// Implements the TimeInterface trait using Rust's standard library.
#[derive(Debug)]
pub struct StdTimeSync;

impl TimeInterface for StdTimeSync {
    /// Returns the current monotonic time as std::time::Instant.
    fn now_monotonic(&self) -> std::time::Instant {
        std::time::Instant::now()
    }

    /// Returns the current wallclock time as std::time::SystemTime.
    fn now_wallclock(&self) -> std::time::SystemTime {
        std::time::SystemTime::now()
    }

    /// Sleeps the current thread for the specified duration.
    fn sleep(&self, duration: std::time::Duration) {
        std::thread::sleep(duration)
    }
}
