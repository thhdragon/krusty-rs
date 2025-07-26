use async_trait::async_trait;

#[async_trait]
pub trait SerialInterface: Send + Sync {
    async fn open(&self, port: &str, baud: u32) -> Result<serial2_tokio::SerialPort, Box<dyn std::error::Error + Send + Sync + 'static>>;
    async fn read(&self, port: &serial2_tokio::SerialPort, buf: &mut [u8]) -> Result<usize, Box<dyn std::error::Error + Send + Sync + 'static>>;
    async fn write(&self, port: &serial2_tokio::SerialPort, buf: &[u8]) -> Result<usize, Box<dyn std::error::Error + Send + Sync + 'static>>;
    fn available_ports(&self) -> Vec<String>;
}
