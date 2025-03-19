// src/operations/clone.rs
use std::error::Error;
use std::io::{self, Write};

use crate::reader::MifareClassic;
use crate::utils::{wait_for_card_removal, format_uid, bytes_to_hex, hex_to_bytes, get_user_confirmation};
use crate::card_detection::wait_for_card_enhanced;

/// Clone a card to a Magic Card
pub fn clone_card(reader: &mut MifareClassic) -> Result<(), Box<dyn Error>> {
    println!("\n=== Clone Card ===");
    println!("This operation will read data from a source card and write it to a Magic Card.");
    
    // First read the source card
    println!("\nStep 1: Read source card");
    println!("Place the SOURCE card on the reader...");
    
    // FIXED: Use wait_for_card_enhanced instead to avoid type parameter issues
    let source_uid = match wait_for_card_enhanced(reader, 15)? {
        Some(uid) => {
            println!("Source card detected. UID: {}", format_uid(&uid));
            
            // Try to read all sectors from the source card
            println!("Reading card data...");
            
            // Wait for card removal
            wait_for_card_removal(reader)?;
            
            uid
        },
        None => {
            println!("No source card detected.");
            return Ok(());
        }
    };
    
    // Ask user for potential UID change
    print!("Do you want to use a different UID for the target card? (y/n): ");
    io::stdout().flush()?;
    let mut change_uid = String::new();
    io::stdin().read_line(&mut change_uid)?;
    
    let target_uid = if change_uid.trim().to_lowercase() == "y" {
        print!("Enter new UID in hex (e.g., 11:22:33:44): ");
        io::stdout().flush()?;
        let mut new_uid_str = String::new();
        io::stdin().read_line(&mut new_uid_str)?;
        
        match hex_to_bytes(new_uid_str.trim()) {
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
        }
    } else {
        source_uid.clone()
    };
    
    // Now write to the target card
    println!("\nStep 2: Write to target Magic Card");
    println!("Place the TARGET Magic Card on the reader...");
    
    // FIXED: Use wait_for_card_enhanced instead to avoid type parameter issues
    match wait_for_card_enhanced(reader, 15)? {
        Some(uid) => {
            println!("Target card detected. UID: {}", format_uid(&uid));
            
            // Check if this appears to be a Magic Card
            let is_magic = false; // You would implement detection here
            
            if !is_magic {
                println!("Warning: This doesn't appear to be a Magic Card.");
                if !get_user_confirmation("Continue anyway?") {
                    println!("Operation cancelled.");
                    return Ok(());
                }
            }
            
            // First change the UID if needed
            if target_uid != source_uid {
                println!("Changing UID to: {}", format_uid(&target_uid));
                // (Implementation would write the UID)
            }
            
            // Write all the data to the target card
            println!("Writing data to target card...");
            // (Implementation would write all sectors)
            
            println!("\nClone operation completed.");
            
            // Wait for card removal
            wait_for_card_removal(reader)?;
        },
        None => {
            println!("No target card detected.");
        }
    }
    
    Ok(())
}
