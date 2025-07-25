/// Print time/MCU clock synchronization stub
#[derive(Debug, Clone)]
pub struct ClockSyncStub;

impl ClockSyncStub {
    pub fn new() -> Self { Self }
    /// Sync host and MCU clocks (stub)
    pub async fn sync(&self) -> Result<(), String> {
        // STUB: Implement print time/MCU clock sync
        Ok(())
    }
}
