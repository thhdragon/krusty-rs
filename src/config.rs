// src/config.rs - Single configuration file
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct McuConfig {
    pub serial: String,
    #[serde(default = "default_baud")]
    pub baud: u32,
}

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

pub fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}