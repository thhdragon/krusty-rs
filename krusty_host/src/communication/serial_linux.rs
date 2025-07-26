use async_trait::async_trait;
use krusty_shared::serial_interface::SerialInterface;

/// Linux implementation of SerialInterface using serial2-tokio
pub struct LinuxSerial;

#[async_trait]
impl SerialInterface for LinuxSerial {
    async fn open(&self, port: &str, baud: u32) -> Result<serial2_tokio::SerialPort, Box<dyn std::error::Error>> {
        let serial = serial2_tokio::SerialPort::open(port, baud)?;
        Ok(serial)
    }

    async fn read(&self, port: &serial2_tokio::SerialPort, buf: &mut [u8]) -> Result<usize, Box<dyn std::error::Error>> {
        let n = port.read(buf).await?;
        Ok(n)
    }

    async fn write(&self, port: &serial2_tokio::SerialPort, buf: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
        let n = port.write(buf).await?;
        Ok(n)
    }

    fn available_ports(&self) -> Vec<String> {
        match serial2_tokio::SerialPort::available_ports() {
            Ok(paths) => paths.iter().map(|p| p.display().to_string()).collect(),
            Err(_) => vec![],
        }
    }
}
