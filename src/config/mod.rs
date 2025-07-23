// src/config/mod.rs - Enhanced configuration system
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

/// Main configuration structure that supports both TOML and legacy printer.cfg
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
    pub fans: HashMap<String, FanConfig>,
    
    #[serde(default)]
    pub steppers: HashMap<String, StepperConfig>,
    
    #[serde(default)]
    pub motion: MotionConfig,
    
    #[serde(default)]
    pub advanced: AdvancedConfig,
    
    #[serde(default)]
    pub web: WebConfig,
}

/// Printer base configuration
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
    
    #[serde(default = "default_square_corner_velocity")]
    pub square_corner_velocity: f64,
    
    #[serde(default)]
    pub bed_size: [f64; 2], // [width, depth]
    
    #[serde(default)]
    pub printer_name: String,
}

/// MCU configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct McuConfig {
    pub serial: String,
    #[serde(default = "default_baud")]
    pub baud: u32,
    #[serde(default)]
    pub restart_method: Option<String>,
    #[serde(default)]
    pub serial_retries: Option<u32>,
}

/// Extruder configuration
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
    #[serde(default)]
    pub heater_pin: Option<String>,
    #[serde(default)]
    pub sensor_type: Option<String>,
    #[serde(default)]
    pub sensor_pin: Option<String>,
    #[serde(default = "default_min_temp")]
    pub min_temp: f64,
    #[serde(default = "default_max_temp")]
    pub max_temp: f64,
}

/// Heater bed configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HeaterBedConfig {
    pub heater_pin: String,
    pub sensor_type: String,
    pub sensor_pin: String,
    #[serde(default = "default_min_temp")]
    pub min_temp: f64,
    #[serde(default = "default_max_temp")]
    pub max_temp: f64,
    #[serde(default)]
    pub pwm_cycle_time: Option<f64>,
}

/// Fan configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FanConfig {
    pub pin: String,
    #[serde(default)]
    pub max_power: Option<f64>,
    #[serde(default)]
    pub shutdown_speed: Option<f64>,
    #[serde(default)]
    pub cycle_time: Option<f64>,
}

/// Stepper motor configuration
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
    #[serde(default)]
    pub endstop_pin: Option<String>,
    #[serde(default)]
    pub position_endstop: Option<f64>,
    #[serde(default)]
    pub position_min: Option<f64>,
    #[serde(default)]
    pub position_max: Option<f64>,
    #[serde(default)]
    pub homing_speed: Option<f64>,
}

/// Motion system configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct MotionConfig {
    #[serde(default = "default_junction_deviation")]
    pub junction_deviation: f64,
    
    #[serde(default = "default_jerk")]
    pub max_jerk: f64,
    
    #[serde(default)]
    pub acceleration_profile: Option<String>, // "trapezoidal", "s-curve"
    
    #[serde(default)]
    pub input_shaping: Option<InputShapingConfig>,
    
    #[serde(default)]
    pub snap_crackle_enabled: bool,
    
    #[serde(default = "default_max_snap")]
    pub max_snap: f64,
    
    #[serde(default = "default_max_crackle")]
    pub max_crackle: f64,
}

/// Input shaping configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct InputShapingConfig {
    #[serde(default)]
    pub shaper_type: String, // "zvd", "zvdd", "ei2"
    #[serde(default)]
    pub frequency: f64,
    #[serde(default)]
    pub damping: f64,
}

/// Advanced configuration options
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AdvancedConfig {
    #[serde(default)]
    pub adaptive_motion: bool,
    
    #[serde(default = "default_adaptation_rate")]
    pub adaptation_rate: f64,
    
    #[serde(default)]
    pub vibration_cancellation: bool,
    
    #[serde(default)]
    pub prediction_horizon: f64,
    
    #[serde(default)]
    pub learning_rate: f64,
    
    #[serde(default)]
    pub debug_logging: bool,
    
    #[serde(default)]
    pub performance_window: usize,
}

/// Web interface configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WebConfig {
    #[serde(default = "default_web_port")]
    pub port: u16,
    
    #[serde(default)]
    pub bind_address: String,
    
    #[serde(default)]
    pub api_key: Option<String>,
    
    #[serde(default)]
    pub cors_enabled: bool,
}

// Default value functions
fn default_kinematics() -> String { "cartesian".to_string() }
fn default_max_velocity() -> f64 { 300.0 }
fn default_max_accel() -> f64 { 3000.0 }
fn default_max_z_velocity() -> f64 { 25.0 }
fn default_max_z_accel() -> f64 { 100.0 }
fn default_square_corner_velocity() -> f64 { 5.0 }
fn default_baud() -> u32 { 250000 }
fn default_rotation_distance() -> f64 { 22.67895 }
fn default_microsteps() -> u32 { 16 }
fn default_full_steps_per_rotation() -> u32 { 200 }
fn default_nozzle_diameter() -> f64 { 0.4 }
fn default_filament_diameter() -> f64 { 1.75 }
fn default_min_temp() -> f64 { 0.0 }
fn default_max_temp() -> f64 { 250.0 }
fn default_junction_deviation() -> f64 { 0.05 }
fn default_jerk() -> f64 { 20.0 }
fn default_max_snap() -> f64 { 1000.0 }
fn default_max_crackle() -> f64 { 5000.0 }
fn default_adaptation_rate() -> f64 { 0.01 }
fn default_web_port() -> u16 { 8080 }

impl Config {
    /// Load configuration from file (supports both TOML and legacy printer.cfg)
    pub fn load_config(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(config_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        // Try to parse as TOML first
        if let Ok(config) = Self::parse_toml(&contents) {
            tracing::info!("Loaded configuration from TOML file: {}", config_path);
            return Ok(config);
        }
        
        // If TOML fails, try to parse as legacy printer.cfg
        if let Ok(config) = Self::parse_legacy_config(&contents) {
            tracing::info!("Loaded configuration from legacy printer.cfg: {}", config_path);
            return Ok(config);
        }
        
        Err(format!("Failed to parse configuration file: {}", config_path).into())
    }
    
    /// Parse TOML configuration
    fn parse_toml(contents: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config: Config = toml::from_str(contents)?;
        Ok(config)
    }
    
    /// Parse legacy printer.cfg style configuration
    fn parse_legacy_config(contents: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = Config::default();
        
        for line in contents.lines() {
            let line = line.trim();
            
            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // Parse section headers [section_name]
            if line.starts_with('[') && line.ends_with(']') {
                // Section parsing would go here
                continue;
            }
            
            // Parse key = value pairs
            if let Some(equals_pos) = line.find('=') {
                let key = line[..equals_pos].trim();
                let value = line[equals_pos + 1..].trim();
                
                Self::parse_config_value(&mut config, key, value)?;
            }
        }
        
        Ok(config)
    }
    
    /// Parse individual configuration values from legacy format
    fn parse_config_value(
        config: &mut Config,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match key.to_lowercase().as_str() {
            // Printer settings
            "kinematics" => config.printer.kinematics = value.to_string(),
            "max_velocity" => config.printer.max_velocity = value.parse()?,
            "max_accel" => config.printer.max_accel = value.parse()?,
            "max_z_velocity" => config.printer.max_z_velocity = value.parse()?,
            "max_z_accel" => config.printer.max_z_accel = value.parse()?,
            "square_corner_velocity" => config.printer.square_corner_velocity = value.parse()?,
            
            // MCU settings
            "serial" => config.mcu.serial = value.to_string(),
            "baud" => config.mcu.baud = value.parse()?,
            
            // Extruder settings (first extruder)
            "extruder_step_pin" => config.extruder.step_pin = value.to_string(),
            "extruder_dir_pin" => config.extruder.dir_pin = value.to_string(),
            "extruder_enable_pin" => config.extruder.enable_pin = value.to_string(),
            "extruder_rotation_distance" => config.extruder.rotation_distance = value.parse()?,
            "extruder_microsteps" => config.extruder.microsteps = value.parse()?,
            "nozzle_diameter" => config.extruder.nozzle_diameter = value.parse()?,
            "filament_diameter" => config.extruder.filament_diameter = value.parse()?,
            
            // Bed settings
            "heater_bed_heater_pin" => config.heater_bed.heater_pin = value.to_string(),
            "heater_bed_sensor_type" => config.heater_bed.sensor_type = value.to_string(),
            "heater_bed_sensor_pin" => config.heater_bed.sensor_pin = value.to_string(),
            "heater_bed_min_temp" => config.heater_bed.min_temp = value.parse()?,
            "heater_bed_max_temp" => config.heater_bed.max_temp = value.parse()?,
            
            // Motion settings
            "junction_deviation" => config.motion.junction_deviation = value.parse()?,
            "max_jerk" => config.motion.max_jerk = value.parse()?,
            
            _ => {
                // Handle stepper configurations
                if key.starts_with("stepper_") {
                    Self::parse_stepper_config(config, key, value)?;
                }
                // Handle fan configurations
                else if key.starts_with("fan_") {
                    Self::parse_fan_config(config, key, value)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Parse stepper motor configuration
    fn parse_stepper_config(
        config: &mut Config,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Extract stepper name (e.g., "stepper_x_step_pin" -> "x")
        if let Some(stepper_name) = key.strip_prefix("stepper_") {
            if let Some(underscore_pos) = stepper_name.find('_') {
                let name = stepper_name[..underscore_pos].to_string();
                let property = &stepper_name[underscore_pos + 1..];
                
                // Get or create stepper config
                let stepper = config.steppers.entry(name.clone()).or_insert_with(|| {
                    tracing::debug!("Creating configuration for stepper: {}", name);
                    StepperConfig::default()
                });
                
                // Set property
                match property {
                    "step_pin" => stepper.step_pin = value.to_string(),
                    "dir_pin" => stepper.dir_pin = value.to_string(),
                    "enable_pin" => stepper.enable_pin = value.to_string(),
                    "rotation_distance" => stepper.rotation_distance = value.parse()?,
                    "microsteps" => stepper.microsteps = value.parse()?,
                    "full_steps_per_rotation" => stepper.full_steps_per_rotation = value.parse()?,
                    "endstop_pin" => stepper.endstop_pin = Some(value.to_string()),
                    "position_endstop" => stepper.position_endstop = Some(value.parse()?),
                    "position_min" => stepper.position_min = Some(value.parse()?),
                    "position_max" => stepper.position_max = Some(value.parse()?),
                    "homing_speed" => stepper.homing_speed = Some(value.parse()?),
                    _ => tracing::warn!("Unknown stepper property: {}", property),
                }
            }
        }
        
        Ok(())
    }
    
    /// Parse fan configuration
    fn parse_fan_config(
        config: &mut Config,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Extract fan name (e.g., "fan_pin" -> "fan")
        if let Some(fan_name) = key.strip_prefix("fan_") {
            if fan_name == "pin" {
                // Main fan
                let fan = config.fans.entry("fan".to_string()).or_insert_with(FanConfig::default);
                fan.pin = value.to_string();
            } else if let Some(underscore_pos) = fan_name.find('_') {
                let name = fan_name[..underscore_pos].to_string();
                let property = &fan_name[underscore_pos + 1..];
                
                let fan = config.fans.entry(name).or_insert_with(FanConfig::default);
                
                match property {
                    "pin" => fan.pin = value.to_string(),
                    "max_power" => fan.max_power = Some(value.parse()?),
                    "shutdown_speed" => fan.shutdown_speed = Some(value.parse()?),
                    "cycle_time" => fan.cycle_time = Some(value.parse()?),
                    _ => tracing::warn!("Unknown fan property: {}", property),
                }
            }
        }
        
        Ok(())
    }
    
    /// Save configuration to TOML file
    pub fn save_config(&self, config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let toml_string = toml::to_string_pretty(self)?;
        std::fs::write(config_path, toml_string)?;
        Ok(())
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate printer settings
        if self.printer.max_velocity <= 0.0 {
            return Err("max_velocity must be positive".into());
        }
        
        if self.printer.max_accel <= 0.0 {
            return Err("max_accel must be positive".into());
        }
        
        // Validate MCU settings
        if self.mcu.serial.is_empty() {
            return Err("MCU serial port must be specified".into());
        }
        
        if self.mcu.baud == 0 {
            return Err("MCU baud rate must be positive".into());
        }
        
        // Validate extruder settings
        if self.extruder.rotation_distance <= 0.0 {
            return Err("extruder rotation_distance must be positive".into());
        }
        
        // Validate bed settings
        if self.heater_bed.heater_pin.is_empty() {
            return Err("heater_bed heater_pin must be specified".into());
        }
        
        // Validate stepper configurations
        for (name, stepper) in &self.steppers {
            if stepper.step_pin.is_empty() {
                return Err(format!("Stepper {} step_pin must be specified", name).into());
            }
            
            if stepper.rotation_distance <= 0.0 {
                return Err(format!("Stepper {} rotation_distance must be positive", name).into());
            }
        }
        
        Ok(())
    }
    
    /// Get steps per mm for a stepper
    pub fn get_steps_per_mm(&self, stepper_name: &str) -> f64 {
        if let Some(stepper) = self.steppers.get(stepper_name) {
            if stepper.full_steps_per_rotation > 0 && stepper.microsteps > 0 {
                stepper.full_steps_per_rotation as f64 * stepper.microsteps as f64 / stepper.rotation_distance
            } else {
                80.0 // Default fallback
            }
        } else {
            80.0 // Default fallback
        }
    }
    
    /// Get all stepper names
    pub fn get_stepper_names(&self) -> Vec<String> {
        self.steppers.keys().cloned().collect()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            printer: PrinterConfig::default(),
            mcu: McuConfig::default(),
            extruder: ExtruderConfig::default(),
            heater_bed: HeaterBedConfig::default(),
            fans: HashMap::new(),
            steppers: HashMap::new(),
            motion: MotionConfig::default(),
            advanced: AdvancedConfig::default(),
            web: WebConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.printer.max_velocity, 300.0);
        assert_eq!(config.printer.max_accel, 3000.0);
        assert_eq!(config.mcu.baud, 250000);
    }

    #[test]
    fn test_parse_legacy_config() {
        let legacy_config = r#"
# Printer configuration
kinematics = cartesian
max_velocity = 300.0
max_accel = 3000.0
max_z_velocity = 25.0
max_z_accel = 100.0

# MCU configuration
serial = /dev/ttyUSB0
baud = 250000

# Extruder configuration
extruder_step_pin = PA0
extruder_dir_pin = PA1
extruder_enable_pin = PA2
extruder_rotation_distance = 22.67895
extruder_microsteps = 16
nozzle_diameter = 0.4
filament_diameter = 1.75

# Heater bed configuration
heater_bed_heater_pin = PA3
heater_bed_sensor_type = EPCOS 100K B57560G104F
heater_bed_sensor_pin = PA4
heater_bed_min_temp = 0
heater_bed_max_temp = 130

# Stepper X
stepper_x_step_pin = PB0
stepper_x_dir_pin = PB1
stepper_x_enable_pin = PB2
stepper_x_rotation_distance = 40
stepper_x_microsteps = 16
stepper_x_full_steps_per_rotation = 200

# Stepper Y
stepper_y_step_pin = PB3
stepper_y_dir_pin = PB4
stepper_y_enable_pin = PB5
stepper_y_rotation_distance = 40
stepper_y_microsteps = 16
stepper_y_full_steps_per_rotation = 200

# Stepper Z
stepper_z_step_pin = PC0
stepper_z_dir_pin = PC1
stepper_z_enable_pin = PC2
stepper_z_rotation_distance = 8
stepper_z_microsteps = 16
stepper_z_full_steps_per_rotation = 200
        "#;
        
        let config = Config::parse_legacy_config(legacy_config).unwrap();
        
        assert_eq!(config.printer.kinematics, "cartesian");
        assert_eq!(config.printer.max_velocity, 300.0);
        assert_eq!(config.mcu.serial, "/dev/ttyUSB0");
        assert_eq!(config.steppers.len(), 3);
        
        let stepper_x = config.steppers.get("x").unwrap();
        assert_eq!(stepper_x.step_pin, "PB0");
        assert_eq!(stepper_x.rotation_distance, 40.0);
    }

    #[test]
    fn test_parse_toml_config() {
        let toml_config = r#"
[printer]
kinematics = "cartesian"
max_velocity = 300.0
max_accel = 3000.0

[mcu]
serial = "/dev/ttyUSB0"
baud = 250000

[extruder]
step_pin = "PA0"
dir_pin = "PA1"
enable_pin = "PA2"
rotation_distance = 22.67895
microsteps = 16
nozzle_diameter = 0.4
filament_diameter = 1.75

[heater_bed]
heater_pin = "PA3"
sensor_type = "EPCOS 100K B57560G104F"
sensor_pin = "PA4"
min_temp = 0.0
max_temp = 130.0

[steppers.stepper_x]
step_pin = "PB0"
dir_pin = "PB1"
enable_pin = "PB2"
rotation_distance = 40.0
microsteps = 16
full_steps_per_rotation = 200

[steppers.stepper_y]
step_pin = "PB3"
dir_pin = "PB4"
enable_pin = "PB5"
rotation_distance = 40.0
microsteps = 16
full_steps_per_rotation = 200
        "#;
        
        let config = Config::parse_toml(toml_config).unwrap();
        
        assert_eq!(config.printer.kinematics, "cartesian");
        assert_eq!(config.steppers.len(), 2);
        
        let stepper_x = config.steppers.get("stepper_x").unwrap();
        assert_eq!(stepper_x.step_pin, "PB0");
    }

    #[test]
    fn test_steps_per_mm_calculation() {
        let mut config = Config::default();
        
        let mut stepper = StepperConfig::default();
        stepper.full_steps_per_rotation = 200;
        stepper.microsteps = 16;
        stepper.rotation_distance = 40.0;
        
        config.steppers.insert("test".to_string(), stepper);
        
        let steps_per_mm = config.get_steps_per_mm("test");
        assert_eq!(steps_per_mm, 80.0); // (200 * 16) / 40 = 80
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Invalid max_velocity should fail
        config.printer.max_velocity = -1.0;
        assert!(config.validate().is_err());
        config.printer.max_velocity = 300.0; // Reset
        
        // Missing MCU serial should fail
        config.mcu.serial = String::new();
        assert!(config.validate().is_err());
    }
}