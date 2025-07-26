pub trait TimeInterface: Send + Sync {
    fn now_monotonic(&self) -> std::time::Instant;
    fn now_wallclock(&self) -> std::time::SystemTime;
    fn sleep(&self, duration: std::time::Duration);
}
