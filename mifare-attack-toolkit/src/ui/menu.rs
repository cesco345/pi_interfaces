use std::error::Error;
use std::io::{self, Write};
use crate::cards::KeyType;

/// Display the main menu and get the user's choice
pub fn display_main_menu() -> Result<u8, Box<dyn Error>> {
    println!("\nSelect an option:");
    println!("1. Read card UID");
    println!("2. Try default keys");
    println!("3. Run Nested Attack (requires a known key)");
    println!("4. Run Darkside Attack");
    println!("5. Detect Magic Card");
    println!("6. Write custom UID (requires Magic Card)");
    println!("7. Dump card contents");
    println!("8. Clone card to Magic Card");
    println!("9. Exit");
    
    print!("Enter choice: ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    match input.trim().parse::<u8>() {
        Ok(choice) => Ok(choice),
        Err(_) => Ok(0), // Return 0 for invalid input
    }
}

/// Get a hex string from the user
pub fn get_hex_input(prompt: &str) -> Result<String, Box<dyn Error>> {
    print!("{}: ", prompt);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    Ok(input.trim().to_string())
}

/// Get a sector number from the user (0-15)
pub fn get_sector_number(prompt: &str) -> Result<u8, Box<dyn Error>> {
    print!("{} (0-15): ", prompt);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    match input.trim().parse::<u8>() {
        Ok(sector) if sector < 16 => Ok(sector),
        _ => {
            println!("Invalid sector number");
            Err("Invalid sector number".into())
        }
    }
}

/// Get a block number from the user
pub fn get_block_number(prompt: &str) -> Result<u8, Box<dyn Error>> {
    print!("{} (0-63): ", prompt);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    match input.trim().parse::<u8>() {
        Ok(block) if block < 64 => Ok(block),
        _ => {
            println!("Invalid block number");
            Err("Invalid block number".into())
        }
    }
}

/// Get key type from the user (A or B)
pub fn get_key_type() -> Result<char, Box<dyn Error>> {
    print!("Enter key type (A or B): ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    match input.trim().to_uppercase().chars().next() {
        Some('A') => Ok('A'),
        Some('B') => Ok('B'),
        _ => {
            println!("Invalid key type");
            Err("Invalid key type".into())
        }
    }
}

/// Get authentication key from user
pub fn get_authentication_key(prompt: &str) -> Result<([u8; 6], KeyType), Box<dyn Error>> {
    println!("{}", prompt);
    
    // Get key type
    let key_type = match get_key_type()? {
        'A' => KeyType::KeyA,
        'B' => KeyType::KeyB,
        _ => return Err("Invalid key type".into()),
    };
    
    // Get key value
    println!("Enter key (6 bytes in hex format, e.g. 'FF FF FF FF FF FF'):");
    print!("> ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let hex_values: Vec<&str> = input.trim().split_whitespace().collect();
    
    if hex_values.len() != 6 {
        return Err("Key must be exactly 6 bytes".into());
    }
    
    let mut key = [0u8; 6];
    
    for i in 0..6 {
        match u8::from_str_radix(hex_values[i], 16) {
            Ok(byte) => key[i] = byte,
            Err(_) => return Err(format!("Invalid hex value '{}'", hex_values[i]).into()),
        }
    }
    
    Ok((key, key_type))
}

/// Confirm an action
pub fn confirm(prompt: &str) -> Result<bool, Box<dyn Error>> {
    print!("{} (y/n): ", prompt);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    match input.trim().to_lowercase().chars().next() {
        Some('y') => Ok(true),
        _ => Ok(false),
    }
}
