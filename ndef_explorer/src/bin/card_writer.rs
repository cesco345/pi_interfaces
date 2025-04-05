// src/bin/card_writer.rs
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::error::Error;
use std::convert::TryInto;

use ndef_explorer::card_handling::card_type_handler::{write_card_data, CardWriteStrategy};
use ndef_explorer::operations::ndef_operations_writer::{self, CardExport, write_text_ndef_message};
use ndef_explorer::commands::ndef_commands::send_apdu;
use ndef_explorer::util::ndef_util::hex_string;

// Additional imports for enhanced card handling
use pcsc::{Card, Context, Scope, ShareMode, Protocols};

// Main function handling command line args and high-level flow
fn main() -> Result<(), Box<dyn Error>> {
    println!("Smart Card Data Writer");
    println!("=====================\n");
    
    // Load data from file or stdin
    let json_data = load_input_data()?;
    
    // Parse JSON data
    let card_data: CardExport = match serde_json::from_str(&json_data) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error parsing JSON data: {}", e);
            return Err(e.into());
        }
    };
    
    // Display card information
    display_card_info(&card_data);
    
    // Analyze the card data to determine the best approach
    let mut strategy = CardWriteStrategy::NdefFormatted;
    
    // Check format field to determine strategy
    if card_data.format.to_lowercase() == "ntag_213" {
        strategy = CardWriteStrategy::NtagDirect;
    }
    
    println!("\nSelected writing strategy: {:?}", strategy);
    
    // Ask for confirmation
    if !confirm_operation("Continue with writing?") {
        println!("Operation cancelled.");
        return Ok(());
    }
    
    // Connect to card reader
    println!("\nConnecting to card reader...");
    let (_, card) = connect_to_card()?;
    
    // Identify card type and adjust strategy if needed
    let strategy = handle_card_type(&card, &card_data, strategy)?;
    
    // Execute the selected strategy
    execute_write_strategy(&card, &card_data, strategy)?;
    
    println!("\nPlease remove the card from the reader when finished.");
    
    Ok(())
}

// Load data from file or stdin
fn load_input_data() -> Result<String, Box<dyn Error>> {
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
    
    Ok(json_data)
}

// Display card information
fn display_card_info(card_data: &CardExport) {
    println!("\nCard Information:");
    println!("  Name: {}", card_data.name);
    println!("  Application ID: {}", card_data.applicationId);
    println!("  File ID: {}", card_data.fileId);
    println!("  Data: {}", card_data.fileData);
    println!("  Format: {}", card_data.format);
    println!("  Export Date: {}", card_data.exportDate);
}

// Confirm operation with user
fn confirm_operation(prompt: &str) -> bool {
    print!("\n{} (y/n): ", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    
    input.trim().to_lowercase() == "y"
}

// Connect to card reader
fn connect_to_card() -> Result<(Context, Card), Box<dyn Error>> {
    println!("\nPlace your card on the reader and press Enter to continue...");
    io::stdout().flush()?;
    let mut _wait = String::new();
    io::stdin().read_line(&mut _wait)?;
    
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

// Handle card type detection and strategy adjustment
fn handle_card_type(card: &Card, _card_data: &CardExport, initial_strategy: CardWriteStrategy) 
    -> Result<CardWriteStrategy, Box<dyn Error>> {
    
    // Identify card type
    let card_type = identify_card_type(card)?;
    println!("Detected card type: {}", card_type);
    
    // Based on card type, adjust strategy if needed
    let strategy = match card_type.as_str() {
        "MIFARE Classic" => CardWriteStrategy::MifareClassicDirect,
        "MIFARE Ultralight" => CardWriteStrategy::MifareUltralight,
        "DESFire" => CardWriteStrategy::DESFireNdefSetup,
        "NTAG213" => CardWriteStrategy::NtagDirect,
        _ => initial_strategy // Keep original strategy for other types
    };
    
    println!("Adjusted writing strategy to: {:?}", strategy);
    
    Ok(strategy)
}

// Identify card type by checking ATR and executing test commands
fn identify_card_type(card: &Card) -> Result<String, Box<dyn Error>> {
    // Get the ATR (Answer To Reset)
    println!("Retrieving card ATR...");
    
    // Using get_attribute with buffer for version of pcsc
    let mut atr_buffer = [0; 64]; // Buffer for ATR
    let atr = card.get_attribute(pcsc::Attribute::AtrString, &mut atr_buffer)?;
    let atr_hex = atr.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
    println!("Card ATR: {}", atr_hex);
    
    // Try to identify based on ATR
    if atr_hex.starts_with("3B 8F 80 01 80 4F 0C A0 00 00 03 06") {
        return Ok("DESFire".to_string());
    } else if atr_hex.starts_with("3B 8F") {
        // Test for MIFARE Classic by attempting a MIFARE-specific command
        let mifare_test = [0xFF, 0xCA, 0x00, 0x00, 0x00]; // GetUID command for MIFARE
        if let Some(_) = send_apdu(card, &mifare_test, "MIFARE Test") {
            return Ok("MIFARE Classic".to_string());
        }
    } else if atr_hex.starts_with("3B 8C 80 01 80 31") {
        return Ok("MIFARE Ultralight".to_string());
    }
    
    // Try to detect NTAG213 specifically
    if verify_ntag213(card).is_ok() {
        return Ok("NTAG213".to_string());
    }
    
    // Test for NDEF compatibility
    let ndef_test = [0x00, 0xA4, 0x04, 0x00, 0x07, 0xD2, 0x76, 0x00, 0x00, 0x85, 0x01, 0x01];
    if let Some(_) = send_apdu(card, &ndef_test, "NDEF App Select Test") {
        return Ok("NDEF Compatible".to_string());
    }
    
    // If we couldn't clearly identify, return generic
    Ok("Unknown ISO14443".to_string())
}

// Verify if card is NTAG213
fn verify_ntag213(card: &Card) -> Result<(), Box<dyn Error>> {
    println!("Checking for NTAG213 compatibility...");
    
    // Get card UID (this command works with NTAG213)
    let get_uid = [0xFF, 0xCA, 0x00, 0x00, 0x00];
    
    if let Some(_response) = send_apdu(card, &get_uid, "Get UID") {
        // Check the SAK and ATQA to verify it's an NTAG213
        let get_sak_atqa = [0xFF, 0xCA, 0x01, 0x00, 0x00];
        if let Some(sak_atqa) = send_apdu(card, &get_sak_atqa, "Get SAK/ATQA") {
            if sak_atqa.len() >= 3 && sak_atqa[0] == 0x00 {
                // Try to read page 3 (CC) to verify it's an NTAG
                let read_page3 = [0xFF, 0xB0, 0x00, 0x03, 0x04];
                if let Some(_) = send_apdu(card, &read_page3, "Read Page 3 (CC)") {
                    return Ok(());
                }
            }
        }
        
        // Alternative method: try to read specific NTAG213 pages
        let read_page0 = [0xFF, 0xB0, 0x00, 0x00, 0x04];
        if let Some(_) = send_apdu(card, &read_page0, "Read Page 0") {
            println!("Card verified as NTAG21x based on successful page read");
            return Ok(());
        }
    }
    
    Err("Card doesn't appear to be an NTAG213".into())
}

// Parse data string to bytes
fn parse_data_to_bytes(data_str: &str) -> Vec<u8> {
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
    
    data_bytes
}

// Execute the selected write strategy
fn execute_write_strategy(card: &Card, card_data: &CardExport, strategy: CardWriteStrategy) 
    -> Result<(), Box<dyn Error>> {
    
    match strategy {
        CardWriteStrategy::MifareClassicDirect => {
            println!("\nUsing MIFARE Classic direct writing approach...");
            write_to_mifare_classic(card, card_data)
        },
        CardWriteStrategy::MifareUltralight => {
            println!("\nUsing MIFARE Ultralight writing approach...");
            write_to_mifare_ultralight(card, card_data)
        },
        CardWriteStrategy::NtagDirect => {
            println!("\nUsing NTAG213 direct writing approach...");
            write_to_ntag213(card, card_data)
        },
        CardWriteStrategy::DESFireNdefSetup => {
            // Show DESFire setup instructions first
            match write_card_data(card_data, strategy) {
                Ok(true) => {
                    if confirm_operation("Would you like to attempt automatic DESFire setup?") {
                        setup_desfire_card(card, card_data)
                    } else {
                        println!("Please follow the DESFire setup instructions shown above manually.");
                        Ok(())
                    }
                },
                _ => Err("Failed to get DESFire setup instructions".into()),
            }
        },
        _ => {
            // For other strategies like NdefFormatted, attempt standard NDEF
            println!("\nUsing standard NDEF write approach...");
            
            // Try to write using ndef_operations_writer
            match write_text_ndef_message(card, &card_data.fileData) {
                Ok(()) => {
                    println!("\n✓ Card write operation completed successfully!");
                    Ok(())
                },
                Err(_) => {
                    // If standard NDEF fails, try direct sector writing as fallback
                    println!("\nStandard NDEF write failed, attempting direct write as fallback...");
                    write_to_mifare_classic(card, card_data)
                }
            }
        }
    }
}

// Write data to MIFARE Classic card
fn write_to_mifare_classic(card: &Card, card_data: &CardExport) -> Result<(), Box<dyn Error>> {
    println!("\nWriting data to MIFARE Classic card...");
    
    // Authentication with key A for sector 1
    let auth_cmd = [0xFF, 0x86, 0x00, 0x00, 0x05, 0x01, 0x00, 0x01, 0x60, 0xFF];
    if let Some(_) = send_apdu(card, &auth_cmd, "Auth Sector 1") {
        println!("Authentication successful");
        
        // Convert data to bytes
        let mut data_bytes = parse_data_to_bytes(&card_data.fileData);
        
        // Pad to 16 bytes
        while data_bytes.len() < 16 {
            data_bytes.push(0x00);
        }
        
        // Truncate if too long
        if data_bytes.len() > 16 {
            data_bytes.truncate(16);
            println!("Note: Data truncated to 16 bytes to fit in one block");
        }
        
        // Create write command
        let mut write_cmd = vec![0xFF, 0xD6, 0x00, 0x04, 0x10]; // Write to block 4
        write_cmd.extend_from_slice(&data_bytes);
        
        if let Some(_) = send_apdu(card, &write_cmd, "Write Data") {
            println!("Successfully wrote data to card block 4");
            Ok(())
        } else {
            Err("Failed to write data to card".into())
        }
    } else {
        Err("Failed to authenticate with card".into())
    }
}

// Write data to MIFARE Ultralight card
fn write_to_mifare_ultralight(card: &Card, card_data: &CardExport) -> Result<(), Box<dyn Error>> {
    println!("\nWriting data to MIFARE Ultralight card...");
    
    // Convert data to bytes
    let data_bytes = parse_data_to_bytes(&card_data.fileData);
    
    // Write in 4-byte chunks to Ultralight pages
    // Start at page 4 to avoid overwriting OTP/lock bytes
    let mut page = 4;
    for chunk in data_bytes.chunks(4) {
        let mut chunk_data = chunk.to_vec();
        // Pad to 4 bytes
        while chunk_data.len() < 4 {
            chunk_data.push(0x00);
        }
        
        let mut write_cmd = vec![0xFF, 0xD6, 0x00, page, 0x04]; // Write to page
        write_cmd.extend_from_slice(&chunk_data);
        
        if let Some(_) = send_apdu(card, &write_cmd, &format!("Write Page {}", page)) {
            println!("Successfully wrote data to page {}", page);
        } else {
            return Err(format!("Failed to write data to page {}", page).into());
        }
        
        page += 1;
    }
    
    println!("Successfully wrote all data to Ultralight card");
    Ok(())
}

// Write data to NTAG213 tag
fn write_to_ntag213(card: &Card, card_data: &CardExport) -> Result<(), Box<dyn Error>> {
    println!("\nWriting data to NTAG213 tag...");
    
    // Convert data to bytes
    let mut data_bytes = parse_data_to_bytes(&card_data.fileData);
    
    // NTAG213 specific - we can only write to pages 4-39
    // Each page is 4 bytes
    let start_page = 4;
    let end_page = 39;
    let page_size = 4; // This is a usize
    
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
        
        // Fixed: Convert current_page from usize to u8
        let current_page_u8: u8 = current_page.try_into().unwrap_or_else(|_| {
            eprintln!("WARNING: Page number {} out of range for u8, clamping to 255", current_page);
            255 // Default to max u8 value if conversion fails
        });
        
        // Use the u8 version in the command
        let cmd_header: [u8; 5] = [0xFF, 0xD6, 0x00, current_page_u8, 0x04];
        let mut write_cmd: Vec<u8> = cmd_header.to_vec();
        write_cmd.extend_from_slice(chunk);
        
        if let Some(_) = send_apdu(card, &write_cmd, &format!("Write Page {}", current_page)) {
            println!("Successfully wrote to page {}", current_page);
            success_count += 1;
        } else {
            println!("Failed to write to page {}", current_page);
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

// Setup DESFire card for NDEF
fn setup_desfire_card(card: &Card, card_data: &CardExport) -> Result<(), Box<dyn Error>> {
    println!("\nSetting up DESFire card for NDEF...");
    
    // Select PICC
    println!("Selecting PICC...");
    let select_picc = [0x90, 0x5A, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00];
    if let Some(_) = send_apdu(card, &select_picc, "Select PICC") {
        println!("Successfully selected PICC");
    } else {
        println!("Failed to select PICC, trying alternative command...");
        
        // Try alternative selection method
        let alt_select = [0x00, 0xA4, 0x00, 0x00, 0x00];
        if let Some(_) = send_apdu(card, &alt_select, "Alt Select") {
            println!("Selected with alternative method");
        } else {
            println!("Warning: Selection failed, continuing anyway...");
        }
    }
    
    let result = setup_desfire_ndef_app(card, card_data)?;
    
    if result {
        println!("Successfully set up DESFire card for NDEF");
        Ok(())
    } else {
        Err("Failed to set up DESFire card".into())
    }
}

// Helper function to set up DESFire NDEF application
fn setup_desfire_ndef_app(card: &Card, card_data: &CardExport) -> Result<bool, Box<dyn Error>> {
    // Create NDEF application
    println!("Creating NDEF application...");
    let create_app = [0x90, 0xCA, 0x00, 0x00, 0x05, 0xD2, 0x76, 0x00, 0x00, 0x85, 0x00];
    
    if let Some(_) = send_apdu(card, &create_app, "Create NDEF App") {
        println!("Successfully created NDEF application");
    } else {
        println!("Failed to create NDEF application, it might already exist");
    }
    
    // Select NDEF application
    println!("Selecting NDEF application...");
    let select_app = [0x90, 0x5A, 0x00, 0x00, 0x03, 0xD2, 0x76, 0x00, 0x00];
    
    if let Some(_) = send_apdu(card, &select_app, "Select NDEF App") {
        println!("Successfully selected NDEF application");
    } else {
        return Err("Failed to select NDEF application".into());
    }
    
    // Create CC file
    println!("Creating Capability Container file...");
    let create_cc = [0x90, 0xCD, 0x00, 0x00, 0x09, 0x01, 0x00, 0x00, 0x00, 0x0F, 0x20, 0x00, 0x00, 0x00, 0x00];
    
    if let Some(_) = send_apdu(card, &create_cc, "Create CC") {
        println!("Successfully created CC file");
    } else {
        println!("Failed to create CC file, it might already exist");
    }
    
    // Create NDEF file
    println!("Creating NDEF file...");
    let create_ndef = [0x90, 0xCD, 0x00, 0x00, 0x09, 0x02, 0x00, 0x00, 0x00, 0xFF, 0x20, 0x00, 0x00, 0x00, 0x00];
    
    if let Some(_) = send_apdu(card, &create_ndef, "Create NDEF File") {
        println!("Successfully created NDEF file");
    } else {
        println!("Failed to create NDEF file, it might already exist");
    }
    
    // Write data to NDEF file
    println!("Writing data to NDEF file...");
    
    // Create NDEF message
    let text = &card_data.fileData;
    let ndef_message = ndef_operations_writer::create_ndef_text_record(text);
    
    // Write to file - using u8 array then converting to Vec<u8>
    let header: [u8; 4] = [0x90, 0x3D, 0x00, 0x00];
    let mut write_cmd: Vec<u8> = header.to_vec();
    
    write_cmd.push(ndef_message.len() as u8 + 2); // Length + file ID + offset
    write_cmd.push(0x02); // File ID
    write_cmd.push(0x00); // Offset (0)
    write_cmd.extend_from_slice(&ndef_message);
    write_cmd.push(0x00); // Le
    
    if let Some(_) = send_apdu(card, &write_cmd, "Write NDEF Data") {
        println!("Successfully wrote NDEF data to DESFire card");
        Ok(true)
    } else {
        Err("Failed to write NDEF data to DESFire card".into())
    }
}
