use async_trait::async_trait;
use tokio::sync::mpsc::{Sender, Receiver, channel};
use tokio::time::{sleep, Duration};
use tokio::sync::Mutex;
use crate::communication::event_interface::EventInterface;

/// Event bus stub
pub struct EventBusStub;
impl EventBusStub {
    pub fn new() -> Self { Self }
    // Add stub methods as needed
}

/// Tokio-based event system abstraction
pub struct TokioEventSystem {
    sender: Sender<String>,
    receiver: Mutex<Receiver<String>>,
}

impl TokioEventSystem {
    pub fn new(buffer: usize) -> Self {
        let (sender, receiver) = channel(buffer);
        Self { sender, receiver: Mutex::new(receiver) }
    }
}

#[async_trait]
impl EventInterface for TokioEventSystem {
    type Event = String;
    async fn send_event(&self, event: Self::Event) -> Result<(), Box<dyn std::error::Error>> {
        self.sender.send(event).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
    async fn recv_event(&self) -> Option<Self::Event> {
        let mut rx = self.receiver.lock().await;
        rx.recv().await
    }
    fn set_timer(&self, duration: std::time::Duration, callback: Box<dyn FnOnce() + Send>) {
        tokio::spawn(async move {
            sleep(Duration::from_secs(duration.as_secs())).await;
            callback();
        });
    }
}
