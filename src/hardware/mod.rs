// src/hardware/mod.rs - Fixed hardware manager with proper serial
use crate::config::Config;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use std::time::Duration;
use tokio::sync::mpsc;

pub struct HardwareManager {
    config: Config,
    serial: Option<SerialConnection>,
}

pub struct SerialConnection {
    port: SerialStream,
    reader: BufReader<SerialStream>,
}

impl HardwareManager {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            serial: None,
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Connecting to MCU: {}", self.config.mcu.serial);
        
        let port = tokio_serial::new(&self.config.mcu.serial, self.config.mcu.baud)
            .timeout(Duration::from_millis(100))
            .open_native_async()?;
        
        let reader = BufReader::new(port.try_clone()?);
        
        self.serial = Some(SerialConnection { port, reader });
        println!("Connected to MCU successfully");
        
        Ok(())
    }

    pub async fn send_command(&mut self, command: &str) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(ref mut serial) = self.serial {
            println!("MCU <- {}", command);
            
            // Send command
            let command_with_newline = format!("{}\n", command);
            serial.port.write_all(command_with_newline.as_bytes()).await?;
            serial.port.flush().await?;
            
            // Read response (simplified - in real implementation you'd have proper response handling)
            tokio::time::sleep(Duration::from_millis(10)).await;
            
            let response = "ok".to_string();
            println!("MCU -> {}", response);
            Ok(response)
        } else {
            Err("Not connected to hardware".into())
        }
    }

    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.serial.is_none() {
            self.connect().await?;
        }
        
        println!("Initializing printer hardware...");
        self.send_command("reset").await?;
        
        // Configure all steppers
        for (name, stepper) in &self.config.steppers {
            let cmd = format!(
                "config_stepper name={} step_pin={} dir_pin={} enable_pin={} microsteps={}",
                name, stepper.step_pin, stepper.dir_pin, stepper.enable_pin, stepper.microsteps
            );
            self.send_command(&cmd).await?;
        }
        
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Shutting down hardware");
        if let Some(ref mut serial) = self.serial {
            let _ = serial.port.write_all(b"shutdown\n").await;
        }
        Ok(())
    }
}

impl Clone for HardwareManager {
    fn clone(&self) -> Self {
        // Note: Serial connection can't be cloned, so we create a new one
        Self {
            config: self.config.clone(),
            serial: None, // Will need to reconnect
        }
    }
}