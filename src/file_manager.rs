// src/file_manager.rs - Fixed file manager
use tokio::fs;

#[derive(Debug, Clone)]
pub struct FileManager {
    current_directory: String,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub is_directory: bool,
    pub modified: std::time::SystemTime,
}

impl FileManager {
    pub fn new() -> Self {
        Self {
            current_directory: ".".to_string(),
        }
    }

    pub async fn read_gcode_file(&self, path: &str) -> Result<String, Box<dyn std::error::Error>> {
        tracing::info!("Reading G-code file: {}", path);
        let content = fs::read_to_string(path).await?;
        Ok(content)
    }

    pub async fn write_gcode_file(&self, path: &str, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Writing G-code file: {}", path);
        fs::write(path, content).await?;
        Ok(())
    }

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
                        is_directory: metadata.is_dir(),
                        modified: metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                    });
                }
            }
        }
        
        Ok(files)
    }

    pub async fn process_gcode_file(&self, path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let content = self.read_gcode_file(path).await?;
        let lines: Vec<String> = content
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && !line.starts_with(';'))
            .collect();
        
        tracing::info!("Processed {} G-code lines from {}", lines.len(), path);
        Ok(lines)
    }
}