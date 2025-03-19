// src/attacks/default_keys.rs
use std::error::Error;
use std::io::{self, Write};

use crate::reader::MifareClassic;
use crate::utils::{format_uid, bytes_to_hex};
use crate::card_detection::wait_for_card_enhanced;

/// Try default keys on a card
pub fn run_default_key_search(reader: &mut MifareClassic) -> Result<(), Box<dyn Error>> {
    println!("\n=== Trying Default Keys ===");
    println!("Hold your card still...");
    
    // Reset the reader for better reliability
    reader.reset_reader()?;
    
    // Wait for a card with 5 second timeout
    match wait_for_card_enhanced(reader, 5)? {
        Some(uid) => {
            println!("Card detected! UID: {}", format_uid(&uid));
            
            // Try to authenticate with default keys
            println!("\nTrying default keys on first block of each sector...");
            
            let mut found_any_key = false;
            
            // Try default keys on each sector
            for sector in 0..16 {
                let block = sector * 4; // First block of sector
                
                println!("\nSector {} (blocks {}-{}):", sector, block, block + 3);
                
                // Try to authenticate with default keys
                match reader.try_default_keys(block)? {
                    Some((key, key_type)) => {
                        found_any_key = true;
                        
                        println!("  SUCCESS! Found key: {}", bytes_to_hex(&key));
                        println!("  Key type: {:?}", key_type);
                        
                        // Store this key for future use
                        reader.last_known_keys.insert((sector, key_type), key);
                        
                        // Try to read the sector blocks
                        println!("  Reading sector blocks:");
                        // (You would implement reading here)
                    },
                    None => {
                        println!("  No default keys work for this sector.");
                    }
                }
            }
            
            if found_any_key {
                println!("\nSuccessfully found keys for some sectors!");
            } else {
                println!("\nFailed to find any default keys.");
            }
        },
        None => {
            println!("No card detected");
        }
    }
    
    Ok(())
}
