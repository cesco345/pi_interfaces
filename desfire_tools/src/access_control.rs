use std::error::Error;
use std::io;
use std::time::SystemTime;

use crate::application::select_application;
use crate::file_operations::{create_standard_file, write_data, read_data};

// Access Control constants
pub const ACCESS_APP_ID: [u8; 3] = [0xA1, 0xC0, 0x01]; // AID for Access Control
pub const USER_FILE_ID: u8 = 0x01; // Store user information
pub const ACCESS_LEVEL_FILE_ID: u8 = 0x02; // Store access level
pub const TIMESTAMP_FILE_ID: u8 = 0x03; // Store last access timestamp

/// Initialize the access control system on a card
pub fn initialize_access_system(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nInitializing card for access control...");
    
    // Create the access control application
    let result = crate::application::create_application(
        card, &ACCESS_APP_ID, 0x0F, 2 // 2 keys (0 for read, 1 for write)
    );
    
    // If an error occurs but it's just that the application already exists, continue
    match result {
        Ok(_) => {},
        Err(e) => {
            if !e.to_string().contains("already exists") {
                return Err(e);
            }
            println!("Application already exists, continuing...");
        }
    }
    
    // Select the application
    select_application(card, &ACCESS_APP_ID)?;
    
    // Create files with appropriate access rights (read with key 0, write with key 1)
    create_standard_file(card, USER_FILE_ID, 32, [0x12, 0x34])?;
    create_standard_file(card, ACCESS_LEVEL_FILE_ID, 1, [0x12, 0x34])?;
    create_standard_file(card, TIMESTAMP_FILE_ID, 4, [0x12, 0x34])?;
    
    println!("Access control system initialized successfully!");
    Ok(())
}

/// Register a user in the access control system
pub fn register_user(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nRegistering new user...");
    
    // Select the access control application
    select_application(card, &ACCESS_APP_ID)?;
    
    // Get user ID
    println!("Enter user ID (max 10 chars):");
    let mut user_id = String::new();
    io::stdin().read_line(&mut user_id)?;
    let user_id = user_id.trim();
    
    if user_id.len() > 10 {
        return Err("User ID must be 10 characters or less".into());
    }
    
    // Pad the user ID to fixed length
    let mut user_data = vec![0; 32];
    for (i, byte) in user_id.bytes().enumerate() {
        if i < user_data.len() {
            user_data[i] = byte;
        }
    }
    
    // Write user data to the file
    write_data(card, USER_FILE_ID, &user_data)?;
    
    // Set initial access level (0 = basic access)
    write_data(card, ACCESS_LEVEL_FILE_ID, &[0])?;
    
    // Set initial timestamp (0)
    write_data(card, TIMESTAMP_FILE_ID, &[0, 0, 0, 0])?;
    
    println!("User registered successfully with ID: {}", user_id);
    Ok(())
}

/// Update a user's access level
pub fn update_access_level(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nUpdating access level...");
    
    // Select the access control application
    select_application(card, &ACCESS_APP_ID)?;
    
    // Read current user data
    let user_data = read_data(card, USER_FILE_ID, 32)?;
    let user_id = String::from_utf8_lossy(&user_data)
        .trim_matches(char::from(0))
        .to_string();
    
    println!("Current user: {}", user_id);
    
    // Get new access level
    println!("Enter new access level (0-5):");
    let mut level = String::new();
    io::stdin().read_line(&mut level)?;
    let level: u8 = level.trim().parse()?;
    
    if level > 5 {
        return Err("Access level must be between 0 and 5".into());
    }
    
    // Write new access level
    write_data(card, ACCESS_LEVEL_FILE_ID, &[level])?;
    
    // Update timestamp (current unix epoch time)
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs() as u32;
    
    let timestamp = [
        (now & 0xFF) as u8,
        ((now >> 8) & 0xFF) as u8,
        ((now >> 16) & 0xFF) as u8,
        ((now >> 24) & 0xFF) as u8,
    ];
    
    write_data(card, TIMESTAMP_FILE_ID, &timestamp)?;
    
    println!("Access level updated to {} for user {}", level, user_id);
    Ok(())
}

/// Check a user's access status
pub fn check_access_status(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nChecking access permissions...");
    
    // Select the access control application
    select_application(card, &ACCESS_APP_ID)?;
    
    // Read user data
    let user_data = read_data(card, USER_FILE_ID, 32)?;
    let user_id = String::from_utf8_lossy(&user_data)
        .trim_matches(char::from(0))
        .to_string();
    
    // Read access level
    let level_data = read_data(card, ACCESS_LEVEL_FILE_ID, 1)?;
    let access_level = level_data[0];
    
    // Read timestamp
    let timestamp_data = read_data(card, TIMESTAMP_FILE_ID, 4)?;
    let timestamp = u32::from_le_bytes([
        timestamp_data[0],
        timestamp_data[1],
        timestamp_data[2],
        timestamp_data[3],
    ]);
    
    // Format timestamp for display
    let datetime = chrono::NaiveDateTime::from_timestamp_opt(timestamp as i64, 0)
        .unwrap_or_default()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    
    // Display access information
    display_access_info(user_id, access_level, datetime);
    
    Ok(())
}

/// Display access information to the user
fn display_access_info(user_id: String, access_level: u8, datetime: String) {
    println!("\nAccess Information:");
    println!("User ID: {}", user_id);
    println!("Access Level: {}", access_level);
    println!("Last Updated: {}", datetime);
    
    // Determine access for different areas
    println!("\nAccess Permissions:");
    println!("Main Entrance: {}", if access_level >= 0 { "GRANTED" } else { "DENIED" });
    println!("Office Area: {}", if access_level >= 1 { "GRANTED" } else { "DENIED" });
    println!("Meeting Rooms: {}", if access_level >= 2 { "GRANTED" } else { "DENIED" });
    println!("R&D Department: {}", if access_level >= 3 { "GRANTED" } else { "DENIED" });
    println!("Server Room: {}", if access_level >= 4 { "GRANTED" } else { "DENIED" });
    println!("Executive Suite: {}", if access_level >= 5 { "GRANTED" } else { "DENIED" });
}
