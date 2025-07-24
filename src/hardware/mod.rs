// src/hardware/mod.rs
// Declare the submodules within the `hardware` module
pub mod temperature; // This refers to src/hardware/temperature.rs

// Re-export items you want easily accessible from the `hardware` module level

use crate::config::Config;
use std::time::Duration;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_serial::{SerialPortBuilderExt, SerialStream};

#[derive(Debug, Error)]
pub enum HardwareError {
    #[error("Serial port error: {0}")]
    Serial(#[from] tokio_serial::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Not connected to hardware")]
    NotConnected,
    #[error("Timeout waiting for response")]
    Timeout,
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

#[derive(Debug)]
pub struct HardwareManager {
    config: Config,
    serial: Option<SerialStream>,
}

impl HardwareManager {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            serial: None,
        }
    }

    pub async fn connect(&mut self) -> Result<(), HardwareError> {
        tracing::info!(
            "Connecting to MCU: {} at {} baud",
            self.config.mcu.serial, self.config.mcu.baud
        );
        let port = tokio_serial::new(&self.config.mcu.serial, self.config.mcu.baud)
            .timeout(Duration::from_millis(100))
            .open_native_async()?;
        self.serial = Some(port);
        tracing::info!("Connected to MCU successfully");
        Ok(())
    }

    pub async fn send_command(&mut self, command: &str) -> Result<String, HardwareError> {
        use tokio::time::timeout;
        if let Some(ref mut port) = self.serial {
            tracing::debug!("MCU <- {}", command);
            let command_with_newline = format!("{}\n", command);
            port.write_all(command_with_newline.as_bytes()).await?;
            port.flush().await?;
            let mut buf = vec![0u8; 1024];
            let n = timeout(Duration::from_millis(500), port.read(&mut buf)).await
                .map_err(|_| HardwareError::Timeout)??;
            let response = String::from_utf8(buf[..n].to_vec())?.trim().to_string();
            tracing::debug!("MCU -> {}", response);
            Ok(response)
        } else {
            Err(HardwareError::NotConnected)
        }
    }

    pub async fn initialize(&mut self) -> Result<(), HardwareError> {
        if self.serial.is_none() {
            self.connect().await?;
        }
        tracing::info!("Initializing printer hardware...");
        match self.send_command("reset").await {
            Ok(response) => {
                tracing::debug!("Reset response: {}", response);
            }
            Err(e) => {
                tracing::warn!("Reset command failed: {}", e);
            }
        }
        tokio::time::sleep(Duration::from_millis(1000)).await;
        let steppers: Vec<_> = self
            .config
            .steppers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
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

    pub async fn shutdown(&mut self) -> Result<(), HardwareError> {
        tracing::info!("Shutting down hardware");
        if let Some(ref mut port) = self.serial {
            let _ = port.write_all(b"shutdown\n").await;
            let _ = port.flush().await;
        }
        Ok(())
    }

    pub async fn get_command_stats(&self) -> CommandStats {
        // Stub: Replace with real stats collection
        CommandStats {
            total_commands: 0,
            failed_commands: 0,
            last_command: None,
        }
    }

    /// Stub: Set heater temperature (async)
    pub async fn set_heater_temperature(&self, _name: &str, _temp: f64) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    pub async fn process_responses(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

/// Statistics for hardware command processing
#[derive(Debug, Clone, Default)]
pub struct CommandStats {
    pub total_commands: u64,
    pub failed_commands: u64,
    pub last_command: Option<String>,
}

// Manual Clone implementation for HardwareManager
impl Clone for HardwareManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            serial: None, // Can't clone the serial connection, so start fresh
        }
    }
}

// --- Additional hardware abstraction stubs ---

/// Placeholder for fan control
#[derive(Debug, Clone, Default)]
pub struct FanControllerStub {
    pub speed: u8, // 0-255
}

impl FanControllerStub {
    pub fn new() -> Self {
        Self { speed: 0 }
    }
    pub fn set_speed(&mut self, speed: u8) {
        // TODO: Implement real fan speed control
        self.speed = speed;
    }
}

/// Placeholder for generic sensor reading
#[derive(Debug, Clone, Default)]
pub struct GenericSensorStub {
    pub value: f64,
}

impl GenericSensorStub {
    pub fn new() -> Self {
        Self { value: 0.0 }
    }
    pub fn read(&self) -> f64 {
        // TODO: Implement real sensor reading
        self.value
    }
}

// TODO: Integrate FanControllerStub and GenericSensorStub with hardware manager when implemented.