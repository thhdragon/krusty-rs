// src/temperature/control.rs
use std::collections::VecDeque;
use std::time::Instant;
use super::hardware_traits::TemperatureControllerTrait;
use krusty_shared::{HeaterState, ThermistorState, ThermalEvent, TemperatureController};

// --- Heater/Thermistor Simulation Types ---
// Removed duplicate definitions of HeaterState, ThermistorState, ThermalEvent, and TemperatureController.

// REMOVED: impl HeaterState { ... }
// REMOVED: impl ThermistorState { ... }

// If any local methods or trait impls are needed for these types, implement them for the krusty_shared types here.

#[derive(Debug, Clone)]
pub struct TemperatureStatus {
    pub current: f64,
    pub target: f64,
    pub error: f64,
    pub output: f64,
}