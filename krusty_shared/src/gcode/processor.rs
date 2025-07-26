//! Shared, hardware-agnostic GCodeProcessor core logic
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::gcode::{MacroProcessor, GCodeError, OwnedGCodeCommand};

#[derive(Debug, Clone)]
pub struct GCodeProcessor<S, M> {
    state: Arc<RwLock<S>>,
    motion_controller: Arc<RwLock<M>>,
    queue: Arc<tokio::sync::Mutex<VecDeque<String>>>,
    macros: Arc<MacroProcessor>,
}

impl<S, M> GCodeProcessor<S, M> {
    pub fn new(
        state: Arc<RwLock<S>>,
        motion_controller: Arc<RwLock<M>>,
    ) -> Self {
        Self {
            state,
            motion_controller,
            queue: Arc::new(tokio::sync::Mutex::new(VecDeque::new())),
            macros: Arc::new(MacroProcessor::new()),
        }
    }

    pub async fn enqueue_command(&self, command: String) {
        let mut queue = self.queue.lock().await;
        queue.push_back(command);
    }

    pub async fn process_next_command(&self) -> Result<(), GCodeError> {
        let mut queue = self.queue.lock().await;
        if let Some(_command) = queue.pop_front() {
            // ...full command processing logic should be implemented here...
            return Ok(());
        }
        Ok(())
    }
}
