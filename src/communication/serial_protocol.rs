/// Serial protocol stub (for parity with Klipper)
#[derive(Debug, Clone)]
pub struct SerialProtocolStub;

impl SerialProtocolStub {
    pub fn new<T>(_port: T, _window_size: usize) -> Self {
        // STUB: Implement serial protocol logic
        Self
    }
}
