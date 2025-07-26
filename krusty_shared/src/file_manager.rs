// Shared file management types and logic for Krusty
use std::path::Path;
use std::time::SystemTime;
use thiserror::Error;
use tokio::fs;

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

#[derive(Debug, Clone)]
pub struct FileManager {
    current_directory: String,
}

impl FileManager {
    /// Create a new FileManager with the default current directory.
    pub fn new() -> Self {
        Self {
            current_directory: ".".to_string(),
        }
    }

    /// Set the current working directory for file operations.
    pub fn set_current_directory(&mut self, dir: &str) {
        self.current_directory = dir.to_string();
    }

    /// Get the current working directory for file operations.
    pub fn get_current_directory(&self) -> &str {
        &self.current_directory
    }

    /// Read a G-code file as a string. Uses current_directory if path is empty or ".".
    pub async fn read_gcode_file(&self, path: &str) -> Result<String, FileManagerError> {
        let use_path = if path.is_empty() || path == "." {
            self.current_directory.clone()
        } else {
            Path::new(&self.current_directory).join(path).to_string_lossy().to_string()
        };
        let content = fs::read_to_string(use_path).await.map_err(|e| FileManagerError::Io(e.to_string()))?;
        Ok(content)
    }

    /// Write a string to a G-code file. Uses current_directory if path is empty or ".".
    pub async fn write_gcode_file(&self, path: &str, content: &str) -> Result<(), FileManagerError> {
        let use_path = if path.is_empty() || path == "." { &self.current_directory } else { path };
        fs::write(use_path, content).await.map_err(|e| FileManagerError::Io(e.to_string()))?;
        Ok(())
    }

    /// List files in a directory. Uses current_directory if path is empty or ".".
    pub async fn list_files(&self, path: &str) -> Result<Vec<FileInfo>, FileManagerError> {
        let use_path = if path.is_empty() || path == "." { &self.current_directory } else { path };
        let mut entries = fs::read_dir(use_path).await.map_err(|e| FileManagerError::Io(e.to_string()))?;
        let mut files = Vec::new();
        while let Some(entry) = entries.next_entry().await.map_err(|e| FileManagerError::Io(e.to_string()))? {
            let path = entry.path();
            if let Some(file_name) = path.file_name() {
                if let Some(name_str) = file_name.to_str() {
                    let metadata = entry.metadata().await.map_err(|e| FileManagerError::Io(e.to_string()))?;
                    let modified = match metadata.modified() {
                        Ok(m) => m,
                        Err(_) => SystemTime::UNIX_EPOCH,
                    };
                    files.push(FileInfo {
                        name: name_str.to_string(),
                        size: metadata.len(),
                        is_directory: metadata.is_dir(),
                        modified,
                    });
                }
            }
        }
        Ok(files)
    }

    /// Read and process a G-code file, returning non-empty, non-comment lines.
    pub async fn process_gcode_file(&self, path: &str) -> Result<Vec<String>, FileManagerError> {
        let use_path = if path.is_empty() || path == "." { &self.current_directory } else { path };
        let content = self.read_gcode_file(use_path).await?;
        let lines: Vec<String> = content
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && !line.starts_with(';'))
            .collect();
        Ok(lines)
    }
}
