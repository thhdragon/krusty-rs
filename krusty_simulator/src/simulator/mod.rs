pub mod event_queue;

use crate::simulator::event_queue::{SimEventQueue, SimClock};
use std::sync::{Arc, Mutex};
use krusty_shared::{HeaterState, ThermistorState, TemperatureController, ThermalEvent, StepCommand};

pub struct Simulator {
    pub event_queue: Arc<Mutex<SimEventQueue>>,
    pub clock: SimClock,
    pub stepper_positions: [i64; 4], // X, Y, Z, E
    pub heater: HeaterState,
    pub thermistor: ThermistorState,
    pub temp_controller: TemperatureController, // NEW
}

impl Simulator {
    pub fn new(event_queue: Arc<Mutex<SimEventQueue>>, clock: SimClock) -> Self {
        // Example: Start simulation clock and event loop
        tracing::info!("Simulator initialized at time: {:?}", clock.current_time);
        Self {
            event_queue,
            clock,
            stepper_positions: [0; 4],
            heater: HeaterState {
                power: 0.0,
                target_temp: 200.0,
                current_temp: 25.0,
                is_on: true,
                runaway_detected: false,
                runaway_check_timer: 0.0, // NEW FIELD
                runaway_enabled: false,   // NEW FIELD
            },
            thermistor: ThermistorState {
                measured_temp: 25.0,
                noise: 0.5,
                last_update: 0.0,
            },
            temp_controller: TemperatureController::new(2.0, 0.08, 3.0), // PID params tuned for realistic hotend
        }
    }

    /// Run the simulation event loop, processing step events
    pub fn run_event_loop(&mut self) {
        loop {
            let mut queue = self.event_queue.lock().unwrap();
            if let Some(event) = queue.pop() {
                self.clock.advance(event.timestamp - self.clock.current_time);
                match event.event_type {
                    crate::simulator::event_queue::SimEventType::Step => {
                        if let Some(payload) = event.payload {
                            if let Some(step_cmd) = payload.downcast_ref::<StepCommand>() {
                                // Update stepper state
                                let axis = step_cmd.axis;
                                let steps = step_cmd.steps as i64;
                                let direction = if step_cmd.direction { 1 } else { -1 };
                                self.stepper_positions[axis] += direction * steps;
                                tracing::info!("Stepper event: axis={}, steps={}, direction={}, new_pos={}", axis, steps, direction, self.stepper_positions[axis]);
                            }
                        }
                    }
                    crate::simulator::event_queue::SimEventType::HeaterUpdate => {
                        // Heater/thermistor physics update
                        let dt = 0.1; // 100ms step
                        let ambient = 25.0;
                        // --- PID CONTROL ---
                        self.temp_controller.update_temperature(self.heater.current_temp as f64);
                        self.temp_controller.set_target(self.heater.target_temp as f64);
                        let pid_power = self.temp_controller.calculate_output() as f32;
                        self.heater.power = if self.heater.is_on { pid_power } else { 0.0 };
                        // --- PHYSICS ---
                        let thermal_event = self.heater.update(dt, ambient);
                        self.thermistor.update(self.heater.current_temp, dt);
                        // Log state (replace with output hook as needed)
                        println!("[Heater] t={:.2}s, power={:.2}, T={:.2}C, target={:.2}C, measured={:.2}C, runaway={}",
                            self.clock.current_time.as_secs_f32(),
                            self.heater.power,
                            self.heater.current_temp,
                            self.heater.target_temp,
                            self.thermistor.measured_temp,
                            self.heater.runaway_detected);
                        match thermal_event {
                            ThermalEvent::RunawayDetected => {
                                println!("[ThermalEvent] Runaway detected! Heater shut off.");
                            }
                            _ => {}
                        }
                        // Schedule next update
                        let next_time = self.clock.current_time + std::time::Duration::from_millis(100);
                        queue.push(crate::simulator::event_queue::SimEvent {
                            timestamp: next_time,
                            event_type: crate::simulator::event_queue::SimEventType::HeaterUpdate,
                            payload: None,
                        });
                    }
                    // ...handle other event types...
                    _ => {}
                }
            } else {
                break; // No more events to process
            }
        }
    }

    /// Run the simulation event loop with a simulated time limit
    pub fn run_event_loop_with_timeout(&mut self, max_time: std::time::Duration) {
        loop {
            if self.clock.current_time >= max_time {
                println!("[Sim] Timeout reached: {:.2}s, exiting simulation.", self.clock.current_time.as_secs_f32());
                break;
            }
            let mut queue = self.event_queue.lock().unwrap();
            if let Some(event) = queue.pop() {
                self.clock.advance(event.timestamp - self.clock.current_time);
                match event.event_type {
                    crate::simulator::event_queue::SimEventType::Step => {
                        if let Some(payload) = event.payload {
                            if let Some(step_cmd) = payload.downcast_ref::<StepCommand>() {
                                let axis = step_cmd.axis;
                                let steps = step_cmd.steps as i64;
                                let direction = if step_cmd.direction { 1 } else { -1 };
                                self.stepper_positions[axis] += direction * steps;
                                tracing::info!("Stepper event: axis={}, steps={}, direction={}, new_pos={}", axis, steps, direction, self.stepper_positions[axis]);
                            }
                        }
                    }
                    crate::simulator::event_queue::SimEventType::HeaterUpdate => {
                        let dt = 0.1;
                        let ambient = 25.0;
                        self.temp_controller.update_temperature(self.heater.current_temp as f64);
                        self.temp_controller.set_target(self.heater.target_temp as f64);
                        let pid_power = self.temp_controller.calculate_output() as f32;
                        self.heater.power = if self.heater.is_on { pid_power } else { 0.0 };
                        let thermal_event = self.heater.update(dt, ambient);
                        self.thermistor.update(self.heater.current_temp, dt);
                        println!("[Heater] t={:.2}s, power={:.2}, T={:.2}C, target={:.2}C, measured={:.2}C, runaway={}",
                            self.clock.current_time.as_secs_f32(),
                            self.heater.power,
                            self.heater.current_temp,
                            self.heater.target_temp,
                            self.thermistor.measured_temp,
                            self.heater.runaway_detected);
                        match thermal_event {
                            ThermalEvent::RunawayDetected => {
                                println!("[ThermalEvent] Runaway detected! Heater shut off.");
                            }
                            _ => {}
                        }
                        let next_time = self.clock.current_time + std::time::Duration::from_millis(100);
                        queue.push(crate::simulator::event_queue::SimEvent {
                            timestamp: next_time,
                            event_type: crate::simulator::event_queue::SimEventType::HeaterUpdate,
                            payload: None,
                        });
                    }
                    _ => {}
                }
            } else {
                break;
            }
        }
    }
}
