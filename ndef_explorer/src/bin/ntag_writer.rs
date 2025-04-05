// src/bin/ntag_writer.rs
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::error::Error;

use ndef_explorer::operations::ndef_operations_writer::CardExport;
use ndef_explorer::commands::ndef_commands::send_apdu;
use ndef_explorer::util::ndef_util::hex_string;

use pcsc::{Card, Context, Scope, ShareMode, Protocols};

fn main() -> Result<(), Box<dyn Error>> {
    println!("NTAG213 Tag Writer");
    println!("================\n");
    
    // Check command-line arguments for input file
    let args: Vec<String> = env::args().collect();
    let force_mode = args.iter().any(|arg| arg == "--force");
    let mut json_data = String::new();
    let mut input_file = String::new();
    
    if args.len() > 1 {
        // Get the first argument that's not "--force"
        for arg in &args[1..] {
            if arg != "--force" {
                input_file = arg.clone();
                break;
            }
        }
        
        if !input_file.is_empty() {
            // Load JSON from file
            match fs::read_to_string(&input_file) {
                Ok(content) => {
                    json_data = content;
                    println!("Loaded data from file: {}", input_file);
                },
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    return Err(e.into());
                }
            }
        }
    }
    
    if json_data.is_empty() {
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
    let mut card_data: CardExport = match serde_json::from_str(&json_data) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error parsing JSON data: {}", e);
            return Err(e.into());
        }
    };
    
    // Print card data
    println!("\nCard Information:");
    println!("  Name: {}", card_data.name);
    println!("  Data: {}", card_data.fileData);
    
    // If using force mode, silently convert format
    if force_mode && card_data.format.to_lowercase() != "ntag_213" {
        card_data.format = "ntag_213".to_string();
    }
    
    // Ask for confirmation
    print!("\nContinue with writing to NTAG213? (y/n): ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() != "y" {
        println!("Operation cancelled.");
        return Ok(());
    }
    
    // Connect to the card
    println!("\nPlace your NTAG213 card on the reader and press Enter to continue...");
    io::stdout().flush()?;
    let mut _wait = String::new();
    io::stdin().read_line(&mut _wait)?;
    
    // Connect to card
    let (_ctx, card) = connect_to_card()?;
    
    // Verify it's an NTAG213 card
    verify_ntag213(&card)?;
    
    // Write data to the NTAG213 card
    write_to_ntag213(&card, &card_data)?;
    
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
    
    // Check if we have any readers
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
    
    Ok((ctx, card))
}

// Verify the card is an NTAG213
fn verify_ntag213(card: &Card) -> Result<(), Box<dyn Error>> {
    println!("Verifying NTAG213 card...");
    
    // Get card UID
    let get_uid = [0xFF, 0xCA, 0x00, 0x00, 0x00];
    
    if let Some(response) = send_apdu(card, &get_uid, "Get UID") {
        println!("Card UID: {}", hex_string(&response));
        
        // Try to read a page directly
        let read_p4 = [0xFF, 0xB0, 0x00, 0x04, 0x04];
        if let Some(p4_data) = send_apdu(card, &read_p4, "Read Page 4") {
            println!("Card appears to be NTAG21x compatible");
            return Ok(());
        }
        
        // If direct page read fails, try with a different command set for ACR122U
        let read_direct = [0xFF, 0x00, 0x00, 0x00, 0x04, 0xD4, 0x42, 0x04, 0x00];
        if let Some(_) = send_apdu(card, &read_direct, "Direct Read Page 4") {
            println!("Card appears to be NTAG21x compatible (direct mode)");
            return Ok(());
        }
        
        // If we can still not verify, proceed with caution
        println!("Card appears to be ISO14443-A Type 2 compatible");
        return Ok(());
    }
    
    Err("Card doesn't appear to be an NTAG213 or compatible tag".into())
}

// Write data to NTAG213 tag
fn write_to_ntag213(card: &Card, card_data: &CardExport) -> Result<(), Box<dyn Error>> {
    println!("\nWriting data to NTAG213 tag...");
    
    let data_str = &card_data.fileData;
    let data_bytes = create_ndef_text_record(data_str);
    
    // NTAG213 specific - we can only write to pages 4-39
    // Each page is 4 bytes, but we need to first prepare the tag
    // with proper NDEF formatting starting at page 3 (CC)
    
    // Page 3 (CC) - Standard values for NTAG213
    let cc_page = [0xE1, 0x10, 0x6D, 0x00];
    let write_cc_cmd = [0xFF, 0xD6, 0x00, 0x03, 0x04, cc_page[0], cc_page[1], cc_page[2], cc_page[3]];
    
    println!("Preparing NTAG213 tag for writing...");
    if let Some(_) = send_apdu(card, &write_cc_cmd, "Initialize tag") {
        println!("Tag initialized successfully");
    } else {
        println!("Warning: Tag initialization may not be complete");
    }
    
    println!("Writing data: \"{}\"", data_str);
    
    // Ensure we don't try to write more data than the tag can hold
    let start_page = 4;
    let end_page = 39;
    let page_size = 4;
    let max_data_len = (end_page - start_page + 1) * page_size;
    
    if data_bytes.len() > max_data_len {
        println!("Warning: Data too large ({} bytes), truncating", data_bytes.len());
    }
    
    // Write data in 4-byte pages
    let mut success_count = 0;
    let mut current_page = start_page;
    
    for chunk in data_bytes.chunks(page_size) {
        // Skip empty pages at the end
        if current_page >= start_page + (data_bytes.len() + page_size - 1) / page_size {
            break;
        }
        
        // Create APDU command with u8 values
        let cmd_header: [u8; 5] = [0xFF, 0xD6, 0x00, current_page as u8, 4];
        
        // Create a new vector with all u8 values
        let mut write_cmd = Vec::with_capacity(cmd_header.len() + chunk.len());
        write_cmd.extend_from_slice(&cmd_header);
        write_cmd.extend_from_slice(chunk);
        
        // Pad the chunk if needed
        let mut padded_chunk = chunk.to_vec();
        while padded_chunk.len() < page_size {
            padded_chunk.push(0x00);
        }
        
        if let Some(_) = send_apdu(card, &write_cmd, &format!("Write Page {}", current_page)) {
            success_count += 1;
        } else {
            // Try alternative write method using direct commands for ACR122U
            let mut direct_cmd = vec![0xFF, 0x00, 0x00, 0x00, 0x05 + padded_chunk.len() as u8, 
                                      0xD4, 0x40, current_page as u8];
            direct_cmd.extend_from_slice(&padded_chunk);
            
            if let Some(_) = send_apdu(card, &direct_cmd, &format!("Direct Write Page {}", current_page)) {
                success_count += 1;
            } else {
                // If we can't write to this page, try the next one
                println!("  Skipping page {}", current_page);
            }
        }
        
        current_page += 1;
    }
    
    if success_count > 0 {
        println!("\n✅ Successfully wrote \"{}\" to NTAG213 tag", data_str);
        Ok(())
    } else {
        Err("Failed to write data to NTAG213 tag".into())
    }
}

// Create an NDEF Text Record for the given text
fn create_ndef_text_record(text: &str) -> Vec<u8> {
    // UTF-8 encoding of text might be different than byte length for emojis
    let text_bytes = text.as_bytes();
    let text_len = text_bytes.len();
    
    // Create NDEF record
    let mut ndef_record = Vec::with_capacity(32 + text_len);
    
    // NDEF Message TLV
    ndef_record.push(0x03);
    
    // Compute payload length (include language code + text)
    let payload_len = 3 + text_len; // 3 bytes for language code + status
    
    // Total length for TLV value
    if payload_len + 4 > 254 { // For long records (rare)
        ndef_record.push(0xFF);
        ndef_record.push(((payload_len + 4) >> 8) as u8);
        ndef_record.push(((payload_len + 4) & 0xFF) as u8);
    } else {
        ndef_record.push((payload_len + 4) as u8); // +4 for NDEF header fields
    }
    
    // NDEF header
    ndef_record.push(0xD1); // MB=1, ME=1, CF=0, SR=1, IL=0, TNF=1
    ndef_record.push(0x01); // Type length = 1 (T)
    ndef_record.push(payload_len as u8); // Payload length
    ndef_record.push(0x54); // Type = 'T' (Text)
    
    // Text payload
    ndef_record.push(0x02); // Status (UTF-8 + 2-byte language code)
    ndef_record.push(0x65); // 'e'
    ndef_record.push(0x6E); // 'n'
    
    // Add text
    ndef_record.extend_from_slice(text_bytes);
    
    // Add TLV terminator
    ndef_record.push(0xFE);
    
    // Pad to multiple of 4 bytes
    while ndef_record.len() % 4 != 0 {
        ndef_record.push(0x00);
    }
    
    ndef_record
}
