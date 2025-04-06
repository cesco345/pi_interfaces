// Functions related to card connection and communication

use std::error::Error;
use pcsc::{Card, Context, Scope, ShareMode, Protocols};
use crate::commands::ndef_commands::send_apdu;
use crate::util::ndef_util::hex_string;

/// Connect to a card reader and establish communication with the card
pub fn connect_to_card() -> Result<(Context, Card), Box<dyn Error>> {
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

/// Get card UID and additional details
pub fn get_card_details(card: &Card) -> Result<Vec<u8>, Box<dyn Error>> {
    println!("Getting card details...");
    
    // Get card UID
    let get_uid = [0xFF, 0xCA, 0x00, 0x00, 0x00];
    
    if let Some(response) = send_apdu(card, &get_uid, "Get UID") {
        println!("Card UID: {}", hex_string(&response));
        
        // Try to get additional card info
        let get_ats = [0xFF, 0xCA, 0x01, 0x00, 0x00];
        if let Some(ats_response) = send_apdu(card, &get_ats, "Get ATS") {
            println!("Card ATS (Answer To Select): {}", hex_string(&ats_response));
        } else {
            println!("ATS not available");
        }
        
        // Try to get historical bytes
        let get_historical = [0xFF, 0xCA, 0x03, 0x00, 0x00];
        if let Some(historical_response) = send_apdu(card, &get_historical, "Get Historical Bytes") {
            println!("Historical Bytes: {}", hex_string(&historical_response));
        }
        
        return Ok(response);
    } else {
        println!("Could not get card UID. Card may not support this command.");
        // Return empty vector as fallback
        return Ok(Vec::new());
    }
}
