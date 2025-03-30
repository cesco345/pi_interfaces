// src/bin/access_simple.rs
use std::error::Error;
use std::io;
use std::thread::sleep;
use std::time::Duration;
use std::io::Write;

// Import from the main crate
use desfire_tools::desfire_common::{
    connect_to_card, send_apdu, HexSlice, DEFAULT_MASTER_KEY, 
    print_desfire_error, des_encrypt, des_decrypt
};
use openssl::rand::rand_bytes;

// Access Control constants
const ACCESS_APP_ID: [u8; 3] = [0xA1, 0xC0, 0x01];
const USER_FILE_ID: u8 = 0x01;
const ACCESS_LEVEL_FILE_ID: u8 = 0x02;
const TIMESTAMP_FILE_ID: u8 = 0x03;

fn main() -> Result<(), Box<dyn Error>> {
    // Connect to a card
    let (_, card) = connect_to_card()?;
    
    // Authenticate directly (our custom function)
    authenticate_card(&card)?;
    
    // Display options to the user
    println!("\nSimple Access Control:");
    println!("1. Initialize card");
    println!("2. Register user");
    println!("3. Exit");
    
    print!("Select option: ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    match input.trim() {
        "1" => initialize_simple(&card)?,
        "2" => register_simple_user(&card)?,
        _ => println!("Exiting"),
    }
    
    Ok(())
}

// Authenticate function as defined above
fn authenticate_card(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    // ... authentication code as above
}

fn rotate_left(data: &[u8]) -> Vec<u8> {
    // ... rotate code as above
}

fn initialize_simple(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nCreating simple access application...");
    
    // Create application
    let mut create_app_cmd = vec![0x90, 0xCA, 0x00, 0x00, 0x05];
    create_app_cmd.extend_from_slice(&ACCESS_APP_ID);
    create_app_cmd.push(0x0F); // All permissions
    create_app_cmd.push(0x01); // Just 1 key for simplicity
    create_app_cmd.push(0x00); // Le byte
    
    let create_result = send_apdu(card, &create_app_cmd)?;
    
    if !(create_result.len() >= 2 && 
        create_result[create_result.len() - 2] == 0x91 && 
        (create_result[create_result.len() - 1] == 0x00 || 
         create_result[create_result.len() - 1] == 0xDE)) { // DE = already exists
        
        println!("Failed to create application: {}", HexSlice(&create_result));
        return Err("Application creation failed".into());
    }
    
    // Select the application
    let mut select_app_cmd = vec![0x90, 0x5A, 0x00, 0x00, 0x03];
    select_app_cmd.extend_from_slice(&ACCESS_APP_ID);
    select_app_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &select_app_cmd) {
        Ok(select_result) => {
            if !(select_result.len() >= 2 && 
                select_result[select_result.len() - 2] == 0x91 && 
                select_result[select_result.len() - 1] == 0x00) {
                
                println!("Failed to select application: {}", HexSlice(&select_result));
                return Err("Application selection failed".into());
            }
        },
        Err(e) => return Err(e)
    }
    
    // Re-authenticate with the application
    authenticate_card(card)?;
    
    // Create a simple file with free access for testing
    println!("Creating user file...");
    
    let mut create_file_cmd = vec![0x90, 0xCD, 0x00, 0x00, 0x07];
    create_file_cmd.push(USER_FILE_ID);
    create_file_cmd.push(0x00); // Plain communication
    create_file_cmd.push(0xFF); // Free access
    create_file_cmd.push(0xFF); // Free access
    create_file_cmd.push(32);   // 32 bytes
    create_file_cmd.push(0x00);
    create_file_cmd.push(0x00);
    create_file_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &create_file_cmd) {
        Ok(create_file_result) => {
            if !(create_file_result.len() >= 2 && 
                create_file_result[create_file_result.len() - 2] == 0x91 && 
                (create_file_result[create_file_result.len() - 1] == 0x00 || 
                 create_file_result[create_file_result.len() - 1] == 0xC3)) { // C3 = already exists
                
                println!("Failed to create file: {}", HexSlice(&create_file_result));
                return Err("File creation failed".into());
            }
        },
        Err(e) => return Err(e)
    }
    
    println!("Card initialized successfully for simple access control!");
    Ok(())
}

fn register_simple_user(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nRegistering simple user...");
    
    // Select the application
    let mut select_app_cmd = vec![0x90, 0x5A, 0x00, 0x00, 0x03];
    select_app_cmd.extend_from_slice(&ACCESS_APP_ID);
    select_app_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &select_app_cmd) {
        Ok(select_result) => {
            if !(select_result.len() >= 2 && 
                select_result[select_result.len() - 2] == 0x91 && 
                select_result[select_result.len() - 1] == 0x00) {
                
                println!("Failed to select application: {}", HexSlice(&select_result));
                return Err("Application selection failed".into());
            }
        },
        Err(e) => return Err(e)
    }
    
    // Re-authenticate with the application
    authenticate_card(card)?;
    
    // Get user ID
    println!("Enter user ID (max 10 chars):");
    let mut user_id = String::new();
    io::stdin().read_line(&mut user_id)?;
    let user_id = user_id.trim();
    
    // Create simple data
    let mut data = vec![0; 32];
    let bytes = user_id.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if i < data.len() {
            data[i] = *b;
        }
    }
    
    // Write using small chunks
    for i in 0..4 { // Write in 8-byte chunks
        let start = i * 8;
        let end = std::cmp::min(start + 8, data.len());
        let chunk = &data[start..end];
        
        // Calculate offset
        let offset = [start as u8, 0x00, 0x00];
        
        let mut write_cmd = vec![0x90, 0x3D, 0x00, 0x00];
        write_cmd.push(7 + chunk.len() as u8); // Lc byte
        write_cmd.push(USER_FILE_ID);
        write_cmd.extend_from_slice(&offset);
        write_cmd.push(chunk.len() as u8);
        write_cmd.extend_from_slice(chunk);
        write_cmd.push(0x00); // Le byte
        
        match send_apdu(card, &write_cmd) {
            Ok(write_result) => {
                if write_result.len() >= 2 {
                    let status = write_result[write_result.len() - 1];
                    if status != 0x00 && status != 0x7E {
                        println!("Write failed with status: {:02X}", status);
                    } else {
                        println!("Chunk {} written successfully", i+1);
                    }
                }
            },
            Err(e) => println!("Error writing chunk {}: {}", i+1, e)
        }
    }
    
    // Try to commit regardless
    let commit_cmd = [0x90, 0xC7, 0x00, 0x00, 0x00];
    match send_apdu(card, &commit_cmd) {
        Ok(commit_result) => {
            println!("Commit result: {}", HexSlice(&commit_result));
        },
        Err(e) => println!("Commit error: {}", e)
    }
    
    // Try to read back
    println!("\nTrying to read user data:");
    let mut read_cmd = vec![0x90, 0xBD, 0x00, 0x00, 0x07];
    read_cmd.push(USER_FILE_ID);
    read_cmd.push(0x00); // Offset (LSB)
    read_cmd.push(0x00); // Offset (middle byte)
    read_cmd.push(0x00); // Offset (MSB)
    read_cmd.push(8);    // Just read 8 bytes
    read_cmd.push(0x00); // Length (middle byte)
    read_cmd.push(0x00); // Length (MSB)
    read_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &read_cmd) {
        Ok(read_result) => {
            if read_result.len() > 2 {
                let data_portion = &read_result[0..read_result.len()-2];
                match std::str::from_utf8(data_portion) {
                    Ok(text) => println!("Read back: \"{}\"", text),
                    Err(_) => println!("Data (hex): {}", HexSlice(data_portion))
                }
                println!("User registered successfully!");
            } else {
                println!("Read returned insufficient data: {}", HexSlice(&read_result));
            }
        },
        Err(e) => println!("Read error: {}", e)
    }
    
    Ok(())
}
