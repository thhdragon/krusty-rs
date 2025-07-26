//! # Motion Shaper and Blending Configuration
//!
//! This module defines configuration structs for advanced motion planning, input shaper, and blending options.
//!
//! ## Example: TOML Configuration
//!
//! ```toml
//! [motion.shaper.x]
//! type = "zvd"
//! frequency = 40.0
//! damping = 0.1
//!
//! [motion.shaper.y]
//! type = "sine"
//! frequency = 35.0
//!
//! [motion.blending]
//! type = "bezier"
//! max_deviation = 0.2
//! ```
//!
//! - Each axis (x, y, z, e) can have its own shaper type and parameters.
//! - Blending (corner smoothing) is configured globally or per-axis as needed.
//!
//! ## Example: Rust Usage
//!
//! ```rust
//! use krusty_rs::config::Config;
//! let toml_str = r#"
//! [motion.shaper.x]
//! type = "zvd"
//! frequency = 40.0
//! damping = 0.1
//!
//! [motion.blending]
//! type = "bezier"
//! max_deviation = 0.2
//! "#;
//! let config: Config = toml::from_str(toml_str).unwrap();
//! let motion = config.motion.as_ref().unwrap();
//! assert_eq!(motion.shaper["x"].frequency, 40.0);
//! assert_eq!(motion.blending.as_ref().unwrap().max_deviation, 0.2);
//! // Validate config
//! assert!(motion.validate().is_ok());
//! ```
//!
//! See also: `src/motion/planner/mod.rs` for planner integration and assignment logic.

// src/config.rs - Single configuration file
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
}

/// Main configuration struct for the printer, MCU, extruder, heater bed, and steppers.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub printer: PrinterConfig,
    #[serde(default)]
    pub mcu: McuConfig,
    #[serde(default)]
    pub extruder: ExtruderConfig,
    #[serde(default)]
    pub heater_bed: HeaterBedConfig,
    #[serde(default)]
    pub steppers: HashMap<String, StepperConfig>,
    #[serde(default)]
    pub motion: Option<MotionConfig>, // Advanced motion/shaper config
}

impl Default for Config {
    fn default() -> Self {
        Self {
            printer: PrinterConfig::default(),
            mcu: McuConfig::default(),
            extruder: ExtruderConfig::default(),
            heater_bed: HeaterBedConfig::default(),
            steppers: HashMap::new(),
            motion: None,
        }
    }
}

/// Printer-level configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PrinterConfig {
    #[serde(default = "default_kinematics")]
    pub kinematics: String,
    #[serde(default = "default_max_velocity")]
    pub max_velocity: f64,
    #[serde(default = "default_max_accel")]
    pub max_accel: f64,
    #[serde(default = "default_max_z_velocity")]
    pub max_z_velocity: f64,
    #[serde(default = "default_max_z_accel")]
    pub max_z_accel: f64,
    #[serde(default)]
    pub printer_name: Option<String>,
}

impl Default for PrinterConfig {
    fn default() -> Self {
        Self {
            kinematics: default_kinematics(),
            max_velocity: default_max_velocity(),
            max_accel: default_max_accel(),
            max_z_velocity: default_max_z_velocity(),
            max_z_accel: default_max_z_accel(),
            printer_name: None,
        }
    }
}

/// Microcontroller configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McuConfig {
    pub serial: String,
    #[serde(default = "default_baud")]
    pub baud: u32,
}

impl Default for McuConfig {
    fn default() -> Self {
        Self {
            serial: "".to_string(),
            baud: default_baud(),
        }
    }
}

/// Extruder configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtruderConfig {
    pub step_pin: String,
    pub dir_pin: String,
    pub enable_pin: String,
    #[serde(default = "default_rotation_distance")]
    pub rotation_distance: f64,
    #[serde(default)]
    pub gear_ratio: Option<(f64, f64)>,
    #[serde(default = "default_microsteps")]
    pub microsteps: u32,
    #[serde(default = "default_nozzle_diameter")]
    pub nozzle_diameter: f64,
    #[serde(default = "default_filament_diameter")]
    pub filament_diameter: f64,
}

impl Default for ExtruderConfig {
    fn default() -> Self {
        Self {
            step_pin: "".to_string(),
            dir_pin: "".to_string(),
            enable_pin: "".to_string(),
            rotation_distance: default_rotation_distance(),
            gear_ratio: None,
            microsteps: default_microsteps(),
            nozzle_diameter: default_nozzle_diameter(),
            filament_diameter: default_filament_diameter(),
        }
    }
}

/// Heated bed configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HeaterBedConfig {
    pub heater_pin: String,
    pub sensor_type: String,
    pub sensor_pin: String,
    #[serde(default = "default_min_temp")]
    pub min_temp: f64,
    #[serde(default = "default_max_temp")]
    pub max_temp: f64,
}

impl Default for HeaterBedConfig {
    fn default() -> Self {
        Self {
            heater_pin: "".to_string(),
            sensor_type: "".to_string(),
            sensor_pin: "".to_string(),
            min_temp: default_min_temp(),
            max_temp: default_max_temp(),
        }
    }
}

/// Stepper motor configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StepperConfig {
    pub step_pin: String,
    pub dir_pin: String,
    pub enable_pin: String,
    #[serde(default = "default_rotation_distance")]
    pub rotation_distance: f64,
    #[serde(default = "default_microsteps")]
    pub microsteps: u32,
    #[serde(default = "default_full_steps_per_rotation")]
    pub full_steps_per_rotation: u32,
}

impl Default for StepperConfig {
    fn default() -> Self {
        Self {
            step_pin: "".to_string(),
            dir_pin: "".to_string(),
            enable_pin: "".to_string(),
            rotation_distance: default_rotation_distance(),
            microsteps: default_microsteps(),
            full_steps_per_rotation: default_full_steps_per_rotation(),
        }
    }
}

/// Advanced motion planning, shaper, and blending configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MotionConfig {
    #[serde(default)]
    pub shaper: HashMap<String, AxisShaperConfig>,
    #[serde(default)]
    pub blending: Option<BlendingConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ShaperType {
    Zvd,
    Sine,
    // Extend with more shaper types as needed
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AxisShaperConfig {
    pub r#type: ShaperType,
    pub frequency: f32,
    pub damping: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BlendingType {
    Bezier,
    // Extend with more blending types as needed
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BlendingConfig {
    pub r#type: BlendingType,
    pub max_deviation: f32,
}

impl MotionConfig {
    /// Validate motion config (frequency, max_deviation, etc.)
    pub fn validate(&self) -> Result<(), String> {
        for (axis, shaper) in &self.shaper {
            if shaper.frequency <= 0.0 {
                return Err(format!("Shaper frequency for axis '{}' must be > 0", axis));
            }
            if let Some(damping) = shaper.damping {
                if damping < 0.0 || damping > 1.0 {
                    return Err(format!("Shaper damping for axis '{}' must be between 0 and 1", axis));
                }
            }
        }
        if let Some(blending) = &self.blending {
            if blending.max_deviation <= 0.0 {
                return Err("Blending max_deviation must be > 0".to_string());
            }
        }
        Ok(())
    }
}

/// Configuration manager
pub struct ConfigManager {
    config: Config,
    config_path: String,
    backup_configs: Vec<Config>,
}

impl ConfigManager {
    pub fn new(config: Config, config_path: String) -> Self {
        Self {
            config,
            config_path,
            backup_configs: Vec::new(),
        }
    }

    pub fn load_config(config_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
        Ok(crate::config::load_config(config_path)?)
    }

    pub async fn save_config(&self, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
        use std::io::Write;
        let toml_string = toml::to_string(config)?;
        let mut file = std::fs::File::create(&self.config_path)?;
        file.write_all(toml_string.as_bytes())?;
        Ok(())
    }

    pub fn reload_config(&self) -> Result<Config, Box<dyn std::error::Error>> {
        Self::load_config(&self.config_path)
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    pub fn backup_config(&mut self) {
        self.backup_configs.push(self.config.clone());
        // Keep only last 5 backups
        while self.backup_configs.len() > 5 {
            self.backup_configs.remove(0);
        }
    }

    pub fn restore_backup(&mut self, index: usize) -> Result<(), Box<dyn std::error::Error>> {
        if index < self.backup_configs.len() {
            self.config = self.backup_configs[index].clone();
            Ok(())
        } else {
            Err("Backup index out of range".into())
        }
    }
}

// Default value functions
fn default_kinematics() -> String { "cartesian".to_string() }
fn default_max_velocity() -> f64 { 300.0 }
fn default_max_accel() -> f64 { 3000.0 }
fn default_max_z_velocity() -> f64 { 25.0 }
fn default_max_z_accel() -> f64 { 100.0 }
fn default_baud() -> u32 { 250000 }
fn default_rotation_distance() -> f64 { 22.67895 }
fn default_microsteps() -> u32 { 16 }
fn default_full_steps_per_rotation() -> u32 { 200 }
fn default_nozzle_diameter() -> f64 { 0.4 }
fn default_filament_diameter() -> f64 { 1.75 }
fn default_min_temp() -> f64 { 0.0 }
fn default_max_temp() -> f64 { 250.0 }

/// Load configuration from a TOML file at the given path.
pub fn load_config(path: &str) -> Result<Config, ConfigError> {
    match std::fs::read_to_string(path) {
        Ok(contents) => match toml::from_str(&contents) {
            Ok(config) => Ok(config),
            Err(e) => {
                tracing::error!("Failed to parse config TOML: {}", e);
                Err(ConfigError::Toml(e))
            }
        },
        Err(e) => {
            tracing::error!("Failed to read config file '{}': {}", path, e);
            Err(ConfigError::Io(e))
        }
    }
}