// Functions for interpreting NDEF data

use crate::util::ndef_util::hex_string;

/// Try to interpret data as NDEF
pub fn interpret_ndef_data(data: &[u8]) {
    if data.len() < 3 {
        return;
    }
    
    // Check for NDEF TLV structure
    if data[0] == 0x03 { // NDEF Message TLV tag
        let length = data[1] as usize;
        
        if data.len() >= 2 + length {
            println!("  NDEF Message Length: {} bytes", length);
            
            // Extract NDEF message
            let ndef_data = &data[2..(2 + length)];
            
            // Basic NDEF parsing
            if ndef_data.len() >= 3 {
                let header = ndef_data[0];
                let type_length = ndef_data[1] as usize;
                let payload_length = ndef_data[2] as usize;
                
                println!("  NDEF Header: 0x{:02X}", header);
                println!("  Type Length: {}", type_length);
                println!("  Payload Length: {}", payload_length);
                
                if ndef_data.len() >= 3 + type_length && type_length > 0 {
                    let record_type = &ndef_data[3..(3 + type_length)];
                    let type_char = if record_type.len() == 1 { 
                        format!("{}", record_type[0] as char) 
                    } else { 
                        hex_string(record_type) 
                    };
                    
                    println!("  Record Type: {}", type_char);
                    
                    // For Text records (type "T")
                    if type_length == 1 && record_type[0] == b'T' && 
                       ndef_data.len() >= 4 + payload_length {
                        
                        let payload = &ndef_data[3 + type_length..(3 + type_length + payload_length)];
                        
                        if payload.len() > 0 {
                            let status = payload[0];
                            let lang_length = status & 0x3F;
                            
                            if payload.len() >= 1 + lang_length as usize {
                                let lang = &payload[1..(1 + lang_length as usize)];
                                let lang_str = String::from_utf8_lossy(lang);
                                
                                let text = &payload[(1 + lang_length as usize)..];
                                let text_str = String::from_utf8_lossy(text);
                                
                                println!("  Language: {}", lang_str);
                                println!("\n========== DECODED TEXT MESSAGE ==========");
                                println!("  Text: {}", text_str);
                                println!("==========================================\n");
                            }
                        }
                    }
                }
            }
        }
    }
}
