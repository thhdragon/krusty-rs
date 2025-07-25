pub mod event_queue;

use crate::simulator::event_queue::{SimEventQueue, SimClock};
use std::sync::{Arc, Mutex};

pub struct Simulator {
    pub event_queue: Arc<Mutex<SimEventQueue>>,
    pub clock: SimClock,
    // ...existing code...
}

impl Simulator {
    pub fn new(event_queue: Arc<Mutex<SimEventQueue>>, clock: SimClock) -> Self {
        // Example: Start simulation clock and event loop
        tracing::info!("Simulator initialized at time: {:?}", clock.current_time);
        Self {
            event_queue,
            clock,
            // ...existing code...
        }
    }
    // ...existing code...
}