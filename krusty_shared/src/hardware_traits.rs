// Trait-based interfaces for modular hardware abstraction (shared)
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HardwareError {
    #[error("Serial port error: {0}")]
    Serial(#[from] std::io::Error),
    #[error("IO error: {0}")]
    Io(std::io::Error),
    #[error("Not connected to hardware")]
    NotConnected,
    #[error("Timeout waiting for response")]
    Timeout,
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

#[derive(Debug, Clone, Default)]
pub struct FanState {
    pub power: f64,
    pub is_on: bool,
    pub rpm: f64,
}

#[derive(Debug, Clone, Default)]
pub struct ThermistorState {
    pub measured_temp: f64,
    pub noise: f64,
    pub last_update: f64,
}

#[derive(Debug, Clone, Default)]
pub struct HeaterState {
    pub power: f64,
    pub target_temp: f64,
    pub current_temp: f64,
    pub is_on: bool,
    pub runaway_detected: bool,
    pub runaway_check_timer: f64,
    pub runaway_enabled: bool,
}

#[derive(Debug, Clone, Default)]
pub struct CommandStats {
    pub total_commands: u64,
    pub failed_commands: u64,
    pub last_command: Option<String>,
}

pub trait TemperatureControllerTrait: Send {
    fn set_target_temperature(&mut self, target: f64);
    fn get_current_temperature(&self) -> f64;
    fn update(&mut self, dt: f64) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;
}

pub trait StepperControllerTrait: Send {
    fn step(&mut self, steps: i32) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;
    fn enable(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;
    fn disable(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;
}

pub trait PeripheralTrait: Send {
    fn perform_action(&mut self, action: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;
}

// These traits can be implemented by hardware modules for extensibility and async safety.
// All methods return Result for robust error handling.
