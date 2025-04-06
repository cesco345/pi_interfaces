// Functions related to MIFARE card authentication

use pcsc::Card;
use crate::commands::ndef_commands::send_apdu;
use crate::util::ndef_util::hex_string;

/// Common MIFARE Classic keys to try
const COMMON_KEYS: [[u8; 6]; 10] = [
    [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], // Default factory key
    [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5], // NDEF key
    [0xD3, 0xF7, 0xD3, 0xF7, 0xD3, 0xF7], // NDEF key alternative
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // All zeros
    [0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5], // Common key
    [0x4D, 0x3A, 0x99, 0xC3, 0x51, 0xDD], // Common key
    [0x1A, 0x98, 0x2C, 0x7E, 0x45, 0x9A], // Common key
    [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56], // Another common key
    [0x71, 0x4C, 0x5C, 0x88, 0x6E, 0x97], // Public transport key
    [0x8F, 0xD0, 0xA4, 0xF2, 0x56, 0xE9]  // Another common key
];

/// Try to authenticate with different keys
pub fn authenticate_sector(card: &Card, sector: u8, key_type: u8) -> bool {
    // Calculate block number (first block of the sector)
    let block = if sector == 0 { 0 } else { sector * 4 };
    
    // Try all keys with two different authentication methods
    for key in COMMON_KEYS.iter() {
        // Method 1: Load Key and Authenticate separately (more common)
        let mut load_key = vec![0xFF, 0x82, 0x00, 0x00, 0x06];
        load_key.extend_from_slice(key);
        
        if let Some(_) = send_apdu(card, &load_key, "Load Key") {
            // Authentication command
            let auth_cmd = [0xFF, 0x88, 0x00, block, key_type, 0x00];
            
            if let Some(_) = send_apdu(card, &auth_cmd, &format!("Auth S{}", sector)) {
                println!("Authentication successful with key: {}", hex_string(key));
                return true;
            }
        }
        
        // Method 2: Combined authentication (alternative method)
        let mut auth_cmd = vec![0xFF, 0x86, 0x00, 0x00, 0x05, 0x01, 0x00, block, key_type];
        auth_cmd.push(0x06); // Key length
        auth_cmd.extend_from_slice(key);
        
        if let Some(_) = send_apdu(card, &auth_cmd, &format!("Alt Auth S{}", sector)) {
            println!("Authentication successful with key (alt method): {}", hex_string(key));
            return true;
        }
    }
    
    false
}
