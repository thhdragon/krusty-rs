//! Serial protocol implementation (CRC, sequence, retransmit, windowing)
use serial2_tokio::SerialPort;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;

pub struct SerialProtocolStub {
    pub port: Arc<SerialPort>,
    pub sequence: AtomicU32,
    pub window_size: usize, // Reserved for future windowing logic
}

impl SerialProtocolStub {
    /// Create a new SerialProtocolStub with a given port and window size
    pub fn new(port: Arc<SerialPort>, window_size: usize) -> Self {
        Self {
            port,
            sequence: AtomicU32::new(0),
            window_size,
        }
    }

    /// CRC-16-CCITT calculation (XMODEM variant)
    pub fn crc16(data: &[u8]) -> u16 {
        let mut crc = 0u16;
        for &b in data {
            crc ^= (b as u16) << 8;
            for _ in 0..8 {
                if (crc & 0x8000) != 0 {
                    crc = (crc << 1) ^ 0x1021;
                } else {
                    crc <<= 1;
                }
            }
        }
        crc
    }

    /// Send a command with CRC and sequence number
    pub async fn send_command(&self, cmd: &str) -> Result<(), String> {
        use tokio::time::{timeout, Duration};
        let seq = self.sequence.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut packet = Vec::new();
        packet.extend_from_slice(&seq.to_le_bytes());
        packet.extend_from_slice(cmd.as_bytes());
        let crc = Self::crc16(&packet);
        packet.extend_from_slice(&crc.to_le_bytes());

        // Write packet to serial port
        match self.port.write(&packet).await {
            Ok(n) if n == packet.len() => {
                // Wait for response with timeout
                match timeout(Duration::from_millis(500), self.receive_response()).await {
                    Ok(Ok(resp)) => {
                        tracing::info!("Serial response: {}", resp);
                        Ok(())
                    }
                    Ok(Err(e)) => {
                        tracing::error!("Serial response error: {}", e);
                        Err(e)
                    }
                    Err(_) => {
                        tracing::warn!("Serial response timeout, retransmitting");
                        // Simple retransmit: try once more
                        match self.port.write(&packet).await {
                            Ok(_) => Ok(()),
                            Err(e) => Err(format!("Retransmit failed: {}", e)),
                        }
                    }
                }
            }
            Ok(n) => Err(format!("Partial write: {} bytes", n)),
            Err(e) => Err(format!("Serial write error: {}", e)),
        }
    }

    /// Receive and validate a response
    pub async fn receive_response(&self) -> Result<String, String> {
        let mut buf = [0u8; 256];
        match self.port.read(&mut buf).await {
            Ok(n) if n >= 6 => {
                let _seq = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]); // Sequence reserved for future use
                let resp_data = &buf[4..n-2];
                let crc_recv = u16::from_le_bytes([buf[n-2], buf[n-1]]);
                let mut check_packet = Vec::new();
                check_packet.extend_from_slice(&buf[0..n-2]);
                let crc_calc = Self::crc16(&check_packet);
                if crc_calc != crc_recv {
                    return Err("CRC mismatch".to_string());
                }
                Ok(String::from_utf8_lossy(resp_data).to_string())
            }
            Ok(n) => Err(format!("Short response: {} bytes", n)),
            Err(e) => Err(format!("Serial read error: {}", e)),
        }
    }
}
