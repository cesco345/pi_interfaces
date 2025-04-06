// src/bin/mifare_reader.rs
// Main entry point for the MIFARE reader application

use std::io::{self, Write};
use std::error::Error;
use std::collections::HashMap;

// Import the needed modules from our crate
use ndef_explorer::operations::mifare::{
    connect_to_card,
    get_card_details,
    detect_card_type,
    read_mifare_classic_data,
    read_type2_tag_data,
    read_desfire_basic_info,
    attempt_generic_read,
    display_summary,
    BlockData
};

fn main() -> Result<(), Box<dyn Error>> {
    println!("MIFARE Classic Card Reader");
    println!("========================\n");
    
    // Connect to the card
    println!("Place your MIFARE card on the reader and press Enter to continue...");
    io::stdout().flush()?;
    let mut _wait = String::new();
    io::stdin().read_line(&mut _wait)?;
    
    // Connect to card
    let (_ctx, card) = connect_to_card()?;
    
    // Get card UID and details
    let uid = get_card_details(&card)?;
    
    // Detect card type based on UID and try appropriate reading method
    let card_type = detect_card_type(&uid);
    println!("\nDetected card type: {}", card_type);
    
    // Store all block data for the summary
    let mut all_blocks: Vec<BlockData> = Vec::new();
    let mut sector_access_map: HashMap<u8, bool> = HashMap::new();
    
    match card_type.as_str() {
        "MIFARE Classic" => {
            // Read the card and collect all block data
            all_blocks = read_mifare_classic_data(&card, &mut sector_access_map)?;
        },
        "MIFARE Ultralight" | "NTAG21x" => {
            println!("This appears to be a {} card, not a MIFARE Classic card.", card_type);
            println!("Use 'read_ntag' tool for better results with this card type.");
            
            // Still try to read some pages as a fallback
            all_blocks = read_type2_tag_data(&card)?;
        },
        "MIFARE DESFire" => {
            println!("This appears to be a MIFARE DESFire card, not a MIFARE Classic card.");
            println!("Use 'focused_ndef_reader' tool for better results with this card type.");
            
            // Try basic DESFire operations as fallback
            read_desfire_basic_info(&card)?;
        },
        _ => {
            println!("Unknown card type. Attempting generic read operations...");
            // Try basic read operations
            all_blocks = attempt_generic_read(&card)?;
        }
    }
    
    // Display the summary of all read data
    if !all_blocks.is_empty() {
        display_summary(&all_blocks, &sector_access_map, &card_type);
    }
    
    println!("\nPlease remove the card from the reader when finished.");
    
    Ok(())
}
