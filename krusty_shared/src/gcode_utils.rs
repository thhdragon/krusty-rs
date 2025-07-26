//! Shared G-code and hardware parsing utilities for Krusty

use crate::{HardwareState, Position};

// Implementation will be moved from krusty_simulator/src/main.rs

// NOTE: The following functions require Position and HardwareState types to be in scope or imported from krusty_shared or the consumer crate.

pub fn parse_gcode_line(line: &str, last_pos: &Position) -> Position {
    let mut pos = last_pos.clone();
    let tokens: Vec<&str> = line.split_whitespace().collect();
    for token in tokens {
        if token.starts_with("X") {
            pos.x = token[1..].parse().unwrap_or(pos.x);
        } else if token.starts_with("Y") {
            pos.y = token[1..].parse().unwrap_or(pos.y);
        } else if token.starts_with("Z") {
            pos.z = token[1..].parse().unwrap_or(pos.z);
        } else if token.starts_with("E") {
            pos.e = token[1..].parse().unwrap_or(pos.e);
        }
    }
    pos
}

pub fn parse_loop_conditional(line: &str, pos: &Position) -> (bool, bool) {
    // Example: ;LOOP X<10 or ;IF Y>20
    let mut loop_until_hit = false;
    let mut conditional = false;
    if line.contains(";LOOP") {
        loop_until_hit = true;
    }
    if line.contains(";IF") {
        conditional = true;
    }
    (loop_until_hit, conditional)
}

pub fn parse_hardware_command(line: &str, hw: &mut HardwareState) {
    // Heater GCode
    if line.contains("M104") {
        hw.heater_on = true; // LEGACY
        hw.heater_state.is_on = true;
        // Parse target temp if present (e.g. M104 S200)
        if let Some(s_idx) = line.find("S") {
            if let Some(temp) = line[s_idx+1..].split_whitespace().next() {
                if let Ok(t) = temp.parse::<f32>() {
                    hw.heater_state.target_temp = t;
                }
            }
        }
        hw.heater_state.power = 1.0;
    }
    if line.contains("M140") {
        hw.heater_on = false; // LEGACY
        hw.heater_state.is_on = false;
        hw.heater_state.power = 0.0;
    }
    // Fan GCode
    if line.contains("M106") {
        hw.fan_on = true; // LEGACY
        hw.fan_state.is_on = true;
        // Parse fan power if present (e.g. M106 S128)
        if let Some(s_idx) = line.find("S") {
            if let Some(pwm) = line[s_idx+1..].split_whitespace().next() {
                if let Ok(val) = pwm.parse::<u32>() {
                    hw.fan_state.power = (val as f32) / 255.0;
                }
            }
        } else {
            hw.fan_state.power = 1.0;
        }
    }
    if line.contains("M107") {
        hw.fan_on = false; // LEGACY
        hw.fan_state.is_on = false;
        hw.fan_state.power = 0.0;
    }
    // Switch GCode
    if line.contains("M80") {
        hw.switch_on = true; // LEGACY
        hw.switch_state.is_on = true;
    }
    if line.contains("M81") {
        hw.switch_on = false; // LEGACY
        hw.switch_state.is_on = false;
    }
}
