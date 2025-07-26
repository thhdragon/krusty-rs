// src/shaper.rs
// Shared input shaper logic for vibration reduction
// Migrated from krusty_host/src/motion/shaper.rs

/// Trait for modular input shapers
pub trait InputShaperTrait {
    /// Process the next input position and return the shaped output
    fn do_step(&mut self, input: f64) -> f64;
}

#[derive(Debug, Clone)]
/// Zero Vibration Derivative (ZVD) shaper implementation
pub struct ZVDShaper {
    pub buffer: Vec<f64>,
    pub coeffs: [f64; 2],
    pub delay: usize,
}

impl ZVDShaper {
    pub fn new(delay: usize, coeffs: [f64; 2]) -> Self {
        Self {
            buffer: vec![0.0; delay + 1],
            coeffs,
            delay,
        }
    }
}

impl InputShaperTrait for ZVDShaper {
    fn do_step(&mut self, input: f64) -> f64 {
        // Shift buffer and insert new input
        self.buffer.rotate_left(1);
        self.buffer[self.delay] = input;
        // Weighted sum of buffer for ZVD
        self.coeffs[0] * self.buffer[0] + self.coeffs[1] * self.buffer[self.delay]
    }
}

#[derive(Debug, Clone)]
/// Sine Wave Demo shaper implementation
pub struct SineWaveShaper {
    pub magnitude: f64,
    pub frequency: f64,
    pub phase: f64,
    pub sample_time: f64,
}

impl SineWaveShaper {
    pub fn new(magnitude: f64, frequency: f64, sample_time: f64) -> Self {
        Self {
            magnitude,
            frequency,
            phase: 0.0,
            sample_time,
        }
    }
}

impl InputShaperTrait for SineWaveShaper {
    fn do_step(&mut self, input: f64) -> f64 {
        let shaped = input + self.magnitude * (self.phase).sin();
        self.phase += 2.0 * std::f64::consts::PI * self.frequency * self.sample_time;
        shaped
    }
}

#[derive(Debug, Clone)]
/// Enum for supported input shaper types (extensible)
pub enum InputShaperType {
    ZVD(ZVDShaper),
    SineWave(SineWaveShaper),
    // Add more shaper types here
}

impl InputShaperTrait for InputShaperType {
    fn do_step(&mut self, input: f64) -> f64 {
        match self {
            InputShaperType::ZVD(shaper) => shaper.do_step(input),
            InputShaperType::SineWave(shaper) => shaper.do_step(input),
            // Add more shaper types here
        }
    }
}

#[derive(Debug, Clone)]
/// Per-axis input shaper assignment (e.g., for X, Y, Z, E)
pub struct PerAxisInputShapers {
    pub shapers: Vec<Option<InputShaperType>>, // None = no shaping for that axis
}

impl PerAxisInputShapers {
    pub fn new(num_axes: usize) -> Self {
        Self {
            shapers: vec![None; num_axes],
        }
    }
    pub fn set_shaper(&mut self, axis: usize, shaper: InputShaperType) {
        if axis < self.shapers.len() {
            self.shapers[axis] = Some(shaper);
        }
    }
    pub fn do_step(&mut self, axis: usize, input: f64) -> f64 {
        if let Some(shaper) = &mut self.shapers[axis] {
            shaper.do_step(input)
        } else {
            input
        }
    }
}
