// src/hardware/serial.rs - Complete serial connection implementation
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Complete serial communication handler for MCU connection
pub struct SerialConnection {
    /// The actual serial port connection
    port: SerialStream,
    
    /// Receiver for incoming responses from MCU
    response_rx: Arc<Mutex<mpsc::UnboundedReceiver<String>>>,
    
    /// Sender for outgoing commands to MCU
    command_tx: mpsc::UnboundedSender<String>,
    
    /// Connection statistics
    stats: Arc<Mutex<SerialStats>>,
    
    /// Connection configuration
    config: SerialConfig,
}

/// Serial connection statistics
#[derive(Debug, Clone, Default)]
pub struct SerialStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub commands_sent: u64,
    pub responses_received: u64,
    pub errors: u64,
    pub timeouts: u64,
}

/// Serial connection configuration
#[derive(Debug, Clone)]
pub struct SerialConfig {
    pub port_name: String,
    pub baud_rate: u32,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub buffer_size: usize,
}

impl SerialConnection {
    /// Create a new serial connection to the specified port
    pub async fn new(port_name: &str, baud_rate: u32) -> Result<Self, Box<dyn std::error::Error>> {
        let config = SerialConfig {
            port_name: port_name.to_string(),
            baud_rate,
            read_timeout: Duration::from_millis(100),
            write_timeout: Duration::from_millis(1000),
            buffer_size: 4096,
        };
        
        // Create serial port with timeout configuration
        let port = tokio_serial::new(&config.port_name, config.baud_rate)
            .timeout(config.read_timeout)
            .open_native_async()?;

        // Create channels for bidirectional communication
        let (response_tx, response_rx) = mpsc::unbounded_channel::<String>();
        let (command_tx, mut command_rx) = mpsc::unbounded_channel::<String>();
        
        // Clone the port for the reader task
        let mut read_port = port.try_clone()?;
        let response_tx_clone = response_tx.clone();

        // Spawn background task to handle incoming data from MCU
        tokio::spawn(async move {
            let mut reader = BufReader::new(read_port);
            let mut buffer = String::new();
            
            loop {
                match reader.read_line(&mut buffer).await {
                    Ok(0) => {
                        tracing::info!("Serial connection closed by remote");
                        break;
                    }
                    Ok(bytes_read) => {
                        tracing::trace!("Read {} bytes from serial", bytes_read);
                        
                        // Process each complete line
                        while let Some(line) = buffer.lines().next() {
                            let line = line.trim();
                            if !line.is_empty() {
                                tracing::debug!("Serial RX: {}", line);
                                
                                if let Err(e) = response_tx_clone.send(line.to_string()) {
                                    tracing::error!("Failed to send serial response: {}", e);
                                    break;
                                }
                            }
                        }
                        
                        buffer.clear();
                    }
                    Err(e) => {
                        tracing::error!("Serial read error: {}", e);
                        // Don't break on timeout - it's expected
                        if !e.to_string().contains("timed out") {
                            break;
                        }
                    }
                }
            }
        });

        // Clone port for writer task
        let mut write_port = port.try_clone()?;
        
        // Spawn background task to handle outgoing commands to MCU
        tokio::spawn(async move {
            while let Some(command) = command_rx.recv().await {
                let command_with_newline = format!("{}\n", command);
                tracing::debug!("Serial TX: {}", command);
                
                // Write with timeout
                match tokio::time::timeout(
                    Duration::from_secs(1),
                    write_port.write_all(command_with_newline.as_bytes())
                ).await {
                    Ok(Ok(())) => {
                        // Ensure data is flushed
                        let _ = write_port.flush().await;
                    }
                    Ok(Err(e)) => {
                        tracing::error!("Serial write error: {}", e);
                        break;
                    }
                    Err(_) => {
                        tracing::error!("Serial write timeout");
                        break;
                    }
                }
            }
            
            tracing::info!("Serial writer task terminated");
        });

        Ok(Self {
            port,
            response_rx: Arc::new(Mutex::new(response_rx)),
            command_tx,
            stats: Arc::new(Mutex::new(SerialStats::default())),
            config,
        })
    }

    /// Send a command to the MCU asynchronously
    pub async fn send_command(&self, command: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Update statistics
        {
            let mut stats_guard = self.stats.lock().await;
            stats_guard.commands_sent += 1;
            stats_guard.bytes_sent += command.len() as u64 + 1; // +1 for newline
        }
        
        // Send command through the channel
        self.command_tx.send(command.to_string())
            .map_err(|e| {
                let error_msg = format!("Failed to queue command: {}", e);
                tracing::error!("{}", error_msg);
                
                // Update error statistics
                tokio::spawn({
                    let stats = self.stats.clone();
                    async move {
                        let mut stats_guard = stats.lock().await;
                        stats_guard.errors += 1;
                    }
                });
                
                error_msg.into()
            })
    }

    /// Wait for a response from the MCU with timeout
    pub async fn wait_for_response(&mut self, timeout_ms: u64) -> Result<String, Box<dyn std::error::Error>> {
        let duration = Duration::from_millis(timeout_ms);
        
        let mut response_rx_guard = self.response_rx.lock().await;
        
        match timeout(duration, response_rx_guard.recv()).await {
            Ok(Some(response)) => {
                // Update statistics
                {
                    let mut stats_guard = self.stats.lock().await;
                    stats_guard.responses_received += 1;
                    stats_guard.bytes_received += response.len() as u64;
                }
                
                Ok(response)
            },
            Ok(None) => {
                let error_msg = "Serial connection closed";
                tracing::error!("{}", error_msg);
                
                // Update error statistics
                {
                    let mut stats_guard = self.stats.lock().await;
                    stats_guard.errors += 1;
                }
                
                Err(error_msg.into())
            },
            Err(_) => {
                let error_msg = format!("Timeout after {}ms waiting for response", timeout_ms);
                tracing::warn!("{}", error_msg);
                
                // Update timeout statistics
                {
                    let mut stats_guard = self.stats.lock().await;
                    stats_guard.timeouts += 1;
                }
                
                Err(error_msg.into())
            },
        }
    }

    /// Non-blocking attempt to receive a response
    pub fn try_recv_response(&self) -> Option<String> {
        let response_rx_guard = match self.response_rx.try_lock() {
            Ok(guard) => guard,
            Err(_) => return None, // Lock busy
        };
        
        match response_rx_guard.try_recv() {
            Ok(response) => {
                // Update statistics in background task to avoid blocking
                tokio::spawn({
                    let stats = self.stats.clone();
                    let response_len = response.len() as u64;
                    async move {
                        let mut stats_guard = stats.lock().await;
                        stats_guard.responses_received += 1;
                        stats_guard.bytes_received += response_len;
                    }
                });
                
                Some(response)
            },
            Err(_) => None,
        }
    }
    
    /// Get connection statistics
    pub async fn get_stats(&self) -> SerialStats {
        let stats_guard = self.stats.lock().await;
        stats_guard.clone()
    }
    
    /// Reset connection statistics
    pub async fn reset_stats(&self) {
        let mut stats_guard = self.stats.lock().await;
        *stats_guard = SerialStats::default();
    }
    
    /// Get connection configuration
    pub fn get_config(&self) -> &SerialConfig {
        &self.config
    }
    
    /// Clone the serial stream (limited functionality)
    pub fn try_clone(&self) -> Result<SerialStream, Box<dyn std::error::Error>> {
        self.port.try_clone()
            .map_err(|e| format!("Failed to clone serial port: {}", e).into())
    }
}

// Implement Debug for SerialConnection
impl std::fmt::Debug for SerialConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SerialConnection")
            .field("config", &self.config)
            .finish()
    }
}