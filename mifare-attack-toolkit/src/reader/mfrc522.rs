// src/reader/mfrc522.rs
use std::error::Error;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};

use crate::cards::KeyType;
use super::commands::*;

/// The main struct for Mifare card operations
pub struct MifareClassic {
    pub(crate) spi: Spi,
    pub(crate) last_known_keys: HashMap<(u8, KeyType), [u8; 6]>, // Stores known keys by (sector, key_type)
    pub(crate) dark_processing_mode: bool, // Special mode for difficult cards
}

impl MifareClassic {
    /// Create a new Mifare card handler - using proven settings from working code
    pub fn new() -> Result<Self, Box<dyn Error>> {
        // FIXED: Using standard SPI speed from working code (1MHz instead of 100KHz)
        let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 1_000_000, Mode::Mode0)?;
        
        let mut instance = Self { 
            spi,
            last_known_keys: HashMap::new(),
            dark_processing_mode: false, // FIXED: Start with disabled dark mode
        };
        instance.init()?;
        
        Ok(instance)
    }
    
    /// Initialize the MFRC522 reader - SIMPLIFIED from working code
    fn init(&mut self) -> Result<(), Box<dyn Error>> {
        // FIXED: Single soft reset just like working code
        self.write_register(COMMAND_REG, PCD_RESETPHASE)?;
        thread::sleep(Duration::from_millis(50));
        
        // Check version
        let version = self.read_register(VERSION_REG)?;
        println!("MFRC522 Version: 0x{:02X}", version);
        
        // FIXED: Timer configurations exactly matching working code
        self.write_register(T_MODE_REG, 0x8D)?;
        self.write_register(T_PRESCALER_REG, 0x3E)?;
        self.write_register(T_RELOAD_REG_L, 30)?;
        self.write_register(T_RELOAD_REG_H, 0)?;
        
        // FIXED: Auto configurations exactly matching working code
        self.write_register(TX_AUTO_REG, 0x40)?;
        self.write_register(MODE_REG, 0x3D)?;
        
        // FIXED: Turn on the antenna with same approach as working code
        self.antenna_on()?;
        
        println!("MFRC522 initialized successfully");
        
        Ok(())
    }
    
    /// Perform a full reset of the MFRC522 reader - simplified
    pub fn reset_reader(&mut self) -> Result<(), Box<dyn Error>> {
        // Software reset - FIXED: single time like working code
        self.write_register(COMMAND_REG, PCD_RESETPHASE)?;
        thread::sleep(Duration::from_millis(50));
        
        // Re-initialize
        self.init()?;
        
        Ok(())
    }
    
    /// Set special processing mode for difficult cards
    pub fn enable_dark_processing_mode(&mut self, enable: bool) {
        self.dark_processing_mode = enable;
        println!("Dark processing mode {}", if enable { "enabled" } else { "disabled" });
    }
    
    /// Perform Darkside attack (simplified)
    pub fn darkside_attack(&mut self, block: u8) -> Result<Option<[u8; 6]>, Box<dyn Error>> {
        // Get card UID
        let _uid = match self.get_uid()? {
            Some(uid) => uid,
            None => return Err("No card detected".into()),
        };
        
        // For demonstration, return the default key for Mifare cards
        // In a real implementation, this would perform the actual attack
        let key = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        
        // Store the key for future use
        let sector = block / 4;
        self.last_known_keys.insert((sector, KeyType::KeyA), key);
        self.last_known_keys.insert((sector, KeyType::KeyB), key);
        
        Ok(Some(key))
    }
    
    /// Perform a nested attack - simplified
    pub fn nested_attack(&mut self, start_sector: u8, known_key: &[u8; 6], key_type: KeyType, target_sector: u8) 
        -> Result<Option<[u8; 6]>, Box<dyn Error>> {
        
        // Get card UID first
        let uid = match self.get_uid()? {
            Some(uid) => uid,
            None => return Err("No card detected".into()),
        };
        
        // Calculate block numbers
        let start_block = start_sector * 4; // First block of sector
        let target_block = target_sector * 4; // First block of target sector
        
        println!("Starting nested attack with known key for block {}", start_block);
        println!("Targeting block: {}", target_block);
        
        // First authenticate with the known key - FIXED: use standard auth
        let auth_success = self.auth_with_key(start_block, key_type, known_key, &uid)?;
        
        if !auth_success {
            println!("Authentication failed with provided key.");
            return Ok(None);
        }
        
        // For demonstration, just return the default key
        let found_key = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        
        // Store for future use - using the correct KeyType variants
        self.last_known_keys.insert((target_sector, KeyType::KeyA), found_key);
        self.last_known_keys.insert((target_sector, KeyType::KeyB), found_key);
        
        Ok(Some(found_key))
    }
}
