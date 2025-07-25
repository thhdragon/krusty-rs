//! Event queue and simulation clock for Krusty Simulator

use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::time::Duration;

/// Simulation event type
#[derive(Debug, Clone)]
pub enum SimEventType {
    Step,
    HeaterUpdate,
    FanUpdate,
    SensorRead,
    GCodeCommand,
    ErrorInject,
    Custom(String),
}

/// Simulation event
#[derive(Debug)]
pub struct SimEvent {
    pub timestamp: Duration,
    pub event_type: SimEventType,
    pub payload: Option<Box<dyn std::any::Any + Send>>,
}

impl PartialEq for SimEvent {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
    }
}
impl Eq for SimEvent {}
impl PartialOrd for SimEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.timestamp.cmp(&other.timestamp))
    }
}
impl Ord for SimEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

/// Event queue for simulation
#[derive(Debug)]
pub struct SimEventQueue {
    pub queue: BinaryHeap<SimEvent>,
}

impl SimEventQueue {
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
        }
    }
    pub fn push(&mut self, event: SimEvent) {
        self.queue.push(event);
    }
    pub fn pop(&mut self) -> Option<SimEvent> {
        self.queue.pop()
    }
}

/// Simulation clock
#[derive(Debug, Clone)]
pub struct SimClock {
    pub current_time: Duration,
}

impl SimClock {
    pub fn new() -> Self {
        Self {
            current_time: Duration::from_secs(0),
        }
    }
    pub fn advance(&mut self, dt: Duration) {
        self.current_time += dt;
    }
}
