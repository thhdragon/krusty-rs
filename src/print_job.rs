// src/print_job.rs
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::file_manager::FileManager;

#[derive(Debug, Clone)]
pub struct PrintJob {
    pub id: String,
    pub file_path: String,
    pub status: PrintStatus,
    pub progress: f64,
    pub elapsed_time: f64,
    pub estimated_time: f64,
    pub current_line: usize,
    pub total_lines: usize,
}

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

pub struct PrintManager {
    current_job: Arc<RwLock<Option<PrintJob>>>,
    file_manager: FileManager,
    is_paused: bool,
}

impl PrintManager {
    pub fn new() -> Self {
        Self {
            current_job: Arc::new(RwLock::new(None)),
            file_manager: FileManager::new(),
            is_paused: false,
        }
    }

    pub async fn start_print(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
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
        
        {
            let mut job_guard = self.current_job.write().await;
            *job_guard = Some(job);
        }
        
        tracing::info!("Print job started with {} lines", total_lines);
        Ok(())
    }

    pub async fn pause_print(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut job_guard = self.current_job.write().await;
        if let Some(ref mut job) = *job_guard {
            job.status = PrintStatus::Paused;
            self.is_paused = true;
            tracing::info!("Print job paused");
        }
        Ok(())
    }

    pub async fn resume_print(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut job_guard = self.current_job.write().await;
        if let Some(ref mut job) = *job_guard {
            job.status = PrintStatus::Printing;
            self.is_paused = false;
            tracing::info!("Print job resumed");
        }
        Ok(())
    }

    pub async fn cancel_print(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut job_guard = self.current_job.write().await;
        if let Some(ref mut job) = *job_guard {
            job.status = PrintStatus::Cancelled;
            tracing::info!("Print job cancelled");
        }
        *job_guard = None;
        Ok(())
    }

    pub async fn get_current_job(&self) -> Option<PrintJob> {
        let job_guard = self.current_job.read().await;
        job_guard.clone()
    }

    pub async fn update_progress(&self, lines_processed: usize) -> Result<(), Box<dyn std::error::Error>> {
        let mut job_guard = self.current_job.write().await;
        if let Some(ref mut job) = *job_guard {
            job.current_line = lines_processed;
            job.progress = (lines_processed as f64 / job.total_lines as f64) * 100.0;
            job.elapsed_time += 0.1; // Simulate time passing
        }
        Ok(())
    }
}