// src/print_job.rs
// Print job management stubs

use crate::GCodeCommand;
use crate::gcode::GCodeError;
use std::sync::Arc;
use tokio::sync::Mutex;
use futures_core::stream::Stream;
use futures_util::StreamExt;
use crate::gcode::parser::{OwnedGCodeCommand, OwnedGCodeError};

/// Placeholder for print job state
#[derive(Debug, Clone, Default)]
pub struct PrintJobState {
    pub queued: bool,
    pub running: bool,
    pub paused: bool,
    pub completed: bool,
    pub progress: f32,
}

/// Main print job manager struct (stub)
pub struct PrintJobManager {
    pub state: PrintJobState,
    pub command_queue: Arc<Mutex<std::collections::VecDeque<Result<GCodeCommand<'static>, GCodeError>>>>,
}

impl PrintJobManager {
    pub fn new() -> Self {
        Self {
            state: PrintJobState::default(),
            command_queue: Arc::new(Mutex::new(std::collections::VecDeque::new())),
        }
    }

    /// Queue a new print job (stub)
    pub fn queue_job(&mut self, _file: &str) {
        // TODO: Implement job queueing
        self.state.queued = true;
    }

    /// Pause the current print job (stub)
    pub fn pause(&mut self) {
        // TODO: Implement job pausing
        self.state.paused = true;
    }

    /// Resume the current print job (stub)
    pub fn resume(&mut self) {
        // TODO: Implement job resuming
        self.state.paused = false;
    }

    /// Cancel the current print job (stub)
    pub fn cancel(&mut self) {
        // TODO: Implement job cancellation
        self.state.completed = true;
    }

    /// Queue a new print job from a Vec of parsed/expanded G-code commands
    pub async fn queue_commands(&mut self, commands: Vec<Result<GCodeCommand<'static>, GCodeError>>) {
        let mut queue = self.command_queue.lock().await;
        for cmd in commands {
            queue.push_back(cmd);
        }
        self.state.queued = true;
    }

    /// Async method to get the next command for processing
    pub async fn next_command(&self) -> Option<Result<GCodeCommand<'static>, GCodeError>> {
        let mut queue = self.command_queue.lock().await;
        queue.pop_front()
    }

    /// Queue a new print job from an async stream of parsed/expanded G-code commands
    pub async fn queue_from_stream<S>(&mut self, mut stream: S)
    where
        S: Stream<Item = Result<OwnedGCodeCommand, OwnedGCodeError>> + Unpin,
    {
        let mut queue = self.command_queue.lock().await;
        while let Some(cmd) = stream.next().await {
            // Convert OwnedGCodeCommand to GCodeCommand<'static> for compatibility
            let cmd = match cmd {
                Ok(owned) => Ok(match owned {
                    OwnedGCodeCommand::Word { letter, value, span } => GCodeCommand::Word { letter, value: Box::leak(value.into_boxed_str()), span },
                    OwnedGCodeCommand::Comment(comment, span) => GCodeCommand::Comment(Box::leak(comment.into_boxed_str()), span),
                    OwnedGCodeCommand::Macro { name, args, span } => GCodeCommand::Macro { name: Box::leak(name.into_boxed_str()), args: Box::leak(args.into_boxed_str()), span },
                    OwnedGCodeCommand::VendorExtension { name, args, span } => GCodeCommand::VendorExtension { name: Box::leak(name.into_boxed_str()), args: Box::leak(args.into_boxed_str()), span },
                    OwnedGCodeCommand::Checksum { command, checksum, span } => GCodeCommand::Checksum { command: Box::new(GCodeCommand::Word { letter: 'N', value: "0", span: span.clone() }), checksum, span },
                }),
                Err(e) => Ok(GCodeCommand::Comment(Box::leak(e.message.into_boxed_str()), e.span)),
            };
            queue.push_back(cmd);
        }
        self.state.queued = true;
    }
}

// TODO: Integrate PrintJobManager with motion and web API when implemented.