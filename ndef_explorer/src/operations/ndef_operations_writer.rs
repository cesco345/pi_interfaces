// File: src/operations/ndef_operations_writer.rs
// Functions for writing NDEF messages
use crate::commands::ndef_commands::send_apdu;
use crate::util::ndef_util::hex_string;
use serde::{Deserialize, Serialize};

// Card data structure for import/export with Android app
#[derive(Debug, Deserialize, Serialize)]
pub struct CardExport {
    pub id: String,
    pub name: String,
    pub applicationId: i32,
    pub fileId: i32,
    pub fileData: String,
    pub format: String,
    pub exportDate: String,
}

// Write a sample NDEF message to the card
pub fn write_ndef_message(card: &pcsc::Card) {
    // Keep existing functionality for backward compatibility
    // But delegate to our more flexible function
    let text = "Hello from NDEF Explorer";
    if let Err(e) = write_text_ndef_message(card, text) {
        println!("Error writing NDEF message: {}", e);
    }
}

// Write a sample NDEF message with custom data to the card
pub fn write_ndef_message_with_data(card: &pcsc::Card, data: &str) -> Result<(), Box<dyn std::error::Error>> {
    write_text_ndef_message(card, data)
}

// Create a NDEF text record with custom text
pub fn create_ndef_text_record(text: &str) -> Vec<u8> {
    // Language code (en)
    let language_code = [0x65, 0x6E];
    
    // Convert text to bytes
    let text_bytes = text.as_bytes();
    
    // Calculate payload size (status byte + language code + text)
    let payload_length = 1 + language_code.len() + text_bytes.len();
    
    // Create NDEF record header
    let mut ndef_record = vec![
        0xD1,                         // TNF=1 (Well Known), MB=1, ME=1, SR=1
        0x01,                         // Type Length = 1
        payload_length as u8,         // Payload Length
        0x54                          // Type = 'T' (Text)
    ];
    
    // Create status byte (UTF-8 encoding + language code length)
    let status_byte = 0x02; // UTF-8 + 2-byte language code
    
    // Add status byte
    ndef_record.push(status_byte);
    
    // Add language code
    ndef_record.extend_from_slice(&language_code);
    
    // Add text bytes
    ndef_record.extend_from_slice(text_bytes);
    
    ndef_record
}

// Format card for NDEF
pub fn format_card_for_ndef(card: &pcsc::Card) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nDetected card is not NDEF formatted.");
    println!("Attempting to format card for NDEF...");
    
    // Select card's master file
    println!("Selecting card master file...");
    let select_mf_apdu = [0x00, 0xA4, 0x00, 0x00, 0x02, 0x3F, 0x00];
    if let Some(_) = send_apdu(card, &select_mf_apdu, "Select MF") {
        println!("Successfully selected master file");
    } else {
        println!("Failed to select master file, continuing anyway...");
    }
    
    // Create NDEF application
    println!("Creating NDEF application...");
    
    // Try to select the NDEF application first to see if it exists
    let select_ndef_apdu = [0x00, 0xA4, 0x04, 0x00, 0x07, 0xD2, 0x76, 0x00, 0x00, 0x85, 0x01, 0x01];
    if let Some(_) = send_apdu(card, &select_ndef_apdu, "Select NDEF Application") {
        println!("NDEF application already exists, no need to create it");
    } else {
        println!("NDEF application not found, need to create it");
        // This part would typically involve creating the NDEF application, 
        // but many cards come pre-formatted with an NDEF application
    }
    
    // Create Capability Container (CC) file if it doesn't exist
    println!("Setting up Capability Container...");
    
    // Initialize CC with standard values
    // CC Length: 0x000F, Mapping Version: 0x20, Max Data Size: 0x0F0, Access: 0x00
    let init_cc_apdu = [0x00, 0xD6, 0x00, 0x03, 0x04, 0x00, 0x0F, 0x20, 0x00];
    if let Some(_) = send_apdu(card, &init_cc_apdu, "Initialize CC") {
        println!("Successfully initialized Capability Container");
    } else {
        println!("Failed to initialize CC, trying to continue...");
    }
    
    // Set NDEF file control
    println!("Setting up NDEF file control...");
    let ndef_fc_apdu = [0x00, 0xD6, 0x00, 0x07, 0x08, 0x00, 0xFF, 0x54, 0x01, 0x0F, 0x54, 0x00, 0x00];
    if let Some(_) = send_apdu(card, &ndef_fc_apdu, "Set NDEF File Control") {
        println!("Successfully set up NDEF file control");
    } else {
        println!("Failed to set NDEF file control, trying to continue...");
    }
    
    println!("Card formatting completed!");
    Ok(())
}

// Check if card is NDEF formatted
pub fn is_ndef_formatted(card: &pcsc::Card) -> bool {
    println!("Checking if card is NDEF formatted...");
    
    // Try to read NDEF capability container
    let read_cc_apdu = [0x00, 0xB0, 0x00, 0x03, 0x04];
    if let Some(response) = send_apdu(card, &read_cc_apdu, "Read CC") {
        if response.len() >= 4 {
            println!("Card is NDEF formatted (CC found)");
            return true;
        }
    }
    
    println!("Card is not NDEF formatted (no valid CC found)");
    false
}

// Write a text NDEF message to the card
pub fn write_text_ndef_message(card: &pcsc::Card, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nWriting NDEF Text Message: \"{}\"", text);
    
    // Create the NDEF message
    let ndef_message = create_ndef_text_record(text);
    
    println!("Prepared NDEF message: {}", hex_string(&ndef_message));
    
    // First, check if the card is NDEF formatted
    if !is_ndef_formatted(card) {
        // Try to format the card
        format_card_for_ndef(card)?;
    }
    
    // Write NDEF message length (2 bytes, big-endian)
    let length_bytes = [(ndef_message.len() >> 8) as u8, ndef_message.len() as u8];
    println!("Writing NDEF length: {} bytes", ndef_message.len());
    
    let mut length_apdu = vec![0x00, 0xD6, 0x00, 0x0F, 0x02];
    length_apdu.extend_from_slice(&length_bytes);
    
    if let Some(_) = send_apdu(card, &length_apdu, "Write NDEF Length") {
        println!("Successfully wrote NDEF message length");
        
        // Write NDEF message content
        println!("Writing NDEF message content...");
        
        // Handle potential large messages by chunking
        let max_chunk_size = 255; // Maximum data size in a single APDU
        let mut offset = 0x11; // Start at offset 0x11 for NDEF data
        
        for (i, chunk) in ndef_message.chunks(max_chunk_size).enumerate() {
            let chunk_len = chunk.len();
            println!("Writing chunk {} ({} bytes) at offset 0x{:04X}...", i+1, chunk_len, offset);
            
            let mut message_apdu = vec![0x00, 0xD6, (offset >> 8) as u8, (offset & 0xFF) as u8, chunk_len as u8];
            message_apdu.extend_from_slice(chunk);
            
            if let Some(_) = send_apdu(card, &message_apdu, &format!("Write NDEF Chunk {}", i+1)) {
                println!("Successfully wrote NDEF chunk {}", i+1);
            } else {
                println!("Failed to write NDEF chunk {}", i+1);
                return Err("Failed to write NDEF message".into());
            }
            
            offset += chunk_len as u16;
        }
        
        println!("Successfully wrote complete NDEF message");
    } else {
        println!("Failed to write NDEF message length, trying alternative approach...");
        
        // Try an alternative approach for cards that might need different commands
        let alt_length_apdu = vec![0x00, 0xA4, 0x04, 0x00, 0x07, 0xD2, 0x76, 0x00, 0x00, 0x85, 0x01, 0x01];
        if let Some(_) = send_apdu(card, &alt_length_apdu, "Select NDEF Application (Alt)") {
            println!("Selected NDEF application with alternative method");
            
            // Now try writing the data again
            let mut alt_data_apdu = vec![0x00, 0xD6, 0x00, 0x00, 0x00];
            alt_data_apdu[4] = (ndef_message.len() + 2) as u8; // Data length + 2 bytes for length field
            alt_data_apdu.push((ndef_message.len() >> 8) as u8);
            alt_data_apdu.push((ndef_message.len() & 0xFF) as u8);
            alt_data_apdu.extend_from_slice(&ndef_message);
            
            if let Some(_) = send_apdu(card, &alt_data_apdu, "Write NDEF Data (Alt)") {
                println!("Successfully wrote NDEF data with alternative method");
                return Ok(());
            } else {
                return Err("Failed to write NDEF message with alternative method".into());
            }
        } else {
            return Err("Failed to write NDEF message length".into());
        }
    }
    
    Ok(())
}

// Parse and write imported card data
pub fn write_imported_card_data(card: &pcsc::Card, json_data: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Parse JSON
    match serde_json::from_str::<CardExport>(json_data) {
        Ok(card_data) => {
            println!("\nCard Information:");
            println!("  Name: {}", card_data.name);
            println!("  Application ID: {}", card_data.applicationId);
            println!("  File ID: {}", card_data.fileId);
            println!("  Data: {}", card_data.fileData);
            println!("  Export Date: {}", card_data.exportDate);
            
            // Write the data
            write_text_ndef_message(card, &card_data.fileData)?;
            
            println!("\n✅ Card data successfully written!");
        },
        Err(e) => {
            println!("Error parsing JSON data: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}

// New convenience function to make it easier to write card data
pub fn write_ndef_to_card(card_data: &CardExport) -> Result<(), Box<dyn std::error::Error>> {
    println!("Setting up connection to card...");
    
    // Establish a PC/SC context
    let ctx = pcsc::Context::establish(pcsc::Scope::User)?;
    
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
    let card = ctx.connect(reader, pcsc::ShareMode::Shared, pcsc::Protocols::ANY)?;
    println!("Successfully connected to card");
    
    // Use the existing function to write the data
    write_text_ndef_message(&card, &card_data.fileData)?;
    
    Ok(())
}
