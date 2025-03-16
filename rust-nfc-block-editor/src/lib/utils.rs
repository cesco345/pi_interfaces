// Utility functions for formatting and data conversion

// Format UID as a hex string
pub fn uid_to_string(uid: &[u8]) -> String {
    uid.iter()
        .map(|byte| format!("{:02X}", byte))
        .collect::<Vec<String>>()
        .join(":")
}

// Convert UID to a single decimal number
pub fn uid_to_num(uid: &[u8]) -> u64 {
    let mut num: u64 = 0;
    
    for &byte in uid {
        num = num * 256 + (byte as u64);
    }
    
    num
}

// Format bytes as a hex string
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|byte| format!("{:02X}", byte))
        .collect::<Vec<String>>()
        .join(" ")
}

// Convert bytes to ASCII string (replacing non-printable chars with dots)
pub fn bytes_to_ascii(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|&byte| {
            if byte >= 32 && byte <= 126 {
                byte as char
            } else {
                '.'
            }
        })
        .collect()
}

// Convert a hex string to bytes
pub fn hex_string_to_bytes(hex_str: &str) -> Option<Vec<u8>> {
    // Remove spaces and other non-hex characters
    let cleaned = hex_str.chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>();
    
    // Check if we have a valid hex string (must be even length)
    if cleaned.len() % 2 != 0 {
        return None;
    }
    
    // Convert to bytes
    let mut bytes = Vec::with_capacity(cleaned.len() / 2);
    
    for i in (0..cleaned.len()).step_by(2) {
        if let Ok(byte) = u8::from_str_radix(&cleaned[i..i+2], 16) {
            bytes.push(byte);
        } else {
            return None;
        }
    }
    
    Some(bytes)
}
