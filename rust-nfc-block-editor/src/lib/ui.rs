use std::error::Error;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use rppal::spi::Spi;

use crate::lib::mfrc522::{
    mfrc522_request, mfrc522_anticoll, mfrc522_select_tag, 
    mfrc522_auth, mfrc522_stop_crypto1, mfrc522_read, mfrc522_write,
    PICC_REQIDL, PICC_AUTHENT1A, PICC_AUTHENT1B, MI_OK
};

use crate::lib::mifare::{
    read_card_uid, read_sector_data, write_block_data, write_block_raw,
    modify_sector_access, change_sector_keys, format_card, dump_card,
    AccessBits
};

use crate::lib::utils::{
    uid_to_string, bytes_to_hex, bytes_to_ascii, hex_string_to_bytes
};

// Helper function for countdown timer when placing card
pub fn countdown_for_card_placement(seconds: u64) -> Result<(), Box<dyn Error>> {
    println!("\nPrepare your card. You have {} seconds to place it on the reader...", seconds);
    
    // Progress bar width
    let width = 30;
    
    for i in (1..=seconds).rev() {
        let filled = ((seconds - i) as f64 / seconds as f64 * width as f64) as usize;
        
        print!("\r[");
        for j in 0..width {
            if j < filled {
                print!("=");
            } else if j == filled {
                print!(">");
            } else {
                print!(" ");
            }
        }
        print!("] {:2}/{} seconds", seconds - i + 1, seconds);
        io::stdout().flush()?;
        
        thread::sleep(Duration::from_secs(1));
    }
    
    println!("\n\nReading card now...");
    Ok(())
}

// Wait for user input
pub fn wait_for_input(prompt: &str) -> Result<String, Box<dyn Error>> {
    print!("{}", prompt);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    // Trim newline characters
    Ok(input.trim().to_string())
}

// Clear the terminal screen
pub fn clear_screen() {
    print!("{}[2J", 27 as char);
    print!("{}[1;1H", 27 as char);
}

// UI Main Menu
pub fn main_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    loop {
        clear_screen();
        println!("==========================");
        println!("  NFC/RFID BLOCK EDITOR  ");
        println!("==========================");
        
        println!("\nMAIN MENU:");
        println!("1. Read Card UID");
        println!("2. Read Block");
        println!("3. Write Block");
        println!("4. Dump Card");
        println!("5. Format Card");
        println!("6. Change Keys");
        println!("7. Modify Access Bits");
        println!("8. Block Editor (Interactive)");  // Added this option
        println!("9. Test Keys");                   // Added this option
        println!("0. Exit");
        
        let choice = wait_for_input("\nEnter your choice: ")?;
        
        match choice.as_str() {
            "1" => read_uid_menu(spi)?,
            "2" => read_block_menu(spi)?,
            "3" => write_block_menu(spi)?,
            "4" => dump_card_menu(spi)?,
            "5" => format_card_menu(spi)?,
            "6" => change_keys_menu(spi)?,
            "7" => access_bits_menu(spi)?,
            "8" => block_editor_menu(spi)?,  // New menu function
            "9" => test_keys_menu(spi)?,     // New menu function
            "0" => {
                println!("Exiting...");
                break;
            },
            _ => {
                println!("Invalid choice. Press Enter to continue...");
                wait_for_input("")?;
            }
        }
    }
    
    Ok(())
}
// Read Card UID Menu
fn read_uid_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("READ CARD UID");
    println!("=============");
    
    countdown_for_card_placement(5)?;
    
    match read_card_uid(spi)? {
        Some(uid) => {
            println!("\nCard UID: {}", uid_to_string(&uid));
            println!("UID as decimal: {}", crate::lib::utils::uid_to_num(&uid));
        },
        None => {
            println!("\nNo card detected or error reading card.");
        }
    }
    
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}

// Access Bits Menu
fn access_bits_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
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
    println!("\nSelect access configuration:");
    println!("1. Transport (all open, default)");
    println!("2. Secure (read with Key A, write with Key B)");
    println!("3. Read-only (no writes allowed)");
    println!("4. Custom (advanced, not implemented)");
    
    let access_choice = wait_for_input("\nEnter choice (1-3): ")?;
    
    let access_bits = match access_choice.as_str() {
        "1" => AccessBits::get_predefined_config("transport"),
        "2" => AccessBits::get_predefined_config("secure"),
        "3" => AccessBits::get_predefined_config("readonly"),
        "4" => {
            println!("Custom access bits not implemented yet. Operation cancelled.");
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        },
        _ => {
            println!("Invalid choice. Operation cancelled.");
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
    };
    
    // Get authentication key
    println!("\nYou need the current Key A to modify access bits.");
    let key_str = wait_for_input("Enter Key A (12 hex chars, default FFFFFFFFFFFF): ")?;
    
    let key = if key_str.is_empty() {
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].to_vec()
    } else {
        match hex_string_to_bytes(&key_str) {
            Some(bytes) if bytes.len() == 6 => bytes,
            _ => {
                println!("Invalid key format. Using default key.");
                [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].to_vec()
            }
        }
    };
    
    // Show the new access conditions
    println!("\nNew access conditions:");
    println!("{}", access_bits);
    
    let confirm = wait_for_input("\nConfirm access bits change? (y/n): ")?.to_lowercase();
    if confirm != "y" {
        println!("Operation cancelled.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    countdown_for_card_placement(5)?;
    
    // Modify access bits
    let result = modify_sector_access(spi, sector, &access_bits)?;
    
    if result {
        println!("\nAccess bits modified successfully!");
    } else {
        println!("\nFailed to modify access bits. Check authentication key and access rights.");
    }
    
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}

// Read Block Menu
fn read_block_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
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
        println!("\nWarning: Block {} is a sector trailer containing access bits and keys.", block_number);
    }
    
    // Get authentication key
    println!("\nSelect authentication method:");
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
        println!("\nError: Could not detect card.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Anti-collision and get UID
    let (status, uid) = mfrc522_anticoll(spi)?;
    if status != MI_OK {
        println!("\nError: Could not read card UID.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    println!("\nCard detected. UID: {}", uid_to_string(&uid));
    
    // Select card
    let size = mfrc522_select_tag(spi, &uid)?;
    if size == 0 {
        println!("\nError: Could not select card.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Authenticate
    let status = mfrc522_auth(spi, auth_mode, block_number, &key, &uid)?;
    if status != MI_OK {
        println!("\nAuthentication failed. Try a different key.");
        mfrc522_stop_crypto1(spi)?;
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Read the block
    match mfrc522_read(spi, block_number)? {
        Some(data) => {
            println!("\nBlock {} data:", block_number);
            println!("HEX: {}", bytes_to_hex(&data));
            
            // If this is a sector trailer, display keys and access bits
            if block_number % 4 == 3 {
                println!("  Key A: {}", bytes_to_hex(&data[0..6]));
                println!("  Access Bits: {}", bytes_to_hex(&data[6..10]));
                println!("  Key B: {}", bytes_to_hex(&data[10..16]));
                
                // Parse and display access conditions
                let access_bytes = [data[6], data[7], data[8], data[9]];
                let access_bits = AccessBits::from_bytes(&access_bytes);
                println!("\nAccess Conditions:");
                println!("{}", access_bits);
            } else {
                println!("ASCII: {}", bytes_to_ascii(&data));
            }
        },
        None => {
            println!("\nError reading block data.");
        }
    }
    
    mfrc522_stop_crypto1(spi)?;
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}

// Write Block Menu
fn write_block_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
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
        println!("\nWARNING: Block 0 contains manufacturer data and card UID.");
        println!("Writing to this block may brick your card permanently!");
        
        let confirm = wait_for_input("\nAre you ABSOLUTELY sure? (type YES in uppercase): ")?;
        if confirm != "YES" {
            println!("Operation cancelled.");
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
    } else if block_number % 4 == 3 {
        println!("\nWARNING: Block {} is a sector trailer containing access bits and keys.", block_number);
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
    println!("\nChoose data format:");
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
    println!("\nSelect authentication method:");
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
    println!("\nData to be written:");
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
        println!("\nError: Could not detect card.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Anti-collision and get UID
    let (status, uid) = mfrc522_anticoll(spi)?;
    if status != MI_OK {
        println!("\nError: Could not read card UID.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    println!("\nCard detected. UID: {}", uid_to_string(&uid));
    
    // Select card
    let size = mfrc522_select_tag(spi, &uid)?;
    if size == 0 {
        println!("\nError: Could not select card.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Authenticate
    let status = mfrc522_auth(spi, auth_mode, block_number, &key, &uid)?;
    if status != MI_OK {
        println!("\nAuthentication failed. Try a different key.");
        mfrc522_stop_crypto1(spi)?;
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Write the block
    let write_status = mfrc522_write(spi, block_number, &data)?;
    if write_status == MI_OK {
        println!("\nBlock written successfully!");
    } else {
        println!("\nError writing block. Check access rights.");
    }
    
    mfrc522_stop_crypto1(spi)?;
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}

// Write Sector Trailer Menu (special handling for block 3, 7, 11, etc.)
fn write_sector_trailer_menu(spi: &mut Spi, block_number: u8) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("WRITE SECTOR TRAILER");
    println!("===================");
    println!("\nBlock: {} (Sector {})", block_number, block_number / 4);
    
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
    println!("\nSelect access bits configuration:");
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
    println!("\nAuthentication needed for current sector trailer.");
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
    println!("\nTrailer data to be written:");
    println!("Key A: {}", bytes_to_hex(&key_a));
    println!("Access Bits: {}", bytes_to_hex(&access_bytes));
    println!("Key B: {}", bytes_to_hex(&key_b));
    println!("\nAccess conditions:");
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
        println!("\nError: Could not detect card.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Anti-collision and get UID
    let (status, uid) = mfrc522_anticoll(spi)?;
    if status != MI_OK {
        println!("\nError: Could not read card UID.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    println!("\nCard detected. UID: {}", uid_to_string(&uid));
    
    // Select card
    let size = mfrc522_select_tag(spi, &uid)?;
    if size == 0 {
        println!("\nError: Could not select card.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Authenticate
    let status = mfrc522_auth(spi, auth_mode, block_number, &auth_key, &uid)?;
    if status != MI_OK {
        println!("\nAuthentication failed. Try a different key.");
        mfrc522_stop_crypto1(spi)?;
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Write the trailer
    let write_status = mfrc522_write(spi, block_number, &trailer_data)?;
    if write_status == MI_OK {
        println!("\nSector trailer written successfully!");
    } else {
        println!("\nError writing sector trailer. Check access rights.");
    }
    
    mfrc522_stop_crypto1(spi)?;
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}

// Dump Card Menu
fn dump_card_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("DUMP CARD");
    println!("=========");
    
    let confirm = wait_for_input("\nDump entire card? This may take a while. Continue? (y/n): ")?.to_lowercase();
    if confirm != "y" {
        return Ok(());
    }
    
    countdown_for_card_placement(5)?;
    
    match dump_card(spi)? {
        Some(_) => {
            // Card dump was successful, output is already printed by the dump_card function
        },
        None => {
            println!("\nError dumping card.");
        }
    }
    
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}

// Block Editor Menu
fn block_editor_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("BLOCK EDITOR");
    println!("============");
    
    // Launch interactive block editor
    crate::lib::mifare::block_editor::interactive_edit(spi)?;
    
    Ok(())
}

// Test Keys Menu
fn test_keys_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("TEST KEYS");
    println!("=========");
    
    println!("This will test multiple keys against all sectors of your card.");
    println!("This process may take some time.");
    
    let confirm = wait_for_input("\nProceed? (y/n): ")?.to_lowercase();
    if confirm != "y" {
        return Ok(());
    }
    
    countdown_for_card_placement(5)?;
    
    match crate::lib::mifare::dump::test_keys(spi) {
        Ok(results) => {
            println!("\nKey Testing Results:");
            println!("====================");
            
            if results.is_empty() {
                println!("No working keys found for any sector.");
            } else {
                for (sector, key) in results {
                    println!("Sector {}: Key {}", sector, crate::lib::utils::bytes_to_hex(&key));
                }
            }
        },
        Err(e) => {
            println!("Error testing keys: {}", e);
        }
    }
    
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}

// Format Card Menu
fn format_card_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("FORMAT CARD");
    println!("===========");
    
    println!("\nWARNING: This will reset all sectors to default transport configuration.");
    println!("All data will be lost. Sector 0 (manufacturer block) will not be modified.");
    
    let confirm = wait_for_input("\nAre you sure you want to format the card? (type FORMAT to confirm): ")?;
    if confirm != "FORMAT" {
        println!("Operation cancelled.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    countdown_for_card_placement(5)?;
    
    if format_card(spi)? {
        println!("\nCard formatted successfully.");
    } else {
        println!("\nError formatting card.");
    }
    
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}

// Change Keys Menu
fn change_keys_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
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
    println!("\nYou need the current key to change keys.");
    
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
    println!("\nWhich keys do you want to change?");
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
    println!("\nChanging keys for sector {}:", sector);
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
        println!("\nKeys changed successfully!");
    } else {
        println!("\nFailed to change keys. Check authentication key and access rights.");
    }
    
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}
