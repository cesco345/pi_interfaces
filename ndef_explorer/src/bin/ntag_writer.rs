// src/bin/ntag_writer.rs
// Specialized writer for NTAG213 tags

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
    println!("  Format: {}", card_data.format);
    println!("  Export Date: {}", card_data.exportDate);
    
    // Verify it's intended for NTAG213
    if card_data.format.to_lowercase() != "ntag_213" && !args.iter().any(|arg| arg == "--force") {
        println!("\nWarning: This JSON file isn't marked for NTAG213 (format = {})", card_data.format);
        println!("If you want to proceed anyway, use the --force flag");
        return Ok(());
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
    
    Ok((ctx, card))
}

// Verify the card is an NTAG213
fn verify_ntag213(card: &Card) -> Result<(), Box<dyn Error>> {
    println!("Verifying NTAG213 card...");
    
    // Get card UID
    let get_uid = [0xFF, 0xCA, 0x00, 0x00, 0x00];
    
    if let Some(response) = send_apdu(card, &get_uid, "Get UID") {
        println!("Card UID: {}", hex_string(&response));
        
        // Check the SAK value (should be 0x00 for NTAG213)
        let get_sak_atqa = [0xFF, 0xCA, 0x01, 0x00, 0x00];
        if let Some(sak_atqa) = send_apdu(card, &get_sak_atqa, "Get SAK/ATQA") {
            if sak_atqa.len() >= 1 {
                let sak = sak_atqa[0];
                if sak == 0x00 {
                    println!("Card SAK value (0x00) matches NTAG21x");
                    
                    // Try to read the capability container (page 3)
                    let read_cc = [0xFF, 0xB0, 0x00, 0x03, 0x04];
                    if let Some(cc_data) = send_apdu(card, &read_cc, "Read Capability Container") {
                        println!("CC Data: {}", hex_string(&cc_data));
                        
                        // For NTAG213, the first byte is typically 0xE1 and the second is 0x10
                        if cc_data.len() >= 2 && cc_data[0] == 0xE1 {
                            println!("Card verified as NTAG213");
                            return Ok(());
                        }
                    }
                    
                    // Even if not NDEF formatted, it could still be an NTAG213
                    // Try to read a few pages to confirm it behaves like an NTAG213
                    let read_p4 = [0xFF, 0xB0, 0x00, 0x04, 0x04];
                    if let Some(_) = send_apdu(card, &read_p4, "Read Page 4") {
                        println!("Card verified as NTAG21x based on memory access");
                        return Ok(());
                    }
                }
            }
        }
        
        // If we can't verify exactly, but the card at least responds to basic commands
        // we'll proceed with caution
        println!("Warning: Could not positively identify as NTAG213, but proceeding anyway");
        println!("The card appears to be ISO14443-A Type 2 compatible");
        return Ok(());
    }
    
    Err("Card doesn't appear to be an NTAG213 or compatible tag".into())
}

// Write data to NTAG213 tag
fn write_to_ntag213(card: &Card, card_data: &CardExport) -> Result<(), Box<dyn Error>> {
    println!("\nWriting data to NTAG213 tag...");
    
    // Convert data to bytes
    let mut data_bytes = Vec::new();
    let data_str = &card_data.fileData;
    
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
    
    // NTAG213 specific - we can only write to pages 4-39
    // Each page is 4 bytes
    let start_page = 4;
    let end_page = 39;
    let page_size = 4;
    
    println!("Preparing to write {} bytes to NTAG213", data_bytes.len());
    
    // Ensure we don't try to write more data than the tag can hold
    let max_data_len = (end_page - start_page + 1) * page_size;
    if data_bytes.len() > max_data_len {
        println!("Warning: Data too large ({} bytes), truncating to {} bytes", 
                 data_bytes.len(), max_data_len);
        data_bytes.truncate(max_data_len);
    }
    
    // Pad data to multiple of page size
    while data_bytes.len() % page_size != 0 {
        data_bytes.push(0x00);
    }
    
    // Write data in 4-byte pages
    let mut success_count = 0;
    let mut current_page = start_page;
    
    for chunk in data_bytes.chunks(page_size) {
        // Skip writing if the chunk is all zeros (to avoid unnecessary writes)
        if chunk.iter().all(|&b| b == 0x00) {
            println!("Skipping page {} (all zeros)", current_page);
            current_page += 1;
            continue;
        }
        
        println!("Writing to page {}: {:02X} {:02X} {:02X} {:02X}", 
                 current_page, 
                 chunk.get(0).unwrap_or(&0), 
                 chunk.get(1).unwrap_or(&0), 
                 chunk.get(2).unwrap_or(&0), 
                 chunk.get(3).unwrap_or(&0));
        
        // Create APDU command with u8 values
        let cmd_header: [u8; 5] = [0xFF, 0xD6, 0x00, current_page as u8, 4];
        
        // Create a new vector with all u8 values
        let mut write_cmd = Vec::with_capacity(cmd_header.len() + chunk.len());
        write_cmd.extend_from_slice(&cmd_header);
        write_cmd.extend_from_slice(chunk);
        
        if let Some(_) = send_apdu(card, &write_cmd, &format!("Write Page {}", current_page)) {
            println!("Successfully wrote to page {}", current_page);
            success_count += 1;
        } else {
            println!("Failed to write to page {}", current_page);
            
            // If we hit a protected page, we can try to skip it and continue
            if current_page >= 36 { // Pages 36-39 are often protected
                println!("This might be a protected page, attempting to continue with next page");
            } else {
                // For lower pages, if we fail, it's likely a problem with the card or reader
                return Err(format!("Failed to write to page {}", current_page).into());
            }
        }
        
        current_page += 1;
    }
    
    if success_count > 0 {
        println!("\n✅ Successfully wrote to {} pages of NTAG213 tag", success_count);
        Ok(())
    } else {
        Err("Failed to write any data to NTAG213 tag".into())
    }
}
