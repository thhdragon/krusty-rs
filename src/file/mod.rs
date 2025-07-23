// src/file/mod.rs - File management system
use std::path::Path;
use tokio::fs;

/// File manager for 3D printer operations
pub struct FileManager {
    watch_paths: Vec<String>,
    file_cache: std::collections::HashMap<String, String>,
}

impl FileManager {
    pub fn new() -> Self {
        Self {
            watch_paths: vec!["/home/user/printer_files".to_string()],
            file_cache: std::collections::HashMap::new(),
        }
    }

    /// Read a file asynchronously
    pub async fn read_file(&self, path: &str) -> Result<String, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path).await?;
        Ok(content)
    }

    /// Write a file asynchronously
    pub async fn write_file(&self, path: &str, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(path, content).await?;
        Ok(())
    }

    /// List files in a directory
    pub async fn list_files(&self, path: &str) -> Result<Vec<FileInfo>, Box<dyn std::error::Error>> {
        let mut entries = fs::read_dir(path).await?;
        let mut files = Vec::new();
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(file_name) = path.file_name() {
                if let Some(name_str) = file_name.to_str() {
                    let metadata = entry.metadata().await?;
                    files.push(FileInfo {
                        name: name_str.to_string(),
                        size: metadata.len(),
                        modified: metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                        is_directory: metadata.is_dir(),
                    });
                }
            }
        }
        
        Ok(files)
    }

    /// Delete a file
    pub async fn delete_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::remove_file(path).await?;
        Ok(())
    }

    /// Check for file updates (for monitoring)
    pub async fn check_for_updates(&self) -> Result<(), Box<dyn std::error::Error>> {
        // In a real implementation, this would check watched directories
        // for new or modified files
        Ok(())
    }

    /// Add a path to watch
    pub fn add_watch_path(&mut self, path: String) {
        self.watch_paths.push(path);
    }

    /// Get file information
    pub async fn get_file_info(&self, path: &str) -> Result<FileInfo, Box<dyn std::error::Error>> {
        let metadata = fs::metadata(path).await?;
        let file_name = Path::new(path).file_name().unwrap_or_default().to_str().unwrap_or("").to_string();
        
        Ok(FileInfo {
            name: file_name,
            size: metadata.len(),
            modified: metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH),
            is_directory: metadata.is_dir(),
        })
    }

    /// Cache a file in memory
    pub async fn cache_file(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = self.read_file(path).await?;
        self.file_cache.insert(path.to_string(), content);
        Ok(())
    }

    /// Get cached file content
    pub fn get_cached_file(&self, path: &str) -> Option<&String> {
        self.file_cache.get(path)
    }

    /// Clear file cache
    pub fn clear_cache(&mut self) {
        self.file_cache.clear();
    }
}

impl Clone for FileManager {
    fn clone(&self) -> Self {
        Self {
            watch_paths: self.watch_paths.clone(),
            file_cache: std::collections::HashMap::new(), // Don't clone cache
        }
    }
}

/// File information structure
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub modified: std::time::SystemTime,
    pub is_directory: bool,
}