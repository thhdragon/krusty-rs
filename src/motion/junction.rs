// src/motion/junction.rs - Fix JunctionDeviation definition
#[derive(Debug, Clone)]
pub struct JunctionDeviation {
    pub deviation: f64,
}

impl JunctionDeviation {
    pub fn new(deviation: f64) -> Self {
        Self { deviation }
    }
    
    // Add the missing calculate_unit_vector method
    pub fn calculate_unit_vector(start: &[f64; 4], end: &[f64; 4]) -> [f64; 4] {
        let delta = [
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

    /// Calculate the maximum junction speed based on the angle between two segments
    pub fn calculate_junction_speed(&self, prev: &[f64; 4], next: &[f64; 4], acceleration: f64) -> f64 {
        // Dot product for cosine of angle
        let dot = prev.iter().zip(next.iter()).map(|(a, b)| a * b).sum::<f64>();
        let cos_theta = dot.clamp(-1.0, 1.0);
        // If the direction doesn't change, allow max speed
        if cos_theta >= 0.9999 {
            return f64::INFINITY;
        }
        // Calculate sin(theta/2)
        let sin_half_theta = ((0.5 * (1.0 - cos_theta)).max(0.0)).sqrt();
        // Standard formula for junction speed
        let v = ((acceleration * self.deviation * sin_half_theta) / (1.0 - sin_half_theta)).sqrt();
        if v.is_finite() { v } else { 0.0 }
    }
}