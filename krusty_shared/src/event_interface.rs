use async_trait::async_trait;

#[async_trait]
pub trait EventInterface: Send + Sync {
    type Event: Send + Sync;
    async fn send_event(&self, event: Self::Event) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;
    async fn recv_event(&self) -> Option<Self::Event>;
    fn set_timer(&self, duration: std::time::Duration, callback: Box<dyn FnOnce() + Send>);
}
