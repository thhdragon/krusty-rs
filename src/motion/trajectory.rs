// src/motion/trajectory.rs
/// Trajectory generator for smooth motion with acceleration planning
/// 
/// This module implements trapezoidal velocity profiles and S-curve acceleration
/// for smooth, precise motion control
pub struct TrajectoryGenerator {
    /// Maximum velocity (mm/s)
    max_velocity: f64,
    
    /// Maximum acceleration (mm/s²)
    max_acceleration: f64,
    
    /// Maximum jerk (mm/s³)
    max_jerk: f64,
    
    /// Use S-curve acceleration instead of trapezoidal
    use_s_curve: bool,
}

/// Motion profile types
#[derive(Debug, Clone, PartialEq)]
pub enum MotionProfile {
    /// Trapezoidal velocity profile (accelerate, cruise, decelerate)
    Trapezoidal,
    
    /// S-curve velocity profile (smooth acceleration/deceleration)
    SCurve,
}

/// Motion state at a specific time
#[derive(Debug, Clone)]
pub struct MotionState {
    /// Position at this time (mm)
    pub position: f64,
    
    /// Velocity at this time (mm/s)
    pub velocity: f64,
    
    /// Acceleration at this time (mm/s²)
    pub acceleration: f64,
    
    /// Time since start of move (seconds)
    pub time: f64,
}

impl TrajectoryGenerator {
    /// Create a new trajectory generator
    pub fn new(
        max_velocity: f64,
        max_acceleration: f64,
        max_jerk: f64,
        use_s_curve: bool,
    ) -> Self {
        Self {
            max_velocity,
            max_acceleration,
            max_jerk,
            use_s_curve,
        }
    }

    /// Generate a motion trajectory for a linear move
    /// 
    /// # Arguments
    /// * `distance` - Total distance to move (mm)
    /// * `start_velocity` - Initial velocity (mm/s)
    /// * `end_velocity` - Final velocity (mm/s)
    /// * `requested_feedrate` - Desired cruising velocity (mm/s)
    /// 
    /// # Returns
    /// * `Vec<MotionState>` - Motion states at regular time intervals
    pub fn generate_trajectory(
        &self,
        distance: f64,
        start_velocity: f64,
        end_velocity: f64,
        requested_feedrate: f64,
    ) -> Result<Vec<MotionState>, Box<dyn std::error::Error>> {
        if distance <= 0.0 {
            return Ok(vec![]);
        }
        
        // Calculate cruise velocity (limited by distance and acceleration)
        let cruise_velocity = self.calculate_cruise_velocity(
            distance,
            start_velocity,
            end_velocity,
            requested_feedrate,
        );
        
        // Generate trajectory based on profile type
        if self.use_s_curve {
            self.generate_s_curve_trajectory(
                distance,
                start_velocity,
                end_velocity,
                cruise_velocity,
            )
        } else {
            self.generate_trapezoidal_trajectory(
                distance,
                start_velocity,
                end_velocity,
                cruise_velocity,
            )
        }
    }

    /// Calculate maximum achievable cruise velocity for a move
    fn calculate_cruise_velocity(
        &self,
        distance: f64,
        start_velocity: f64,
        end_velocity: f64,
        requested_feedrate: f64,
    ) -> f64 {
        // Limit by maximum velocity setting
        let mut cruise_velocity = requested_feedrate.min(self.max_velocity);
        
        // Limit by acceleration constraints
        // Calculate minimum distance needed to accelerate from start to max velocity
        let accel_distance = (cruise_velocity * cruise_velocity - start_velocity * start_velocity) 
            / (2.0 * self.max_acceleration);
        
        // Calculate minimum distance needed to decelerate from max velocity to end
        let decel_distance = (cruise_velocity * cruise_velocity - end_velocity * end_velocity) 
            / (2.0 * self.max_acceleration);
        
        // If total required distance exceeds available distance, we can't reach max velocity
        let total_required = accel_distance + decel_distance;
        if total_required > distance {
            // Solve for maximum achievable velocity
            // This is the velocity where accel_distance + decel_distance = distance
            // (v² - v₀²) / (2a) + (v² - v₁²) / (2a) = d
            // Solving for v: v = sqrt((v₀² + v₁² + 2ad) / 2)
            let achievable_velocity_squared = 
                (start_velocity * start_velocity + end_velocity * end_velocity + 2.0 * self.max_acceleration * distance) / 2.0;
            
            cruise_velocity = achievable_velocity_squared.sqrt().min(self.max_velocity);
        }
        
        cruise_velocity
    }

    /// Generate trapezoidal velocity profile trajectory
    fn generate_trapezoidal_trajectory(
        &self,
        distance: f64,
        start_velocity: f64,
        end_velocity: f64,
        cruise_velocity: f64,
    ) -> Result<Vec<MotionState>, Box<dyn std::error::Error>> {
        // Calculate phase distances
        let accel_distance = (cruise_velocity * cruise_velocity - start_velocity * start_velocity) 
            / (2.0 * self.max_acceleration);
        
        let decel_distance = (cruise_velocity * cruise_velocity - end_velocity * end_velocity) 
            / (2.0 * self.max_acceleration);
        
        let cruise_distance = distance - accel_distance - decel_distance;
        
        // Calculate phase times
        let accel_time = (cruise_velocity - start_velocity) / self.max_acceleration;
        let cruise_time = if cruise_distance > 0.0 {
            cruise_distance / cruise_velocity
        } else {
            0.0
        };
        let decel_time = (cruise_velocity - end_velocity) / self.max_acceleration;
        
        let total_time = accel_time + cruise_time + decel_time;
        
        // Generate trajectory points at regular intervals
        let time_step = 0.001; // 1ms intervals
        let mut trajectory = Vec::new();
        let mut current_time = 0.0;
        
        while current_time <= total_time {
            let state = self.calculate_trapezoidal_state(
                current_time,
                distance,
                start_velocity,
                end_velocity,
                cruise_velocity,
                accel_time,
                cruise_time,
                decel_time,
            );
            
            trajectory.push(state);
            current_time += time_step;
        }
        
        Ok(trajectory)
    }

    /// Calculate motion state at specific time for trapezoidal profile
    fn calculate_trapezoidal_state(
        &self,
        time: f64,
        distance: f64,
        start_velocity: f64,
        end_velocity: f64,
        cruise_velocity: f64,
        accel_time: f64,
        cruise_time: f64,
        decel_time: f64,
    ) -> MotionState {
        let total_time = accel_time + cruise_time + decel_time;
        
        if time <= 0.0 {
            return MotionState {
                position: 0.0,
                velocity: start_velocity,
                acceleration: self.max_acceleration,
                time: 0.0,
            };
        }
        
        if time >= total_time {
            return MotionState {
                position: distance,
                velocity: end_velocity,
                acceleration: -self.max_acceleration,
                time: total_time,
            };
        }
        
        // Acceleration phase
        if time <= accel_time {
            let velocity = start_velocity + self.max_acceleration * time;
            let position = start_velocity * time + 0.5 * self.max_acceleration * time * time;
            return MotionState {
                position,
                velocity,
                acceleration: self.max_acceleration,
                time,
            };
        }
        
        // Cruise phase
        if time <= accel_time + cruise_time {
            let cruise_time_elapsed = time - accel_time;
            let position = (start_velocity * accel_time + 0.5 * self.max_acceleration * accel_time * accel_time)
                + cruise_velocity * cruise_time_elapsed;
            return MotionState {
                position,
                velocity: cruise_velocity,
                acceleration: 0.0,
                time,
            };
        }
        
        // Deceleration phase
        let decel_time_elapsed = time - accel_time - cruise_time;
        let velocity = cruise_velocity - self.max_acceleration * decel_time_elapsed;
        let position = (start_velocity * accel_time + 0.5 * self.max_acceleration * accel_time * accel_time)
            + cruise_velocity * cruise_time
            + cruise_velocity * decel_time_elapsed - 0.5 * self.max_acceleration * decel_time_elapsed * decel_time_elapsed;
        
        MotionState {
            position,
            velocity,
            acceleration: -self.max_acceleration,
            time,
        }
    }

    /// Generate S-curve trajectory (simplified implementation)
    fn generate_s_curve_trajectory(
        &self,
        distance: f64,
        start_velocity: f64,
        end_velocity: f64,
        cruise_velocity: f64,
    ) -> Result<Vec<MotionState>, Box<dyn std::error::Error>> {
        // S-curve implementation would involve:
        // 1. Jerk-limited acceleration phases
        // 2. Constant acceleration phases
        // 3. Jerk-limited deceleration phases
        // 4. Constant velocity cruise phase
        
        // For now, we'll fall back to trapezoidal
        // A full S-curve implementation is quite complex
        self.generate_trapezoidal_trajectory(
            distance,
            start_velocity,
            end_velocity,
            cruise_velocity,
        )
    }

    /// Get duration of a move with given parameters
    pub fn calculate_move_duration(
        &self,
        distance: f64,
        start_velocity: f64,
        end_velocity: f64,
        requested_feedrate: f64,
    ) -> f64 {
        let cruise_velocity = self.calculate_cruise_velocity(
            distance,
            start_velocity,
            end_velocity,
            requested_feedrate,
        );
        
        let accel_time = (cruise_velocity - start_velocity) / self.max_acceleration;
        let decel_time = (cruise_velocity - end_velocity) / self.max_acceleration;
        
        let accel_distance = (cruise_velocity * cruise_velocity - start_velocity * start_velocity) 
            / (2.0 * self.max_acceleration);
        let decel_distance = (cruise_velocity * cruise_velocity - end_velocity * end_velocity) 
            / (2.0 * self.max_acceleration);
        let cruise_distance = distance - accel_distance - decel_distance;
        
        let cruise_time = if cruise_distance > 0.0 {
            cruise_distance / cruise_velocity
        } else {
            0.0
        };
        
        accel_time + cruise_time + decel_time
    }
}