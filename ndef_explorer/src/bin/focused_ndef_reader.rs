// File: src/bin/focused_ndef_reader.rs
use std::io::{self, Write, BufRead};
use pcsc::{Context, Protocols, Scope, ShareMode};
use std::ffi::CStr;
use std::fs;
use std::env;

use ndef_explorer::util::ndef_util::{hex_string, interpret_status_code, parse_hex_string};
use ndef_explorer::commands::ndef_commands::send_apdu;
use ndef_explorer::operations::ndef_operations::select_ndef_application;
use ndef_explorer::operations::ndef_operations_reader::read_ndef_message;
use ndef_explorer::operations::ndef_operations_writer::{write_ndef_message, write_imported_card_data, write_text_ndef_message};
use ndef_explorer::operations::ndef_operations_scanner::scan_readable_memory;
use ndef_explorer::interpreter::ndef_interpreter::parse_capability_container;
use ndef_explorer::commands::raw_commands::send_raw_command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check for import mode (command line argument)
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "--import" {
        if args.len() > 2 {
            // Import from file
            return import_and_write_card_data(&args[2]);
        } else {
            // Import from stdin
            return import_and_write_card_data_from_stdin();
        }
    }

    println!("NDEF Emulator Explorer");
    println!("=====================\n");

    // Establish PC/SC context
    let ctx = Context::establish(Scope::User)?;
    let mut reader_buffer = [0; 2048];
    let readers = ctx.list_readers(&mut reader_buffer)?;
    
    let mut reader_list = Vec::new();
    for reader in readers {
        reader_list.push(reader.to_owned());
    }
    
    if reader_list.is_empty() {
        return Err("No card readers found.".into());
    }

    // Select the first reader
    let reader_name = &reader_list[0];
    let reader_display = unsafe {
        CStr::from_ptr(reader_name.as_ptr()).to_string_lossy()
    };
    
    println!("Using reader: {}", reader_display);
    println!("Place your card on the reader and press Enter...");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    // Connect to the card
    let card = ctx.connect(reader_name, ShareMode::Shared, Protocols::ANY)?;
    println!("Connected to card successfully\n");

    // Main menu loop
    loop {
        println!("\n======= NDEF Explorer Menu =======");
        println!("1. Select NDEF Application");
        println!("2. Read NDEF Capability Container (CC)");
        println!("3. Read NDEF Message Length");
        println!("4. Read NDEF Message");
        println!("5. Write Sample NDEF Message");
        println!("6. Scan Memory");
        println!("7. Send Raw APDU Command");
        println!("8. Import Card Data for Writing");
        println!("9. Exit");
        print!("\nChoose an option (1-9): ");
        io::stdout().flush()?;
        
        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;
        
        match choice.trim() {
            "1" => select_ndef_application(&card),
            "2" => read_capability_container(&card),
            "3" => read_ndef_length(&card),
            "4" => read_ndef_message_enhanced(&card),
            "5" => write_ndef_message(&card),
            "6" => scan_readable_memory(&card),
            "7" => send_raw_command(&card),
            "8" => import_card_data_menu(&card)?,
            "9" => break,
            _ => println!("Invalid option, please try again."),
        }
    }

    Ok(())
}

// Function to detect if card is MIFARE Classic
fn is_mifare_classic(card: &pcsc::Card) -> bool {
    // Try MIFARE specific command (Get UID)
    let get_uid = [0xFF, 0xCA, 0x00, 0x00, 0x00];
    if let Some(response) = send_apdu(card, &get_uid, "Get UID") {
        println!("Card UID: {}", hex_string(&response));
        println!("Detected MIFARE Classic card");
        return true;
    }
    
    return false;
}

// Load key and authenticate with MIFARE Classic sector
fn authenticate_mifare_sector(card: &pcsc::Card, sector: u8) -> bool {
    // Common MIFARE Classic keys to try
    let keys = [
        [0xD3, 0xF7, 0xD3, 0xF7, 0xD3, 0xF7], // NDEF key
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], // Default factory key
        [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5], // NDEF MAD key
        [0x00, 0x00, 0x00, 0x00, 0x00, 0x00]  // All zeros
    ];
    
    // Calculate first block of sector
    let block = if sector == 0 { 0 } else { sector * 4 };
    
    // Try each key
    for key in &keys {
        // Load key command
        let mut load_key = vec![0xFF, 0x82, 0x00, 0x00, 0x06];
        load_key.extend_from_slice(key);
        
        if let Some(_) = send_apdu(card, &load_key, "Load Key") {
            // Authentication command (key A)
            let auth_cmd = [0xFF, 0x88, 0x00, block, 0x60, 0x00];
            
            if let Some(_) = send_apdu(card, &auth_cmd, &format!("Auth S{}", sector)) {
                println!("Authentication successful for sector {}", sector);
                return true;
            }
        }
    }
    
    println!("Authentication failed for sector {}", sector);
    false
}

// Read MIFARE Classic NDEF data
fn read_mifare_classic_ndef(card: &pcsc::Card) -> Option<Vec<u8>> {
    println!("Reading MIFARE Classic NDEF data...");
    
    // First check MAD in sector 0 to find NDEF location
    if authenticate_mifare_sector(card, 0) {
        // Read MAD in block 1
        let read_mad = [0xFF, 0xB0, 0x00, 0x01, 0x10];
        if let Some(mad_data) = send_apdu(card, &read_mad, "Read MAD") {
            println!("MAD data: {}", hex_string(&mad_data));
            
            // Check for NDEF application code (03E1)
            // Usually stored in sector 1
            let sector_to_check = 1;
            
            if authenticate_mifare_sector(card, sector_to_check) {
                // Read block 4 (first data block in sector 1)
                let read_block = [0xFF, 0xB0, 0x00, 0x04, 0x10];
                if let Some(block_data) = send_apdu(card, &read_block, "Read Block 4") {
                    println!("Block 4 data: {}", hex_string(&block_data));
                    
                    // Check if this contains NDEF TLV
                    if block_data.len() >= 2 && block_data[0] == 0x03 {
                        let ndef_length = block_data[1] as usize;
                        println!("Found NDEF TLV with length: {} bytes", ndef_length);
                        
                        // Extract NDEF message
                        if block_data.len() >= 2 + ndef_length {
                            let ndef_data = block_data[2..(2 + ndef_length)].to_vec();
                            return Some(ndef_data);
                        } else {
                            // Need to read more blocks
                            let mut ndef_data = block_data[2..].to_vec();
                            let remaining = ndef_length - (block_data.len() - 2);
                            
                            // Read additional blocks as needed
                            let blocks_to_read = (remaining + 15) / 16;
                            for i in 1..=blocks_to_read {
                                if 4 + i >= 7 { // Skip sector trailer (block 7)
                                    continue;
                                }
                                let read_next = [0xFF, 0xB0, 0x00, (4 + i) as u8, 0x10];
                                if let Some(next_data) = send_apdu(card, &read_next, &format!("Read Block {}", 4 + i)) {
                                    let bytes_to_take = std::cmp::min(remaining, 16);
                                    ndef_data.extend_from_slice(&next_data[0..bytes_to_take]);
                                } else {
                                    break;
                                }
                            }
                            
                            return Some(ndef_data);
                        }
                    }
                }
            }
        }
    }
    
    None
}

// Interpret NDEF records
fn interpret_ndef_record(data: &[u8]) {
    if data.len() < 3 {
        println!("NDEF data too short to interpret");
        return;
    }
    
    // Parse NDEF header
    let header = data[0];
    let type_length = data[1] as usize;
    let payload_length = data[2] as usize;
    
    println!("NDEF Header: 0x{:02X}", header);
    println!("Type Length: {}", type_length);
    println!("Payload Length: {}", payload_length);
    
    // Check if valid NDEF
    if data.len() < 3 + type_length + payload_length {
        println!("NDEF data incomplete");
        return;
    }
    
    // Get record type
    let record_type = &data[3..(3 + type_length)];
    
    // For Text records
    if type_length == 1 && record_type[0] == b'T' {
        let payload = &data[3 + type_length..(3 + type_length + payload_length)];
        
        if payload.len() > 0 {
            let status = payload[0];
            let lang_length = status & 0x3F;
            
            if payload.len() >= 1 + lang_length as usize {
                let lang = &payload[1..(1 + lang_length as usize)];
                let lang_str = String::from_utf8_lossy(lang);
                
                let text = &payload[(1 + lang_length as usize)..];
                let text_str = String::from_utf8_lossy(text);
                
                println!("Language: {}", lang_str);
                println!("\n========== DECODED TEXT MESSAGE ==========");
                println!("  Text: {}", text_str);
                println!("==========================================\n");
            }
        }
    } else {
        println!("Unknown or unsupported NDEF record type");
    }
}

// Enhanced read_ndef_message function that handles both standard and MIFARE Classic cards
fn read_ndef_message_enhanced(card: &pcsc::Card) {
    println!("\nReading full NDEF Message...");
    
    // First try to detect if this is a MIFARE Classic card
    if is_mifare_classic(card) {
        println!("Detected MIFARE Classic card, using special read mode...");
        if let Some(ndef_data) = read_mifare_classic_ndef(card) {
            println!("Successfully read NDEF message from MIFARE Classic");
            
            // Interpret the NDEF record
            interpret_ndef_record(&ndef_data);
        } else {
            println!("Could not find NDEF data on this MIFARE Classic card");
        }
    } else {
        // Original standard NDEF card handling
        println!("First, we need to locate the NDEF data...");
        
        // Try different offsets for the NDEF length
        let offsets = [0x0F, 0x00, 0x03, 0x04, 0x10];
        
        for &offset in &offsets {
            println!("\nTrying offset 0x{:02X} for NDEF length...", offset);
            
            let cmd = [0x00, 0xB0, 0x00, offset, 0x02];
            if let Some(length_data) = send_apdu(card, &cmd, &format!("Read NDEF Length at offset 0x{:02X}", offset)) {
                if length_data.len() >= 2 {
                    let length = ((length_data[0] as u16) << 8) | (length_data[1] as u16);
                    println!("Found NDEF length: {} bytes at offset 0x{:02X}", length, offset);
                    
                    // Now read the NDEF message
                    if length > 0 {
                        let data_offset = offset + 2;
                        let mut data_cmd = [0x00, 0xB0, 0x00, data_offset, length as u8];
                        
                        // For large messages, adjust Le
                        if length > 255 {
                            data_cmd[4] = 0x00; // Use extended Le format
                        }
                        
                        if let Some(ndef_data) = send_apdu(card, &data_cmd, "Read NDEF Data") {
                            println!("NDEF Message: {}", hex_string(&ndef_data));
                            read_ndef_message(card); // Call the original function for further interpretation
                            return;
                        }
                    }
                }
            }
        }
        
        println!("Could not find NDEF message length at common offsets.");
    }
}

fn read_capability_container(card: &pcsc::Card) {
    println!("\nReading Capability Container...");
    if let Some(cc_data) = send_apdu(card, &[0x00, 0xB0, 0x00, 0x00, 0x0F], "Read CC (15 bytes)") {
        println!("CC Data: {}", hex_string(&cc_data));
        
        if !cc_data.is_empty() {
            parse_capability_container(&cc_data);
        }
    }
}

fn read_ndef_length(card: &pcsc::Card) {
    println!("\nReading NDEF Message Length...");
    
    // First check if this is a MIFARE Classic card
    if is_mifare_classic(card) {
        println!("Detected MIFARE Classic card, using special read mode...");
        if authenticate_mifare_sector(card, 1) {
            // Read block 4 (first data block in sector 1) to check for TLV
            let read_block = [0xFF, 0xB0, 0x00, 0x04, 0x10];
            if let Some(block_data) = send_apdu(card, &read_block, "Read Block 4") {
                // Check if this contains NDEF TLV
                if block_data.len() >= 2 && block_data[0] == 0x03 {
                    let ndef_length = block_data[1] as usize;
                    println!("NDEF Message Length: {} bytes", ndef_length);
                    return;
                }
            }
        }
        println!("Could not find NDEF message length on MIFARE Classic card.");
    } else {
        // Standard NDEF card
        if let Some(length_data) = send_apdu(card, &[0x00, 0xB0, 0x00, 0x0F, 0x02], "Read NDEF Length") {
            if length_data.len() >= 2 {
                let length = ((length_data[0] as u16) << 8) | (length_data[1] as u16);
                println!("NDEF Message Length: {} bytes", length);
            }
        }
    }
}

// Function to handle importing card data via menu
fn import_card_data_menu(card: &pcsc::Card) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Import Card Data ===");
    println!("1. Import from file");
    println!("2. Paste JSON data");
    println!("3. Write custom text");
    println!("4. Cancel");
    print!("\nChoose option (1-4): ");
    io::stdout().flush()?;
    
    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;
    
    match choice.trim() {
        "1" => {
            print!("Enter filename: ");
            io::stdout().flush()?;
            let mut filename = String::new();
            io::stdin().read_line(&mut filename)?;
            let filename = filename.trim();
            
            if let Ok(json_data) = fs::read_to_string(filename) {
                // Select NDEF application first
                select_ndef_application(card);
                
                // Parse and write card data
                write_imported_card_data(card, &json_data)?;
            } else {
                println!("Error: Could not read file.");
            }
        },
        "2" => {
            println!("Paste JSON data below (press Enter, then Ctrl+D when finished):");
            let mut json_data = String::new();
            let stdin = io::stdin();
            let mut lines = stdin.lock().lines();
            
            while let Some(line) = lines.next() {
                match line {
                    Ok(line) => json_data.push_str(&format!("{}\n", line)),
                    Err(_) => break,
                }
            }
            
            // Select NDEF application first
            select_ndef_application(card);
            
            // Parse and write card data
            if !json_data.is_empty() {
                write_imported_card_data(card, &json_data)?;
            } else {
                println!("Error: No data provided.");
            }
        },
        "3" => {
            print!("Enter text to write: ");
            io::stdout().flush()?;
            let mut text = String::new();
            io::stdin().read_line(&mut text)?;
            let text = text.trim();
            
            if !text.is_empty() {
                // Select NDEF application first
                select_ndef_application(card);
                
                // Write the custom text
                write_text_ndef_message(card, text)?;
            } else {
                println!("Error: No text provided.");
            }
        },
        "4" | _ => {
            println!("Import canceled.");
        }
    }
    
    Ok(())
}

// Function to handle importing card data from a file (for command-line usage)
fn import_and_write_card_data(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("NDEF Card Writer");
    println!("===============");
    
    // Read the JSON data from file
    let json_data = fs::read_to_string(filename)?;
    println!("Loaded data from file: {}", filename);
    
    // Connect to card reader
    println!("\nConnecting to card reader...");
    let ctx = Context::establish(Scope::User)?;
    
    // List available readers
    let mut reader_buffer = [0; 2048];
    let readers = ctx.list_readers(&mut reader_buffer)?;
    
    let mut reader_list = Vec::new();
    for reader in readers {
        reader_list.push(reader.to_owned());
    }
    
    if reader_list.is_empty() {
        return Err("No card readers found.".into());
    }
    
    // Select the first reader
    let reader_name = &reader_list[0];
    let reader_display = unsafe {
        CStr::from_ptr(reader_name.as_ptr()).to_string_lossy()
    };
    
    println!("Using reader: {}", reader_display);
    println!("Please place a writable NFC card on the reader and press Enter...");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    // Connect to the card
    let card = ctx.connect(reader_name, ShareMode::Shared, Protocols::ANY)?;
    println!("Connected to card successfully");
    
    // Select NDEF application
    select_ndef_application(&card);
    
    // Write the data
    write_imported_card_data(&card, &json_data)?;
    
    println!("\n✅ Card successfully written!");
    
    Ok(())
}

// Function to import card data from stdin (for command-line usage)
fn import_and_write_card_data_from_stdin() -> Result<(), Box<dyn std::error::Error>> {
    println!("NDEF Card Writer");
    println!("===============");
    println!("Paste the exported JSON data (press Enter, then Ctrl+D when finished):");
    
    // Read JSON data from stdin
    let mut json_data = String::new();
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    
    while let Some(line) = lines.next() {
        match line {
            Ok(line) => json_data.push_str(&format!("{}\n", line)),
            Err(_) => break,
        }
    }
    
    if json_data.is_empty() {
        return Err("No input data provided.".into());
    }
    
    // Connect to card reader
    println!("\nConnecting to card reader...");
    let ctx = Context::establish(Scope::User)?;
    
    // List available readers
    let mut reader_buffer = [0; 2048];
    let readers = ctx.list_readers(&mut reader_buffer)?;
    
    let mut reader_list = Vec::new();
    for reader in readers {
        reader_list.push(reader.to_owned());
    }
    
    if reader_list.is_empty() {
        return Err("No card readers found.".into());
    }
    
    // Select the first reader
    let reader_name = &reader_list[0];
    let reader_display = unsafe {
        CStr::from_ptr(reader_name.as_ptr()).to_string_lossy()
    };
    
    println!("Using reader: {}", reader_display);
    println!("Please place a writable NFC card on the reader and press Enter...");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    // Connect to the card
    let card = ctx.connect(reader_name, ShareMode::Shared, Protocols::ANY)?;
    println!("Connected to card successfully");
    
    // Select NDEF application
    select_ndef_application(&card);
    
    // Write the data
    write_imported_card_data(&card, &json_data)?;
    
    println!("\n✅ Card successfully written!");
    
    Ok(())
}
