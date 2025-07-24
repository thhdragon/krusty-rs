//! Defines the communication channel messages between the web server and the printer task.

use super::models::PrinterStatusResponse;
use tokio::sync::oneshot;

/// Represents a request sent from a web handler to the main printer task.
#[derive(Debug)]
pub enum PrinterRequest {
    /// A request to get the current status of the printer.
    GetStatus {
        /// The channel to send the response back on.
        respond_to: oneshot::Sender<PrinterStatusResponse>,
    },
    /// A request to execute a G-code command.
    ExecuteGcode {
        command: String,
        respond_to: oneshot::Sender<Result<(), String>>,
    },
    /// Pause the current print job.
    PauseJob {
        respond_to: oneshot::Sender<Result<(), String>>,
    },
    /// Resume the current print job.
    ResumeJob {
        respond_to: oneshot::Sender<Result<(), String>>,
    },
    /// Cancel the current print job.
    CancelJob {
        respond_to: oneshot::Sender<Result<(), String>>,
    },
}