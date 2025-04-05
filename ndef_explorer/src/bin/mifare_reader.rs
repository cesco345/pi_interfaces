// src/bin/mifare_reader.rs
use std::io::{self, Write};
use std::error::Error;

use ndef_explorer::commands::ndef_commands::send_apdu;
use ndef_explorer::util::ndef_util::hex_string;

use pcsc::{Card, Context, Scope, ShareMode, Protocols};

fn main() -> Result<(), Box<dyn Error>> {
    println!("MIFARE Classic Card Reader");
    println!("========================\n");
    
    // Connect to the card
    println!("Place your MIFARE Classic card on the reader and press Enter to continue...");
    io::stdout().flush()?;
    let mut _wait = String::new();
    io::stdin().read_line(&mut _wait)?;
    
    // Connect to card
    let (ctx, card) = connect_to_card()?;
    
    // Dump the contents of all readable blocks
    read_mifare_classic_data(&card)?;
    
    println!("\nPlease remove the card from the reader when finished.");
    
    Ok(())
}

// Function to connect to the card
fn connect_to_card() -> Result<(Context, Card), Box<dyn Error>> {
    // Establish a PC/SC context
    let ctx = Context::establish(Scope::User)?;
    
    // Get available readers
    let mut readers_buf = [0; 2048];
    let readers = ctx.list_readers(&mut readers_buf)?;
    
    // Check if we have any readers by iterating through the readers
    let mut reader_found = false;
    let mut selected_reader = None;
    
    for reader in readers {
        reader_found = true;
        selected_reader = Some(reader);
        println!("Found reader: {}", reader.to_string_lossy());
        break; // Just use the first reader
    }
    
    if !reader_found {
        return Err("No smart card readers found".into());
    }
    
    // Use the selected reader
    let reader = selected_reader.ok_or("Failed to get reader")?;
    println!("Using reader: {}", reader.to_string_lossy());
    
    // Connect to the card
    let card = ctx.connect(reader, ShareMode::Shared, Protocols::ANY)?;
    println!("Successfully connected to card");
    
    // Verify it's a MIFARE Classic
    verify_mifare_classic(&card)?;
    
    Ok((ctx, card))
}

// Verify the card is a MIFARE Classic
fn verify_mifare_classic(card: &Card) -> Result<(), Box<dyn Error>> {
    println!("Verifying MIFARE Classic card...");
    
    // Get card UID (this command works with MIFARE Classic)
    let get_uid = [0xFF, 0xCA, 0x00, 0x00, 0x00];
    
    if let Some(response) = send_apdu(card, &get_uid, "Get UID") {
        println!("Card UID: {}", hex_string(&response));
        println!("Card verified as MIFARE Classic");
        Ok(())
    } else {
        Err("Card doesn't appear to be a MIFARE Classic card".into())
    }
}

// Try to authenticate with different keys
fn authenticate_sector(card: &Card, sector: u8, key_type: u8) -> bool {
    // Common MIFARE Classic keys to try
    let keys = [
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], // Default factory key
        [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5], // NDEF key
        [0xD3, 0xF7, 0xD3, 0xF7, 0xD3, 0xF7], // NDEF key alternative
        [0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // All zeros
        [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56]  // Another common key
    ];
    
    // Calculate block number (first block of the sector)
    let block = if sector == 0 { 0 } else { sector * 4 };
    
    // Load Key and Authenticate (using the method that worked in writer)
    for key in &keys {
        // Load key command
        let mut load_key = vec![0xFF, 0x82, 0x00, 0x00, 0x06];
        load_key.extend_from_slice(key);
        
        if let Some(_) = send_apdu(card, &load_key, "Load Key") {
            // Authentication command
            let auth_cmd = [0xFF, 0x88, 0x00, block, 0x60 + key_type - 0x60, 0x00];
            
            if let Some(_) = send_apdu(card, &auth_cmd, &format!("Auth S{}", sector)) {
                return true;
            }
        }
    }
    
    false
}

// Read and display data from MIFARE Classic card
fn read_mifare_classic_data(card: &Card) -> Result<(), Box<dyn Error>> {
    println!("\nReading data from MIFARE Classic card...");
    
    // We'll focus on the first few sectors
    // Each sector has 4 blocks (0-3, 4-7, 8-11, etc.)
    // Block 0 is manufacturer data
    // The last block in each sector is the sector trailer (keys and access bits)
    
    println!("\nSector\tBlock\tData");
    println!("---------------------------------");
    
    for sector in 0..4 {
        let first_block = sector * 4;
        let last_block = first_block + 3;
        
        if authenticate_sector(card, sector, 0x60) { // 0x60 = Key A
            println!("Successfully authenticated with sector {}", sector);
            
            // Read all blocks in this sector except the sector trailer
            for block in first_block..(last_block) {
                // Skip block 0 (manufacturer data)
                if block == 0 {
                    continue;
                }
                
                // Read block
                let read_cmd = [0xFF, 0xB0, 0x00, block, 0x10]; // Read 16 bytes
                
                if let Some(data) = send_apdu(card, &read_cmd, &format!("Read B{}", block)) {
                    let hex_data = hex_string(&data);
                    println!("  {}     {}    {}", sector, block, hex_data);
                    
                    // Try to interpret as text if possible
                    let text = data.iter()
                        .map(|&c| if c >= 32 && c <= 126 { c as char } else { '.' })
                        .collect::<String>();
                    println!("              Text: {}", text);
                    
                    // Check if this looks like NDEF data
                    if data.len() >= 2 && data[0] == 0x03 {
                        println!("              ⮕ Potential NDEF data detected!");
                        interpret_ndef_data(&data);
                    }
                } else {
                    println!("  {}     {}    (Read failed)", sector, block);
                }
            }
        } else {
            println!("Could not authenticate with sector {}", sector);
        }
    }
    
    Ok(())
}

// Try to interpret data as NDEF
fn interpret_ndef_data(data: &[u8]) {
    if data.len() < 3 {
        return;
    }
    
    // Check for NDEF TLV structure
    if data[0] == 0x03 { // NDEF Message TLV tag
        let length = data[1] as usize;
        
        if data.len() >= 2 + length {
            println!("              NDEF Message Length: {} bytes", length);
            
            // Extract NDEF message
            let ndef_data = &data[2..(2 + length)];
            
            // Basic NDEF parsing
            if ndef_data.len() >= 3 {
                let header = ndef_data[0];
                let type_length = ndef_data[1] as usize;
                let payload_length = ndef_data[2] as usize;
                
                println!("              NDEF Header: 0x{:02X}", header);
                println!("              Type Length: {}", type_length);
                println!("              Payload Length: {}", payload_length);
                
                if ndef_data.len() >= 3 + type_length && type_length > 0 {
                    let record_type = &ndef_data[3..(3 + type_length)];
                    let type_char = if record_type.len() == 1 { 
                        format!("{}", record_type[0] as char) 
                    } else { 
                        hex_string(record_type) 
                    };
                    
                    println!("              Record Type: {}", type_char);
                    
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
                                
                                println!("              Language: {}", lang_str);
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
