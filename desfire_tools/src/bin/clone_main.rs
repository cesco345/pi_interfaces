// In src/bin/clone_main.rs
use std::error::Error;
use std::io::{self, Write};

use desfire_tools::desfire_common::{connect_to_card, send_apdu, HexSlice};

mod clone_auth;
mod clone_files;
mod clone_read;
mod clone_write;
mod clone_low_level;
mod enhanced_access;

// Test constants
pub const TEST_APP_ID: [u8; 3] = [0xA1, 0xA2, 0xA3];
pub const TEST_FILE_ID: u8 = 0x01;

fn main() -> Result<(), Box<dyn Error>> {
    println!("DESFire Card Access Tool");
    println!("=======================\n");
    
    // Try to connect to a card
    println!("Searching for NFC card...");
    
    // Use the existing function without arguments
    let (context, card) = connect_to_card()?;
    
    // Create a buffer for reader names
    let mut reader_buffer = [0u8; 2048];
    let mut readers = context.list_readers(&mut reader_buffer)?;
    
    // Check if we found any readers
    if readers.next().is_none() {
        return Err("No readers found".into());
    }
    
    // Main menu
    loop {
        println!("\nDESFire Card Operations Menu:");
        println!("1. Basic Clone Card Operations");
        println!("2. Access Control Application");
        println!("0. Exit");
        
        print!("Select an option: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        match input.trim() {
            "1" => show_clone_test_menu(&card)?,
            "2" => show_access_control_menu(&card)?,
            "0" => {
                println!("Exiting program...");
                break;
            },
            _ => println!("Invalid option! Please try again.")
        }
    }
    
    Ok(())
}

fn show_clone_test_menu(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nClone Card Test Menu:");
    println!("1. Format card");
    println!("2. Create application & file variants (tries multiple types)");
    println!("3. Write test data (tries multiple methods)");
    println!("4. Read test data (tries multiple approaches)");
    println!("5. Raw command mode");
    println!("6. Return to main menu");
    
    print!("Select an option: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    match input.trim() {
        "1" => clone_files::format_card(card)?,
        "2" => clone_files::create_test_variants(card)?,
        "3" => clone_write::write_test_variants(card)?,
        "4" => clone_read::read_test_variants(card)?,
        "5" => clone_low_level::raw_command_mode(card)?,
        _ => println!("Returning to main menu"),
    }
    
    Ok(())
}

fn show_access_control_menu(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nAccess Control Application Menu:");
    println!("1. Authenticate with card");
    println!("2. Create access control application");
    println!("3. Create user data file");
    println!("4. Register user");
    println!("5. Update access level");
    println!("6. Check access");
    println!("7. Return to main menu");
    
    print!("Select an option: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let app_id = enhanced_access::ACCESS_APP_ID;
    
    match input.trim() {
        "1" => {
            if let Err(e) = enhanced_access::authenticate_enhanced(card) {
                println!("Authentication failed: {}", e);
            } else {
                println!("Authentication successful!");
            }
        },
        "2" => {
            if let Err(e) = enhanced_access::authenticate_enhanced(card) {
                println!("Authentication failed: {}", e);
                return Err("Must authenticate before creating application".into());
            }
            
            match enhanced_access::create_application(card, &app_id) {
                Ok(_) => println!("Access control application created successfully!"),
                Err(e) => println!("Failed to create application: {}", e)
            }
        },
        "3" => create_user_files(card, &app_id)?,
        "4" => register_user(card, &app_id)?,
        "5" => update_access_level(card, &app_id)?,
        "6" => check_access(card, &app_id)?,
        _ => println!("Returning to main menu"),
    }
    
    Ok(())
}

fn create_user_files(card: &pcsc::Card, app_id: &[u8; 3]) -> Result<(), Box<dyn Error>> {
    // Authenticate 
    if let Err(e) = enhanced_access::authenticate_enhanced(card) {
        println!("Authentication failed: {}", e);
        return Err("Must authenticate before creating files".into());
    }
    
    // Select the application
    if let Err(e) = enhanced_access::select_app(card, app_id) {
        println!("Failed to select application: {}", e);
        return Err("Application selection failed".into());
    }
    
    // Authenticate again after selecting application
    if let Err(e) = enhanced_access::authenticate_enhanced(card) {
        println!("Authentication failed in application context: {}", e);
        return Err("Must authenticate in application context".into());
    }
    
    // Create files
    println!("Creating user information file...");
    if let Err(e) = enhanced_access::create_std_file(card, enhanced_access::USER_FILE_ID, 32) {
        println!("Failed to create user file: {}", e);
    } else {
        println!("User file created successfully!");
    }
    
    println!("Creating access level file...");
    if let Err(e) = enhanced_access::create_std_file(card, enhanced_access::CONFIG_FILE_ID, 1) {
        println!("Failed to create access level file: {}", e);
    } else {
        println!("Access level file created successfully!");
    }
    
    println!("Creating value file for access counter...");
    if let Err(e) = enhanced_access::create_value_file(card, enhanced_access::VALUE_FILE_ID) {
        println!("Failed to create value file: {}", e);
    } else {
        println!("Value file created successfully!");
    }
    
    println!("Creating record file for access log...");
    if let Err(e) = enhanced_access::create_record_file(card, enhanced_access::RECORD_FILE_ID, 8, 4) {
        println!("Failed to create record file: {}", e);
    } else {
        println!("Record file created successfully!");
    }
    
    println!("File creation completed!");
    Ok(())
}

fn register_user(card: &pcsc::Card, app_id: &[u8; 3]) -> Result<(), Box<dyn Error>> {
    // Authenticate and select application
    if let Err(e) = enhanced_access::authenticate_enhanced(card) {
        println!("Authentication failed: {}", e);
        return Err("Must authenticate before registering user".into());
    }
    
    if let Err(e) = enhanced_access::select_app(card, app_id) {
        println!("Failed to select application: {}", e);
        return Err("Application selection failed".into());
    }
    
    if let Err(e) = enhanced_access::authenticate_enhanced(card) {
        println!("Authentication failed in application context: {}", e);
        return Err("Must authenticate in application context".into());
    }
    
    println!("Enter user ID (max 30 chars):");
    let mut user_id = String::new();
    io::stdin().read_line(&mut user_id)?;
    let user_id = user_id.trim();
    
    // Create user data
    let mut user_data = vec![0; 32];
    for (i, byte) in user_id.bytes().enumerate() {
        if i < user_data.len() {
            user_data[i] = byte;
        }
    }
    
    // Try alternative write method for user data
    println!("Writing user data using modified method...");
    
    // First, try the modified write approach
    let mut write_cmd = vec![0x90, 0xDD, 0x00, 0x00]; // Modified instruction byte 0xDD
    write_cmd.push(7 + user_data.len() as u8); // Lc byte
    write_cmd.push(enhanced_access::USER_FILE_ID);
    write_cmd.push(0x00); // Offset (LSB)
    write_cmd.push(0x00); // Offset (middle byte)
    write_cmd.push(0x00); // Offset (MSB)
    write_cmd.push(user_data.len() as u8);
    write_cmd.extend_from_slice(&user_data);
    write_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &write_cmd) {
        Ok(response) => {
            println!("Modified write response: {}", HexSlice(&response));
            
            // Try standard commit
            let commit_cmd = [0x90, 0xC7, 0x00, 0x00, 0x00];
            let _ = send_apdu(card, &commit_cmd)?;
            
            // Also try alternative commit
            let alt_commit = [0x90, 0xDF, 0x00, 0x00, 0x00];
            let alt_resp = send_apdu(card, &alt_commit);
            if let Ok(resp) = alt_resp {
                println!("Alternative commit response: {}", HexSlice(&resp));
            }
            
            // And try abort to ensure clean state
            let abort_cmd = [0x90, 0xA7, 0x00, 0x00, 0x00];
            let abort_resp = send_apdu(card, &abort_cmd);
            if let Ok(resp) = abort_resp {
                println!("Abort response: {}", HexSlice(&resp));
            }
        },
        Err(e) => {
            println!("Modified write error: {}", e);
            // Fall back to standard write method
            enhanced_access::write_file_data(card, enhanced_access::USER_FILE_ID, &user_data)?;
        }
    }
    
    // Set access level using value file instead of standard file
    println!("Setting initial access level...");
    
    // Since value files are working, use one for access level
    if let Err(e) = enhanced_access::credit_value(card, enhanced_access::VALUE_FILE_ID, 1) {
        println!("Failed to set access level: {}", e);
    } else {
        println!("Access level set to 1 via value file");
    }
    
    println!("User '{}' registered successfully!", user_id);
    Ok(())
}

fn update_access_level(card: &pcsc::Card, app_id: &[u8; 3]) -> Result<(), Box<dyn Error>> {
    // Authenticate and select application
    if let Err(e) = enhanced_access::authenticate_enhanced(card) {
        println!("Authentication failed: {}", e);
        return Err("Must authenticate before updating access level".into());
    }
    
    if let Err(e) = enhanced_access::select_app(card, app_id) {
        println!("Failed to select application: {}", e);
        return Err("Application selection failed".into());
    }
    
    if let Err(e) = enhanced_access::authenticate_enhanced(card) {
        println!("Authentication failed in application context: {}", e);
        return Err("Must authenticate in application context".into());
    }
    
    // Read current user information...
    println!("Reading current user information...");
    
    // Try alternative read method for user data
    let mut got_user_id = String::new();
    
    // First try standard read
    match enhanced_access::read_file_data(card, enhanced_access::USER_FILE_ID, 32) {
        Ok(data) => {
            got_user_id = String::from_utf8_lossy(&data)
                .trim_matches(char::from(0))
                .to_string();
        },
        Err(_) => {
            // Fallback to detecting from access counter
            println!("Could not read user ID directly");
        }
    }
    
    println!("User: {}", got_user_id);
    
    // Get current access level from value file
    let current_level = match enhanced_access::get_value(card, enhanced_access::VALUE_FILE_ID) {
        Ok(value) => value as u8,
        Err(_) => 0
    };
    
    println!("Current access level: {}", current_level);
    println!("Enter new access level (0-5):");
    
    let mut level_str = String::new();
    io::stdin().read_line(&mut level_str)?;
    let new_level: u8 = match level_str.trim().parse() {
        Ok(l) => {
            if l > 5 {
                println!("Invalid level. Using maximum level 5.");
                5
            } else {
                l
            }
        },
        Err(_) => {
            println!("Invalid input. Using level 0.");
            0
        }
    };
    
    // Use value file for access level
    println!("Updating access level to {}...", new_level);
    if let Err(e) = enhanced_access::credit_value(card, enhanced_access::VALUE_FILE_ID, new_level as u32) {
        println!("Failed to update access level: {}", e);
        return Err("Access level update failed".into());
    }
    
    // Add record to access log
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;
    
    let mut log_entry = vec![0; 8];
    log_entry[0] = new_level;
    log_entry[1..5].copy_from_slice(&timestamp.to_le_bytes());
    
    if let Err(e) = enhanced_access::write_record(card, enhanced_access::RECORD_FILE_ID, &log_entry) {
        println!("Failed to write to access log: {}", e);
    } else {
        println!("Access log updated.");
    }
    
    println!("Access level for user '{}' updated from {} to {}!", got_user_id, current_level, new_level);
    Ok(())
}

fn check_access(card: &pcsc::Card, app_id: &[u8; 3]) -> Result<(), Box<dyn Error>> {
    // Authenticate and select application
    if let Err(e) = enhanced_access::authenticate_enhanced(card) {
        println!("Authentication failed: {}", e);
        return Err("Must authenticate before checking access".into());
    }
    
    if let Err(e) = enhanced_access::select_app(card, app_id) {
        println!("Failed to select application: {}", e);
        return Err("Application selection failed".into());
    }
    
    if let Err(e) = enhanced_access::authenticate_enhanced(card) {
        println!("Authentication failed in application context: {}", e);
        return Err("Must authenticate in application context".into());
    }
    
    // Read user data
    println!("Reading user information...");
    let user_id = match enhanced_access::read_file_data(card, enhanced_access::USER_FILE_ID, 32) {
        Ok(data) => {
            String::from_utf8_lossy(&data)
                .trim_matches(char::from(0))
                .to_string()
        },
        Err(_) => {
            // Fallback to empty string if we can't read the user ID
            String::new()  
        }
    };
    
    // Read access level from value file
    let access_level = match enhanced_access::get_value(card, enhanced_access::VALUE_FILE_ID) {
        Ok(value) => value as u8,
        Err(_) => 0
    };
    
    // Read access count is the same as access level in this approach
    let access_count = access_level as u32;
    
    // Try to read access log
    let mut log_entries = Vec::new();
    if let Ok(log_data) = enhanced_access::read_records(card, enhanced_access::RECORD_FILE_ID, 4) {
        for chunk in log_data.chunks(8) {
            if chunk.len() >= 5 {
                let level = chunk[0];
                let timestamp = u32::from_le_bytes([chunk[1], chunk[2], chunk[3], chunk[4]]);
                log_entries.push((level, timestamp));
            }
        }
    }
    
    // Display access information
    println!("\n===== ACCESS CONTROL INFORMATION =====");
    println!("User ID: {}", user_id);
    println!("Access Level: {}", access_level);
    println!("Access Count: {}", access_count);
    
    // Determine access for different areas
    println!("\nAccess Permissions:");
    println!("Main Entrance: {}", if access_level >= 0 { "GRANTED" } else { "DENIED" });
    println!("Office Area: {}", if access_level >= 1 { "GRANTED" } else { "DENIED" });
    println!("Meeting Rooms: {}", if access_level >= 2 { "GRANTED" } else { "DENIED" });
    println!("R&D Department: {}", if access_level >= 3 { "GRANTED" } else { "DENIED" });
    println!("Server Room: {}", if access_level >= 4 { "GRANTED" } else { "DENIED" });
    println!("Executive Suite: {}", if access_level >= 5 { "GRANTED" } else { "DENIED" });
    
    if !log_entries.is_empty() {
        println!("\nAccess History:");
        for (i, (level, timestamp)) in log_entries.iter().enumerate() {
            let datetime = chrono::NaiveDateTime::from_timestamp_opt(*timestamp as i64, 0)
                .unwrap_or_default()
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();
            println!("{}. Level changed to {} on {}", i+1, level, datetime);
        }
    }
    
    Ok(())
}
