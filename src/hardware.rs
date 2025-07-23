// src/hardware.rs - Remove unused import
use crate::config::Config;
use tokio::io::{AsyncWriteExt};  // Remove SerialPort import
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use std::time::Duration;

#[derive(Debug)]
pub struct HardwareManager {
    config: Config,
    serial: Option<SerialStream>, // Just store the stream directly
}

impl HardwareManager {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            serial: None,
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Connecting to MCU: {} at {} baud", 
                      self.config.mcu.serial, self.config.mcu.baud);
        
        let port = tokio_serial::new(&self.config.mcu.serial, self.config.mcu.baud)
            .timeout(Duration::from_millis(100))
            .open_native_async()?;
        
        self.serial = Some(port);
        tracing::info!("Connected to MCU successfully");
        
        Ok(())
    }

    pub async fn send_command(&mut self, command: &str) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(ref mut port) = self.serial {
            tracing::debug!("MCU <- {}", command);
            
            // Send command with newline
            let command_with_newline = format!("{}\n", command);
            port.write_all(command_with_newline.as_bytes()).await?;
            port.flush().await?;
            
            // For now, return immediate OK response
            // In real implementation, you'd read the actual response
            tokio::time::sleep(Duration::from_millis(10)).await;
            
            let response = "ok".to_string();
            tracing::debug!("MCU -> {}", response);
            Ok(response)
        } else {
            Err("Not connected to hardware".into())
        }
    }

    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.serial.is_none() {
            self.connect().await?;
        }
        
        tracing::info!("Initializing printer hardware...");
        
        // Reset MCU
        match self.send_command("reset").await {
            Ok(response) => {
                tracing::debug!("Reset response: {}", response);
            }
            Err(e) => {
                tracing::warn!("Reset command failed: {}", e);
            }
        }
        
        tokio::time::sleep(Duration::from_millis(1000)).await;
        
        // Create a copy of steppers for iteration to avoid borrowing conflicts
        let steppers: Vec<_> = self.config.steppers.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        
        // Configure all steppers
        for (name, stepper) in steppers {
            let cmd = format!(
                "config_stepper name={} step_pin={} dir_pin={} enable_pin={} microsteps={}",
                name, stepper.step_pin, stepper.dir_pin, stepper.enable_pin, stepper.microsteps
            );
            match self.send_command(&cmd).await {
                Ok(response) => {
                    if !response.starts_with("ok") {
                        tracing::warn!("Stepper {} config response: {}", name, response);
                    }
                }
                Err(e) => {
                    tracing::warn!("Stepper {} config failed: {}", name, e);
                }
            }
        }
        
        tracing::info!("Hardware initialization complete");
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Shutting down hardware");
        if let Some(ref mut port) = self.serial {
            let _ = port.write_all(b"shutdown\n").await;
            let _ = port.flush().await;
        }
        Ok(())
    }
}

// Manual Clone implementation
impl Clone for HardwareManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            serial: None, // Can't clone the serial connection, so start fresh
        }
    }
}