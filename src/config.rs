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
}

/// Printer-level configuration.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
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
}

/// Microcontroller configuration.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct McuConfig {
    pub serial: String,
    #[serde(default = "default_baud")]
    pub baud: u32,
}

/// Extruder configuration.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
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

/// Heated bed configuration.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HeaterBedConfig {
    pub heater_pin: String,
    pub sensor_type: String,
    pub sensor_pin: String,
    #[serde(default = "default_min_temp")]
    pub min_temp: f64,
    #[serde(default = "default_max_temp")]
    pub max_temp: f64,
}

/// Stepper motor configuration.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_default_values() {
        let printer = PrinterConfig::default();
        assert_eq!(printer.kinematics, "cartesian");
        assert_eq!(printer.max_velocity, 300.0);
        assert_eq!(printer.max_accel, 3000.0);
        assert_eq!(printer.max_z_velocity, 25.0);
        assert_eq!(printer.max_z_accel, 100.0);
        let extruder = ExtruderConfig::default();
        assert_eq!(extruder.microsteps, 16);
        assert_eq!(extruder.nozzle_diameter, 0.4);
        assert_eq!(extruder.filament_diameter, 1.75);
    }

    #[test]
    fn test_load_config_success() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_config.toml");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "[printer]\nkinematics = 'corexy'\nmax_velocity = 500.0").unwrap();
        file.flush().unwrap();
        let config = load_config(file_path.to_str().unwrap()).unwrap();
        assert_eq!(config.printer.kinematics, "corexy");
        assert_eq!(config.printer.max_velocity, 500.0);
        // Defaults for missing fields
        assert_eq!(config.printer.max_accel, 3000.0);
    }

    #[test]
    fn test_load_config_missing_file() {
        let result = load_config("nonexistent_file.toml");
        assert!(matches!(result, Err(ConfigError::Io(_))));
    }

    #[test]
    fn test_load_config_invalid_toml() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("bad.toml");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "not a valid toml").unwrap();
        file.flush().unwrap();
        let result = load_config(file_path.to_str().unwrap());
        assert!(matches!(result, Err(ConfigError::Toml(_))));
    }
}