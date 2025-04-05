// File: src/util/ndef_util.rs

pub fn hex_string(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<String>>()
        .join(" ")
}

// Function to parse hex strings into bytes
pub fn parse_hex_string(hex_str: &str) -> Vec<u8> {
    let hex_str = hex_str.replace(" ", "").replace("0x", "");
    let mut result = Vec::new();
    
    for i in (0..hex_str.len()).step_by(2) {
        if i + 1 < hex_str.len() {
            if let Ok(byte) = u8::from_str_radix(&hex_str[i..i+2], 16) {
                result.push(byte);
            }
        }
    }
    
    result
}

pub fn protocol_to_string(protocol: pcsc::Protocol) -> String {
    match protocol {
        pcsc::Protocol::T0 => "T=0 (ISO 7816-3)".to_string(),
        pcsc::Protocol::T1 => "T=1 (ISO 7816-3)".to_string(),
        pcsc::Protocol::RAW => "RAW".to_string(),
        // There are only these three variants in this version of the crate
        #[allow(unreachable_patterns)]
        _ => format!("Unknown protocol ({:?})", protocol),
    }
}

// Status code interpreter
pub fn interpret_status_code(sw1: u8, sw2: u8) -> String {
    match (sw1, sw2) {
        (0x90, 0x00) => "Success".to_string(),
        (0x6A, 0x82) => "File not found".to_string(),
        (0x6A, 0x86) => "Incorrect parameters P1-P2".to_string(),
        (0x69, 0x86) => "Command not allowed (no EF selected)".to_string(),
        (0x6F, 0x00) => "Command not supported or invalid".to_string(),
        (0x61, _) => format!("More data available: {} bytes", sw2),
        (0x67, 0x00) => "Wrong length".to_string(),
        (0x6C, _) => format!("Wrong length, expected {}", sw2),
        (0x69, 0x82) => "Security status not satisfied".to_string(),
        (0x69, 0x85) => "Conditions of use not satisfied".to_string(),
        (0x6A, 0x87) => "Lc inconsistent with P1-P2".to_string(),
        _ => "Unknown status code".to_string(),
    }
}

// Convert NDEF Type Name Format (TNF) to string
pub fn tnf_to_string(tnf: u8) -> String {
    match tnf {
        0x00 => "Empty (0x00)".to_string(),
        0x01 => "NFC Forum well-known type (0x01)".to_string(),
        0x02 => "Media-type as defined in RFC 2046 (0x02)".to_string(),
        0x03 => "Absolute URI as defined in RFC 3986 (0x03)".to_string(),
        0x04 => "NFC Forum external type (0x04)".to_string(),
        0x05 => "Unknown (0x05)".to_string(),
        0x06 => "Unchanged (0x06)".to_string(),
        0x07 => "Reserved (0x07)".to_string(),
        _ => format!("Invalid TNF value (0x{:02X})", tnf),
    }
}
