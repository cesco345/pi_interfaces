use std::error::Error;
use rppal::spi::Spi;

use crate::lib::mfrc522::{
    mfrc522_request, mfrc522_anticoll, mfrc522_select_tag, 
    mfrc522_auth, mfrc522_stop_crypto1, mfrc522_read, mfrc522_write,
    PICC_REQIDL, PICC_AUTHENT1A, PICC_AUTHENT1B, MI_OK
};
use crate::lib::mifare::{AccessBits};
use crate::lib::utils::{uid_to_string, bytes_to_hex, bytes_to_ascii, hex_string_to_bytes};
use super::common::{clear_screen, wait_for_input, countdown_for_card_placement};

/// Block Editor Menu
pub fn block_editor_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("BLOCK EDITOR");
    println!("============");
    
    // Launch interactive block editor
    crate::lib::mifare::block_editor::interactive_edit(spi)?;
    
    Ok(())
}

/// Read Block Menu
pub fn read_block_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("READ BLOCK");
    println!("==========");
    
    let block_str = wait_for_input("\nEnter block number (0-63): ")?;
    let block_number = match block_str.parse::<u8>() {
        Ok(num) if num <= 63 => num,
        _ => {
            println!("Invalid block number. Must be between 0 and 63.");
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
    };
    
    // Show a warning for sector trailers (block numbers 3, 7, 11, etc.)
    if block_number % 4 == 3 {
        println!("");
        println!("Warning: Block {} is a sector trailer containing access bits and keys.", block_number);
    }
    
    // Get authentication key
    println!("");
    println!("Select authentication method:");
    println!("1. Key A (default: FFFFFFFFFFFF)");
    println!("2. Key B (default: FFFFFFFFFFFF)");
    
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
    
    let key = if use_custom_key == "y" {
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
    let status = mfrc522_auth(spi, auth_mode, block_number, &key, &uid)?;
    if status != MI_OK {
        println!("");
        println!("Authentication failed. Try a different key.");
        mfrc522_stop_crypto1(spi)?;
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Read the block
    match mfrc522_read(spi, block_number)? {
        Some(data) => {
            println!("");
            println!("Block {} data:", block_number);
            println!("HEX: {}", bytes_to_hex(&data));
            
            // If this is a sector trailer, display keys and access bits
            if block_number % 4 == 3 {
                println!("  Key A: {}", bytes_to_hex(&data[0..6]));
                println!("  Access Bits: {}", bytes_to_hex(&data[6..10]));
                println!("  Key B: {}", bytes_to_hex(&data[10..16]));
                
                // Parse and display access conditions
                let access_bytes = [data[6], data[7], data[8], data[9]];
                let access_bits = AccessBits::from_bytes(&access_bytes);
                println!("");
                println!("Access Conditions:");
                println!("{}", access_bits);
            } else {
                println!("ASCII: {}", bytes_to_ascii(&data));
            }
        },
        None => {
            println!("");
            println!("Error reading block data.");
        }
    }
    
    mfrc522_stop_crypto1(spi)?;
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}

/// Write Block Menu
pub fn write_block_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("WRITE BLOCK");
    println!("===========");
    
    let block_str = wait_for_input("\nEnter block number (0-63): ")?;
    let block_number = match block_str.parse::<u8>() {
        Ok(num) if num <= 63 => num,
        _ => {
            println!("Invalid block number. Must be between 0 and 63.");
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
    };
    
    // Show warnings for special blocks
    if block_number == 0 {
        println!("");
        println!("WARNING: Block 0 contains manufacturer data and card UID.");
        println!("Writing to this block may brick your card permanently!");
        
        let confirm = wait_for_input("\nAre you ABSOLUTELY sure? (type YES in uppercase): ")?;
        if confirm != "YES" {
            println!("Operation cancelled.");
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
    } else if block_number % 4 == 3 {
        println!("");
        println!("WARNING: Block {} is a sector trailer containing access bits and keys.", block_number);
        println!("Incorrect values may lock the sector or the entire card permanently!");
        
        let confirm = wait_for_input("\nAre you sure you want to continue? (y/n): ")?.to_lowercase();
        if confirm != "y" {
            println!("Operation cancelled.");
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
        
        // For sector trailers, use special writing function
        return write_sector_trailer_menu(spi, block_number);
    }
    
    // Get data entry method
    println!("");
    println!("Choose data format:");
    println!("1. Text (will be padded to 16 bytes)");
    println!("2. Hexadecimal (must be exactly 32 hex chars)");
    
    let format_choice = wait_for_input("\nEnter choice (1-2): ")?;
    
    let mut data = Vec::new();
    
    match format_choice.as_str() {
        "1" => {
            let text = wait_for_input("\nEnter text (max 16 chars): ")?;
            data = text.as_bytes()[0..std::cmp::min(text.len(), 16)].to_vec();
            
            // Pad with zeros to 16 bytes
            while data.len() < 16 {
                data.push(0);
            }
        },
        "2" => {
            let hex = wait_for_input("\nEnter hex data (32 chars): ")?;
            match hex_string_to_bytes(&hex) {
                Some(bytes) if bytes.len() == 16 => {
                    data = bytes;
                },
                _ => {
                    println!("Invalid hex data. Must be exactly 16 bytes (32 hex chars).");
                    wait_for_input("\nPress Enter to continue...")?;
                    return Ok(());
                }
            }
        },
        _ => {
            println!("Invalid choice.");
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
    }
    
    // Get authentication key
    println!("");
    println!("Select authentication method:");
    println!("1. Key A (default: FFFFFFFFFFFF)");
    println!("2. Key B (default: FFFFFFFFFFFF)");
    
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
    
    let key = if use_custom_key == "y" {
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
    
    // Show data to be written
    println!("");
    println!("Data to be written:");
    println!("HEX: {}", bytes_to_hex(&data));
    println!("ASCII: {}", bytes_to_ascii(&data));
    
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
    let status = mfrc522_auth(spi, auth_mode, block_number, &key, &uid)?;
    if status != MI_OK {
        println!("");
        println!("Authentication failed. Try a different key.");
        mfrc522_stop_crypto1(spi)?;
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Write the block
    let write_status = mfrc522_write(spi, block_number, &data)?;
    if write_status == MI_OK {
        println!("");
        println!("Block written successfully!");
    } else {
        println!("");
        println!("Error writing block. Check access rights.");
    }
    
    mfrc522_stop_crypto1(spi)?;
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}

/// Write Sector Trailer Menu (special handling for block 3, 7, 11, etc.)
fn write_sector_trailer_menu(spi: &mut Spi, block_number: u8) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("WRITE SECTOR TRAILER");
    println!("===================");
    println!("");
    println!("Block: {} (Sector {})", block_number, block_number / 4);
    
    // Get Key A
    let key_a_str = wait_for_input("\nEnter Key A (12 hex chars, default FFFFFFFFFFFF): ")?;
    let key_a = if key_a_str.is_empty() {
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].to_vec()
    } else {
        match hex_string_to_bytes(&key_a_str) {
            Some(bytes) if bytes.len() == 6 => bytes,
            _ => {
                println!("Invalid key format. Using default key.");
                [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].to_vec()
            }
        }
    };
    
    // Choose access bits configuration
    println!("");
    println!("Select access bits configuration:");
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
