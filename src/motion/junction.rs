// src/motion/junction.rs
/// Junction deviation calculation for smooth cornering
/// 
/// This module implements the junction deviation algorithm used in modern
/// 3D printer firmware to calculate optimal cornering speeds
pub struct JunctionDeviation {
    /// Junction deviation value (mm)
    /// Smaller values = tighter corners, larger values = smoother motion
    deviation: f64,
}

impl JunctionDeviation {
    pub fn new(deviation: f64) -> Self {
        Self { deviation }
    }

    /// Calculate maximum junction speed for smooth cornering
    /// 
    /// This uses the junction deviation formula to determine the maximum
    /// speed that can be achieved while maintaining the specified deviation
    /// from the corner path
    /// 
    /// # Arguments
    /// * `unit_a` - Unit vector of incoming move
    /// * `unit_b` - Unit vector of outgoing move
    /// * `acceleration` - Acceleration limit for this junction
    /// 
    /// # Returns
    /// * Maximum junction speed (mm/s)
    pub fn calculate_junction_speed(
        &self,
        unit_a: &[f64; 4],
        unit_b: &[f64; 4],
        acceleration: f64,
    ) -> f64 {
        // Calculate dot product of unit vectors
        let dot_product = unit_a[0] * unit_b[0] + 
                         unit_a[1] * unit_b[1] + 
                         unit_a[2] * unit_b[2] + 
                         unit_a[3] * unit_b[3];
        
        // Clamp dot product to valid range [-1, 1]
        let dot_product = dot_product.max(-1.0).min(1.0);
        
        // Calculate angle between vectors (in radians)
        let angle = dot_product.acos();
        
        // Special case: straight line or very small angle
        if angle < 0.01 {
            return f64::INFINITY; // No speed limit needed
        }
        
        // Calculate maximum junction speed using junction deviation formula
        // v = sqrt(a * d * tan(theta/2))
        // where d = deviation, a = acceleration, theta = angle between moves
        let tan_half_angle = (angle / 2.0).tan();
        let max_speed = (acceleration * self.deviation * tan_half_angle).sqrt();
        
        max_speed
    }

    /// Calculate unit vector for a move
    pub fn calculate_unit_vector(start: &[f64; 4], end: &[f64; 4]) -> [f64; 4] {
        let mut delta = [
            end[0] - start[0],
            end[1] - start[1],
            end[2] - start[2],
            end[3] - start[3],
        ];
        
        let distance = (delta[0] * delta[0] + 
                       delta[1] * delta[1] + 
                       delta[2] * delta[2] + 
                       delta[3] * delta[3]).sqrt();
        
        if distance > 0.0 {
            [
                delta[0] / distance,
                delta[1] / distance,
                delta[2] / distance,
                delta[3] / distance,
            ]
        } else {
            [0.0, 0.0, 0.0, 0.0]
        }
    }
}