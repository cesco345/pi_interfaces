// src/cards/card_types.rs
use std::fmt;

/// Card type identification
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CardType {
    MifareClassic1K,
    MifareClassic4K,
    MifareUltralight,
    MifarePlus,
    MifareDesfire,
    MagicCard,
    Unknown,
}

impl fmt::Display for CardType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CardType::MifareClassic1K => write!(f, "MIFARE Classic 1K"),
            CardType::MifareClassic4K => write!(f, "MIFARE Classic 4K"),
            CardType::MifareUltralight => write!(f, "MIFARE Ultralight"),
            CardType::MifarePlus => write!(f, "MIFARE Plus"),
            CardType::MifareDesfire => write!(f, "MIFARE DESFire"),
            CardType::MagicCard => write!(f, "Magic Card (Configurable)"),
            CardType::Unknown => write!(f, "Unknown MIFARE card type"),
        }
    }
}

/// Key type enum
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum KeyType {
    KeyA,
    KeyB,
}

impl fmt::Display for KeyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyType::KeyA => write!(f, "Key A"),
            KeyType::KeyB => write!(f, "Key B"),
        }
    }
}

/// Magic card operations trait
pub trait MagicCardOperations {
    /// Enter special backdoor mode
    fn enter_backdoor_mode(&mut self) -> Result<bool, String>;
    
    /// Write direct block
    fn direct_write_block(&mut self, block: u8, data: &[u8]) -> Result<bool, String>;
    
    /// Reset card
    fn reset_card(&mut self) -> Result<(), String>;
    
    /// Check if card is in factory default state
    fn is_factory_default(&mut self) -> Result<bool, String>;
}
