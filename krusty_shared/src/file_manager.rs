// Shared file management types for Krusty
use std::time::SystemTime;
use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum FileManagerError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("UTF-8 error: {0}")]
    Utf8(String),
    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub is_directory: bool,
    pub modified: SystemTime,
}
