// Functions for detecting card types

/// Detect card type based on UID and other factors
pub fn detect_card_type(uid: &[u8]) -> String {
    if uid.is_empty() {
        return "Unknown".to_string();
    }
    
    // Check UID length
    match uid.len() {
        4 => {
            // 4-byte UID: Could be MIFARE Classic or Ultralight or NTAG
            // First byte can sometimes help identify the type
            if uid[0] == 0x04 {
                return "NTAG21x".to_string();
            } else {
                return "MIFARE Classic".to_string();
            }
        },
        7 => {
            // 7-byte UID: Usually DESFire or other high-security cards
            return "MIFARE DESFire".to_string();
        },
        10 => {
            // 10-byte UID: Usually DESFire
            return "MIFARE DESFire".to_string();
        },
        _ => {
            // Other lengths: Generic determination
            if uid.len() < 4 {
                return "Unknown (Short UID)".to_string();
            } else {
                return "MIFARE Classic".to_string(); // Assume Classic as fallback
            }
        }
    }
}
