// src/bin/read_ntag.rs
use std::error::Error;
use std::io::{self, Write};

use pcsc::{Card, Context, Scope, ShareMode, Protocols};
use ndef_explorer::commands::ndef_commands::send_apdu;
use ndef_explorer::util::ndef_util::hex_string;

fn main() -> Result<(), Box<dyn Error>> {
    println!("NTAG213 Tag Reader");
    println!("================\n");
    
    // Connect to the card
    println!("Place your NTAG213 card on the reader and press Enter to continue...");
    io::stdout().flush()?;
    let mut _wait = String::new();
    io::stdin().read_line(&mut _wait)?;
    
    let (_ctx, card) = connect_to_card()?;
    
    // Read the tag
    read_ntag213(&card)?;
    
    println!("\nPlease remove the card from the reader when finished.");
    
    Ok(())
}

fn connect_to_card() -> Result<(Context, Card), Box<dyn Error>> {
    let ctx = Context::establish(Scope::User)?;
    
    let mut readers_buf = [0; 2048];
    let readers = ctx.list_readers(&mut readers_buf)?;
    
    let mut reader_found = false;
    let mut selected_reader = None;
    
    for reader in readers {
        reader_found = true;
        selected_reader = Some(reader);
        println!("Found reader: {}", reader.to_string_lossy());
        break;
    }
    
    if !reader_found {
        return Err("No smart card readers found".into());
    }
    
    let reader = selected_reader.ok_or("Failed to get reader")?;
    println!("Using reader: {}", reader.to_string_lossy());
    
    let card = ctx.connect(reader, ShareMode::Shared, Protocols::ANY)?;
    println!("Successfully connected to card");
    
    Ok((ctx, card))
}

fn read_ntag213(card: &Card) -> Result<(), Box<dyn Error>> {
    println!("Reading NTAG213 tag...");
    
    // Get card UID
    let get_uid = [0xFF, 0xCA, 0x00, 0x00, 0x00];
    
    if let Some(response) = send_apdu(card, &get_uid, "Get UID") {
        println!("Card UID: {}", hex_string(&response));
        
        // Read manufacturer data (pages 0-3)
        println!("\nManufacturer Data (Read-Only):");
        println!("==============================");
        
        let mut manufacturer_data = Vec::new();
        
        for page in 0..4 {
            let read_cmd = [0xFF, 0xB0, 0x00, page, 0x04];
            if let Some(page_data) = send_apdu(card, &read_cmd, &format!("Read Page {}", page)) {
                print!("Page {:02}: ", page);
                
                // Print hex representation
                for byte in &page_data {
                    print!("{:02X} ", byte);
                }
                
                // Try to print ASCII representation if printable
                print!(" | ");
                for &byte in &page_data {
                    if byte >= 32 && byte <= 126 {
                        print!("{}", byte as char);
                    } else {
                        print!(".");
                    }
                }
                
                // Add interpretation based on page number
                match page {
                    0 => print!(" « Serial Number Part 1"),
                    1 => print!(" « Serial Number Part 2"),
                    2 => print!(" « Internal/Lock Bytes"),
                    3 => print!(" « Capability Container (CC)"),
                    _ => {}
                }
                
                println!();
                
                manufacturer_data.extend_from_slice(&page_data);
            } else {
                println!("Page {:02}: Failed to read", page);
            }
        }
        
        // Read user memory (pages 4-39) - 36 pages total = 144 bytes
        println!("\nUser Memory (36 pages, 144 bytes):");
        println!("================================");
        
        // Collect all page data
        let mut all_memory = Vec::with_capacity(36 * 4); // 36 pages * 4 bytes
        
        for page in 4..40 {
            let read_cmd = [0xFF, 0xB0, 0x00, page, 0x04];
            if let Some(page_data) = send_apdu(card, &read_cmd, &format!("Read Page {}", page)) {
                print!("Page {:02}: ", page);
                
                // Print hex representation
                for byte in &page_data {
                    print!("{:02X} ", byte);
                }
                
                // Try to print ASCII representation if printable
                print!(" | ");
                for &byte in &page_data {
                    if byte >= 32 && byte <= 126 {
                        print!("{}", byte as char);
                    } else {
                        print!(".");
                    }
                }
                
                // Add information about special pages
                if page == 4 && page_data.len() > 0 && page_data[0] == 0x03 {
                    print!(" « NDEF Message Start");
                }
                
                println!();
                
                // Add to combined memory
                all_memory.extend_from_slice(&page_data);
            } else {
                println!("Page {:02}: Failed to read", page);
            }
        }
        
        // Try to parse NDEF message
        if !all_memory.is_empty() {
            extract_and_display_ndef(&all_memory);
        }
        
        // Display memory summary
        println!("\nMemory Summary:");
        println!("==============");
        println!("• Manufacturer Data: Pages 0-3 (16 bytes) - Read-only");
        println!("• User Memory: Pages 4-39 (144 bytes) - Read/Write");
        println!("• Total NTAG213 memory: 40 pages × 4 bytes = 160 bytes");
        
        Ok(())
    } else {
        Err("Failed to read card UID".into())
    }
}

fn extract_and_display_ndef(memory: &[u8]) {
    println!("\nNDEF Message Content:");
    println!("====================");
    
    // Look for NDEF message TLV (0x03)
    let mut i = 0;
    while i < memory.len() {
        if memory[i] == 0x03 {
            // Found NDEF Message TLV
            if i + 1 >= memory.len() {
                println!("NDEF TLV found but incomplete");
                return;
            }
            
            // Get length
            let mut length = memory[i + 1] as usize;
            let mut offset = i + 2;
            
            // Check for extended length format
            if length == 0xFF {
                if i + 3 >= memory.len() {
                    println!("NDEF TLV with extended length but incomplete");
                    return;
                }
                length = ((memory[i + 2] as usize) << 8) | (memory[i + 3] as usize);
                offset = i + 4;
            }
            
            if offset + length > memory.len() {
                println!("NDEF message truncated or invalid length");
                return;
            }
            
            // Check if it's a Text record (TNF=1, RTD=T)
            if length >= 3 && memory[offset] == 0xD1 && memory[offset + 3] == 0x54 {
                // It's a Text record
                let type_length = memory[offset + 1] as usize;
                let payload_length = memory[offset + 2] as usize;
                
                if offset + 4 + payload_length > memory.len() {
                    println!("Text record truncated or invalid length");
                    return;
                }
                
                let payload_offset = offset + 4; // Skip TNF + Type Length + Payload Length + Type
                let payload = &memory[payload_offset..payload_offset + payload_length];
                
                if payload.len() > 0 {
                    // First byte is status (encoding + language length)
                    let language_length = (payload[0] & 0x3F) as usize;
                    
                    if 1 + language_length < payload.len() {
                        // Extract the actual text
                        let text_offset = 1 + language_length;
                        let text_bytes = &payload[text_offset..];
                        
                        // Convert to UTF-8 string with proper handling for special chars/emojis
                        match std::str::from_utf8(text_bytes) {
                            Ok(text) => {
                                println!("\n✅ NDEF Text Content: \"{}\"", text);
                                println!("\nTechnical Details:");
                                println!("• Record type: Text (RTD_TEXT)");
                                println!("• Language code: {}", 
                                       std::str::from_utf8(&payload[1..1+language_length])
                                           .unwrap_or("invalid"));
                                println!("• Character encoding: {}", 
                                       if payload[0] & 0x80 == 0 { "UTF-8" } else { "UTF-16" });
                                println!("• Text length: {} bytes", text_bytes.len());
                            },
                            Err(_) => {
                                println!("Invalid UTF-8 text data");
                            }
                        }
                    }
                }
            } else {
                println!("Found NDEF message but not a Text record or unsupported format");
                println!("Record header: {:02X} {:02X} {:02X} {:02X}", 
                       memory[offset], 
                       memory[offset+1], 
                       memory[offset+2], 
                       memory[offset+3]);
            }
            
            // Found what we were looking for, so exit
            return;
        }
        
        i += 1;
    }
    
    println!("No NDEF message found");
}
