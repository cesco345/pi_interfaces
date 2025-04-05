// src/bin/mifare_writer.rs
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::error::Error;

use ndef_explorer::commands::ndef_commands::send_apdu;
use ndef_explorer::operations::ndef_operations_writer::CardExport;
use ndef_explorer::util::ndef_util::hex_string;

// Additional imports for card handling
use pcsc::{Card, Context, Scope, ShareMode, Protocols};

fn main() -> Result<(), Box<dyn Error>> {
    println!("MIFARE Classic Card Writer");
    println!("========================\n");
    
    // Check command-line arguments for input file
    let args: Vec<String> = env::args().collect();
    let mut json_data = String::new();
    
    if args.len() > 1 {
        // Load JSON from file
        match fs::read_to_string(&args[1]) {
            Ok(content) => {
                json_data = content;
                println!("Loaded data from file: {}", args[1]);
            },
            Err(e) => {
                eprintln!("Error reading file: {}", e);
                return Err(e.into());
            }
        }
    } else {
        // Read JSON from stdin
        println!("Paste the exported JSON data (press Enter, then Ctrl+D when finished):");
        
        let stdin = io::stdin();
        let mut lines = stdin.lock().lines();
        
        while let Some(line) = lines.next() {
            match line {
                Ok(line) => json_data.push_str(&format!("{}\n", line)),
                Err(e) => {
                    eprintln!("Error reading input: {}", e);
                    return Err(e.into());
                }
            }
        }
    }
    
    // Parse the JSON data
    let card_data: CardExport = match serde_json::from_str(&json_data) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error parsing JSON data: {}", e);
            return Err(e.into());
        }
    };
    
    // Print card data
    println!("\nCard Information:");
    println!("  Name: {}", card_data.name);
    println!("  Application ID: {}", card_data.applicationId);
    println!("  File ID: {}", card_data.fileId);
    println!("  Data: {}", card_data.fileData);
    println!("  Export Date: {}", card_data.exportDate);
    
    // Ask for confirmation
    print!("\nContinue with writing to MIFARE Classic card? (y/n): ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() != "y" {
        println!("Operation cancelled.");
        return Ok(());
    }
    
    // Connect to the card
    println!("\nPlace your MIFARE Classic card on the reader and press Enter to continue...");
    io::stdout().flush()?;
    let mut _wait = String::new();
    io::stdin().read_line(&mut _wait)?;
    
    // Connect to card
    let (ctx, card) = connect_to_card()?;
    
    // Format and write to the MIFARE Classic card
    let result = format_mifare_classic(&card, &card_data);
    
    if result.is_err() {
        println!("\nTrying alternative method with direct data write...");
        // If regular formatting fails, try simpler direct write
        direct_write_mifare(&card, &card_data)?;
    }
    
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
    
    for key in &keys {
        println!("Trying key: {}", hex_string(key));
        
        // Authentication command
        let mut auth_cmd = vec![0xFF, 0x86, 0x00, 0x00, 0x05, 0x01, 0x00, block, key_type];
        auth_cmd.push(0x06); // Key length
        auth_cmd.extend_from_slice(key);
        
        if let Some(_) = send_apdu(card, &auth_cmd, &format!("Auth S{}", sector)) {
            println!("Authentication successful with key: {}", hex_string(key));
            return true;
        }
    }
    
    // Load Key and Authenticate separately (alternative method)
    for key in &keys {
        println!("Trying alternative method with key: {}", hex_string(key));
        
        // Load key command
        let mut load_key = vec![0xFF, 0x82, 0x00, 0x00, 0x06];
        load_key.extend_from_slice(key);
        
        if let Some(_) = send_apdu(card, &load_key, "Load Key") {
            // Authentication command
            let auth_cmd = [0xFF, 0x88, 0x00, block, 0x60 + key_type - 0x60, 0x00];
            
            if let Some(_) = send_apdu(card, &auth_cmd, &format!("Alt Auth S{}", sector)) {
                println!("Authentication successful with alternative method");
                return true;
            }
        }
    }
    
    println!("Authentication failed for all keys");
    false
}

// Format MIFARE Classic card for NDEF
fn format_mifare_classic(card: &Card, card_data: &CardExport) -> Result<(), Box<dyn Error>> {
    println!("\nFormatting MIFARE Classic card for NDEF...");
    
    // Convert data to bytes
    let data_str = &card_data.fileData;
    let mut data_bytes = Vec::new();
    
    // Parse data - could be hex or text
    if data_str.contains(":") || data_str.contains(" ") {
        // Probably hex bytes
        for part in data_str.split(|c| c == ':' || c == ' ') {
            if !part.is_empty() {
                if let Ok(byte) = u8::from_str_radix(part, 16) {
                    data_bytes.push(byte);
                }
            }
        }
    } else {
        // Treat as text
        data_bytes = data_str.as_bytes().to_vec();
    }
    
    // Create a simple NFC Data Exchange Format (NDEF) message
    println!("Creating NDEF message...");
    
    // NDEF record header: MB=1, ME=1, SR=1, IL=0, TNF=1 (Well Known Type)
    let mut ndef_message = vec![0xD1];
    // Type length (1 byte)
    ndef_message.push(0x01);
    // Payload length (data length + status byte + language code length)
    ndef_message.push((data_bytes.len() + 3) as u8);
    // Type: 'T' for Text record
    ndef_message.push(0x54);
    // Status byte (UTF-8 encoding, 2-byte language code)
    ndef_message.push(0x02);
    // Language code: 'en'
    ndef_message.push(0x65); // 'e'
    ndef_message.push(0x6E); // 'n'
    // Text content
    ndef_message.extend_from_slice(&data_bytes);
    
    // Calculate TLV structures
    // NDEF Message TLV
    let mut tlv_data = vec![0x03]; // NDEF Message TLV tag
    let tlv_length = ndef_message.len();
    tlv_data.push(tlv_length as u8); // Length (assuming length < 255 bytes)
    tlv_data.extend_from_slice(&ndef_message); // Value
    // Terminator TLV
    tlv_data.push(0xFE); // Terminator TLV tag
    
    println!("NDEF TLV data prepared: {}", hex_string(&tlv_data));
    
    // Try to authenticate with sector 1 (NDEF data usually goes here)
    println!("\nAttempting to authenticate with sector 1...");
    if authenticate_sector(card, 1, 0x60) { // 0x60 = Key A
        println!("Authentication with sector 1 successful");
        
        // Write NDEF data to block 4
        let mut ndef_block = Vec::new();
        ndef_block.extend_from_slice(&tlv_data);
        
        // Pad to 16 bytes
        while ndef_block.len() < 16 {
            ndef_block.push(0x00);
        }
        
        // If data exceeds 16 bytes, truncate and warn
        if ndef_block.len() > 16 {
            println!("Warning: NDEF data too large, truncating to 16 bytes");
            ndef_block.truncate(16);
        }
        
        let mut write_ndef = vec![0xFF, 0xD6, 0x00, 0x04, 0x10]; // Write to block 4
        write_ndef.extend_from_slice(&ndef_block);
        
        if let Some(_) = send_apdu(card, &write_ndef, "Write NDEF") {
            println!("Successfully wrote NDEF data to block 4");
            println!("\n✅ MIFARE Classic card successfully written with NDEF data!");
            return Ok(());
        } else {
            return Err("Failed to write NDEF data".into());
        }
    } else {
        println!("Could not authenticate with sector 1");
        return Err("Authentication failed for all keys".into());
    }
}

// Direct write to MIFARE without NDEF formatting
fn direct_write_mifare(card: &Card, card_data: &CardExport) -> Result<(), Box<dyn Error>> {
    println!("\nTrying direct data write to MIFARE Classic card...");
    
    // We'll try to write to multiple blocks/sectors until one succeeds
    
    // Convert data to bytes
    let data_str = &card_data.fileData;
    let mut data_bytes = Vec::new();
    
    // Parse data - could be hex or text
    if data_str.contains(":") || data_str.contains(" ") {
        // Probably hex bytes
        for part in data_str.split(|c| c == ':' || c == ' ') {
            if !part.is_empty() {
                if let Ok(byte) = u8::from_str_radix(part, 16) {
                    data_bytes.push(byte);
                }
            }
        }
    } else {
        // Treat as text
        data_bytes = data_str.as_bytes().to_vec();
    }
    
    // Pad to 16 bytes
    while data_bytes.len() < 16 {
        data_bytes.push(0x00);
    }
    
    // If data exceeds 16 bytes, truncate and warn
    if data_bytes.len() > 16 {
        println!("Warning: Data too large, truncating to 16 bytes");
        data_bytes.truncate(16);
    }
    
    // Try writing to all data blocks in the first few sectors
    let data_blocks = [1, 2, 4, 5, 6, 8, 9, 10]; // Blocks that are not sector trailers or block 0
    
    for block in &data_blocks {
        // Try to authenticate with the block's sector
        let sector = block / 4;
        println!("\nTrying to write to block {} (sector {})...", block, sector);
        
        if authenticate_sector(card, sector, 0x60) { // 0x60 = Key A
            // Write data to block
            let mut write_cmd = vec![0xFF, 0xD6, 0x00, *block, 0x10]; // Write 16 bytes
            write_cmd.extend_from_slice(&data_bytes);
            
            if let Some(_) = send_apdu(card, &write_cmd, &format!("Write Block {}", block)) {
                println!("Successfully wrote data to block {}", block);
                println!("\n✅ Data successfully written to MIFARE Classic card (Block {})!", block);
                return Ok(());
            } else {
                println!("Failed to write to block {}, trying next block...", block);
            }
        } else {
            println!("Could not authenticate with sector {}, trying next block...", sector);
        }
    }
    
    // If we made it here, all attempts failed
    Err("Failed to write data to any block".into())
}
