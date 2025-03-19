// src/reader/utils.rs
use std::error::Error;
use super::commands::*;
use super::mfrc522::MifareClassic;

impl MifareClassic {
    /// Read register - FIXED to match working code
    pub(crate) fn read_register(&mut self, addr: u8) -> Result<u8, Box<dyn Error>> {
        let tx_buf = [((addr << 1) & 0x7E) | 0x80, 0x00];
        let mut rx_buf = [0u8; 2];
        
        self.spi.transfer(&mut rx_buf, &tx_buf)?;
        
        Ok(rx_buf[1])
    }
    
    /// Write register - FIXED to match working code
    pub(crate) fn write_register(&mut self, addr: u8, val: u8) -> Result<(), Box<dyn Error>> {
        let tx_buf = [(addr << 1) & 0x7E, val];
        let mut rx_buf = [0u8; 2];
        
        self.spi.transfer(&mut rx_buf, &tx_buf)?;
        
        Ok(())
    }
    
    /// Set bit mask
    pub(crate) fn set_bit_mask(&mut self, addr: u8, mask: u8) -> Result<(), Box<dyn Error>> {
        let tmp = self.read_register(addr)?;
        self.write_register(addr, tmp | mask)?;
        Ok(())
    }
    
    /// Clear bit mask
    pub(crate) fn clear_bit_mask(&mut self, addr: u8, mask: u8) -> Result<(), Box<dyn Error>> {
        let tmp = self.read_register(addr)?;
        self.write_register(addr, tmp & !mask)?;
        Ok(())
    }
    
    /// Turn antenna on - FIXED to match working code
    pub(crate) fn antenna_on(&mut self) -> Result<(), Box<dyn Error>> {
        let temp = self.read_register(TX_CONTROL_REG)?;
        if (temp & 0x03) != 0x03 {
            self.set_bit_mask(TX_CONTROL_REG, 0x03)?;
        }
        
        // Debug info
        let state = self.read_register(TX_CONTROL_REG)?;
        println!("Antenna state: 0x{:02X}", state);
        
        Ok(())
    }
    
    /// Turn antenna off
    pub(crate) fn antenna_off(&mut self) -> Result<(), Box<dyn Error>> {
        self.clear_bit_mask(TX_CONTROL_REG, 0x03)?;
        Ok(())
    }
    
    /// Format a UID as a string
    pub fn format_uid(&self, uid: &[u8]) -> String {
        uid.iter()
            .map(|byte| format!("{:02X}", byte))
            .collect::<Vec<String>>()
            .join(":")
    }
    
    /// Convert bytes to hex string
    pub fn bytes_to_hex(&self, bytes: &[u8]) -> String {
        bytes.iter()
            .map(|byte| format!("{:02X}", byte))
            .collect::<Vec<String>>()
            .join(" ")
    }
    
    /// Convert bytes to ASCII representation (printable only)
    pub fn bytes_to_ascii(&self, bytes: &[u8]) -> String {
        bytes.iter()
            .map(|&byte| {
                if byte >= 32 && byte <= 126 {
                    byte as char
                } else {
                    '.'
                }
            })
            .collect()
    }
}
