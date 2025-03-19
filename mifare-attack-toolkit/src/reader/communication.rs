// src/reader/communication.rs
use std::error::Error;
use std::thread;
use std::time::Duration;

use super::commands::*;
use super::mfrc522::MifareClassic;

impl MifareClassic {
    /// Communicate with the card - FIXED version matching working code
    pub(crate) fn to_card(&mut self, command: u8, data: &[u8]) -> Result<(u8, Vec<u8>, usize), Box<dyn Error>> {
        let mut back_data: Vec<u8> = Vec::new();
        let mut back_len: usize = 0;
        let mut status = MI_ERR;
        let mut irq_en: u8 = 0x00;
        let mut wait_irq: u8 = 0x00;
        
        // Determine interrupts based on command
        if command == PCD_AUTHENT {
            irq_en = 0x12;
            wait_irq = 0x10;
        } else if command == PCD_TRANSCEIVE {
            irq_en = 0x77;
            wait_irq = 0x30;
        }
        
        // FIXED: Removed reset command and delay that wasn't in working code
        
        // Enable interrupts
        self.write_register(COM_IEN_REG, irq_en | 0x80)?;
        // Clear interrupt request bits
        self.clear_bit_mask(COM_IRQ_REG, 0x80)?;
        // FlushBuffer=1, FIFO initialization
        self.set_bit_mask(FIFO_LEVEL_REG, 0x80)?;
        // FIXED: Removed delay that wasn't in working code
        
        // No action, cancel current commands
        self.write_register(COMMAND_REG, PCD_IDLE)?;
        
        // Write data to FIFO
        for &byte in data {
            self.write_register(FIFO_DATA_REG, byte)?;
        }
        
        // Execute command
        self.write_register(COMMAND_REG, command)?;
        
        // StartSend=1, transmission of data starts
        if command == PCD_TRANSCEIVE {
            self.set_bit_mask(BIT_FRAMING_REG, 0x80)?;
        }
        
        // Wait for the command to complete - FIXED to match working code
        let mut i = 2000; // Standard timeout from working code
        let mut n: u8;
        
        loop {
            n = self.read_register(COM_IRQ_REG)?;
            i -= 1;
            
            // RxIRq or IdleIRq or Timer is set, or timeout
            if (i == 0) || ((n & 0x01) != 0) || ((n & wait_irq) != 0) {
                break;
            }
            
            thread::sleep(Duration::from_micros(100));
        }
        
        // Clear StartSend bit
        self.clear_bit_mask(BIT_FRAMING_REG, 0x80)?;
        
        // Check for errors and retrieve data
        if i != 0 {
            // FIXED: Simplified error check to match working code
            if (self.read_register(ERROR_REG)? & 0x1B) == 0x00 {
                status = MI_OK;
                
                // Check if CardIRq bit is set (timeout)
                if (n & irq_en & 0x01) != 0 {
                    status = MI_NOTAGERR;
                }
                
                // Read data from FIFO if it's a transceive command
                if command == PCD_TRANSCEIVE {
                    // Number of bytes in FIFO
                    let mut fifo_len = self.read_register(FIFO_LEVEL_REG)? as usize;
                    // Last bits = Number of valid bits in the last received byte
                    let last_bits = (self.read_register(CONTROL_REG)? & 0x07) as usize;
                    
                    if last_bits != 0 {
                        back_len = (fifo_len - 1) * 8 + last_bits;
                    } else {
                        back_len = fifo_len * 8;
                    }
                    
                    // No data in FIFO
                    if fifo_len == 0 {
                        fifo_len = 1;
                    }
                    
                    // Cap maximum read to MAX_LEN (16)
                    let read_len = if fifo_len > MAX_LEN { MAX_LEN } else { fifo_len };
                    
                    // Read the data from FIFO
                    for _ in 0..read_len {
                        back_data.push(self.read_register(FIFO_DATA_REG)?);
                    }
                }
            } else {
                // Communication error
                status = MI_ERR;
            }
        }
        
        Ok((status, back_data, back_len))
    }
    
    /// Calculate CRC - FIXED to match working code
    pub(crate) fn calculate_crc(&mut self, data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        self.clear_bit_mask(DIV_IRQ_REG, 0x04)?;
        self.set_bit_mask(FIFO_LEVEL_REG, 0x80)?;
        
        // Write data to FIFO
        for &byte in data {
            self.write_register(FIFO_DATA_REG, byte)?;
        }
        
        self.write_register(COMMAND_REG, PCD_CALCCRC)?;
        
        // Wait for CRC calculation to complete
        let mut i = 0xFF;
        let mut n: u8;
        
        loop {
            n = self.read_register(DIV_IRQ_REG)?;
            i -= 1;
            
            if (i == 0) || ((n & 0x04) != 0) {
                break;
            }
        }
        
        // Read CRC result
        let mut result = Vec::new();
        result.push(self.read_register(CRC_RESULT_REG_L)?);
        result.push(self.read_register(CRC_RESULT_REG_M)?);
        
        Ok(result)
    }
}
