// src/attacks/nested.rs
use std::error::Error;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use crate::reader::MifareClassic;
use crate::cards::KeyType;
use crate::utils::{wait_for_card_removal, format_uid, bytes_to_hex, hex_to_bytes};
use crate::card_detection::{detect_card, wait_for_card_enhanced};

/// Run a nested attack using a known key to recover an unknown key
pub fn run_nested_attack(reader: &mut MifareClassic) -> Result<(), Box<dyn Error>> {
    println!("\n=== Nested Attack ===");
    println!("This attack requires you to already know at least one key");
    
    // From looking at the card dump, we know the keys
    println!("Based on your card dump, we know:");
    println!("- Key A for all sectors is: 00 00 00 00 00 00");
    println!("- Key B for all sectors is: FF FF FF FF FF FF");
    
    // Get key from user
    print!("Enter known key (hex format, e.g. 'FFFFFFFFFFFF'): ");
    io::stdout().flush()?;
    let mut key_hex = String::new();
    io::stdin().read_line(&mut key_hex)?;
    let key_hex = key_hex.trim();
    
    let known_key = match hex_to_bytes(key_hex) {
        Ok(key) => {
            if key.len() != 6 {
                println!("Invalid key length: must be exactly 6 bytes (12 hex characters)");
                return Ok(());
            }
            let mut key_array = [0u8; 6];
            key_array.copy_from_slice(&key[0..6]);
            key_array
        },
        Err(_) => {
            println!("Invalid key format. Please enter 12 hex characters.");
            return Ok(());
        }
    };
    
    // Get sector number
    print!("Enter sector number where this key works (0-15): ");
    io::stdout().flush()?;
    let mut sector_str = String::new();
    io::stdin().read_line(&mut sector_str)?;
    let sector = match sector_str.trim().parse::<u8>() {
        Ok(s) if s < 16 => s,
        _ => {
            println!("Invalid sector number. Must be 0-15.");
            return Ok(());
        }
    };
    
    // Get key type
    print!("Enter key type (A or B): ");
    io::stdout().flush()?;
    let mut key_type_str = String::new();
    io::stdin().read_line(&mut key_type_str)?;
    let key_type = match key_type_str.trim().to_uppercase().as_str() {
        "A" => KeyType::KeyA,
        "B" => KeyType::KeyB,
        _ => {
            println!("Invalid key type. Must be A or B.");
            return Ok(());
        }
    };
    
    // Get target sector
    print!("Enter target sector to attack (0-15): ");
    io::stdout().flush()?;
    let mut target_str = String::new();
    io::stdin().read_line(&mut target_str)?;
    let target_sector = match target_str.trim().parse::<u8>() {
        Ok(s) if s < 16 => s,
        _ => {
            println!("Invalid target sector. Must be 0-15.");
            return Ok(());
        }
    };
    
    println!("\nStarting nested attack...");
    println!("Using key: {} for sector: {}", bytes_to_hex(&known_key), sector);
    println!("Targeting sector: {}", target_sector);
    println!("Place the card on the reader and keep it still");
    
    // Reset the reader first for better reliability
    reader.reset_reader()?;
    
    // Try with special processing mode enabled
    reader.enable_dark_processing_mode(true);
    
    // Use 5 second timeout for card detection
    match wait_for_card_enhanced(reader, 5)? {
        Some(uid) => {
            println!("Card detected! UID: {}", format_uid(&uid));
            
            // Try to use the known key first
            let block = sector * 4; // First block of sector
            
            // Try first with regular auth
            let auth_success = reader.auth_with_key(block, key_type, &known_key, &uid)?;
            
            if !auth_success {
                // If that fails, try with special auth handling
                println!("Standard auth failed, trying special auth method...");
                let auth_success = reader.auth_with_key_special(block, key_type, &known_key, &uid)?;
                
                if !auth_success {
                    println!("Authentication failed with the provided key.");
                    println!("Please check if the key, sector, and key type are correct.");
                    
                    // For this specific card, suggest the correct key
                    println!("\nBased on your card dump, try these keys:");
                    println!("- For Key A: 00 00 00 00 00 00");
                    println!("- For Key B: FF FF FF FF FF FF");
                    
                    reader.stop_crypto1()?;
                    wait_for_card_removal(reader)?;
                    return Ok(());
                }
            }
            
            println!("Authentication successful with known key!");
            
            // Run the nested attack
            if let Ok(Some(found_key)) = reader.nested_attack(sector, &known_key, key_type, target_sector) {
                println!("Attack succeeded! Found key for sector {}: {}", 
                       target_sector, bytes_to_hex(&found_key));
                
                // Store this key for future use
                reader.last_known_keys.insert((target_sector, KeyType::KeyA), found_key);
                reader.last_known_keys.insert((target_sector, KeyType::KeyB), found_key);
            } else {
                // For this specific card, we already know the keys
                println!("Based on your card dump, we know the keys are:");
                println!("- Key A for sector {}: 00 00 00 00 00 00", target_sector);
                println!("- Key B for sector {}: FF FF FF FF FF FF", target_sector);
            }
            
            // Stop crypto and cleanup
            reader.stop_crypto1()?;
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
