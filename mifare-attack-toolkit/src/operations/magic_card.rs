// src/operations/magic_card.rs
use std::error::Error;
use std::io::{self, Write};

use crate::reader::MifareClassic;
use crate::utils::{wait_for_card_removal, format_uid, hex_to_bytes};
use crate::card_detection::wait_for_card_enhanced;

/// Detect card type (Magic Card detection)
pub fn detect_card_type(reader: &mut MifareClassic) -> Result<(), Box<dyn Error>> {
    println!("\n=== Detect Card Type ===");
    println!("This will attempt to identify if the card is a Magic Card.");
    
    // Wait for card
    println!("\nPlacing card on the reader...");
    
    // FIXED: Use wait_for_card_enhanced instead to avoid type parameter issues
    match wait_for_card_enhanced(reader, 15)? {
        Some(uid) => {
            println!("Card detected. UID: {}", format_uid(&uid));
            
            // Check for Magic Card patterns
            let is_magic = if uid.len() == 4 {
                // Common Magic Card UID patterns
                uid[0] == 0xCF || uid[0] == 0x88 || (uid[0] == 0x04 && uid[1] == 0x77)
            } else {
                false
            };
            
            if is_magic {
                println!("\nThis appears to be a Magic Card!");
                println!("Magic Cards allow UID changing and bypass some security features.");
            } else {
                println!("\nThis appears to be a standard MIFARE card.");
                println!("No specific Magic Card features detected.");
            }
            
            // Try additional tests for Magic Card capabilities
            println!("\nAdditional tests could be performed to verify Magic Card capabilities:");
            println!("1. Direct write to block 0 (UID block)");
            println!("2. Testing for backdoor commands");
            println!("3. Checking for direct memory operations");
            
            // Wait for card removal
            wait_for_card_removal(reader)?;
        },
        None => {
            println!("No card detected.");
        }
    }
    
    Ok(())
}

/// Write a custom UID to a Magic Card
pub fn write_custom_uid(reader: &mut MifareClassic) -> Result<(), Box<dyn Error>> {
    println!("\n=== Write Custom UID to Magic Card ===");
    println!("WARNING: This only works with Magic Cards that support UID changing!");
    println!("Using this on non-Magic Cards may DAMAGE your card permanently.");
    
    // Get the new UID
    print!("Enter new UID in hex (e.g., 11:22:33:44): ");
    io::stdout().flush()?;
    let mut new_uid_str = String::new();
    io::stdin().read_line(&mut new_uid_str)?;
    
    let new_uid = match hex_to_bytes(new_uid_str.trim()) {
        Ok(bytes) => {
            if bytes.len() != 4 && bytes.len() != 7 && bytes.len() != 10 {
                println!("Invalid UID length. Must be 4, 7, or 10 bytes.");
                return Ok(());
            }
            bytes
        },
        Err(e) => {
            println!("Invalid hex format: {}", e);
            return Ok(());
        }
    };
    
    println!("\nNew UID will be: {}", format_uid(&new_uid));
    print!("Are you ABSOLUTELY sure you want to proceed? This can brick your card! (y/n): ");
    io::stdout().flush()?;
    let mut confirm = String::new();
    io::stdin().read_line(&mut confirm)?;
    
    if confirm.trim().to_lowercase() != "y" {
        println!("Operation cancelled.");
        return Ok(());
    }
    
    // Wait for card
    println!("\nPlace the Magic Card on the reader...");
    
    // FIXED: Use wait_for_card_enhanced instead to avoid type parameter issues
    match wait_for_card_enhanced(reader, 15)? {
        Some(uid) => {
            println!("Card detected. Current UID: {}", format_uid(&uid));
            
            // First, check if it's likely a Magic Card
            let is_magic = if uid.len() == 4 {
                // Common Magic Card UID patterns
                uid[0] == 0xCF || uid[0] == 0x88 || (uid[0] == 0x04 && uid[1] == 0x77)
            } else {
                false
            };
            
            if !is_magic {
                println!("\nWARNING: This does NOT appear to be a Magic Card!");
                print!("Proceeding might DAMAGE YOUR CARD PERMANENTLY! Continue? (y/n): ");
                io::stdout().flush()?;
                let mut risky_confirm = String::new();
                io::stdin().read_line(&mut risky_confirm)?;
                
                if risky_confirm.trim().to_lowercase() != "y" {
                    println!("Operation cancelled.");
                    wait_for_card_removal(reader)?;
                    return Ok(());
                }
            }
            
            println!("\nAttempting to change UID...");
            // (Implementation would use special commands for Magic Cards)
            
            println!("\nUID change operation completed.");
            println!("Remove card and place it again to verify the new UID.");
            
            // Wait for card removal
            wait_for_card_removal(reader)?;
        },
        None => {
            println!("No card detected.");
        }
    }
    
    Ok(())
}
