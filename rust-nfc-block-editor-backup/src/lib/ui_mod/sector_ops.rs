use std::error::Error;
use rppal::spi::Spi;

use crate::lib::mfrc522::{
    mfrc522_request, mfrc522_anticoll, mfrc522_select_tag, 
    mfrc522_auth, mfrc522_stop_crypto1, mfrc522_write,
    PICC_REQIDL, PICC_AUTHENT1A, PICC_AUTHENT1B, MI_OK
};
use crate::lib::mifare::{
    AccessBits, modify_sector_access, change_sector_keys
};
use crate::lib::utils::{uid_to_string, bytes_to_hex, hex_string_to_bytes};
use super::common::{clear_screen, wait_for_input, countdown_for_card_placement};

/// Access Bits Menu
pub fn access_bits_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("MODIFY ACCESS BITS");
    println!("=================");
    
    let sector_str = wait_for_input("\nEnter sector number (0-15): ")?;
    let sector = match sector_str.parse::<u8>() {
        Ok(num) if num <= 15 => num,
        _ => {
            println!("Invalid sector number. Must be between 0 and 15.");
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
    };
    
    // Choose access condition
    println!("");
    println!("Select access configuration:");
    println!("1. Transport (all open, default)");
    println!("2. Secure (read with Key A, write with Key B)");
    println!("3. Read-only (no writes allowed)");
    println!("4. Custom (advanced)");
    
    let access_choice = wait_for_input("\nEnter choice (1-4): ")?;
    
    let access_bits = match access_choice.as_str() {
        "1" => AccessBits::get_predefined_config("transport"),
        "2" => AccessBits::get_predefined_config("secure"),
        "3" => AccessBits::get_predefined_config("readonly"),
        "4" => {
            // TODO: Implement custom access bits configuration
            println!("Custom access bits not implemented yet. Using transport configuration.");
            AccessBits::get_predefined_config("transport")
        },
        _ => {
            println!("Invalid choice. Using transport configuration.");
            AccessBits::get_predefined_config("transport")
        }
    };
    
    // Define Key A (this was missing in the original)
    let key_a = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].to_vec();
    
    // Get Key B
    let key_b_str = wait_for_input("\nEnter Key B (12 hex chars, default FFFFFFFFFFFF): ")?;
    let key_b = if key_b_str.is_empty() {
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].to_vec()
    } else {
        match hex_string_to_bytes(&key_b_str) {
            Some(bytes) if bytes.len() == 6 => bytes,
            _ => {
                println!("Invalid key format. Using default key.");
                [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].to_vec()
            }
        }
    };
    
    // Get authentication key for current sector trailer
    println!("");
    println!("Authentication needed for current sector trailer.");
    println!("1. Use Key A (default: FFFFFFFFFFFF)");
    println!("2. Use Key B (default: FFFFFFFFFFFF)");
    
    let key_choice = wait_for_input("\nEnter choice (1-2): ")?;
    
    let auth_mode = match key_choice.as_str() {
        "1" => PICC_AUTHENT1A,
        "2" => PICC_AUTHENT1B,
        _ => {
            println!("Invalid choice. Using Key A by default.");
            PICC_AUTHENT1A
        }
    };
    
    // Ask for custom key if needed
    let use_custom_key = wait_for_input("\nUse custom key? (y/n): ")?.to_lowercase();
    
    let auth_key = if use_custom_key == "y" {
        let key_str = wait_for_input("Enter key (12 hex chars): ")?;
        match hex_string_to_bytes(&key_str) {
            Some(bytes) if bytes.len() == 6 => bytes,
            _ => {
                println!("Invalid key format. Using default key.");
                [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].to_vec()
            }
        }
    } else {
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].to_vec()
    };
    
    // Calculate the block number for the sector trailer
    let block_number = (sector * 4) + 3;
    
    // Construct the trailer data
    let access_bytes = access_bits.to_bytes();
    
    let mut trailer_data = [0u8; 16];
    trailer_data[0..6].copy_from_slice(&key_a);
    trailer_data[6..10].copy_from_slice(&access_bytes);
    trailer_data[10..16].copy_from_slice(&key_b);
    
    // Show trailer data to be written
    println!("");
    println!("Trailer data to be written:");
    println!("Key A: {}", bytes_to_hex(&key_a));
    println!("Access Bits: {}", bytes_to_hex(&access_bytes));
    println!("Key B: {}", bytes_to_hex(&key_b));
    println!("");
    println!("Access conditions:");
    println!("{}", access_bits);
    
    let confirm = wait_for_input("\nConfirm write? (y/n): ")?.to_lowercase();
    if confirm != "y" {
        println!("Operation cancelled.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    countdown_for_card_placement(5)?;
    
    // Request tag and get UID
    let (status, _) = mfrc522_request(spi, PICC_REQIDL)?;
    if status != MI_OK {
        println!("");
        println!("Error: Could not detect card.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Anti-collision and get UID
    let (status, uid) = mfrc522_anticoll(spi)?;
    if status != MI_OK {
        println!("");
        println!("Error: Could not read card UID.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    println!("");
    println!("Card detected. UID: {}", uid_to_string(&uid));
    
    // Select card
    let size = mfrc522_select_tag(spi, &uid)?;
    if size == 0 {
        println!("");
        println!("Error: Could not select card.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Authenticate
    let status = mfrc522_auth(spi, auth_mode, block_number, &auth_key, &uid)?;
    if status != MI_OK {
        println!("");
        println!("Authentication failed. Try a different key.");
        mfrc522_stop_crypto1(spi)?;
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Write the trailer
    let write_status = mfrc522_write(spi, block_number, &trailer_data)?;
    if write_status == MI_OK {
        println!("");
        println!("Sector trailer written successfully!");
    } else {
        println!("");
        println!("Error writing sector trailer. Check access rights.");
    }
    
    mfrc522_stop_crypto1(spi)?;
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}

/// Change Keys Menu
pub fn change_keys_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("CHANGE KEYS");
    println!("===========");
    
    let sector_str = wait_for_input("\nEnter sector number (0-15): ")?;
    let sector = match sector_str.parse::<u8>() {
        Ok(num) if num <= 15 => num,
        _ => {
            println!("Invalid sector number. Must be between 0 and 15.");
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
    };
    
    // Get current key
    println!("");
    println!("You need the current key to change keys.");
    
    // Choose key type for authentication
    println!("1. Authenticate with Key A");
    println!("2. Authenticate with Key B");
    
    let key_type_choice = wait_for_input("\nEnter choice (1-2): ")?;
    
    let auth_type = match key_type_choice.as_str() {
        "1" => PICC_AUTHENT1A,
        "2" => PICC_AUTHENT1B,
        _ => {
            println!("Invalid choice. Using Key A by default.");
            PICC_AUTHENT1A
        }
    };
    
    // Get current key
    let current_key_str = wait_for_input("\nEnter current key (12 hex chars, default FFFFFFFFFFFF): ")?;
    let current_key = if current_key_str.is_empty() {
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].to_vec()
    } else {
        match hex_string_to_bytes(&current_key_str) {
            Some(bytes) if bytes.len() == 6 => bytes,
            _ => {
                println!("Invalid key format. Using default key.");
                [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].to_vec()
            }
        }
    };
    
    // Ask which keys to change
    println!("");
    println!("Which keys do you want to change?");
    println!("1. Key A only");
    println!("2. Key B only");
    println!("3. Both keys");
    
    let change_choice = wait_for_input("\nEnter choice (1-3): ")?;
    
    let change_key_a = change_choice == "1" || change_choice == "3";
    let change_key_b = change_choice == "2" || change_choice == "3";
    
    if !change_key_a && !change_key_b {
        println!("No keys selected for change. Operation cancelled.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Get new keys
    let mut new_key_a = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].to_vec();
    let mut new_key_b = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].to_vec();
    
    if change_key_a {
        let key_a_str = wait_for_input("\nEnter new Key A (12 hex chars): ")?;
        match hex_string_to_bytes(&key_a_str) {
            Some(bytes) if bytes.len() == 6 => {
                new_key_a = bytes;
            },
            _ => {
                println!("Invalid key format. Using default key.");
            }
        }
    }
    
    if change_key_b {
        let key_b_str = wait_for_input("\nEnter new Key B (12 hex chars): ")?;
        match hex_string_to_bytes(&key_b_str) {
            Some(bytes) if bytes.len() == 6 => {
                new_key_b = bytes;
            },
            _ => {
                println!("Invalid key format. Using default key.");
            }
        }
    }
    
    // Confirmation
    println!("");
    println!("Changing keys for sector {}:", sector);
    if change_key_a {
        println!("New Key A: {}", bytes_to_hex(&new_key_a));
    } else {
        println!("Key A: (unchanged)");
    }
    
    if change_key_b {
        println!("New Key B: {}", bytes_to_hex(&new_key_b));
    } else {
        println!("Key B: (unchanged)");
    }
    
    let confirm = wait_for_input("\nConfirm key change? (y/n): ")?.to_lowercase();
    if confirm != "y" {
        println!("Operation cancelled.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    countdown_for_card_placement(5)?;
    
    // Change the keys
    let result = change_sector_keys(spi, sector, &current_key, 
                                  change_key_a, &new_key_a,
                                  change_key_b, &new_key_b)?;
    
    if result {
        println!("");
        println!("Keys changed successfully!");
    } else {
        println!("");
        println!("Failed to change keys. Check authentication key and access rights.");
    }
    
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}
