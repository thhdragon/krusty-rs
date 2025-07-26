// Trait-based interfaces for modular hardware abstraction (shared)

pub trait TemperatureControllerTrait: Send {
    fn set_target_temperature(&mut self, target: f64);
    fn get_current_temperature(&self) -> f64;
    fn update(&mut self, dt: f64) -> Result<(), Box<dyn std::error::Error + Send>>;
}

pub trait StepperControllerTrait: Send {
    fn step(&mut self, steps: i32) -> Result<(), Box<dyn std::error::Error + Send>>;
    fn enable(&mut self) -> Result<(), Box<dyn std::error::Error + Send>>;
    fn disable(&mut self) -> Result<(), Box<dyn std::error::Error + Send>>;
}

pub trait PeripheralTrait: Send {
    fn perform_action(&mut self, action: &str) -> Result<(), Box<dyn std::error::Error + Send>>;
}

// These traits can be implemented by hardware modules for extensibility and async safety.
// All methods return Result for robust error handling.
