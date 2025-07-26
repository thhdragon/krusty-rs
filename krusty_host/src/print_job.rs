use futures_core::stream::Stream;
use futures_util::StreamExt;
use krusty_shared::print_job::{JobState, PrintJobError, PrintJob as SharedPrintJob};
use krusty_shared::gcode::{GCodeCommand, GCodeError, OwnedGCodeCommand, OwnedGCodeError};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::VecDeque;
use tokio::sync::mpsc::Sender;

#[derive(Debug)]
pub struct PrintJob {
    pub inner: SharedPrintJob,
    pub commands: VecDeque<Result<GCodeCommand<'static>, GCodeError>>,
}

impl PrintJob {
    pub fn new(id: u64, commands: Vec<Result<GCodeCommand<'static>, GCodeError>>) -> Self {
        Self {
            inner: SharedPrintJob::new(id),
            commands: VecDeque::from(commands),
        }
    }
    pub fn id(&self) -> u64 { self.inner.id }
    pub fn state(&self) -> &JobState { &self.inner.state }
    pub fn state_mut(&mut self) -> &mut JobState { &mut self.inner.state }
    pub fn progress(&self) -> f32 { self.inner.progress }
    pub fn progress_mut(&mut self) -> &mut f32 { &mut self.inner.progress }
}

#[derive(Debug)]
pub struct PrintJobManager {
    pub jobs: Arc<Mutex<VecDeque<PrintJob>>>,
    pub next_id: Arc<Mutex<u64>>, // For unique job IDs
    pub command_sender: Sender<Result<GCodeCommand<'static>, GCodeError>>, // Channel to motion queue/streaming
}

impl PrintJobManager {
    pub fn new(command_sender: Sender<Result<GCodeCommand<'static>, GCodeError>>) -> Self {
        Self {
            jobs: Arc::new(Mutex::new(VecDeque::new())),
            next_id: Arc::new(Mutex::new(1)),
            command_sender,
        }
    }
    /// Process the currently running job, sending commands to the motion queue/streaming subsystem
    pub async fn process_current_job(&self) -> Result<(), PrintJobError> {
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.front_mut() {
            if job.state() == &JobState::Running {
                while let Some(cmd) = job.commands.pop_front() {
                    self.command_sender.send(cmd).await.map_err(|e| PrintJobError::ChannelSend(e.to_string()))?;
                }
                *job.state_mut() = JobState::Completed;
                Ok(())
            } else {
                Err(PrintJobError::InvalidTransition("Job is not running".to_string()))
            }
        } else {
            Err(PrintJobError::NoJob)
        }
    }

    /// Enqueue a new print job
    pub async fn enqueue_job(&self, commands: Vec<Result<GCodeCommand<'static>, GCodeError>>) -> u64 {
        let mut id_lock = self.next_id.lock().await;
        let id = *id_lock;
        *id_lock += 1;
        drop(id_lock);
        let job = PrintJob::new(id, commands);
        let mut jobs = self.jobs.lock().await;
        jobs.push_back(job);
        id
    }

    /// Dequeue the next job (set to Running)
    pub async fn start_next_job(&self) -> Result<u64, PrintJobError> {
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.front_mut() {
            match job.state() {
                JobState::Queued => {
                    *job.state_mut() = JobState::Running;
                    Ok(job.id())
                },
                _ => Err(PrintJobError::InvalidTransition("Job is not queued".to_string())),
            }
        } else {
            Err(PrintJobError::NoJob)
        }
    }

    /// Pause the currently running job
    pub async fn pause_current_job(&self) -> Result<u64, PrintJobError> {
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.front_mut() {
            match job.state() {
                JobState::Running => {
                    *job.state_mut() = JobState::Paused;
                    Ok(job.id())
                },
                JobState::Paused => Err(PrintJobError::InvalidTransition("Job is already paused".to_string())),
                JobState::Completed => Err(PrintJobError::InvalidTransition("Cannot pause a completed job".to_string())),
                JobState::Cancelled => Err(PrintJobError::InvalidTransition("Cannot pause a cancelled job".to_string())),
                _ => Err(PrintJobError::InvalidTransition("Cannot pause job in current state".to_string())),
            }
        } else {
            Err(PrintJobError::NoJob)
        }
    }

    /// Resume the currently paused job
    pub async fn resume_current_job(&self) -> Result<u64, PrintJobError> {
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.front_mut() {
            match job.state() {
                JobState::Paused => {
                    *job.state_mut() = JobState::Running;
                    Ok(job.id())
                },
                JobState::Running => Err(PrintJobError::InvalidTransition("Job is already running".to_string())),
                JobState::Completed => Err(PrintJobError::InvalidTransition("Cannot resume a completed job".to_string())),
                JobState::Cancelled => Err(PrintJobError::InvalidTransition("Cannot resume a cancelled job".to_string())),
                _ => Err(PrintJobError::InvalidTransition("Cannot resume job in current state".to_string())),
            }
        } else {
            Err(PrintJobError::NoJob)
        }
    }

    /// Cancel the currently running or paused job
    pub async fn cancel_current_job(&self) -> Result<u64, PrintJobError> {
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.front_mut() {
            match job.state() {
                JobState::Running | JobState::Paused | JobState::Queued => {
                    *job.state_mut() = JobState::Cancelled;
                    Ok(job.id())
                },
                JobState::Completed => Err(PrintJobError::InvalidTransition("Cannot cancel a completed job".to_string())),
                JobState::Cancelled => Err(PrintJobError::InvalidTransition("Job is already cancelled".to_string())),
                _ => Err(PrintJobError::InvalidTransition("Cannot cancel job in current state".to_string())),
            }
        } else {
            Err(PrintJobError::NoJob)
        }
    }

    /// Get the next command for the currently running job
    pub async fn next_command(&self) -> Result<Option<Result<GCodeCommand<'static>, GCodeError>>, PrintJobError> {
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.front_mut() {
            if job.state() == &JobState::Running {
                let cmd = job.commands.pop_front();
                if job.commands.is_empty() {
                    *job.state_mut() = JobState::Completed;
                }
                Ok(cmd)
            } else {
                Err(PrintJobError::InvalidTransition("Job is not running".to_string()))
            }
        } else {
            Err(PrintJobError::NoJob)
        }
    }

    /// Queue a new print job from an async stream of parsed/expanded G-code commands
    pub async fn enqueue_job_from_stream<S>(&self, mut stream: S) -> u64
    where
        S: Stream<Item = Result<OwnedGCodeCommand, OwnedGCodeError>> + Unpin,
    {
        let mut commands = Vec::new();
        while let Some(cmd) = stream.next().await {
            // Convert OwnedGCodeCommand to GCodeCommand<'static> for compatibility
            let cmd = match cmd {
                Ok(owned) => Ok(match owned {
                    OwnedGCodeCommand::Word { letter, value, span } => GCodeCommand::Word { letter, value: Box::leak(value.into_boxed_str()), span },
                    OwnedGCodeCommand::Comment(comment, span) => GCodeCommand::Comment(Box::leak(comment.into_boxed_str()), span),
                    OwnedGCodeCommand::Macro { name, args, span } => GCodeCommand::Macro { name: Box::leak(name.into_boxed_str()), args: Box::leak(args.into_boxed_str()), span },
                    OwnedGCodeCommand::VendorExtension { name, args, span } => GCodeCommand::VendorExtension { name: Box::leak(name.into_boxed_str()), args: Box::leak(args.into_boxed_str()), span },
                    OwnedGCodeCommand::Checksum { checksum, span, .. } => GCodeCommand::Checksum { command: Box::new(GCodeCommand::Word { letter: 'N', value: "0", span: span.clone() }), checksum, span },
                }),
                Err(e) => Ok(GCodeCommand::Comment(Box::leak(e.message.into_boxed_str()), e.span)),
            };
            commands.push(cmd);
        }
        self.enqueue_job(commands).await
    }
}

// TODO: Integrate PrintJobManager with motion and web API when implemented.