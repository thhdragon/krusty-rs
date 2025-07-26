//! Board abstraction and pin mapping for Krusty Simulator (shared)

use std::collections::{HashMap, HashSet};

/// Represents a physical or virtual board configuration
#[derive(Debug, Clone)]
pub struct BoardConfig {
    /// Board name/model
    pub name: String,
    /// Pin mapping for steppers, heaters, fans, sensors, etc.
    pub pins: HashMap<String, u32>,
    /// Timing constraints (e.g., max step rate)
    pub timing: BoardTiming,
    /// Other board-specific features
    pub features: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct BoardTiming {
    pub max_step_rate: u32,
    pub min_pulse_width_ns: u32,
    pub comm_baud: u32,
}

impl BoardConfig {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            pins: HashMap::new(),
            timing: BoardTiming {
                max_step_rate: 100_000,
                min_pulse_width_ns: 500,
                comm_baud: 250_000,
            },
            features: HashSet::new(),
        }
    }
}
