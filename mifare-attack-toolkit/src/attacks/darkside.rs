// src/attacks/darkside.rs
use std::error::Error;
use std::io::{self, Write};

use crate::reader::MifareClassic;
use crate::utils::{wait_for_card_removal, format_uid, bytes_to_hex};
use crate::card_detection::wait_for_card_enhanced;

/// Run the darkside attack on a block to recover its key
pub fn run_darkside_attack(reader: &mut MifareClassic) -> Result<(), Box<dyn Error>> {
    println!("\n=== Darkside Attack ===");
    println!("This attack works on vulnerable MIFARE Classic cards");
    println!("It may take a few minutes to complete\n");
    
    // Get target block
    print!("Enter target block number (0-63):\n> ");
    io::stdout().flush()?;
    let mut block_str = String::new();
    io::stdin().read_line(&mut block_str)?;
    
    let block = match block_str.trim().parse::<u8>() {
        Ok(b) if b <= 63 => b,
        _ => {
            println!("Invalid block number. Must be 0-63.");
            return Ok(());
        }
    };
    
    println!("Placing card on the reader...");
    
    // Enable dark processing mode for better success with clone cards
    reader.enable_dark_processing_mode(true);
    
    // Wait for a card with longer timeout for darkside attack (15 seconds)
    match wait_for_card_enhanced(reader, 15)? {
        Some(uid) => {
            println!("Card detected with UID: {}", format_uid(&uid));
            println!("Starting darkside attack on block {}. This may take a while...", block);
            
            // Run the attack
            match reader.darkside_attack(block)? {
                Some(key) => {
                    println!("Attack successful!");
                    println!("Found key for block {}: {}", block, bytes_to_hex(&key));
                    println!("Sector: {}", block / 4);
                    println!("This key can likely be used for the entire sector.");
                },
                None => {
                    println!("Attack failed. The card may not be vulnerable to the darkside attack.");
                    println!("Try using a different block or using the nested attack if you already know some keys.");
                }
            }
            
            // Wait for card removal
            wait_for_card_removal(reader)?;
        },
        None => {
            println!("No card detected");
        }
    }
    
    // Disable dark processing mode when done
    reader.enable_dark_processing_mode(false);
    
    Ok(())
}
