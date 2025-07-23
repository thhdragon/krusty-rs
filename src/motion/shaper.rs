// src/motion/shaper.rs
/// Input shapers for reducing vibrations and ringing
/// 
/// These filters reduce the oscillations that occur when the printer
/// changes direction rapidly, improving print quality
#[derive(Debug, Clone)]
pub enum InputShaper {
    None,
    ZVD,      // Zero Vibration Derivative
    ZVDD,     // Zero Vibration Derivative and Double Derivative
    EI2,      // Extra Immunity 2
    Custom { amplitudes: Vec<f64>, durations: Vec<f64> },
}

/// Input shaper configuration
pub struct ShaperConfig {
    pub shaper_type: InputShaper,
    pub frequency: f64,  // Hz
    pub damping: f64,    // 0.0 to 1.0
}

impl ShaperConfig {
    pub fn new(shaper_type: InputShaper, frequency: f64, damping: f64) -> Self {
        Self {
            shaper_type,
            frequency,
            damping,
        }
    }

    /// Apply input shaping to a step sequence
    /// 
    /// This method takes a simple step command and returns a sequence
    /// of shaped steps that reduce vibration
    pub fn apply_shaping(&self, steps: Vec<(f64, bool)>) -> Vec<(f64, bool)> {
        match &self.shaper_type {
            InputShaper::None => steps,
            InputShaper::ZVD => self.apply_zvd_shaping(steps),
            InputShaper::ZVDD => self.apply_zvdd_shaping(steps),
            InputShaper::EI2 => self.apply_ei2_shaping(steps),
            InputShaper::Custom { amplitudes, durations } => {
                self.apply_custom_shaping(steps, amplitudes, durations)
            }
        }
    }

    fn apply_zvd_shaping(&self, steps: Vec<(f64, bool)>) -> Vec<(f64, bool)> {
        // ZVD (Zero Vibration Derivative) shaper
        // Two impulses: 0.25 at t=0, 0.75 at t=T
        // where T = π/(2*frequency*sqrt(1-damping²))
        
        let period = std::f64::consts::PI / 
            (2.0 * self.frequency * (1.0 - self.damping * self.damping).sqrt());
        
        let mut shaped_steps = Vec::new();
        
        for (time, direction) in steps {
            // First impulse (25%)
            shaped_steps.push((time, direction));
            
            // Second impulse (75%) delayed by period
            shaped_steps.push((time + period, direction));
        }
        
        shaped_steps
    }

    fn apply_zvdd_shaping(&self, steps: Vec<(f64, bool)>) -> Vec<(f64, bool)> {
        // ZVDD (Zero Vibration Derivative and Double Derivative) shaper
        // Three impulses with specific amplitudes and timing
        
        let period = std::f64::consts::PI / 
            (2.0 * self.frequency * (1.0 - self.damping * self.damping).sqrt());
        
        let mut shaped_steps = Vec::new();
        
        for (time, direction) in steps {
            // Three impulses with ZVDD coefficients
            shaped_steps.push((time, direction));                    // 12.5%
            shaped_steps.push((time + period, direction));          // 75%
            shaped_steps.push((time + 2.0 * period, direction));    // 12.5%
        }
        
        shaped_steps
    }

    fn apply_ei2_shaping(&self, steps: Vec<(f64, bool)>) -> Vec<(f64, bool)> {
        // EI2 (Extra Immunity 2) shaper - more robust to frequency variations
        // Four impulses with specific coefficients
        
        let period = std::f64::consts::PI / 
            (2.0 * self.frequency * (1.0 - self.damping * self.damping).sqrt());
        
        let mut shaped_steps = Vec::new();
        
        for (time, direction) in steps {
            // Four impulses with EI2 coefficients
            shaped_steps.push((time, direction));                    // 6.25%
            shaped_steps.push((time + period, direction));          // 37.5%
            shaped_steps.push((time + 2.0 * period, direction));    // 37.5%
            shaped_steps.push((time + 3.0 * period, direction));    // 6.25%
        }
        
        shaped_steps
    }

    fn apply_custom_shaping(
        &self,
        steps: Vec<(f64, bool)>,
        amplitudes: &[f64],
        durations: &[f64],
    ) -> Vec<(f64, bool)> {
        let mut shaped_steps = Vec::new();
        
        for (time, direction) in steps {
            let mut cumulative_time = 0.0;
            for (i, &amplitude) in amplitudes.iter().enumerate() {
                if amplitude > 0.01 { // Ignore very small amplitudes
                    shaped_steps.push((time + cumulative_time, direction));
                }
                if i < durations.len() {
                    cumulative_time += durations[i];
                }
            }
        }
        
        shaped_steps
    }
}