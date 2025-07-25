/// Multi-MCU abstraction stub
#[derive(Debug, Clone)]
pub struct MultiMCUManagerStub;

impl MultiMCUManagerStub {
    pub fn new() -> Self { Self }
    /// Register a new MCU (stub)
    pub fn register_mcu(&self, _id: &str) {
        // STUB: Implement multi-MCU support
    }
}
