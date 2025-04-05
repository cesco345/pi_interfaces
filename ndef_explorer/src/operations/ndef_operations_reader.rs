// File: src/operations/ndef_operations_reader.rs
// Functions for reading NDEF messages
use crate::commands::ndef_commands::send_apdu;
use crate::interpreter::ndef_interpreter::interpret_ndef_message;
use crate::util::ndef_util::hex_string;

// Read complete NDEF message
pub fn read_ndef_message(card: &pcsc::Card) {
    println!("\nReading full NDEF Message...");
    println!("First, we need to locate the NDEF data...");
    
    // Common offsets for NDEF length in different cards
    let possible_offsets = [0x0F, 0x00, 0x03, 0x04, 0x10];
    
    for offset in possible_offsets.iter() {
        println!("\nTrying offset 0x{:02X} for NDEF length...", offset);
        if let Some(length_data) = send_apdu(card, &[0x00, 0xB0, 0x00, *offset, 0x02], 
                                   &format!("Read NDEF Length at offset 0x{:02X}", offset)) {
            if length_data.len() >= 2 {
                let length = (u16::from(length_data[0]) << 8) | u16::from(length_data[1]);
                println!("  NDEF Message Length: {} bytes", length);
                
                // For NDEF Type 4 tags, data typically starts at offset 0x10
                // regardless of where the length is found
                let mut ndef_data = Vec::new();
                
                // Try direct reading at offset 0x10 (NDEF message typically starts here)
                if let Some(data) = read_ndef_data_direct(card, 0x10, length) {
                    ndef_data = data;
                } else {
                    // If direct reading fails, try reading in chunks with different offsets
                    println!("Direct read failed, trying chunk-based read...");
                    let offsets_to_try = [0x10, 0x11, offset + 2];
                    
                    for &start_offset in &offsets_to_try {
                        if let Some(data) = read_ndef_data_in_chunks(card, start_offset, length) {
                            ndef_data = data;
                            break;
                        }
                    }
                }
                
                if !ndef_data.is_empty() {
                    // Process the NDEF data
                    process_ndef_data(&ndef_data);
                    return;
                }
            }
        }
    }
    
    println!("Could not find NDEF message length at common offsets.");
}

// Attempt to read NDEF data directly in one command
fn read_ndef_data_direct(card: &pcsc::Card, offset: u8, length: u16) -> Option<Vec<u8>> {
    if length > 255 {
        // Too large for a single read
        return None;
    }
    
    println!("\nAttempting direct read of {} bytes at offset 0x{:02X}...", length, offset);
    let apdu = [0x00, 0xB0, 0x00, offset, length as u8];
    
    match send_apdu(card, &apdu, &format!("Read NDEF Data (offset 0x{:02X})", offset)) {
        Some(data) if !data.is_empty() => {
            println!("  Successfully read {} bytes", data.len());
            Some(data)
        },
        _ => {
            println!("  Direct read failed");
            None
        }
    }
}

// Read NDEF data in chunks to handle large messages or fragmented storage
fn read_ndef_data_in_chunks(card: &pcsc::Card, start_offset: u8, total_length: u16) -> Option<Vec<u8>> {
    println!("\nReading NDEF data in chunks from offset 0x{:02X}...", start_offset);
    
    let mut full_data = Vec::new();
    let mut bytes_read = 0;
    let mut offset = start_offset;
    let mut consecutive_empty_reads = 0;
    
    // Read in smaller chunks to handle potential issues with large reads
    let chunk_size: u8 = 4; // Use small chunks (4 bytes) based on the logs
    
    while bytes_read < total_length && consecutive_empty_reads < 3 {
        let remaining = total_length - bytes_read;
        let to_read = std::cmp::min(u16::from(chunk_size), remaining) as u8;
        
        println!("Reading chunk of {} bytes at offset 0x{:X}...", to_read, offset);
        if let Some(chunk_data) = send_apdu(card, &[0x00, 0xB0, 0x00, offset, to_read], 
                                 &format!("Read chunk at 0x{:02X}", offset)) {
            if chunk_data.is_empty() {
                println!("  Empty chunk received");
                consecutive_empty_reads += 1;
                
                // Move to the next offset even if empty
                offset = offset.wrapping_add(to_read);
            } else {
                println!("  Got {} bytes: {}", chunk_data.len(), hex_string(&chunk_data));
                full_data.extend_from_slice(&chunk_data);
                bytes_read += chunk_data.len() as u16;
                offset = offset.wrapping_add(chunk_data.len() as u8);
                consecutive_empty_reads = 0;
            }
        } else {
            println!("  Failed to read chunk");
            consecutive_empty_reads += 1;
            // Still increment offset to try the next position
            offset = offset.wrapping_add(to_read);
        }
    }
    
    if full_data.is_empty() {
        println!("No NDEF data could be read using chunks");
        None
    } else {
        println!("Read total of {} bytes via chunks", full_data.len());
        Some(full_data)
    }
}

// Process and display the NDEF data
fn process_ndef_data(data: &[u8]) {
    println!("\nComplete NDEF Message ({} bytes):", data.len());
    println!("  Raw data: {}", hex_string(data));
    
    // Try to interpret NDEF message
    interpret_ndef_message(data);
    
    // Basic text extraction for readability
    extract_text_from_ndef(data);
}

// Extract readable text from NDEF data
fn extract_text_from_ndef(data: &[u8]) {
    // Look for NDEF Text record (Type T)
    if data.len() >= 7 {
        // Check for a Text record header pattern
        // First byte is often 0xD1 (TNF=1, MB=1, ME=1, SR=1)
        // For a text record, type length is usually 1, and type is 'T'
        for i in 0..data.len() - 6 {
            if (data[i] & 0x07) == 0x01 && // TNF = 1 (Well-Known)
               data[i+1] == 0x01 &&        // Type Length = 1
               data[i+3] == b'T' {         // Type = 'T'
                
                let payload_length = data[i+2] as usize;
                if i + 4 + payload_length <= data.len() {
                    // This looks like a text record, try to decode it
                    let payload = &data[i+4..i+4+payload_length];
                    if !payload.is_empty() {
                        // Text record payload starts with a status byte
                        let status = payload[0];
                        let lang_code_len = status & 0x3F;
                        
                        if 1 + lang_code_len as usize <= payload.len() {
                            // Skip status byte and language code
                            let text_start = 1 + lang_code_len as usize;
                            let text_bytes = &payload[text_start..];
                            
                            // Try to convert to UTF-8 string
                            match std::str::from_utf8(text_bytes) {
                                Ok(text) => println!("\n📝 Text content: \"{}\"", text),
                                Err(_) => println!("\nUnable to decode text as UTF-8")
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Generic ASCII extraction for any text-like content
    let mut ascii_text = String::new();
    for &byte in data {
        if byte >= 32 && byte <= 126 {
            ascii_text.push(byte as char);
        } else if !ascii_text.is_empty() {
            // Add space for non-printable chars
            ascii_text.push(' ');
        }
    }
    
    // Clean up multiple spaces
    let cleaned_text = ascii_text.split_whitespace().collect::<Vec<&str>>().join(" ");
    
    if cleaned_text.len() >= 4 {  // Only show if we have reasonable text
        println!("\n📄 Extracted ASCII text: \"{}\"", cleaned_text);
    }
}
