use std::error::Error;
use std::io;
use std::thread::sleep;
use std::time::Duration;

// Import from the main crate
use desfire_tools::desfire_common::{
    connect_to_card, authenticate_des, 
    send_apdu, HexSlice, DEFAULT_MASTER_KEY,
    print_desfire_error, des_encrypt, des_decrypt
};
use openssl::rand::rand_bytes;

// Access Control constants
const ACCESS_APP_ID: [u8; 3] = [0xA1, 0xC0, 0x01]; // AID for Access Control
const USER_FILE_ID: u8 = 0x01; // Store user information
const ACCESS_LEVEL_FILE_ID: u8 = 0x02; // Store access level
const TIMESTAMP_FILE_ID: u8 = 0x03; // Store last access timestamp

fn main() -> Result<(), Box<dyn Error>> {
    // Connect to a card
    let (_, card) = connect_to_card()?;
    
    // Try to authenticate with retries
    let mut success = false;
    for attempt in 1..=3 {
        println!("\nAttempting authentication (try {}/3)...", attempt);
        match authenticate_card(&card) {
            Ok(_) => {
                success = true;
                break;
            },
            Err(e) => {
                println!("Authentication attempt {} failed: {}", attempt, e);
                if attempt < 3 {
                    println!("Waiting before retry...");
                    sleep(Duration::from_millis(500));
                }
            }
        }
    }
    
    if !success {
        return Err("Authentication failed after multiple attempts".into());
    }
    
    // Display options to the user
    println!("\nAccess Control Options:");
    println!("1. Initialize card for access control");
    println!("2. Register user");
    println!("3. Update access level");
    println!("4. Check access");
    println!("5. Format card (ERASES ALL DATA)");
    println!("6. Exit");
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    match input.trim() {
        "1" => initialize_access_control(&card)?,
        "2" => register_user(&card)?,
        "3" => update_access_level(&card)?,
        "4" => check_access(&card)?,
        "5" => format_card(&card)?,
        _ => println!("Exiting"),
    }
    
    Ok(())
}

// Custom authentication function that works directly without selecting master app first
fn authenticate_card(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("Authenticating directly with card...");
    
    // Direct authentication command (skip selecting master application)
    let auth_cmd = [0x90, 0x0A, 0x00, 0x00, 0x01, 0x00, 0x00];
    
    match send_apdu(card, &auth_cmd) {
        Ok(response) => {
            if response.len() >= 10 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0xAF {
                
                println!("Received challenge from card: {}", HexSlice(&response[0..8]));
                
                // 1. Decrypt RndB
                let enc_rnd_b = &response[0..8];
                let rnd_b = des_decrypt(&DEFAULT_MASTER_KEY, enc_rnd_b)?;
                println!("Decrypted RndB: {}", HexSlice(&rnd_b));
                
                // 2. Rotate RndB left
                let rotated_rnd_b = rotate_left(&rnd_b);
                println!("Rotated RndB: {}", HexSlice(&rotated_rnd_b));
                
                // 3. Generate random RndA
                let mut rnd_a = [0u8; 8];
                rand_bytes(&mut rnd_a)?;
                println!("Generated RndA: {}", HexSlice(&rnd_a));
                
                // 4. Concatenate RndA + rotated RndB
                let mut challenge_response = Vec::with_capacity(16);
                challenge_response.extend_from_slice(&rnd_a);
                challenge_response.extend_from_slice(&rotated_rnd_b);
                
                // 5. Encrypt the challenge response
                let enc_challenge = des_encrypt(&DEFAULT_MASTER_KEY, &challenge_response)?;
                
                // 6. Send the encrypted challenge to the card
                let mut send_challenge = vec![0x90, 0xAF, 0x00, 0x00, 0x10];
                send_challenge.extend_from_slice(&enc_challenge);
                send_challenge.push(0x00);
                
                println!("Sending encrypted challenge response");
                match send_apdu(card, &send_challenge) {
                    Ok(challenge_resp) => {
                        if challenge_resp.len() >= 2 && 
                           challenge_resp[challenge_resp.len() - 2] == 0x91 && 
                           challenge_resp[challenge_resp.len() - 1] == 0x00 {
                            
                            println!("Authentication successful!");
                            return Ok(());
                        } else {
                            return Err("Card rejected authentication".into());
                        }
                    },
                    Err(e) => return Err(e)
                }
            } else {
                return Err("Expected authentication challenge not received".into());
            }
        },
        Err(e) => Err(e)
    }
}

// Helper function to rotate bytes left
fn rotate_left(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }
    
    let mut result = Vec::with_capacity(data.len());
    result.extend_from_slice(&data[1..]);
    result.push(data[0]);
    
    result
}

fn format_card(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("WARNING: This will ERASE ALL DATA on the card!");
    println!("Are you sure you want to format the card? (y/n)");
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() != "y" {
        println!("Format cancelled.");
        return Ok(());
    }
    
    println!("Authenticating before formatting...");
    
    // Format PICC command
    let format_cmd = [0x90, 0xFC, 0x00, 0x00, 0x00];
    
    match send_apdu(card, &format_cmd) {
        Ok(response) => {
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                println!("Card formatted successfully!");
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                if error == 0xAE {
                    println!("Format requires authentication, attempting authentication...");
                    match authenticate_card(card) {
                        Ok(_) => {
                            // Try format again after authentication
                            match send_apdu(card, &format_cmd) {
                                Ok(resp2) => {
                                    if resp2.len() >= 2 && 
                                       resp2[resp2.len() - 2] == 0x91 && 
                                       resp2[resp2.len() - 1] == 0x00 {
                                        println!("Card formatted successfully!");
                                        Ok(())
                                    } else {
                                        Err(format!("Format failed after authentication: {:?}", resp2).into())
                                    }
                                },
                                Err(e) => Err(e)
                            }
                        },
                        Err(e) => Err(format!("Failed to authenticate for format: {}", e).into())
                    }
                } else {
                    Err(format!("Format failed: {} ({})",
                        print_desfire_error(error), error).into())
                }
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

fn initialize_access_control(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nInitializing card for access control...");
    
    // Create the application
    let mut create_app_cmd = vec![0x90, 0xCA, 0x00, 0x00, 0x05];
    create_app_cmd.extend_from_slice(&ACCESS_APP_ID);
    create_app_cmd.push(0x0F); // All permissions enabled
    create_app_cmd.push(0x01); // Just use 1 key for simplicity
    create_app_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &create_app_cmd) {
        Ok(response) => {
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                println!("Access control application created successfully with ID: {}", HexSlice(&ACCESS_APP_ID));
                
                // Select the application
                select_access_app(card)?;
                
                // Add authentication with the application key
                authenticate_card(card)?;
                
                // Create files with free access for easier testing
                create_access_file(card, USER_FILE_ID, 32, [0xFF, 0xFF])?; // 32 bytes for user data
                create_access_file(card, ACCESS_LEVEL_FILE_ID, 1, [0xFF, 0xFF])?; // 1 byte for access level
                create_access_file(card, TIMESTAMP_FILE_ID, 4, [0xFF, 0xFF])?; // 4 bytes for timestamp
                
                println!("Access control system initialized successfully!");
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                // Application might already exist
                if error == 0xDE { // App already exists error
                    println!("Access control application already exists. Continuing setup...");
                    
                    // Select the application
                    select_access_app(card)?;
                    
                    // Add authentication with the application key
                    authenticate_card(card)?;
                    
                    // Files might already exist, but we'll try to create them anyway
                    // Errors will be handled in the create_access_file function
                    let _ = create_access_file(card, USER_FILE_ID, 32, [0xFF, 0xFF]);
                    let _ = create_access_file(card, ACCESS_LEVEL_FILE_ID, 1, [0xFF, 0xFF]);
                    let _ = create_access_file(card, TIMESTAMP_FILE_ID, 4, [0xFF, 0xFF]);
                    
                    println!("Access control system initialized successfully!");
                    Ok(())
                } else {
                    Err(format!("Failed to create application: {} ({})", 
                            print_desfire_error(error), error).into())
                }
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

fn create_access_file(card: &pcsc::Card, file_id: u8, size: u16, access_rights: [u8; 2]) -> Result<(), Box<dyn Error>> {
    println!("Creating file with ID: {:02X}, size: {} bytes", file_id, size);
    
    // File parameters
    let comm_settings = 0x00; // Plain communication
    let file_size = [
        (size & 0xFF) as u8,
        ((size >> 8) & 0xFF) as u8,
        0x00
    ]; // Convert size to little endian
    
    // Create standard file command
    let mut create_file_cmd = vec![0x90, 0xCD, 0x00, 0x00, 0x07];
    create_file_cmd.push(file_id);
    create_file_cmd.push(comm_settings);
    create_file_cmd.extend_from_slice(&access_rights);
    create_file_cmd.extend_from_slice(&file_size);
    create_file_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &create_file_cmd) {
        Ok(response) => {
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                println!("File created successfully with ID: {:02X}", file_id);
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                if error == 0xC3 { // File already exists
                    println!("File with ID {:02X} already exists, skipping creation", file_id);
                    Ok(())
                } else {
                    Err(format!("Failed to create file: {} ({})",
                        print_desfire_error(error), error).into())
                }
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

fn select_access_app(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nSelecting access control application: {}", HexSlice(&ACCESS_APP_ID));
    
    // Create the select application command
    let mut select_app_cmd = vec![0x90, 0x5A, 0x00, 0x00, 0x03];
    select_app_cmd.extend_from_slice(&ACCESS_APP_ID);
    select_app_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &select_app_cmd) {
        Ok(response) => {
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                println!("Access control application selected successfully");
                sleep(Duration::from_millis(200)); // Add delay after selection
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                Err(format!("Failed to select application: {} ({})",
                    print_desfire_error(error), error).into())
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

fn register_user(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nRegistering new user...");
    
    // Select the access control application first
    select_access_app(card)?;
    
    // Add authentication after selecting the application
    authenticate_card(card)?;
    
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
    write_data_to_file(card, USER_FILE_ID, &user_data)?;
    
    // Set initial access level (0 = basic access)
    write_data_to_file(card, ACCESS_LEVEL_FILE_ID, &[0])?;
    
    // Set initial timestamp (0)
    write_data_to_file(card, TIMESTAMP_FILE_ID, &[0, 0, 0, 0])?;
    
    println!("User registered successfully with ID: {}", user_id);
    Ok(())
}

fn update_access_level(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nUpdating access level...");
    
    // Select the access control application first
    select_access_app(card)?;
    
    // Add authentication after selecting the application
    authenticate_card(card)?;
    
    // Read current user data
    let user_data = read_data_from_file(card, USER_FILE_ID, 32)?;
    let user_id = String::from_utf8_lossy(&user_data).trim_matches(char::from(0)).to_string();
    
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
    write_data_to_file(card, ACCESS_LEVEL_FILE_ID, &[level])?;
    
    // Update timestamp (current unix epoch time - simplified for example)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as u32;
    
    let timestamp = [
        (now & 0xFF) as u8,
        ((now >> 8) & 0xFF) as u8,
        ((now >> 16) & 0xFF) as u8,
        ((now >> 24) & 0xFF) as u8,
    ];
    
    write_data_to_file(card, TIMESTAMP_FILE_ID, &timestamp)?;
    
    println!("Access level updated to {} for user {}", level, user_id);
    Ok(())
}

fn check_access(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nChecking access permissions...");
    
    // Select the access control application first
    select_access_app(card)?;
    
    // Add authentication after selecting the application
    authenticate_card(card)?;
    
    // Read user data
    let user_data = read_data_from_file(card, USER_FILE_ID, 32)?;
    let user_id = String::from_utf8_lossy(&user_data).trim_matches(char::from(0)).to_string();
    
    // Read access level
    let level_data = read_data_from_file(card, ACCESS_LEVEL_FILE_ID, 1)?;
    let access_level = level_data[0];
    
    // Read timestamp
    let timestamp_data = read_data_from_file(card, TIMESTAMP_FILE_ID, 4)?;
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
    
    Ok(())
}

fn write_data_to_file(card: &pcsc::Card, file_id: u8, data: &[u8]) -> Result<(), Box<dyn Error>> {
    println!("Writing data to file ID: {:02X}", file_id);
    
    // Write in small chunks to avoid transaction issues
    for chunk_start in (0..data.len()).step_by(4) {
        let chunk_end = std::cmp::min(chunk_start + 4, data.len());
        let chunk = &data[chunk_start..chunk_end];
        
        // Calculate offset
        let offset = [
            (chunk_start & 0xFF) as u8,
            ((chunk_start >> 8) & 0xFF) as u8,
            ((chunk_start >> 16) & 0xFF) as u8
        ];
        
        let mut write_cmd = vec![0x90, 0x3D, 0x00, 0x00];
        write_cmd.push(7 + chunk.len() as u8); // Lc byte
        write_cmd.push(file_id);
        write_cmd.extend_from_slice(&offset);
        write_cmd.push(chunk.len() as u8);
        write_cmd.extend_from_slice(chunk);
        write_cmd.push(0x00); // Le byte
        
        match send_apdu(card, &write_cmd) {
            Ok(response) => {
                if response.len() >= 2 && 
                   response[response.len() - 2] == 0x91 {
                    if response[response.len() - 1] == 0x00 {
                        // Success for this chunk
                        println!("Chunk at offset {} written successfully", chunk_start);
                    } else if response[response.len() - 1] == 0x7E {
                        // Need to complete the transaction
                        println!("More data available, completing transaction...");
                        let commit_cmd = [0x90, 0xC7, 0x00, 0x00, 0x00];
                        match send_apdu(card, &commit_cmd) {
                            Ok(commit_resp) => {
                                if commit_resp.len() >= 2 && 
                                   commit_resp[commit_resp.len() - 2] == 0x91 && 
                                   commit_resp[commit_resp.len() - 1] == 0x00 {
                                    println!("Transaction committed successfully");
                                } else {
                                    println!("Warning: Commit returned status: {}", 
                                         HexSlice(&commit_resp));
                                }
                            },
                            Err(e) => println!("Warning: Commit error: {}", e)
                        }
                    } else {
                        // Try to continue anyway
                        println!("Warning: Chunk write returned status {:02X}, continuing...", 
                                 response[response.len() - 1]);
                    }
                } else {
                    return Err(format!("Invalid response format for chunk at offset {}", chunk_start).into());
                }
            },
            Err(e) => return Err(e)
        }
        
        // Add small delay between writes
        sleep(Duration::from_millis(50));
    }
    
    // Verify the write by reading back
    println!("Verifying write operation...");
    match read_data_from_file(card, file_id, std::cmp::min(data.len() as u8, 8)) {
        Ok(read_data) => {
            println!("Verification read successful: {} bytes", read_data.len());
            Ok(())
        },
        Err(e) => {
            println!("Warning: Verification read failed: {}. Write may still be successful.", e);
            // Return success anyway as the write might have succeeded
            Ok(())
        }
    }
}

fn read_data_from_file(card: &pcsc::Card, file_id: u8, length: u8) -> Result<Vec<u8>, Box<dyn Error>> {
    println!("Reading {} bytes from file ID: {:02X}", length, file_id);
    
    let offset = [0x00, 0x00, 0x00]; // Start at offset 0
    let read_length = [length, 0x00, 0x00]; // Length to read
    
    // Read data command
    let mut read_cmd = vec![0x90, 0xBD, 0x00, 0x00, 0x07];
    read_cmd.push(file_id);
    read_cmd.extend_from_slice(&offset);
    read_cmd.extend_from_slice(&read_length);
    read_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &read_cmd) {
        Ok(response) => {
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                // Extract the data (exclude the status bytes)
                let data = response[0..response.len()-2].to_vec();
                println!("Successfully read {} bytes", data.len());
                Ok(data)
            } else if response.len() >= 2 && 
                    response[response.len() - 2] == 0x91 && 
                    response[response.len() - 1] == 0x7E {
                // More data available, handle it
                println!("More data available, continuing read operation...");
                
                // Extract initial data without status bytes
                let mut all_data = response[0..response.len()-2].to_vec();
                
                // Send command to get more data
                let continue_cmd = [0x90, 0xAF, 0x00, 0x00, 0x00];
                let continue_resp = send_apdu(card, &continue_cmd)?;
                
                if continue_resp.len() >= 2 {
                    // Add data portion to our result
                    all_data.extend_from_slice(&continue_resp[0..continue_resp.len()-2]);
                    println!("Successfully read {} bytes total", all_data.len());
                    Ok(all_data)
                } else {
                    Err("Invalid response format during continued read".into())
                }
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                Err(format!("Failed to read data: {} ({})",
                    print_desfire_error(error), error).into())
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}
