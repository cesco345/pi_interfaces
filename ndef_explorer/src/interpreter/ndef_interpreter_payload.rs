// Functions for interpreting specific NDEF payload types
use crate::util::ndef_util::hex_string;

// Interpret the payload based on TNF and record type
pub fn interpret_ndef_payload(tnf: u8, record_type: &str, payload: &[u8]) {
    match tnf {
        0x01 => { // NFC Forum well-known type
            match record_type {
                "T" => interpret_text_record(payload),
                "U" => interpret_uri_record(payload),
                "Sp" => println!("    Smart Poster Record"),
                _ => println!("    Well-known type: {}", record_type),
            }
        },
        0x02 => { // Media type (MIME)
            if record_type.starts_with("text/") {
                println!("    Text Content: {}", String::from_utf8_lossy(payload));
            } else {
                println!("    MIME Type: {}", record_type);
            }
        },
        0x03 => { // Absolute URI
            println!("    URI: {}", record_type);
        },
        0x04 => { // External type
            println!("    External Type: {}", record_type);
        },
        _ => {},
    }
}

// Interpret NDEF Text record
pub fn interpret_text_record(payload: &[u8]) {
    if payload.is_empty() {
        println!("    Empty Text Record");
        return;
    }
    
    let status_byte = payload[0];
    let utf16_flag = (status_byte & 0x80) != 0;
    let language_code_length = status_byte & 0x3F;
    
    if 1 + language_code_length as usize > payload.len() {
        println!("    Invalid Text Record");
        return;
    }
    
    let language_code = &payload[1..1 + language_code_length as usize];
    let language = String::from_utf8_lossy(language_code);
    
    let text_start = 1 + language_code_length as usize;
    let text_data = &payload[text_start..payload.len()];
    
    let text = if utf16_flag {
        // UTF-16 handling (simplified - would need proper UTF-16 decoding)
        "[UTF-16 text - display not implemented]".to_string()
    } else {
        String::from_utf8_lossy(text_data).to_string()
    };
    
    println!("\n========== DECODED TEXT MESSAGE ==========");
    println!("  Language: {}", language);
    println!("  Encoding: {}", if utf16_flag { "UTF-16" } else { "UTF-8" });
    
    if !utf16_flag {
        println!("  Text: {}", text);
    } else {
        println!("  Text: [UTF-16 text not decoded]");
    }
    println!("=========================================\n");
}

// Interpret NDEF URI record
pub fn interpret_uri_record(payload: &[u8]) {
    if payload.is_empty() {
        println!("    Empty URI Record");
        return;
    }
    
    let prefix_id = payload[0];
    let uri_field = &payload[1..];
    
    let prefix = match prefix_id {
        0x00 => "",
        0x01 => "http://www.",
        0x02 => "https://www.",
        0x03 => "http://",
        0x04 => "https://",
        0x05 => "tel:",
        0x06 => "mailto:",
        0x07 => "ftp://anonymous:anonymous@",
        0x08 => "ftp://ftp.",
        0x09 => "ftps://",
        0x0A => "sftp://",
        0x0B => "smb://",
        0x0C => "nfs://",
        0x0D => "ftp://",
        0x0E => "dav://",
        0x0F => "news:",
        0x10 => "telnet://",
        0x11 => "imap:",
        0x12 => "rtsp://",
        0x13 => "urn:",
        0x14 => "pop:",
        0x15 => "sip:",
        0x16 => "sips:",
        0x17 => "tftp:",
        0x18 => "btspp://",
        0x19 => "btl2cap://",
        0x1A => "btgoep://",
        0x1B => "tcpobex://",
        0x1C => "irdaobex://",
        0x1D => "file://",
        0x1E => "urn:epc:id:",
        0x1F => "urn:epc:tag:",
        0x20 => "urn:epc:pat:",
        0x21 => "urn:epc:raw:",
        0x22 => "urn:epc:",
        0x23 => "urn:nfc:",
        _ => "[Unknown prefix]",
    };
    
    println!("  URI Record:");
    println!("    Full URI: {}{}", prefix, String::from_utf8_lossy(uri_field));
}
