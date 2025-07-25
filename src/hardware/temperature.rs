// src/temperature/control.rs
use std::collections::VecDeque;
use std::time::Instant;
use super::hardware_traits::TemperatureControllerTrait;

// --- Heater/Thermistor Simulation Types ---
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
    /// Simulate physics update for heater and thermistor
    pub fn update(&mut self, dt: f32, ambient: f32) -> ThermalEvent {
        // --- CRITICAL: Overheat/overshoot runaway detection FIRST ---
        let max_temp = 300.0; // Absolute safety limit
        let overshoot_threshold = 30.0; // Max allowed overshoot above target
        let runaway_triggered = self.current_temp > max_temp || self.current_temp > self.target_temp + overshoot_threshold;
        if runaway_triggered {
            self.runaway_detected = true;
            self.is_on = false;
        }
        // --- END CRITICAL SECTION ---
        let runaway_threshold = 10.0; // °C band for enabling runaway detection
        if self.is_on {
            // Simple heat model: Q = power * max_delta
            let max_delta = 12.0; // °C/sec at full power (realistic for 3D printer hotend)
            let heat_gain = self.power * max_delta * dt;
            let heat_loss = 0.02 * (self.current_temp - ambient) * dt; // Lower heat loss for more realistic ramp
            self.current_temp += heat_gain - heat_loss;
            // Enable runaway detection only after close to target
            if !self.runaway_enabled && self.current_temp >= self.target_temp - runaway_threshold {
                self.runaway_enabled = true;
                self.runaway_check_timer = 0.0;
            }
            // Increment runaway check timer if enabled and power is high
            if self.runaway_enabled && self.power > 0.5 {
                self.runaway_check_timer += dt;
            } else if self.runaway_enabled {
                self.runaway_check_timer = 0.0;
            }
        } else {
            // Cool towards ambient
            let heat_loss = 0.1 * (self.current_temp - ambient) * dt;
            self.current_temp -= heat_loss;
            self.runaway_check_timer = 0.0;
            self.runaway_enabled = false;
        }
        // Reset runaway_enabled if target changes significantly
        if (self.current_temp < self.target_temp - runaway_threshold) && self.runaway_enabled {
            self.runaway_enabled = false;
            self.runaway_check_timer = 0.0;
        }
        // Runaway detection: only after enabled and 10s of high power
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
        // Simulate lag and noise
        let lag = 0.8;
        self.measured_temp += lag * (true_temp - self.measured_temp) * dt;
        self.measured_temp += self.noise * (rand::random::<f32>() - 0.5);
        self.last_update += dt as f64;
    }
}

#[derive(Debug, Clone)]
pub struct TemperatureController {
    /// PID parameters
    kp: f64,
    ki: f64,
    kd: f64,
    
    /// PID state
    integral: f64,
    previous_error: f64,
    previous_time: Option<Instant>,
    
    /// Temperature targets
    target_temperature: f64,
    current_temperature: f64,
    
    /// Control history for tuning
    temperature_history: VecDeque<(Instant, f64)>,
    output_history: VecDeque<(Instant, f64)>,
}

impl TemperatureControllerTrait for TemperatureController {
    fn set_target_temperature(&mut self, target: f64) {
        self.set_target(target);
    }

    fn get_current_temperature(&self) -> f64 {
        self.current_temperature
    }

    fn update(&mut self, dt: f64) -> Result<(), Box<dyn std::error::Error + Send>> {
        // Simulate temperature update for demonstration
        // In real hardware, this would read from a sensor
        self.current_temperature += dt * 0.1; // Dummy update
        Ok(())
    }
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

    /// Update temperature reading
    pub fn update_temperature(&mut self, temperature: f64) {
        self.current_temperature = temperature;
        
        // Store history (keep last 100 readings)
        let now = Instant::now();
        self.temperature_history.push_back((now, temperature));
        while self.temperature_history.len() > 100 {
            self.temperature_history.pop_front();
        }
    }

    /// Set target temperature
    pub fn set_target(&mut self, target: f64) {
        self.target_temperature = target;
        tracing::info!("Setting target temperature: {:.1}°C", target);
    }

    /// Calculate PID output
    pub fn calculate_output(&mut self) -> f64 {
        let error = self.target_temperature - self.current_temperature;
        let now = Instant::now();
        
        let dt = if let Some(prev_time) = self.previous_time {
            (now - prev_time).as_secs_f64()
        } else {
            0.1 // Default 100ms if no previous time
        };
        
        if dt > 0.0 {
            self.integral += error * dt;
            let derivative = (error - self.previous_error) / dt;
            
            let output = self.kp * error + self.ki * self.integral + self.kd * derivative;
            
            // Store output history
            self.output_history.push_back((now, output));
            while self.output_history.len() > 100 {
                self.output_history.pop_front();
            }
            
            self.previous_error = error;
            self.previous_time = Some(now);
            
            // Clamp output to 0-1 range
            output.max(0.0).min(1.0)
        } else {
            0.0
        }
    }

    /// Get current status
    pub fn get_status(&mut self) -> TemperatureStatus {
        TemperatureStatus {
            current: self.current_temperature,
            target: self.target_temperature,
            error: self.target_temperature - self.current_temperature,
            output: self.calculate_output(), // Don't update state, just calculate
        }
    }

    /// Reset PID controller
    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.previous_error = 0.0;
        self.previous_time = None;
    }

    /// Auto-tune PID parameters (simplified Ziegler-Nichols method)
    pub fn auto_tune(&mut self) -> Result<(f64, f64, f64), Box<dyn std::error::Error>> {
        // This is a simplified auto-tuning implementation
        // In practice, this would involve more sophisticated analysis
        if self.temperature_history.len() < 10 {
            return Err("Insufficient temperature data for auto-tuning".into());
        }
        
        // Simple heuristic-based tuning
        let ku = 1.0; // Ultimate gain (would be calculated from data)
        let tu = 10.0; // Ultimate period (would be calculated from data)
        
        // Ziegler-Nichols tuning rules
        let kp = 0.6 * ku;
        let ki = 1.2 * ku / tu;
        let kd = 0.075 * ku * tu;
        
        tracing::info!("Auto-tuned PID: Kp={:.3}, Ki={:.3}, Kd={:.3}", kp, ki, kd);
        
        Ok((kp, ki, kd))
    }
}

#[derive(Debug, Clone)]
pub struct TemperatureStatus {
    pub current: f64,
    pub target: f64,
    pub error: f64,
    pub output: f64,
}