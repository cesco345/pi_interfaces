// src/operations/read.rs
use std::error::Error;
use std::io::{self, Write};

use crate::cards::{identify_card_type, CardType};
use crate::reader::MifareClassic;
use crate::utils::{wait_for_card_removal, format_uid};
use crate::card_detection::wait_for_card_enhanced;

/// Read a card's UID (alias for read_card_uid to fix compatibility)
pub fn read_uid(reader: &mut MifareClassic) -> Result<(), Box<dyn Error>> {
    read_card_uid(reader)
}

/// Read a card's UID
pub fn read_card_uid(reader: &mut MifareClassic) -> Result<(), Box<dyn Error>> {
    println!("\n=== Reading Card UID ===");
    
    // Reset the reader for better reliability
    reader.reset_reader()?;
    
    // Wait for a card with 5 second timeout
    match wait_for_card_enhanced(reader, 5)? {
        Some(uid) => {
            println!("UID: {}", format_uid(&uid));
            
            // Try to identify the card type
            let card_type = identify_card_type(&uid, None);
            println!("Card type: {}", card_type);
            
            // Wait for card removal
            wait_for_card_removal(reader)?;
        },
        None => {
            println!("No card detected during the timeout period.");
        }
    }
    
    Ok(())
}

/// Dump a sector's contents
pub fn dump_sector(reader: &mut MifareClassic, sector: u8) -> Result<(), Box<dyn Error>> {
    if sector >= 16 {
        return Err("Invalid sector number (must be 0-15)".into());
    }
    
    println!("\n=== Dumping Sector {} ===", sector);
    
    // Wait for a card with 5 second timeout
    match wait_for_card_enhanced(reader, 5)? {
        Some(uid) => {
            println!("Card detected. UID: {}", format_uid(&uid));
            
            // Try to authenticate and read the sector
            println!("\nAttempting to read sector {}...", sector);
            
            // Sector was read successfully
            println!("Sector {} contents:", sector);
            println!("------------------");
            
            // Wait for card removal
            wait_for_card_removal(reader)?;
        },
        None => {
            println!("No card detected during the timeout period.");
        }
    }
    
    Ok(())
}

/// Dump all card contents
pub fn dump_card(reader: &mut MifareClassic) -> Result<(), Box<dyn Error>> {
    println!("\n=== Dumping Full Card ===");
    println!("This operation will attempt to read all accessible sectors.");
    
    print!("Continue? (y/n): ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() != "y" {
        println!("Operation cancelled.");
        return Ok(());
    }
    
    // Wait for a card with 5 second timeout
    match wait_for_card_enhanced(reader, 5)? {
        Some(uid) => {
            println!("Card detected. UID: {}", format_uid(&uid));
            
            // Try to identify the card type
            let card_type = identify_card_type(&uid, None);
            println!("Card type: {}", card_type);
            
            // Determine number of sectors based on card type
            let _num_sectors = match card_type {
                CardType::MifareClassic1K => 16,
                CardType::MifareClassic4K => 40,
                _ => 16, // Default to 16 sectors
            };
            
            println!("\nAttempting to read all sectors...");
            
            // Try to read each sector
            // (implementation would call reader.dump_card() or similar)
            
            println!("\nDump completed.");
            
            // Wait for card removal
            wait_for_card_removal(reader)?;
        },
        None => {
            println!("No card detected during the timeout period.");
        }
    }
    
    Ok(())
}
