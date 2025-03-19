// src/reader/card_operations.rs
use std::error::Error;

use crate::cards::{KeyType, CardType};
use super::commands::*;
use super::mfrc522::MifareClassic;

impl MifareClassic {
    /// Get card UID - FIXED to match working code
    pub fn get_uid(&mut self) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        // FIXED: Simple approach from working code
        
        // Request card
        let (status, _) = self.request_card(PICC_REQIDL)?;
        if status != MI_OK {
            return Ok(None);
        }
        
        // Anti-collision
        let (status, uid) = self.anticoll()?;
        if status != MI_OK {
            return Ok(None);
        }
        
        Ok(Some(uid))
    }
    
    /// Request card presence - FIXED to match working code
    pub(crate) fn request_card(&mut self, req_mode: u8) -> Result<(u8, u8), Box<dyn Error>> {
        // Set bit framing for 7 bits
        self.write_register(BIT_FRAMING_REG, 0x07)?;
        
        let tag_type = vec![req_mode];
        let (status, _back_data, back_bits) = self.to_card(PCD_TRANSCEIVE, &tag_type)?;
        
        if (status != MI_OK) || (back_bits != 0x10) {
            return Ok((MI_ERR, 0));
        }
        
        Ok((MI_OK, back_bits as u8))
    }
    
    /// Anti-collision detection - FIXED to match working code
    pub(crate) fn anticoll(&mut self) -> Result<(u8, Vec<u8>), Box<dyn Error>> {
        self.write_register(BIT_FRAMING_REG, 0x00)?;
        
        let ser_num = vec![PICC_ANTICOLL, 0x20];
        let (status, back_data, _) = self.to_card(PCD_TRANSCEIVE, &ser_num)?;
        
        if status == MI_OK {
            // Verify checksum
            if back_data.len() == 5 {
                let mut check_sum: u8 = 0;
                for i in 0..4 {
                    check_sum ^= back_data[i];
                }
                if check_sum != back_data[4] {
                    return Ok((MI_ERR, vec![]));
                }
            } else {
                return Ok((MI_ERR, vec![]));
            }
        }
        
        Ok((status, back_data))
    }
    
    /// Select the card and return its type
    pub fn select_card(&mut self, _uid: &[u8]) -> Result<CardType, Box<dyn Error>> {
        // For now, we'll assume it's a Classic 1K
        Ok(CardType::MifareClassic1K)
    }
    
    /// Read a block from the card - FIXED to match working code
    pub fn read_block(&mut self, block_addr: u8) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        let mut recv_data: Vec<u8> = Vec::new();
        recv_data.push(PICC_READ);
        recv_data.push(block_addr);
        
        let crc = self.calculate_crc(&recv_data)?;
        recv_data.push(crc[0]);
        recv_data.push(crc[1]);
        
        let (status, back_data, _) = self.to_card(PCD_TRANSCEIVE, &recv_data)?;
        
        if status != MI_OK {
            println!("Error while reading!");
            return Ok(None);
        }
        
        if back_data.len() == 16 {
            return Ok(Some(back_data));
        } else {
            return Ok(None);
        }
    }
    
    /// Write a block to the card - FIXED to match working code
    pub fn write_block(&mut self, block_addr: u8, data: &[u8]) -> Result<bool, Box<dyn Error>> {
        let mut buf: Vec<u8> = Vec::new();
        buf.push(PICC_WRITE);
        buf.push(block_addr);
        
        let crc = self.calculate_crc(&buf)?;
        buf.push(crc[0]);
        buf.push(crc[1]);
        
        let (status, back_data, back_len) = self.to_card(PCD_TRANSCEIVE, &buf)?;
        
        if (status != MI_OK) || (back_len != 4) || ((back_data[0] & 0x0F) != 0x0A) {
            println!("Write command failed: status={}, back_len={}", status, back_len);
            return Ok(false);
        }
        
        // If status is OK, and we have received 4 bytes with the correct response (0x0A)
        // then proceed with writing the data
        if status == MI_OK {
            // Prepare the data with CRC
            let mut buf: Vec<u8> = Vec::new();
            
            // Data must be exactly 16 bytes
            for i in 0..16 {
                if i < data.len() {
                    buf.push(data[i]);
                } else {
                    buf.push(0);
                }
            }
            
            let crc = self.calculate_crc(&buf)?;
            buf.push(crc[0]);
            buf.push(crc[1]);
            
            let (status, back_data, back_len) = self.to_card(PCD_TRANSCEIVE, &buf)?;
            
            if (status != MI_OK) || (back_len != 4) || ((back_data[0] & 0x0F) != 0x0A) {
                println!("Error while writing data");
                return Ok(false);
            } else {
                println!("Data written successfully to block {}", block_addr);
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// Try authentication with all default keys - FIXED to use standard approach
    pub fn try_default_keys(&mut self, block: u8) -> Result<Option<([u8; 6], KeyType)>, Box<dyn Error>> {
        // Get card UID first
        let uid = match self.get_uid()? {
            Some(uid) => uid,
            None => return Ok(None),
        };
        
        println!("Card UID: {}", self.format_uid(&uid));
        
        // Try each default key
        for key in DEFAULT_KEYS.iter() {
            // Try Key A
            if self.auth_with_key(block, KeyType::KeyA, key, &uid)? {
                println!("Success with Key A: {}", self.bytes_to_hex(key));
                self.last_known_keys.insert((block / 4, KeyType::KeyA), *key);
                return Ok(Some((*key, KeyType::KeyA)));
            }
            
            // Try Key B
            if self.auth_with_key(block, KeyType::KeyB, key, &uid)? {
                println!("Success with Key B: {}", self.bytes_to_hex(key));
                self.last_known_keys.insert((block / 4, KeyType::KeyB), *key);
                return Ok(Some((*key, KeyType::KeyB)));
            }
        }
        
        println!("Failed with all default keys");
        Ok(None)
    }
    
    /// Special handling for reading a sector - FIXED to use standard approach
    pub fn read_sector_with_special_handling(&mut self, sector: u8, key: &[u8; 6], key_type: KeyType, uid: &[u8]) 
        -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        
        let mut sector_blocks = Vec::new();
        
        for i in 0..4 {
            let block_addr = sector * 4 + i;
            
            // Reset crypto state before each attempt
            self.stop_crypto1()?;
            
            let auth_success = self.auth_with_key(block_addr, key_type, key, uid)?;
            
            if auth_success {
                if let Some(block_data) = self.read_block(block_addr)? {
                    sector_blocks.push(block_data);
                } else {
                    // If read fails, stop and return what we have
                    self.stop_crypto1()?;
                    break;
                }
            } else {
                // Authentication failed for this block
                break;
            }
            
            // Stop crypto after each block
            self.stop_crypto1()?;
        }
        
        Ok(sector_blocks)
    }
}
