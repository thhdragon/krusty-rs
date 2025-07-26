// krusty_shared: shared traits and types for host, simulator, and MCU

pub mod event_queue;
pub mod gcode_utils;
pub mod trajectory;
pub mod s_curve;
pub mod shaper;
pub mod print_job;
pub mod api_models;

// --- Shared Traits and Types ---

// TimeInterface trait
pub trait TimeInterface: Send + Sync {
    fn now_monotonic(&self) -> std::time::Instant;
    fn now_wallclock(&self) -> std::time::SystemTime;
    fn sleep(&self, duration: std::time::Duration);
}

// Kinematics traits
pub trait Kinematics: KinematicsClone + Send + Sync {
    fn cartesian_to_motors(&self, cartesian: &[f64; 3]) -> Result<[f64; 4], Box<dyn std::error::Error>>;
    fn motors_to_cartesian(&self, motors: &[f64; 4]) -> Result<[f64; 3], Box<dyn std::error::Error>>;
    fn is_valid_position(&self, cartesian: &[f64; 3]) -> bool;
}

pub trait KinematicsClone {
    fn clone_box(&self) -> Box<dyn Kinematics + Send + Sync>;
}

impl<T> KinematicsClone for T
where
    T: 'static + Kinematics + Clone + Send + Sync,
{
    fn clone_box(&self) -> Box<dyn Kinematics + Send + Sync> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Kinematics + Send + Sync> {
    fn clone(&self) -> Box<dyn Kinematics + Send + Sync> {
        self.clone_box()
    }
}

// InputShaperTrait
pub trait InputShaperTrait {
    fn do_step(&mut self, input: f64) -> f64;
}

// AuthBackend trait
use async_trait::async_trait;
#[async_trait]
pub trait AuthBackend: Send + Sync + 'static {
    async fn validate(&self, username: &str, password: &str) -> bool;
}

// --- Shared simulation types for temperature and stepper ---
use std::collections::VecDeque;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct HeaterState {
    pub power: f32,           // 0.0-1.0
    pub target_temp: f32,     // °C
    pub current_temp: f32,    // °C
    pub is_on: bool,
    pub runaway_detected: bool,
    pub runaway_check_timer: f32, // seconds since heater turned on
    pub runaway_enabled: bool,    // Only enable runaway detection after close to target
}

#[derive(Debug, Clone)]
pub struct ThermistorState {
    pub measured_temp: f32,   // °C
    pub noise: f32,           // Simulated sensor noise
    pub last_update: f64,     // Sim time
}

#[derive(Debug, Clone)]
pub enum ThermalEvent {
    HeaterOn,
    HeaterOff,
    TempUpdate(f32),
    RunawayDetected,
    Recovery,
}

impl HeaterState {
    pub fn update(&mut self, dt: f32, ambient: f32) -> ThermalEvent {
        let max_temp = 300.0;
        let overshoot_threshold = 30.0;
        let runaway_triggered = self.current_temp > max_temp || self.current_temp > self.target_temp + overshoot_threshold;
        if runaway_triggered {
            self.runaway_detected = true;
            self.is_on = false;
        }
        let runaway_threshold = 10.0;
        if self.is_on {
            let max_delta = 12.0;
            let heat_gain = self.power * max_delta * dt;
            let heat_loss = 0.02 * (self.current_temp - ambient) * dt;
            self.current_temp += heat_gain - heat_loss;
            if !self.runaway_enabled && self.current_temp >= self.target_temp - runaway_threshold {
                self.runaway_enabled = true;
                self.runaway_check_timer = 0.0;
            }
            if self.runaway_enabled && self.power > 0.5 {
                self.runaway_check_timer += dt;
            } else if self.runaway_enabled {
                self.runaway_check_timer = 0.0;
            }
        } else {
            let heat_loss = 0.1 * (self.current_temp - ambient) * dt;
            self.current_temp -= heat_loss;
            self.runaway_check_timer = 0.0;
            self.runaway_enabled = false;
        }
        if (self.current_temp < self.target_temp - runaway_threshold) && self.runaway_enabled {
            self.runaway_enabled = false;
            self.runaway_check_timer = 0.0;
        }
        if self.is_on && self.runaway_enabled && self.power > 0.5 && self.runaway_check_timer > 10.0 && self.current_temp < self.target_temp - runaway_threshold {
            self.runaway_detected = true;
            self.is_on = false;
            return ThermalEvent::RunawayDetected;
        }
        if runaway_triggered {
            return ThermalEvent::RunawayDetected;
        }
        ThermalEvent::TempUpdate(self.current_temp)
    }
}

impl ThermistorState {
    pub fn update(&mut self, true_temp: f32, dt: f32) {
        let lag = 0.8;
        self.measured_temp += lag * (true_temp - self.measured_temp) * dt;
        self.measured_temp += self.noise * (rand::random::<f32>() - 0.5);
        self.last_update += dt as f64;
    }
}

#[derive(Debug, Clone)]
pub struct TemperatureController {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
    pub integral: f64,
    pub previous_error: f64,
    pub previous_time: Option<Instant>,
    pub target_temperature: f64,
    pub current_temperature: f64,
    pub temperature_history: VecDeque<(Instant, f64)>,
    pub output_history: VecDeque<(Instant, f64)>,
}

impl TemperatureController {
    pub fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self {
            kp,
            ki,
            kd,
            integral: 0.0,
            previous_error: 0.0,
            previous_time: None,
            target_temperature: 0.0,
            current_temperature: 0.0,
            temperature_history: VecDeque::new(),
            output_history: VecDeque::new(),
        }
    }
    pub fn set_target(&mut self, target: f64) {
        self.target_temperature = target;
    }
    pub fn update_temperature(&mut self, measured: f64) {
        self.current_temperature = measured;
    }
    pub fn set_target_temperature(&mut self, target: f64) {
        self.set_target(target);
    }
    pub fn get_current_temperature(&self) -> f64 {
        self.current_temperature
    }
    pub fn calculate_output(&mut self) -> f64 {
        let error = self.target_temperature - self.current_temperature;
        self.integral += error;
        let derivative = error - self.previous_error;
        self.previous_error = error;
        self.kp * error + self.ki * self.integral + self.kd * derivative
    }
}

#[derive(Debug, Clone)]
pub struct StepCommand {
    pub axis: usize,
    pub steps: u32,
    pub direction: bool,
}

impl StepCommand {
    pub fn to_mcu_command(&self) -> String {
        let axis_name = match self.axis {
            0 => "X",
            1 => "Y",
            2 => "Z",
            3 => "E",
            _ => "U",
        };
        format!("step {} {} {}", axis_name, self.steps, if self.direction { 1 } else { 0 })
    }
}

#[derive(Debug, Clone)]
pub struct StepGenerator {
    pub steps_per_mm: [f64; 4],
    pub direction_invert: [bool; 4],
    pub current_steps: [i64; 4],
}

impl StepGenerator {
    pub fn new(steps_per_mm: [f64; 4], direction_invert: [bool; 4]) -> Self {
        Self {
            steps_per_mm,
            direction_invert,
            current_steps: [0; 4],
        }
    }

    pub fn generate_steps(&mut self, position: &[f64; 4]) -> Vec<StepCommand> {
        let mut target_steps = [0i64; 4];
        for i in 0..4 {
            target_steps[i] = (position[i] * self.steps_per_mm[i]).round() as i64;
        }
        let mut step_deltas = [0i64; 4];
        for i in 0..4 {
            step_deltas[i] = target_steps[i] - self.current_steps[i];
        }
        self.current_steps = target_steps;
        let mut commands = Vec::new();
        for i in 0..4 {
            if step_deltas[i] != 0 {
                let steps = step_deltas[i].abs() as u32;
                let direction = if step_deltas[i] > 0 {
                    !self.direction_invert[i]
                } else {
                    self.direction_invert[i]
                };
                commands.push(StepCommand {
                    axis: i,
                    steps,
                    direction,
                });
            }
        }
        commands
    }

    pub fn reset_steps(&mut self) {
        self.current_steps = [0; 4];
    }
}

#[derive(Debug, Clone)]
pub struct FanState {
    pub power: f32,    // 0.0-1.0
    pub is_on: bool,
    pub rpm: f32,      // Simulated RPM
}

impl FanState {
    pub fn update(&mut self, dt: f32) {
        let target_rpm = if self.is_on { self.power * 2500.0 } else { 0.0 };
        let ramp = 500.0 * dt;
        if self.rpm < target_rpm {
            self.rpm = (self.rpm + ramp).min(target_rpm);
        } else {
            self.rpm = (self.rpm - ramp).max(target_rpm);
        }
    }
}

#[derive(Debug, Clone)]
pub struct SwitchState {
    pub is_on: bool,
    pub debounce_counter: u32, // For future debounce simulation
}

impl SwitchState {
    pub fn update(&mut self, _dt: f32) {
        // For now, no debounce simulation
    }
}

use serde::{Serialize, Deserialize};

#[derive(Debug, Default, Clone, Serialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub e: f64,
}

#[derive(Debug, Clone)]
pub struct HardwareState {
    pub heater_on: bool, // LEGACY: use heater_state.is_on instead
    pub fan_on: bool,    // LEGACY: not yet simulated
    pub switch_on: bool, // LEGACY: not yet simulated
    pub heater_state: HeaterState,
    pub thermistor_state: ThermistorState,
    pub fan_state: FanState, // Simulated fan state
    pub switch_state: SwitchState, // Simulated switch state
}
