// src/hardware.rs
use crate::config::Config;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use thiserror::Error;
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use std::time::Duration;

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
    serial: Option<SerialStream>, // Just store the stream directly
}

impl HardwareManager {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            serial: None,
        }
    }

    pub async fn connect(&mut self) -> Result<(), HardwareError> {
        tracing::info!("Connecting to MCU: {} at {} baud", 
                      self.config.mcu.serial, self.config.mcu.baud);
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
        let steppers: Vec<_> = self.config.steppers.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
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