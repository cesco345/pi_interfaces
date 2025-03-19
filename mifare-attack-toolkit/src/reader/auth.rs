// src/reader/auth.rs
use std::error::Error;
use std::thread;
use std::time::Duration;

use crate::cards::KeyType;
use super::commands::*;
use super::mfrc522::MifareClassic;

impl MifareClassic {
    /// Authenticate with a key - IMPROVED for better crypto handling
    pub fn auth_with_key(&mut self, block: u8, key_type: KeyType, key: &[u8], serial_num: &[u8]) 
        -> Result<bool, Box<dyn Error>> {
        // Reset crypto state first - this is essential for clone cards
        self.stop_crypto1()?;
        thread::sleep(Duration::from_millis(5));
        
        // Convert key_type to auth_mode
        let auth_mode = match key_type {
            KeyType::KeyA => PICC_AUTHENT1A,
            KeyType::KeyB => PICC_AUTHENT1B,
        };
        
        // Debug output - this is helpful for troubleshooting
        println!("Authenticating with block={}, mode={:02X}, key={}", 
               block, auth_mode, self.bytes_to_hex(key));
        
        // Build auth buffer
        let mut buf: Vec<u8> = Vec::new();
        
        // First byte is authMode (A or B)
        buf.push(auth_mode);
        // Second byte is the block address
        buf.push(block);
        
        // Append the key (exactly 6 bytes)
        for i in 0..6 {
            if i < key.len() {
                buf.push(key[i]);
            } else {
                buf.push(0x00); // Pad if needed
            }
        }
        
        // Append first 4 bytes of UID
        for i in 0..4 {
            if i < serial_num.len() {
                buf.push(serial_num[i]);
            } else {
                if i == 0 {
                    // If we don't even have the first byte, something's wrong
                    return Err("Invalid serial number".into());
                }
                // Don't pad UID - we need exactly what the card provided
                break;
            }
        }
        
        // Debug output
        println!("Auth buffer: {}", self.bytes_to_hex(&buf));
        
        // Clear any pending interrupts and reset command states
        self.write_register(COMMAND_REG, PCD_IDLE)?;
        self.clear_bit_mask(COM_IRQ_REG, 0x80)?;
        self.set_bit_mask(FIFO_LEVEL_REG, 0x80)?; // Flush FIFO
        
        // Send the auth command with explicit handling
        let (status, _, _) = self.to_card(PCD_AUTHENT, &buf)?;
        
        // Small delay to let crypto bit settle - important for some clone cards
        thread::sleep(Duration::from_millis(5));
        
        // Check the crypto bit in STATUS2 register
        let status2 = self.read_register(STATUS2_REG)?;
        let crypto_bit = status2 & 0x08;
        
        println!("Auth status: {}, STATUS2: {:02X}, Crypto bit: {}", 
               status, status2, if crypto_bit != 0 { "SET" } else { "NOT SET" });
        
        // Additional check for error register
        let error_reg = self.read_register(ERROR_REG)?;
        if error_reg != 0 {
            println!("Error register: 0x{:02X}", error_reg);
        }
        
        if crypto_bit == 0 {
            // Authentication failed - crypto bit not set
            return Ok(false);
        }
        
        let success = status == MI_OK;
        
        if success {
            // Store successful key
            let sector = block / 4;
            self.last_known_keys.insert((sector, key_type), [key[0], key[1], key[2], key[3], key[4], key[5]]);
            println!("Authentication succeeded!");
        } else {
            // Stop crypto on failure
            self.stop_crypto1()?;
            println!("Authentication failed: status not OK");
        }
        
        Ok(success)
    }
    
    /// Auth with key - special handling for clone cards that behave differently
    pub fn auth_with_key_special(&mut self, block: u8, key_type: KeyType, key: &[u8], serial_num: &[u8]) 
        -> Result<bool, Box<dyn Error>> {
        // Special authentication for clone cards needs a full reset sequence
        
        // First completely reset the reader
        self.write_register(COMMAND_REG, PCD_RESETPHASE)?;
        thread::sleep(Duration::from_millis(50));
        
        // Reset the MFRC522 antenna - this helps with power issues in clone cards
        self.antenna_off()?;
        thread::sleep(Duration::from_millis(50));
        self.antenna_on()?;
        thread::sleep(Duration::from_millis(50));
        
        // Initialize registers for authentication
        self.write_register(COMMAND_REG, PCD_IDLE)?;
        self.clear_bit_mask(STATUS2_REG, 0x08)?; // Clear crypto1 bit
        self.clear_bit_mask(COM_IRQ_REG, 0x80)?; // Clear interrupts
        
        // Convert key_type to auth_mode
        let auth_mode = match key_type {
            KeyType::KeyA => PICC_AUTHENT1A,
            KeyType::KeyB => PICC_AUTHENT1B,
        };
        
        // Build auth buffer
        let mut buf: Vec<u8> = Vec::new();
        buf.push(auth_mode);
        buf.push(block);
        
        // Add key - making sure we have exactly 6 bytes
        if key.len() == 6 {
            buf.extend_from_slice(key);
        } else {
            // Pad or truncate to 6 bytes
            for i in 0..6 {
                if i < key.len() {
                    buf.push(key[i]);
                } else {
                    buf.push(0x00);
                }
            }
        }
        
        // Add UID - special handling for different UID lengths
        if serial_num.len() >= 4 {
            // Use first 4 bytes
            buf.extend_from_slice(&serial_num[0..4]);
        } else {
            // Use what we have and no padding
            buf.extend_from_slice(serial_num);
        }
        
        println!("Special auth with: {}", self.bytes_to_hex(&buf));
        
        // Enhanced handling for MFRC522
        let irq_en: u8 = 0x12;
        let wait_irq: u8 = 0x10;
        
        // Enable interrupts
        self.write_register(COM_IEN_REG, irq_en | 0x80)?;
        // Clear interrupt request bits
        self.clear_bit_mask(COM_IRQ_REG, 0x80)?;
        // FlushBuffer=1, FIFO initialization
        self.set_bit_mask(FIFO_LEVEL_REG, 0x80)?;
        
        // Write data to FIFO
        for &byte in &buf {
            self.write_register(FIFO_DATA_REG, byte)?;
        }
        
        // Execute auth command
        self.write_register(COMMAND_REG, PCD_AUTHENT)?;
        
        // Wait for the command to complete
        let mut i = 3000; // Enhanced timeout for clone cards
        let mut n: u8;
        
        loop {
            n = self.read_register(COM_IRQ_REG)?;
            i -= 1;
            
            // IdleIRq bit set or timeout
            if (i == 0) || ((n & 0x01) != 0) || ((n & wait_irq) != 0) {
                break;
            }
            
            thread::sleep(Duration::from_micros(100));
        }
        
        // Check crypto state with better debug
        let status2 = self.read_register(STATUS2_REG)?;
        println!("STATUS2 after auth: 0x{:02X}", status2);
        
        let success = (status2 & 0x08) != 0;
        
        if success {
            // Store successful key
            let sector = block / 4;
            self.last_known_keys.insert((sector, key_type), [key[0], key[1], key[2], key[3], key[4], key[5]]);
            println!("Special authentication succeeded!");
            
            // Don't stop crypto here - we need it for the next operation
        } else {
            println!("Special authentication failed");
            // Stop crypto when failed
            self.clear_bit_mask(STATUS2_REG, 0x08)?;
        }
        
        Ok(success)
    }
    
    /// Stop crypto1 operations
    pub(crate) fn stop_crypto1(&mut self) -> Result<(), Box<dyn Error>> {
        self.clear_bit_mask(STATUS2_REG, 0x08)?;
        Ok(())
    }
}
