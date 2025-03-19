use std::error::Error;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MagicCardType {
    /// Gen1A - Original Chinese Magic Cards
    Gen1A,
    /// Gen2 - Second generation with extended features
    Gen2,
    /// UID changeable cards
    Cuid,
    /// Unknown type
    Unknown,
}

/// Magic card detection and UID writing methods
pub trait MagicCardOperations {
    /// Check if a card is a Magic Card
    fn detect_magic_card(&mut self) -> Result<bool, Box<dyn Error>>;
    
    /// Write custom UID to a Magic Card
    fn write_custom_uid(&mut self, new_uid: &[u8]) -> Result<bool, Box<dyn Error>>;
    
    /// Clone a card to a Magic Card
    fn clone_card(&mut self, source_uid: &[u8], target_uid: &[u8]) -> Result<bool, Box<dyn Error>>;
}
