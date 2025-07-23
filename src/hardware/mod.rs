// src/hardware/mod.rs - Fixed hardware manager
use crate::config::Config;
use std::time::Duration;

pub struct HardwareManager {
    config: Config,
    connected: bool,
}

impl HardwareManager {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            connected: false,
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Connecting to MCU: {}", self.config.mcu.serial);
        // In real implementation, this would open the serial port
        // For now, we'll simulate connection
        self.connected = true;
        Ok(())
    }

    pub async fn send_command(&self, command: &str) -> Result<String, Box<dyn std::error::Error>> {
        if !self.connected {
            return Err("Not connected to hardware".into());
        }
        
        tracing::debug!("MCU <- {}", command);
        
        // Simulate typical responses
        let response = match command {
            "reset" => "ok",
            cmd if cmd.starts_with("config_stepper") => "ok",
            cmd if cmd.starts_with("step") => "ok",
            _ => "ok",
        };
        
        tracing::debug!("MCU -> {}", response);
        Ok(response.to_string())
    }

    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.connected {
            self.connect().await?;
        }
        
        tracing::info!("Initializing printer hardware...");
        self.send_command("reset").await?;
        
        // Configure all steppers
        for (name, stepper) in &self.config.steppers {
            let cmd = format!(
                "config_stepper name={} step_pin={} dir_pin={} enable_pin={} microsteps={}",
                name, stepper.step_pin, stepper.dir_pin, stepper.enable_pin, stepper.microsteps
            );
            self.send_command(&cmd).await?;
        }
        
        tracing::info!("Hardware initialization complete");
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Shutting down hardware");
        if self.connected {
            let _ = self.send_command("disable_all_motors").await;
            let _ = self.send_command("disable_heaters").await;
        }
        Ok(())
    }
}

impl Clone for HardwareManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            connected: self.connected,
        }
    }
}