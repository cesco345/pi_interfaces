use std::error::Error;
use rppal::spi::Spi;
use std::io::{self, Write};

use crate::lib::mfrc522::{
    mfrc522_request, mfrc522_anticoll, mfrc522_select_tag, 
    mfrc522_auth, mfrc522_stop_crypto1, mfrc522_read, mfrc522_write,
    PICC_REQIDL, PICC_AUTHENT1A, PICC_AUTHENT1B, MI_OK
};

use crate::lib::utils::{bytes_to_hex, bytes_to_ascii, hex_string_to_bytes, uid_to_string};
use crate::lib::mifare::access::AccessBits;

/// Read a specific block's data and display it in both hex and ASCII formats
pub fn read_block(spi: &mut Spi, block_addr: u8, auth_mode: u8, key: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    // Validate input parameters
    if block_addr > 63 {
        return Err("Invalid block address (must be 0-63)".into());
    }
    
    if key.len() != 6 {
        return Err("Invalid key length (must be 6 bytes)".into());
    }

    // Check if it's a sector trailer
    let is_trailer = block_addr % 4 == 3;
    
    // Connect to the card
    let (status, _) = mfrc522_request(spi, PICC_REQIDL)?;
    if status != MI_OK {
        return Err("No card detected".into());
    }
    
    let (status, uid) = mfrc522_anticoll(spi)?;
    if status != MI_OK {
        return Err("Failed to get card UID".into());
    }
    
    let size = mfrc522_select_tag(spi, &uid)?;
    if size == 0 {
        return Err("Failed to select card".into());
    }
    
    println!("Card detected. UID: {}", uid_to_string(&uid));
    
    // Try to authenticate
    let status = mfrc522_auth(spi, auth_mode, block_addr, key, &uid)?;
    if status != MI_OK {
        mfrc522_stop_crypto1(spi)?;
        return Err("Authentication failed. Check your key.".into());
    }
    
    // Read the block data
    let data_opt = mfrc522_read(spi, block_addr)?;
    mfrc522_stop_crypto1(spi)?;
    
    if let Some(data) = data_opt {
        println!("Block {} data:", block_addr);
        println!("HEX: {}", bytes_to_hex(&data));
        
        if is_trailer {
            // This is a sector trailer - show detailed information
            println!("This is a sector trailer block (Key A, Access Bits, Key B)");
            println!("Key A: {}", bytes_to_hex(&data[0..6]));
            println!("Access Bits: {}", bytes_to_hex(&data[6..10]));
            println!("Key B: {}", bytes_to_hex(&data[10..16]));
            
            // Show interpreted access conditions
            let access_bytes = [data[6], data[7], data[8], data[9]];
            let access_bits = AccessBits::from_bytes(&access_bytes);
            println!("\nAccess Conditions:");
            let sector = block_addr / 4;
            let first_block = sector * 4;
            println!("Block {}: {}", first_block, access_bits.interpret_access("data", 0));
            println!("Block {}: {}", first_block + 1, access_bits.interpret_access("data", 1));
            println!("Block {}: {}", first_block + 2, access_bits.interpret_access("data", 2));
            println!("Block {} (Trailer): {}", block_addr, 
                    access_bits.interpret_access("trailer", 0).replace("\n", "\n  "));
        } else {
            // Regular data block
            println!("ASCII: {}", bytes_to_ascii(&data));
        }
        
        return Ok(Some(data));
    } else {
        println!("Failed to read block data.");
        return Ok(None);
    }
}

/// Write data to a specific block
pub fn write_block(spi: &mut Spi, block_addr: u8, auth_mode: u8, key: &[u8], data: &[u8]) -> Result<bool, Box<dyn Error>> {
    // Validate input parameters
    if block_addr > 63 {
        return Err("Invalid block address (must be 0-63)".into());
    }
    
    if key.len() != 6 {
        return Err("Invalid key length (must be 6 bytes)".into());
    }
    
    if data.len() != 16 {
        return Err("Data must be exactly 16 bytes".into());
    }
    
    // Check for special blocks that need warnings
    if block_addr == 0 {
        println!("WARNING: Block 0 contains manufacturer data and card UID.");
        println!("Writing to this block may brick your card permanently!");
        
        let mut input = String::new();
        print!("Are you ABSOLUTELY sure? (type YES in uppercase to confirm): ");
        io::stdout().flush()?;
        io::stdin().read_line(&mut input)?;
        if input.trim() != "YES" {
            return Err("Operation cancelled by user".into());
        }
    } else if block_addr % 4 == 3 {
        println!("WARNING: Block {} is a sector trailer containing keys and access conditions.", block_addr);
        println!("Writing incorrect data may lock your card or sector permanently!");
        
        let mut input = String::new();
        print!("Are you sure you want to continue? (y/n): ");
        io::stdout().flush()?;
        io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase() != "y" {
            return Err("Operation cancelled by user".into());
        }
    }
    
    // Connect to the card
    let (status, _) = mfrc522_request(spi, PICC_REQIDL)?;
    if status != MI_OK {
        return Err("No card detected".into());
    }
    
    let (status, uid) = mfrc522_anticoll(spi)?;
    if status != MI_OK {
        return Err("Failed to get card UID".into());
    }
    
    let size = mfrc522_select_tag(spi, &uid)?;
    if size == 0 {
        return Err("Failed to select card".into());
    }
    
    println!("Card detected. UID: {}", uid_to_string(&uid));
    
    // Try to authenticate
    let status = mfrc522_auth(spi, auth_mode, block_addr, key, &uid)?;
    if status != MI_OK {
        mfrc522_stop_crypto1(spi)?;
        return Err("Authentication failed. Check your key.".into());
    }
    
    // Write the data
    let status = mfrc522_write(spi, block_addr, data)?;
    mfrc522_stop_crypto1(spi)?;
    
    if status == MI_OK {
        println!("Block {} written successfully!", block_addr);
        println!("Data written: {}", bytes_to_hex(data));
        println!("ASCII: {}", bytes_to_ascii(data));
        return Ok(true);
    } else {
        println!("Failed to write to block {}.", block_addr);
        return Ok(false);
    }
}

/// Prepare a sector trailer with custom keys and access bits
pub fn create_sector_trailer(key_a: &[u8], key_b: &[u8], access_config: &str) -> Result<[u8; 16], Box<dyn Error>> {
    if key_a.len() != 6 || key_b.len() != 6 {
        return Err("Keys must be exactly 6 bytes each".into());
    }
    
    let mut trailer = [0u8; 16];
    
    // Set Key A
    trailer[0..6].copy_from_slice(key_a);
    
    // Set access bits based on configuration
    let access_bits = AccessBits::get_predefined_config(access_config);
    let access_bytes = access_bits.to_bytes();
    trailer[6..10].copy_from_slice(&access_bytes);
    
    // Set Key B
    trailer[10..16].copy_from_slice(key_b);
    
    Ok(trailer)
}

/// Format a block with text, padding with zeros if needed
pub fn format_text_block(text: &str) -> [u8; 16] {
    let mut block = [0u8; 16];
    let bytes = text.as_bytes();
    
    // Copy text bytes, up to 16 bytes
    for (i, &byte) in bytes.iter().enumerate().take(16) {
        block[i] = byte;
    }
    
    block
}

/// Interactive block editor menu
pub fn interactive_edit(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    loop {
        println!("\nBLOCK EDITOR MENU");
        println!("=================");
        println!("1. Read block");
        println!("2. Write block (text)");
        println!("3. Write block (hex)");
        println!("4. Create sector trailer");
        println!("0. Exit to main menu");
        
        let mut choice = String::new();
        print!("Enter choice: ");
        io::stdout().flush()?;
        io::stdin().read_line(&mut choice)?;
        
        match choice.trim() {
            "1" => {
                // Read block
                let block_addr = get_block_number()?;
                let (auth_mode, key) = get_authentication_info()?;
                
                match read_block(spi, block_addr, auth_mode, &key) {
                    Ok(_) => println!("Block read successful."),
                    Err(e) => println!("Error: {}", e),
                }
            },
            "2" => {
                // Write text to block
                let block_addr = get_block_number()?;
                let (auth_mode, key) = get_authentication_info()?;
                
                print!("Enter text to write (max 16 chars): ");
                io::stdout().flush()?;
                let mut text = String::new();
                io::stdin().read_line(&mut text)?;
                
                let block_data = format_text_block(text.trim());
                
                match write_block(spi, block_addr, auth_mode, &key, &block_data) {
                    Ok(_) => println!("Block write successful."),
                    Err(e) => println!("Error: {}", e),
                }
            },
            "3" => {
                // Write hex to block
                let block_addr = get_block_number()?;
                let (auth_mode, key) = get_authentication_info()?;
                
                print!("Enter hex data (32 hex chars without spaces): ");
                io::stdout().flush()?;
                let mut hex_str = String::new();
                io::stdin().read_line(&mut hex_str)?;
                
                match hex_string_to_bytes(hex_str.trim()) {
                    Some(data) if data.len() == 16 => {
                        match write_block(spi, block_addr, auth_mode, &key, &data) {
                            Ok(_) => println!("Block write successful."),
                            Err(e) => println!("Error: {}", e),
                        }
                    },
                    _ => println!("Invalid hex data. Must be exactly 32 hex characters (16 bytes)."),
                }
            },
            "4" => {
                // Create and write sector trailer
                let sector = get_sector_number()?;
                let block_addr = sector * 4 + 3; // Sector trailer block
                
                println!("\nCreating sector trailer for sector {} (block {})", sector, block_addr);
                
                // Get current authentication info
                let (auth_mode, current_key) = get_authentication_info()?;
                
                // Get new keys
                print!("Enter new Key A (12 hex chars, default FFFFFFFFFFFF): ");
                io::stdout().flush()?;
                let mut key_a_str = String::new();
                io::stdin().read_line(&mut key_a_str)?;
                
                let key_a = if key_a_str.trim().is_empty() {
                    [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].to_vec()
                } else {
                    match hex_string_to_bytes(key_a_str.trim()) {
                        Some(bytes) if bytes.len() == 6 => bytes,
                        _ => {
                            println!("Invalid key format. Using default key.");
                            [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].to_vec()
                        }
                    }
                };
                
                print!("Enter new Key B (12 hex chars, default FFFFFFFFFFFF): ");
                io::stdout().flush()?;
                let mut key_b_str = String::new();
                io::stdin().read_line(&mut key_b_str)?;
                
                let key_b = if key_b_str.trim().is_empty() {
                    [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].to_vec()
                } else {
                    match hex_string_to_bytes(key_b_str.trim()) {
                        Some(bytes) if bytes.len() == 6 => bytes,
                        _ => {
                            println!("Invalid key format. Using default key.");
                            [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].to_vec()
                        }
                    }
                };
                
                // Choose access configuration
                println!("\nSelect access configuration:");
                println!("1. Transport (all open, default)");
                println!("2. Secure (read with Key A, write with Key B)");
                println!("3. Read-only (no writes allowed)");
                
                let mut config_choice = String::new();
                print!("Enter choice (1-3): ");
                io::stdout().flush()?;
                io::stdin().read_line(&mut config_choice)?;
                
                let access_config = match config_choice.trim() {
                    "1" => "transport",
                    "2" => "secure",
                    "3" => "readonly",
                    _ => {
                        println!("Invalid choice. Using transport configuration.");
                        "transport"
                    }
                };
                
                // Create the trailer
                match create_sector_trailer(&key_a, &key_b, access_config) {
                    Ok(trailer) => {
                        println!("\nSector trailer created:");
                        println!("Key A: {}", bytes_to_hex(&trailer[0..6]));
                        println!("Access Bits: {}", bytes_to_hex(&trailer[6..10]));
                        println!("Key B: {}", bytes_to_hex(&trailer[10..16]));
                        
                        let access_bytes = [trailer[6], trailer[7], trailer[8], trailer[9]];
                        let access_bits = AccessBits::from_bytes(&access_bytes);
                        println!("\nAccess Conditions:");
                        println!("{}", access_bits);
                        
                        let mut confirm = String::new();
                        print!("\nWrite this trailer to block {}? (y/n): ", block_addr);
                        io::stdout().flush()?;
                        io::stdin().read_line(&mut confirm)?;
                        
                        if confirm.trim().to_lowercase() == "y" {
                            match write_block(spi, block_addr, auth_mode, &current_key, &trailer) {
                                Ok(_) => println!("Sector trailer written successfully!"),
                                Err(e) => println!("Error writing sector trailer: {}", e),
                            }
                        } else {
                            println!("Operation cancelled.");
                        }
                    },
                    Err(e) => println!("Error creating sector trailer: {}", e),
                }
            },
            "0" => {
                println!("Returning to main menu...");
                break;
            },
            _ => println!("Invalid choice. Please try again."),
        }
    }
    
    Ok(())
}

// Helper function to get a block number from user input
fn get_block_number() -> Result<u8, Box<dyn Error>> {
    let mut input = String::new();
    print!("Enter block number (0-63): ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut input)?;
    
    match input.trim().parse::<u8>() {
        Ok(num) if num <= 63 => Ok(num),
        _ => Err("Invalid block number. Must be between 0 and 63.".into()),
    }
}

// Helper function to get a sector number from user input
fn get_sector_number() -> Result<u8, Box<dyn Error>> {
    let mut input = String::new();
    print!("Enter sector number (0-15): ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut input)?;
    
    match input.trim().parse::<u8>() {
        Ok(num) if num <= 15 => Ok(num),
        _ => Err("Invalid sector number. Must be between 0 and 15.".into()),
    }
}

// Helper function to get authentication info (mode and key)
fn get_authentication_info() -> Result<(u8, Vec<u8>), Box<dyn Error>> {
    println!("\nSelect authentication method:");
    println!("1. Key A (default: FFFFFFFFFFFF)");
    println!("2. Key B (default: FFFFFFFFFFFF)");
    
    let mut choice = String::new();
    print!("Enter choice (1-2): ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut choice)?;
    
    let auth_mode = match choice.trim() {
        "1" => PICC_AUTHENT1A,
        "2" => PICC_AUTHENT1B,
        _ => {
            println!("Invalid choice. Using Key A by default.");
            PICC_AUTHENT1A
        }
    };
    
    print!("Enter key (12 hex chars, default FFFFFFFFFFFF): ");
    io::stdout().flush()?;
    let mut key_str = String::new();
    io::stdin().read_line(&mut key_str)?;
    
    let key = if key_str.trim().is_empty() {
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].to_vec()
    } else {
        match hex_string_to_bytes(key_str.trim()) {
            Some(bytes) if bytes.len() == 6 => bytes,
            _ => {
                println!("Invalid key format. Using default key.");
                [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF].to_vec()
            }
        }
    };
    
    Ok((auth_mode, key))
}
