// src/print_job.rs
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tokio::sync::RwLock;
use crate::file_manager::FileManager;
use thiserror::Error;

/// Represents a print job with its metadata and progress.
#[derive(Debug, Clone)]
pub struct PrintJob {
    /// Unique identifier for the print job.
    pub id: String,
    /// File path of the G-code file to be printed.
    pub file_path: String,
    /// Current status of the print job.
    pub status: PrintStatus,
    /// Progress of the print job in percentage.
    pub progress: f64,
    /// Elapsed time since the print job started.
    pub elapsed_time: f64,
    /// Estimated time for the print job to complete.
    pub estimated_time: f64,
    /// Current line being processed in the G-code file.
    pub current_line: usize,
    /// Total number of lines in the G-code file.
    pub total_lines: usize,
}

/// Status of a print job.
#[derive(Debug, Clone, PartialEq)]
pub enum PrintStatus {
    Idle,
    Preparing,
    Printing,
    Paused,
    Completed,
    Cancelled,
    Error,
}

#[derive(Debug, Error)]
pub enum PrintJobError {
    #[error("A print job is already active")] 
    JobAlreadyActive,
    #[error("File manager error: {0}")]
    FileManager(#[from] crate::file_manager::FileManagerError),
    #[error("Unknown error: {0}")]
    Other(String),
}

/// Manages print jobs and their state.
pub struct PrintManager {
    current_job: Arc<RwLock<Option<PrintJob>>>,
    file_manager: FileManager,
    is_paused: Arc<AtomicBool>,
}

impl PrintManager {
    /// Creates a new PrintManager.
    pub fn new() -> Self {
        Self {
            current_job: Arc::new(RwLock::new(None)),
            file_manager: FileManager::new(),
            is_paused: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Starts a new print job if none is active.
    pub async fn start_print(&mut self, file_path: &str) -> Result<(), PrintJobError> {
        let mut job_guard = self.current_job.write().await;
        if job_guard.is_some() {
            return Err(PrintJobError::JobAlreadyActive);
        }
        tracing::info!("Starting print job for file: {}", file_path);
        
        let lines = self.file_manager.process_gcode_file(file_path).await?;
        let total_lines = lines.len();
        
        let job = PrintJob {
            id: uuid::Uuid::new_v4().to_string(),
            file_path: file_path.to_string(),
            status: PrintStatus::Printing,
            progress: 0.0,
            elapsed_time: 0.0,
            estimated_time: total_lines as f64 * 0.01, // Rough estimate
            current_line: 0,
            total_lines,
        };
        
        *job_guard = Some(job);
        
        tracing::info!("Print job started with {} lines", total_lines);
        Ok(())
    }

    /// Pauses the current print job.
    pub async fn pause_print(&mut self) -> Result<(), PrintJobError> {
        let mut job_guard = self.current_job.write().await;
        if let Some(ref mut job) = *job_guard {
            job.status = PrintStatus::Paused;
            self.is_paused.store(true, Ordering::SeqCst);
            tracing::info!("Print job paused");
        }
        Ok(())
    }

    /// Resumes the current print job.
    pub async fn resume_print(&mut self) -> Result<(), PrintJobError> {
        let mut job_guard = self.current_job.write().await;
        if let Some(ref mut job) = *job_guard {
            job.status = PrintStatus::Printing;
            self.is_paused.store(false, Ordering::SeqCst);
            tracing::info!("Print job resumed");
        }
        Ok(())
    }

    /// Cancels the current print job.
    pub async fn cancel_print(&mut self) -> Result<(), PrintJobError> {
        let mut job_guard = self.current_job.write().await;
        if let Some(ref mut job) = *job_guard {
            job.status = PrintStatus::Cancelled;
            tracing::info!("Print job cancelled");
        }
        *job_guard = None;
        Ok(())
    }

    /// Gets a copy of the current print job, if any.
    pub async fn get_current_job(&self) -> Option<PrintJob> {
        let job_guard = self.current_job.read().await;
        job_guard.clone()
    }

    /// Updates the progress of the current print job.
    pub async fn update_progress(&self, lines_processed: usize) -> Result<(), PrintJobError> {
        let mut job_guard = self.current_job.write().await;
        if let Some(ref mut job) = *job_guard {
            job.current_line = lines_processed;
            let progress = (lines_processed as f64 / job.total_lines as f64) * 100.0;
            job.progress = progress.min(100.0);
            job.elapsed_time += 0.1; // Simulate time passing
        }
        Ok(())
    }
}