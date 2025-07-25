// src/motion/s_curve.rs
/// S-curve motion profile generator
/// 
/// This implements smooth S-curve acceleration profiles that provide
/// better control over jerk and reduce vibrations compared to
/// trapezoidal profiles
pub struct SCurveGenerator {
    /// Maximum velocity (mm/s)
    max_velocity: f64,
    
    /// Maximum acceleration (mm/s²)
    max_acceleration: f64,
    
    /// Maximum jerk (mm/s³)
    max_jerk: f64,
}

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SCurveError {
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    #[error("Other: {0}")]
    Other(String),
}

impl SCurveGenerator {
    pub fn new(max_velocity: f64, max_acceleration: f64, max_jerk: f64) -> Self {
        Self {
            max_velocity,
            max_acceleration,
            max_jerk,
        }
    }

    /// Generate S-curve trajectory
    pub fn generate_s_curve(
        &self,
        distance: f64,
        start_velocity: f64,
        end_velocity: f64,
        cruise_velocity: f64,
    ) -> Result<Vec<MotionPoint>, SCurveError> {
        // S-curve consists of 7 phases:
        // 1. Jerk increase (acceleration increases linearly)
        // 2. Constant acceleration
        // 3. Jerk decrease (acceleration decreases linearly)
        // 4. Constant velocity (cruise)
        // 5. Jerk increase (deceleration increases linearly)
        // 6. Constant deceleration
        // 7. Jerk decrease (deceleration decreases linearly)

        let jerk_time = self.max_acceleration / self.max_jerk;
        let accel_distance = self.calculate_accel_distance(jerk_time);
        
        let total_accel_decel_distance = 2.0 * accel_distance;
        let cruise_distance = distance - total_accel_decel_distance;
        
        let mut trajectory = Vec::new();
        let mut time = 0.0;
        let mut position = 0.0;
        let mut velocity = start_velocity;
        
        // Phase 1: Jerk increase (positive)
        for t in (0..100).map(|i| i as f64 * jerk_time / 100.0) {
            let point = self.calculate_jerk_phase(t, jerk_time, start_velocity, 1.0);
            trajectory.push(point);
        }
        
        // Phase 2: Constant acceleration
        let const_accel_time = (cruise_velocity - start_velocity - self.max_acceleration * jerk_time) 
            / self.max_acceleration;
        
        if const_accel_time > 0.0 {
            for t in (0..50).map(|i| i as f64 * const_accel_time / 50.0) {
                let point = MotionPoint {
                    time: time + t + jerk_time,
                    position: position + start_velocity * jerk_time + 
                             0.5 * self.max_acceleration * jerk_time * jerk_time +
                             start_velocity * t + 
                             0.5 * self.max_acceleration * t * t,
                    velocity: start_velocity + self.max_acceleration * jerk_time + 
                             self.max_acceleration * t,
                    acceleration: self.max_acceleration,
                    jerk: 0.0,
                };
                trajectory.push(point);
            }
        }
        
        // Continue with remaining phases...
        // (Implementation would be quite lengthy, this is the concept)
        
        Ok(trajectory)
    }

    /// Generate and schedule S-curve trajectory and schedule step events
    pub fn generate_and_schedule_s_curve(
        &self,
        distance: f64,
        start_velocity: f64,
        end_velocity: f64,
        cruise_velocity: f64,
        axis: usize,
        event_queue: &std::sync::Arc<std::sync::Mutex<crate::simulator::event_queue::SimEventQueue>>,
        clock: &crate::simulator::event_queue::SimClock,
    ) -> Result<(), SCurveError> {
        let trajectory = self.generate_s_curve(distance, start_velocity, end_velocity, cruise_velocity)?;
        for point in trajectory {
            let event = crate::simulator::event_queue::SimEvent {
                timestamp: clock.current_time + std::time::Duration::from_secs_f64(point.time),
                event_type: crate::simulator::event_queue::SimEventType::Step,
                payload: Some(Box::new(crate::motion::stepper::StepCommand {
                    axis,
                    steps: (point.position.round() as u32),
                    direction: point.velocity >= 0.0,
                })),
            };
            event_queue.lock().unwrap().push(event);
        }
        Ok(())
    }

    /// Generate and schedule S-curve trajectories for all axes
    pub fn generate_and_schedule_multi_axis_s_curve(
        &self,
        distances: [f64; 4],
        start_velocities: [f64; 4],
        end_velocities: [f64; 4],
        cruise_velocities: [f64; 4],
        event_queue: &std::sync::Arc<std::sync::Mutex<crate::simulator::event_queue::SimEventQueue>>,
        clock: &crate::simulator::event_queue::SimClock,
    ) -> Result<(), SCurveError> {
        for axis in 0..4 {
            let trajectory = self.generate_s_curve(
                distances[axis],
                start_velocities[axis],
                end_velocities[axis],
                cruise_velocities[axis],
            )?;
            for point in trajectory {
                let event = crate::simulator::event_queue::SimEvent {
                    timestamp: clock.current_time + std::time::Duration::from_secs_f64(point.time),
                    event_type: crate::simulator::event_queue::SimEventType::Step,
                    payload: Some(Box::new(crate::motion::stepper::StepCommand {
                        axis,
                        steps: (point.position.round() as u32),
                        direction: point.velocity >= 0.0,
                    })),
                };
                event_queue.lock().unwrap().push(event);
            }
        }
        Ok(())
    }

    fn calculate_jerk_phase(&self, t: f64, jerk_time: f64, start_velocity: f64, direction: f64) -> MotionPoint {
        // During jerk phase: jerk = ±max_jerk
        // acceleration = ±max_jerk * t
        // velocity = start_velocity ± 0.5 * max_jerk * t²
        // position = start_position + start_velocity * t ± (1/6) * max_jerk * t³
        
        let acceleration = direction * self.max_jerk * t;
        let velocity = start_velocity + direction * 0.5 * self.max_jerk * t * t;
        let position = start_velocity * t + direction * (1.0/6.0) * self.max_jerk * t * t * t;
        
        MotionPoint {
            time: t,
            position,
            velocity,
            acceleration,
            jerk: direction * self.max_jerk,
        }
    }

    fn calculate_accel_distance(&self, jerk_time: f64) -> f64 {
        // Distance covered during full acceleration S-curve
        // This is complex math - simplified here
        self.max_acceleration * jerk_time * jerk_time
    }
}

/// Motion state at a specific point in time
#[derive(Debug, Clone)]
pub struct MotionPoint {
    pub time: f64,
    pub position: f64,
    pub velocity: f64,
    pub acceleration: f64,
    pub jerk: f64,
}