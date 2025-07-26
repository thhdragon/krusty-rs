use thiserror::Error;

#[derive(Debug, Error)]
pub enum PrintJobError {
    #[error("No job available")] 
    NoJob,
    #[error("Invalid state transition: {0}")]
    InvalidTransition(String),
    #[error("Channel send error: {0}")]
    ChannelSend(String), // Use String for cross-crate compatibility
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running,
    Paused,
    Completed,
    Cancelled,
    Error(String),
}

/// PrintJob is a shared state struct. Command queue is host/simulator-specific.
#[derive(Debug)]
pub struct PrintJob {
    pub id: u64,
    pub state: JobState,
    pub progress: f32,
    // Command queue and types are host/simulator-specific and should be added via wrapper or extension.
}

impl PrintJob {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            state: JobState::Queued,
            progress: 0.0,
        }
    }
}
// Note: Command queue and manager logic remain in host for now; only state/enums/data moved for sharing.
