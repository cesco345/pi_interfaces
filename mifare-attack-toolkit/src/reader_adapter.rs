use std::error::Error;
use crate::reader::MifareClassic;
use crate::crypto1::MifareReader;

/// Adapter to wrap the MifareClassic reader for use with trait-based functions
pub struct ReaderAdapter<'a> {
    reader: &'a mut MifareClassic,
    current_uid: Option<Vec<u8>>,  // Keep track of the current card UID
}

impl<'a> ReaderAdapter<'a> {
    /// Create a new reader adapter
    pub fn new(reader: &'a mut MifareClassic) -> Self {
        Self { 
            reader,
            current_uid: None,
        }
    }
    
    /// Get the inner reader reference
    pub fn get_reader(&mut self) -> &mut MifareClassic {
        self.reader
    }
    
    /// Detect card and store UID
    fn detect_card(&mut self) -> Result<Option<Vec<u8>>, String> {
        match self.reader.get_uid() {
            Ok(Some(uid)) => {
                self.current_uid = Some(uid.clone());
                Ok(Some(uid))
            },
            Ok(None) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
}

impl<'a> MifareReader for ReaderAdapter<'a> {
    /// Read a block from the card
    fn read_block(&mut self, block: u8) -> Result<Option<Vec<u8>>, String> {
        // Ensure we have a card UID
        if self.current_uid.is_none() {
            if let Some(uid) = self.detect_card()? {
                self.current_uid = Some(uid);
            } else {
                return Err("No card detected".to_string());
            }
        }
        
        // Read block using the underlying reader
        match self.reader.read_block(block) {
            Ok(data) => Ok(data),
            Err(e) => Err(e.to_string()),
        }
    }
    
    /// Write a block to the card
    fn write_block(&mut self, block: u8, data: &[u8]) -> Result<bool, String> {
        if data.len() != 16 {
            return Err("Data must be exactly 16 bytes".to_string());
        }
        
        // Ensure we have a card UID
        if self.current_uid.is_none() {
            if let Some(uid) = self.detect_card()? {
                self.current_uid = Some(uid);
            } else {
                return Err("No card detected".to_string());
            }
        }
        
        // Write block using the underlying reader
        match self.reader.write_block(block, data) {
            Ok(success) => Ok(success),
            Err(e) => Err(e.to_string()),
        }
    }
    
    /// Send raw command to the card and get response
    fn transceive(&mut self, command: &[u8]) -> Result<Vec<u8>, String> {
        // This is a simplified implementation since your MifareClassic doesn't have
        // a direct transceive method. We'll handle the most common commands.
        
        if command.is_empty() {
            return Err("Empty command".to_string());
        }
        
        // Ensure we have a card UID
        if self.current_uid.is_none() {
            if let Some(uid) = self.detect_card()? {
                self.current_uid = Some(uid);
            } else {
                return Err("No card detected".to_string());
            }
        }
        
        let uid = self.current_uid.as_ref().unwrap();
        
        // Handle different command types based on first byte
        match command[0] {
            0x60 | 0x61 => { // AUTHENT1A or AUTHENT1B
                if command.len() < 12 {
                    return Err("Invalid authentication command".to_string());
                }
                
                let key_type = if command[0] == 0x60 { 
                    crate::cards::KeyType::KeyA 
                } else { 
                    crate::cards::KeyType::KeyB 
                };
                
                let block = command[1];
                let mut key = [0u8; 6];
                key.copy_from_slice(&command[2..8]);
                
                match self.reader.auth_with_key(block, key_type, &key, uid) {
                    Ok(true) => Ok(vec![0x90, 0x00]), // Success response
                    Ok(false) => Ok(vec![0x63, 0x00]), // Authentication failed
                    Err(e) => Err(e.to_string()),
                }
            },
            0x30 => { // READ
                if command.len() < 2 {
                    return Err("Invalid read command".to_string());
                }
                
                let block = command[1];
                
                match self.reader.read_block(block) {
                    Ok(Some(data)) => Ok(data),
                    Ok(None) => Err("Read failed".to_string()),
                    Err(e) => Err(e.to_string()),
                }
            },
            0xA0 => { // WRITE
                if command.len() < 18 {
                    return Err("Invalid write command".to_string());
                }
                
                let block = command[1];
                let data = &command[2..18];
                
                match self.reader.write_block(block, data) {
                    Ok(true) => Ok(vec![0x90, 0x00]), // Success response
                    Ok(false) => Err("Write failed".to_string()),
                    Err(e) => Err(e.to_string()),
                }
            },
            _ => Err(format!("Unsupported command: 0x{:02X}", command[0])),
        }
    }
}
