// src/config.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub printer: PrinterConfig,
    pub mcu: McuConfig,
    pub extruder: ExtruderConfig,
    pub heater_bed: HeaterBedConfig,
    #[serde(default)]
    pub steppers: HashMap<String, StepperConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PrinterConfig {
    pub kinematics: String,
    pub max_velocity: f64,
    pub max_accel: f64,
    pub max_z_velocity: f64,
    pub max_z_accel: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McuConfig {
    pub serial: String,
    pub baud: u32,
    pub restart_method: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtruderConfig {
    pub step_pin: String,
    pub dir_pin: String,
    pub enable_pin: String,
    pub rotation_distance: f64,
    pub gear_ratio: Option<(f64, f64)>,
    pub microsteps: u32,
    pub nozzle_diameter: f64,
    pub filament_diameter: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HeaterBedConfig {
    pub heater_pin: String,
    pub sensor_type: String,
    pub sensor_pin: String,
    pub min_temp: f64,
    pub max_temp: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StepperConfig {
    pub step_pin: String,
    pub dir_pin: String,
    pub enable_pin: String,
    pub rotation_distance: f64,
    pub microsteps: u32,
    pub full_steps_per_rotation: u32,
}

pub fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}