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
}

// Make sure to export it properly
pub use self::JunctionDeviation;