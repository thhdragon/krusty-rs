// src/motion/kinematics.rs
use krusty_shared::KinematicsType;

/// Kinematics handler for different printer types
pub trait Kinematics: KinematicsClone + Send + Sync {
    /// Convert Cartesian coordinates to motor positions
    fn cartesian_to_motors(&self, cartesian: &[f64; 3]) -> Result<[f64; 4], Box<dyn std::error::Error>>;
    
    /// Convert motor positions to Cartesian coordinates
    fn motors_to_cartesian(&self, motors: &[f64; 4]) -> Result<[f64; 3], Box<dyn std::error::Error>>;
    
    /// Check if position is valid for this kinematics
    fn is_valid_position(&self, cartesian: &[f64; 3]) -> bool;
}

pub trait KinematicsClone {
    fn clone_box(&self) -> Box<dyn Kinematics + Send + Sync>;
}

impl<T> KinematicsClone for T
where
    T: 'static + Kinematics + Clone + Send + Sync,
{
    fn clone_box(&self) -> Box<dyn Kinematics + Send + Sync> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Kinematics + Send + Sync> {
    fn clone(&self) -> Box<dyn Kinematics + Send + Sync> {
        self.clone_box()
    }
}

/// Cartesian kinematics (most common 3D printer type)
#[derive(Debug, Clone)]
pub struct CartesianKinematics {
    /// Limits for each axis
    limits: [[f64; 2]; 3], // [min, max] for X, Y, Z
}

impl CartesianKinematics {
    pub fn new(limits: [[f64; 2]; 3]) -> Self {
        Self { limits }
    }
}

impl Kinematics for CartesianKinematics {
    fn cartesian_to_motors(&self, cartesian: &[f64; 3]) -> Result<[f64; 4], Box<dyn std::error::Error>> {
        // For Cartesian, motors directly correspond to axes
        // [X, Y, Z, E]
        Ok([cartesian[0], cartesian[1], cartesian[2], 0.0])
    }
    
    fn motors_to_cartesian(&self, motors: &[f64; 4]) -> Result<[f64; 3], Box<dyn std::error::Error>> {
        Ok([motors[0], motors[1], motors[2]])
    }
    
    fn is_valid_position(&self, cartesian: &[f64; 3]) -> bool {
        for i in 0..3 {
            if cartesian[i] < self.limits[i][0] || cartesian[i] > self.limits[i][1] {
                return false;
            }
        }
        true
    }
}

/// CoreXY kinematics
#[derive(Debug, Clone)]
pub struct CoreXYKinematics {
    limits: [[f64; 2]; 3],
}

impl CoreXYKinematics {
    pub fn new(limits: [[f64; 2]; 3]) -> Self {
        Self { limits }
    }
}

impl Kinematics for CoreXYKinematics {
    fn cartesian_to_motors(&self, cartesian: &[f64; 3]) -> Result<[f64; 4], Box<dyn std::error::Error>> {
        // CoreXY kinematics:
        // Motor A = X + Y
        // Motor B = X - Y
        // Motor C = Z
        let motor_a = cartesian[0] + cartesian[1];
        let motor_b = cartesian[0] - cartesian[1];
        let motor_c = cartesian[2];
        
        Ok([motor_a, motor_b, motor_c, 0.0])
    }
    
    fn motors_to_cartesian(&self, motors: &[f64; 4]) -> Result<[f64; 3], Box<dyn std::error::Error>> {
        // Reverse CoreXY kinematics:
        // X = (A + B) / 2
        // Y = (A - B) / 2
        // Z = C
        let x = (motors[0] + motors[1]) / 2.0;
        let y = (motors[0] - motors[1]) / 2.0;
        let z = motors[2];
        
        Ok([x, y, z])
    }
    
    fn is_valid_position(&self, cartesian: &[f64; 3]) -> bool {
        for i in 0..3 {
            if cartesian[i] < self.limits[i][0] || cartesian[i] > self.limits[i][1] {
                return false;
            }
        }
        true
    }
}

/// Factory for creating kinematics handlers
pub fn create_kinematics(
    kinematics_type: KinematicsType,
    limits: [[f64; 2]; 3],
) -> Box<dyn Kinematics> {
    match kinematics_type {
        KinematicsType::Cartesian => Box::new(CartesianKinematics::new(limits)),
        KinematicsType::CoreXY => Box::new(CoreXYKinematics::new(limits)),
        // Add other kinematics types as needed
        _ => Box::new(CartesianKinematics::new(limits)), // fallback
    }
}