// File: src/interpreter/ndef_interpreter.rs
use crate::util::ndef_util::{hex_string, tnf_to_string};
use crate::interpreter::ndef_interpreter_payload::interpret_ndef_payload;

// Interpret NDEF capability container
pub fn parse_capability_container(cc_data: &[u8]) {
    println!("Capability Container Analysis:");
    
    if cc_data.len() < 2 {
        println!("  CC data too short");
        return;
    }
    
    if cc_data.len() >= 1 {
        println!("  CCLEN (CC Length): {} bytes", cc_data[0]);
    }
    
    if cc_data.len() >= 2 {
        let major = cc_data[1] >> 4;
        let minor = cc_data[1] & 0x0F;
        println!("  Mapping Version: {}.{}", major, minor);
    }
    
    if cc_data.len() >= 3 {
        let max_size = (cc_data[2] as u16) * 8;
        println!("  Maximum NDEF data size: {} bytes", max_size);
    }
    
    if cc_data.len() >= 4 {
        let read_access = (cc_data[3] & 0xF0) >> 4;
        let write_access = cc_data[3] & 0x0F;
        
        println!("  Read Access: 0x{:X} - {}", read_access, match read_access {
            0x0 => "Read access granted without security",
            0x1 => "Read access granted with proprietary security",
            _ => "Unknown read access condition",
        });
        
        println!("  Write Access: 0x{:X} - {}", write_access, match write_access {
            0x0 => "Write access granted without security",
            0x1 => "Write access prohibited",
            0x2 => "Write access granted with proprietary security",
            _ => "Unknown write access condition",
        });
    }
}

// Interpret NDEF message structure
pub fn interpret_ndef_message(data: &[u8]) {
    if data.is_empty() {
        println!("Empty NDEF message");
        return;
    }
    
    println!("\nNDEF Message Interpretation:");
    
    let mut index = 0;
    let mut record_count = 0;
    
    while index < data.len() {
        if index + 3 >= data.len() {
            println!("  Incomplete NDEF record at position {}", index);
            break;
        }
        
        record_count += 1;
        
        // NDEF Record Header
        let header = data[index];
        let message_begin = (header & 0x80) != 0;
        let message_end = (header & 0x40) != 0;
        let chunk_flag = (header & 0x20) != 0;
        let short_record = (header & 0x10) != 0;
        let id_length_present = (header & 0x08) != 0;
        let tnf = header & 0x07; // Type Name Format
        
        println!("\n  Record #{} at position {}:", record_count, index);
        println!("    Header: 0x{:02X}", header);
        println!("    MB (Message Begin): {}", message_begin);
        println!("    ME (Message End): {}", message_end);
        println!("    CF (Chunk Flag): {}", chunk_flag);
        println!("    SR (Short Record): {}", short_record);
        println!("    IL (ID Length Present): {}", id_length_present);
        println!("    TNF (Type Name Format): {} - {}", tnf, tnf_to_string(tnf));
        
        index += 1;
        
        if index >= data.len() {
            println!("    Incomplete record (missing type length)");
            break;
        }
        
        // Type Length
        let type_length = data[index];
        println!("    Type Length: {}", type_length);
        index += 1;
        
        if index >= data.len() {
            println!("    Incomplete record (missing payload length)");
            break;
        }
        
        // Payload Length
        let payload_length: u32;
        if short_record {
            payload_length = data[index] as u32;
            println!("    Payload Length: {}", payload_length);
            index += 1;
        } else if index + 3 < data.len() {
            payload_length = ((data[index] as u32) << 24) |
                             ((data[index + 1] as u32) << 16) |
                             ((data[index + 2] as u32) << 8) |
                              (data[index + 3] as u32);
            println!("    Payload Length: {}", payload_length);
            index += 4;
        } else {
            println!("    Incomplete record (missing full payload length)");
            break;
        }
        
        // ID Length (if present)
        let id_length: u8;
        if id_length_present {
            if index >= data.len() {
                println!("    Incomplete record (missing ID length)");
                break;
            }
            id_length = data[index];
            println!("    ID Length: {}", id_length);
            index += 1;
        } else {
            id_length = 0;
        }
        
        // Extract Type
        let record_type: String;
        if type_length > 0 {
            if index + type_length as usize > data.len() {
                println!("    Incomplete record (missing type data)");
                break;
            }
            
            let type_data = &data[index..index + type_length as usize];
            if type_data.iter().all(|&b| b >= 32 && b <= 126) {
                record_type = String::from_utf8_lossy(type_data).to_string();
            } else {
                record_type = format!("Binary: {}", hex_string(type_data));
            }
            index += type_length as usize;
        } else {
            record_type = "Empty".to_string();
        }
        println!("    Type: {}", record_type);
        
        // Extract ID (if present)
        if id_length > 0 {
            if index + id_length as usize > data.len() {
                println!("    Incomplete record (missing ID data)");
                break;
            }
            
            let id_data = &data[index..index + id_length as usize];
            if id_data.iter().all(|&b| b >= 32 && b <= 126) {
                println!("    ID: {}", String::from_utf8_lossy(id_data));
            } else {
                println!("    ID: {}", hex_string(id_data));
            }
            index += id_length as usize;
        }
        
        // Extract and interpret Payload
        if payload_length > 0 {
            if index + payload_length as usize > data.len() {
                println!("    Incomplete record (missing full payload)");
                println!("    Partial Payload: {}", 
                         hex_string(&data[index..data.len()]));
                
                // Try to interpret partial payload
                interpret_partial_payload(tnf, &record_type, &data[index..data.len()]);
                break;
            }
            
            let payload = &data[index..index + payload_length as usize];
            println!("    Payload ({} bytes): {}", payload.len(), hex_string(payload));
            
            // Interpret payload based on TNF and record type
            interpret_ndef_payload(tnf, &record_type, payload);
            
            // Enhanced text extraction for Text records
            if tnf == 0x01 && (record_type == "T" || record_type == "Text") {
                extract_text_from_payload(payload);
            }
            
            index += payload_length as usize;
        }
        
        if message_end {
            break;
        }
    }
    
    println!("\n  Total NDEF records processed: {}", record_count);
}

// Try to interpret partial payload (for incomplete records)
fn interpret_partial_payload(tnf: u8, record_type: &str, payload: &[u8]) {
    if payload.is_empty() {
        return;
    }
    
    if tnf == 0x01 && (record_type == "T" || record_type == "Text") {
        // This is a Text record, try to extract text
        extract_text_from_payload(payload);
    }
    
    // Attempt to extract any readable text
    extract_readable_text(payload);
}

// Enhanced text extraction for Text records
fn extract_text_from_payload(payload: &[u8]) {
    if payload.is_empty() {
        return;
    }
    
    let status_byte = payload[0];
    let utf16 = (status_byte & 0x80) != 0;
    let lang_code_len = status_byte & 0x3F;
    
    // Check if we have enough data
    if (1 + lang_code_len as usize) > payload.len() {
        println!("      Text record too short to extract language code");
        return;
    }
    
    // Extract language code
    let lang_code = &payload[1..1 + lang_code_len as usize];
    let lang_str = String::from_utf8_lossy(lang_code);
    
    println!("      📝 Language Code: {}", lang_str);
    println!("      📝 Encoding: {}", if utf16 { "UTF-16" } else { "UTF-8" });
    
    // Extract text content
    if 1 + (lang_code_len as usize) < payload.len() {
        let text_bytes = &payload[1 + lang_code_len as usize..];
        
        if !utf16 {
            // UTF-8 encoding
            match std::str::from_utf8(text_bytes) {
                Ok(text) => println!("      📝 Text Content: \"{}\"", text),
                Err(_) => println!("      📝 Text Content: [invalid UTF-8 sequence]")
            }
        } else {
            // UTF-16 encoding - basic handling
            if text_bytes.len() % 2 != 0 {
                println!("      📝 Text Content: [invalid UTF-16 sequence - odd number of bytes]");
                return;
            }
            
            let mut utf16_text = String::new();
            let mut i = 0;
            while i + 1 < text_bytes.len() {
                let code_unit = ((text_bytes[i] as u16) << 8) | (text_bytes[i + 1] as u16);
                match std::char::from_u32(code_unit as u32) {
                    Some(c) => utf16_text.push(c),
                    None => utf16_text.push('�')
                }
                i += 2;
            }
            println!("      📝 Text Content: \"{}\"", utf16_text);
        }
    }
}

// Extract any readable text from data
fn extract_readable_text(data: &[u8]) {
    if data.len() < 4 {
        return; // Too short to bother
    }
    
    // Only extract if more than 50% are printable ASCII
    let printable_count = data.iter().filter(|&&b| b >= 32 && b <= 126).count();
    if printable_count > data.len() / 2 {
        let mut text = String::new();
        for &byte in data {
            if byte >= 32 && byte <= 126 {
                text.push(byte as char);
            } else if byte == 0 {
                // Skip null bytes
            } else {
                // Replace non-printable with space
                text.push(' ');
            }
        }
        
        // Clean up multiple spaces
        let cleaned = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if !cleaned.is_empty() {
            println!("      🔍 Extracted Text: \"{}\"", cleaned);
        }
    }
}

// Try to interpret NDEF data (simpler version for general use)
pub fn try_interpret_ndef(data: &[u8]) {
    if data.is_empty() {
        println!("Empty NDEF data");
        return;
    }
    
    println!("Attempting to interpret NDEF data");
    interpret_ndef_message(data);
}
