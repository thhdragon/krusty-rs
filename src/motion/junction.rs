// src/motion/junction.rs

#[derive(Debug, Clone)]
pub struct JunctionDeviation {
    pub deviation: f64,
}

impl JunctionDeviation {
    pub fn new(deviation: f64) -> Self {
        Self { deviation }
    }

    pub fn calculate_junction_speed(
        &self,
        unit_a: &[f64; 4],
        unit_b: &[f64; 4],
        acceleration: f64,
    ) -> f64 {
        let dot_product = unit_a[0] * unit_b[0] +
                         unit_a[1] * unit_b[1] +
                         unit_a[2] * unit_b[2] +
                         unit_a[3] * unit_b[3];

        let dot_product = dot_product.max(-1.0).min(1.0);
        let angle = dot_product.acos();

        if angle < 0.01 {
            return f64::INFINITY;
        }

        let tan_half_angle = (angle / 2.0).tan();
        let max_speed = (acceleration * self.deviation * tan_half_angle).sqrt();

        max_speed
    }

    // Add the missing calculate_unit_vector function
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

// Explicitly export if needed (though pub struct does this)
// pub use self::JunctionDeviation;