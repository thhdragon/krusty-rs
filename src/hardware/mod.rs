// src/hardware/mod.rs - Complete hardware manager implementation
pub mod serial;

use crate::config::Config;
use crate::hardware::serial::SerialConnection;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, Duration, Instant};

/// Represents a response from the MCU
#[derive(Debug, Clone)]
pub struct McuResponse {
    pub command: String,
    pub response: String,
    pub timestamp: Instant,
}

/// Complete hardware manager with real serial communication
pub struct HardwareManager {
    config: Config,
    serial: Arc<Mutex<Option<SerialConnection>>>,
    pending_commands: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<String>>>>,
    next_command_id: Arc<Mutex<u64>>,
    initialized: Arc<RwLock<bool>>,
    command_stats: Arc<Mutex<CommandStats>>,
}

/// Statistics for command execution
#[derive(Debug, Clone, Default)]
pub struct CommandStats {
    pub total_commands: u64,
    pub successful_commands: u64,
    pub failed_commands: u64,
    pub average_response_time: f64,
    pub total_response_time: f64,
}

impl HardwareManager {
    pub async fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            config: config.clone(),
            serial: Arc::new(Mutex::new(None)),
            pending_commands: Arc::new(Mutex::new(HashMap::new())),
            next_command_id: Arc::new(Mutex::new(0)),
            initialized: Arc::new(RwLock::new(false)),
            command_stats: Arc::new(Mutex::new(CommandStats::default())),
        })
    }

    pub async fn connect(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mcu_config = &self.config.mcu;
        
        tracing::info!("Connecting to MCU on {} at {} baud", 
                      mcu_config.serial, mcu_config.baud);
        
        let serial = SerialConnection::new(&mcu_config.serial, mcu_config.baud).await?;
        
        let mut serial_guard = self.serial.lock().await;
        *serial_guard = Some(serial);
        
        tracing::info!("Connected to MCU successfully");
        Ok(())
    }

    pub async fn initialize(&self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Initializing hardware");
        
        // Ensure we're connected
        {
            let serial_guard = self.serial.lock().await;
            if serial_guard.is_none() {
                drop(serial_guard);
                self.connect().await?;
            }
        }

        // Reset MCU to known state
        tracing::info!("Resetting MCU");
        match self.send_mcu_command("reset").await {
            Ok(response) => {
                tracing::debug!("Reset response: {}", response);
                if !response.starts_with("ok") {
                    tracing::warn!("Reset command returned: {}", response);
                }
            }
            Err(e) => {
                tracing::warn!("Reset command failed: {}", e);
                // Continue anyway - MCU might already be in a good state
            }
        }
        
        // Give MCU time to restart
        sleep(Duration::from_millis(1000)).await;
        
        // Configure MCU with printer settings
        self.configure_mcu().await?;
        
        // Mark as initialized
        {
            let mut init_guard = self.initialized.write().await;
            *init_guard = true;
        }
        
        tracing::info!("Hardware initialization complete");
        Ok(())
    }

    async fn configure_mcu(&self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Configuring MCU hardware");
        
        // Configure all stepper motors
        for (name, stepper) in &self.config.steppers {
            let config_cmd = format!(
                "config_stepper name={} step_pin={} dir_pin={} enable_pin={} microsteps={}",
                name, stepper.step_pin, stepper.dir_pin, stepper.enable_pin, stepper.microsteps
            );
            
            tracing::debug!("Configuring stepper: {}", name);
            let response = self.send_mcu_command(&config_cmd).await?;
            
            if !response.starts_with("ok") {
                tracing::warn!("Stepper {} config response: {}", name, response);
            }
        }
        
        // Configure extruder
        let extruder_cmd = format!(
            "config_extruder step_pin={} dir_pin={} enable_pin={} rotation_distance={} microsteps={}",
            self.config.extruder.step_pin,
            self.config.extruder.dir_pin,
            self.config.extruder.enable_pin,
            self.config.extruder.rotation_distance,
            self.config.extruder.microsteps
        );
        
        tracing::debug!("Configuring extruder");
        let response = self.send_mcu_command(&extruder_cmd).await?;
        if !response.starts_with("ok") {
            tracing::warn!("Extruder config response: {}", response);
        }
        
        // Configure heater bed
        let heater_cmd = format!(
            "config_heater name=bed heater_pin={} sensor_type={} sensor_pin={} min_temp={} max_temp={}",
            self.config.heater_bed.heater_pin,
            self.config.heater_bed.sensor_type,
            self.config.heater_bed.sensor_pin,
            self.config.heater_bed.min_temp,
            self.config.heater_bed.max_temp
        );
        
        tracing::debug!("Configuring heater bed");
        let response = self.send_mcu_command(&heater_cmd).await?;
        if !response.starts_with("ok") {
            tracing::warn!("Heater bed config response: {}", response);
        }
        
        // Set up default MCU parameters
        let setup_commands = vec![
            "set_velocity_limit 300",
            "set_acceleration_limit 3000",
            "set_jerk_limit 20",
            "enable_watchdog",
        ];
        
        for cmd in setup_commands {
            let response = self.send_mcu_command(cmd).await?;
            if !response.starts_with("ok") {
                tracing::warn!("Setup command '{}' response: {}", cmd, response);
            }
        }
        
        Ok(())
    }

    pub async fn send_mcu_command(&self, command: &str) -> Result<String, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        
        let serial_guard = self.serial.lock().await;
        
        if let Some(ref serial) = *serial_guard {
            // Generate unique command ID
            let mut id_guard = self.next_command_id.lock().await;
            let command_id = *id_guard;
            *id_guard += 1;
            drop(id_guard);
            
            // Format command with ID for response tracking
            let full_command = format!("{} #{}", command, command_id);
            
            // Create response channel
            let (response_tx, response_rx) = tokio::sync::oneshot::channel();
            
            // Register pending command
            {
                let mut pending_guard = self.pending_commands.lock().await;
                pending_guard.insert(command_id.to_string(), response_tx);
            }
            
            // Send command
            serial.send_command(&full_command).await?;
            
            // Wait for response with timeout
            match tokio::time::timeout(Duration::from_secs(5), response_rx).await {
                Ok(Ok(response)) => {
                    let response_time = start_time.elapsed().as_secs_f64();
                    
                    // Update statistics
                    {
                        let mut stats_guard = self.command_stats.lock().await;
                        stats_guard.total_commands += 1;
                        stats_guard.successful_commands += 1;
                        stats_guard.total_response_time += response_time;
                        stats_guard.average_response_time = 
                            stats_guard.total_response_time / stats_guard.total_commands as f64;
                    }
                    
                    tracing::debug!("Command '{}' completed in {:.3}ms", command, response_time * 1000.0);
                    Ok(response)
                },
                Ok(Err(_)) => {
                    // Update statistics
                    {
                        let mut stats_guard = self.command_stats.lock().await;
                        stats_guard.total_commands += 1;
                        stats_guard.failed_commands += 1;
                    }
                    
                    Err("Response channel closed".into())
                },
                Err(_) => {
                    // Timeout - clean up pending command
                    {
                        let mut pending_guard = self.pending_commands.lock().await;
                        pending_guard.remove(&command_id.to_string());
                    }
                    
                    // Update statistics
                    {
                        let mut stats_guard = self.command_stats.lock().await;
                        stats_guard.total_commands += 1;
                        stats_guard.failed_commands += 1;
                    }
                    
                    Err(format!("Command '{}' timed out", command).into())
                },
            }
        } else {
            Err("Not connected to MCU".into())
        }
    }

    pub async fn send_step_command(
        &self,
        axis: &str,
        steps: u32,
        direction: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let command = format!("step {} {} {}", axis, steps, if direction { "1" } else { "0" });
        let response = self.send_mcu_command(&command).await?;
        
        if response.starts_with("ok") {
            Ok(())
        } else {
            Err(format!("Step command failed: {}", response).into())
        }
    }

    pub async fn set_heater_temperature(
        &self,
        heater_name: &str,
        temperature: f64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let command = format!("set_heater_temp {} {}", heater_name, temperature);
        let response = self.send_mcu_command(&command).await?;
        
        if response.starts_with("ok") {
            Ok(())
        } else {
            Err(format!("Set heater temperature failed: {}", response).into())
        }
    }

    pub async fn get_temperature(
        &self,
        sensor_name: &str,
    ) -> Result<f64, Box<dyn std::error::Error>> {
        let command = format!("get_temp {}", sensor_name);
        let response = self.send_mcu_command(&command).await?;
        
        // Parse temperature from response (e.g., "temp: 25.3")
        if let Some(temp_str) = response.strip_prefix("temp: ") {
            temp_str.parse::<f64>()
                .map_err(|_| format!("Invalid temperature response: {}", response).into())
        } else {
            Err(format!("Unexpected temperature response: {}", response).into())
        }
    }

    pub async fn process_responses(&self) -> Result<(), Box<dyn std::error::Error>> {
        let serial_guard = self.serial.lock().await;
        
        if let Some(ref serial) = *serial_guard {
            // Process all available responses
            while let Some(response) = serial.try_recv_response() {
                self.handle_mcu_response(&response).await?;
            }
        }
        
        Ok(())
    }

    async fn handle_mcu_response(&self, response: &str) -> Result<(), Box<dyn std::error::Error>> {
        tracing::trace!("MCU Response: {}", response);
        
        // Parse response to find command ID
        if let Some(hash_pos) = response.find('#') {
            let response_content = response[..hash_pos].trim();
            let command_id = &response[hash_pos + 1..];
            
            // Look for pending command
            let mut pending_guard = self.pending_commands.lock().await;
            
            if let Some(sender) = pending_guard.remove(command_id) {
                let _ = sender.send(response_content.to_string());
            } else {
                drop(pending_guard);
                self.handle_unsolicited_message(response_content).await?;
            }
        } else {
            self.handle_unsolicited_message(response).await?;
        }
        
        Ok(())
    }

    async fn handle_unsolicited_message(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let parts: Vec<&str> = message.split_whitespace().collect();
        
        match parts.first() {
            Some(&"status") => {
                tracing::debug!("MCU status: {:?}", parts);
            }
            Some(&"error") => {
                tracing::error!("MCU error: {:?}", parts);
            }
            Some(&"temp") => {
                // Handle temperature updates
                if parts.len() >= 3 {
                    let sensor = parts[1];
                    let temp: f64 = parts[2].parse().unwrap_or(0.0);
                    tracing::debug!("Temperature update - {}: {}°C", sensor, temp);
                }
            }
            Some(&"position") => {
                // Handle position updates
                if parts.len() >= 5 {
                    let x: f64 = parts[1].parse().unwrap_or(0.0);
                    let y: f64 = parts[2].parse().unwrap_or(0.0);
                    let z: f64 = parts[3].parse().unwrap_or(0.0);
                    let e: f64 = parts[4].parse().unwrap_or(0.0);
                    tracing::debug!("Position update: X={:.3}, Y={:.3}, Z={:.3}, E={:.3}", x, y, z, e);
                }
            }
            _ => {
                tracing::debug!("Unknown MCU message: {}", message);
            }
        }
        
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Shutting down hardware");
        
        // Send disable commands
        if let Ok(serial_guard) = self.serial.try_lock() {
            if let Some(ref serial) = *serial_guard {
                let _ = serial.send_command("disable_all_motors").await;
                let _ = serial.send_command("disable_heaters").await;
                let _ = serial.send_command("safe_shutdown").await;
            }
        }
        
        Ok(())
    }

    pub async fn is_initialized(&self) -> bool {
        let init_guard = self.initialized.read().await;
        *init_guard
    }

    pub async fn get_command_stats(&self) -> CommandStats {
        let stats_guard = self.command_stats.lock().await;
        stats_guard.clone()
    }
}

impl Clone for HardwareManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            serial: self.serial.clone(),
            pending_commands: self.pending_commands.clone(),
            next_command_id: self.next_command_id.clone(),
            initialized: self.initialized.clone(),
            command_stats: self.command_stats.clone(),
        }
    }
}