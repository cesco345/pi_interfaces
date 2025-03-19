// src/cards/mod.rs
mod card_types;
mod keys;
mod magic_cards;

// Re-export types and functions
pub use card_types::{CardType, KeyType, MagicCardOperations};
pub use keys::DEFAULT_KEYS;
pub use magic_cards::MagicCardType;

/// Identify card type based on UID and ATQA bytes
pub fn identify_card_type(uid: &[u8], atqa: Option<[u8; 2]>) -> CardType {
    // Check UID length first
    match uid.len() {
        4 => {
            // Standard 4-byte UID - Most likely Mifare Classic
            if let Some(atqa_bytes) = atqa {
                match atqa_bytes {
                    [0x00, 0x04] => CardType::MifareClassic1K,
                    [0x00, 0x02] => CardType::MifareClassic4K,
                    [0x00, 0x44] => CardType::MifareClassic1K,
                    [0x00, 0x84] => CardType::MifarePlus,
                    [0x03, 0x44] => CardType::MifareUltralight,
                    [0x03, 0x04] => CardType::MifareUltralight,
                    [0x03, 0x84] => CardType::MifareDesfire,
                    _ => {
                        // Default to Classic 1K for 4-byte UID
                        CardType::MifareClassic1K
                    }
                }
            } else {
                // Without ATQA, default to Classic 1K for 4-byte UID
                CardType::MifareClassic1K
            }
        },
        7 => {
            // 7-byte UID - Often Mifare Ultralight or DESFire
            CardType::MifareUltralight
        },
        10 => {
            // 10-byte UID - Typically high security cards like DESFire
            CardType::MifareDesfire
        },
        _ => {
            // For any other length
            CardType::Unknown
        }
    }
}

/// Check if a card might be a Magic Card based on UID pattern
pub fn is_magic_card(uid: &[u8]) -> bool {
    // Check for common Magic Card UID patterns
    // Many Magic Cards start with specific byte sequences
    if uid.len() == 4 {
        // Check for Chinese Magic Card patterns
        if uid[0] == 0xCF || uid[0] == 0x88 {
            return true;
        }
        
        // Check for Gen1A/Gen2 patterns
        if uid[0] == 0x04 && uid[1] == 0x77 {
            return true;
        }
    }
    
    false
}
