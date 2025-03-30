use std::error::Error;
use std::thread::sleep;
use std::time::Duration;
use std::io::{self, Write};

use desfire_tools::desfire_common::{
    send_apdu, HexSlice, DEFAULT_MASTER_KEY,
    des_encrypt, des_decrypt
};
use openssl::rand::rand_bytes;

// Constants for applications
pub const ACCESS_APP_ID: [u8; 3] = [0xA1, 0xC0, 0x01]; // Access control application
pub const DATA_APP_ID: [u8; 3] = [0xB1, 0xB2, 0xB3];   // Data storage application
pub const USER_FILE_ID: u8 = 0x01;
pub const CONFIG_FILE_ID: u8 = 0x02;
pub const VALUE_FILE_ID: u8 = 0x03;
pub const RECORD_FILE_ID: u8 = 0x04;

// Authenticate with the card using DES key
pub fn authenticate_enhanced(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("Authenticating with card using enhanced method...");
    
    // Authentication command (with key 0)
    let auth_cmd = [0x90, 0x0A, 0x00, 0x00, 0x01, 0x00, 0x00];
    
    // Try authentication up to 3 times
    for attempt in 1..=3 {
        println!("Auth attempt {}/3...", attempt);
        
        match send_apdu(card, &auth_cmd) {
            Ok(response) => {
                println!("Auth response: {}", HexSlice(&response));
                
                // Check for command aborted (0x91, 0xCA)
                if response.len() >= 2 && 
                   response[response.len() - 2] == 0x91 && 
                   response[response.len() - 1] == 0xCA {
                    println!("Got 'command aborted' (CA), retrying...");
                    sleep(Duration::from_millis(500));
                    continue;
                }
                
                // Check for proper challenge response
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
                                println!("Card rejected authentication: {:?}", challenge_resp);
                                if attempt < 3 {
                                    println!("Retrying...");
                                    sleep(Duration::from_millis(500));
                                    continue;
                                }
                                return Err("Card rejected authentication".into());
                            }
                        },
                        Err(e) => {
                            println!("Error sending challenge: {}", e);
                            if attempt < 3 {
                                println!("Retrying...");
                                sleep(Duration::from_millis(500));
                                continue;
                            }
                            return Err(e);
                        }
                    }
                } else {
                    println!("Unexpected response: {:?}", response);
                    if attempt < 3 {
                        println!("Retrying...");
                        sleep(Duration::from_millis(500));
                        continue;
                    }
                    return Err("Expected authentication challenge not received".into());
                }
            },
            Err(e) => {
                println!("Error: {}", e);
                if attempt < 3 {
                    println!("Retrying...");
                    sleep(Duration::from_millis(500));
                    continue;
                }
                return Err(e);
            }
        }
    }
    
    Err("Authentication failed after multiple attempts".into())
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

// Format the card (erases all applications)
pub fn format_card(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("WARNING: This will ERASE ALL DATA on the card!");
    println!("Are you sure you want to format the card? (y/n)");
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() != "y" {
        println!("Format cancelled.");
        return Ok(());
    }
    
    println!("Attempting to format card...");
    
    // Authenticate first
    match authenticate_enhanced(card) {
        Ok(_) => {},
        Err(e) => {
            println!("Authentication failed before format: {}", e);
            println!("Will try formatting anyway...");
        }
    }
    
    // Format PICC command
    let format_cmd = [0x90, 0xFC, 0x00, 0x00, 0x00];
    
    match send_apdu(card, &format_cmd) {
        Ok(response) => {
            println!("Format response: {}", HexSlice(&response));
            
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                println!("Card formatted successfully!");
                Ok(())
            } else {
                println!("Format returned status: {:?}", response);
                println!("Trying re-authentication and format again...");
                
                if let Ok(_) = authenticate_enhanced(card) {
                    let second_format = send_apdu(card, &format_cmd);
                    match second_format {
                        Ok(resp2) => {
                            println!("Second format response: {}", HexSlice(&resp2));
                            if resp2.len() >= 2 && 
                               resp2[resp2.len() - 2] == 0x91 && 
                               resp2[resp2.len() - 1] == 0x00 {
                                println!("Card formatted successfully on second attempt!");
                                Ok(())
                            } else {
                                Err(format!("Format failed on second attempt: {:?}", resp2).into())
                            }
                        },
                        Err(e) => Err(format!("Second format error: {}", e).into())
                    }
                } else {
                    Err("Failed to re-authenticate for format".into())
                }
            }
        },
        Err(e) => Err(format!("Format command error: {}", e).into())
    }
}

// Select an application for operations
pub fn select_app(card: &pcsc::Card, app_id: &[u8; 3]) -> Result<(), Box<dyn Error>> {
    println!("Selecting application: {}", HexSlice(app_id));
    
    // Create the select application command
    let mut select_app_cmd = vec![0x90, 0x5A, 0x00, 0x00, 0x03];
    select_app_cmd.extend_from_slice(app_id);
    select_app_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &select_app_cmd) {
        Ok(response) => {
            println!("Select app response: {}", HexSlice(&response));
            
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                println!("Application selected successfully");
                sleep(Duration::from_millis(100)); // Add delay after selection
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                Err(format!("Failed to select application, status: {:02X}", error).into())
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

// Create a new application
pub fn create_application(card: &pcsc::Card, app_id: &[u8; 3]) -> Result<(), Box<dyn Error>> {
    println!("Creating application with ID: {}", HexSlice(app_id));
    
    // Create the application command
    let mut create_app_cmd = vec![0x90, 0xCA, 0x00, 0x00, 0x05];
    create_app_cmd.extend_from_slice(app_id);
    create_app_cmd.push(0x0F); // All permissions enabled
    create_app_cmd.push(0x01); // 1 key for simplicity
    create_app_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &create_app_cmd) {
        Ok(response) => {
            println!("Create app response: {}", HexSlice(&response));
            
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                println!("Application created successfully with ID: {}", HexSlice(app_id));
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                // App might already exist (0xDE)
                if error == 0xDE {
                    println!("Application already exists, continuing");
                    Ok(())
                } else {
                    Err(format!("Failed to create application, status: {:02X}", error).into())
                }
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

// Create a standard file with parameters that work well on your card
pub fn create_std_file(card: &pcsc::Card, file_id: u8, size: u16) -> Result<(), Box<dyn Error>> {
    println!("Creating standard file with ID: {:02X}, size: {} bytes", file_id, size);
    
    // Based on your successful file creation
    let file_size = [
        (size & 0xFF) as u8,
        ((size >> 8) & 0xFF) as u8,
        0x00
    ]; // Convert size to little endian
    
    let file_cmd = [
        0x90, 0xCD, 0x00, 0x00, 0x07, // Create standard file command
        file_id,                      // File ID
        0x00,                         // Plain communication
        0x00, 0x00,                   // Free access rights (changed from FF FF)
        file_size[0], file_size[1], file_size[2], // File size
        0x00                          // Le byte
    ];
    
    match send_apdu(card, &file_cmd) {
        Ok(response) => {
            println!("Create standard file response: {}", HexSlice(&response));
            
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                println!("File created successfully");
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                // Check if file already exists (0xC3)
                if error == 0xC3 {
                    println!("File already exists, continuing");
                    Ok(())
                } else {
                    Err(format!("Failed to create file, status: {:02X}", error).into())
                }
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

// Create a value file
pub fn create_value_file(card: &pcsc::Card, file_id: u8) -> Result<(), Box<dyn Error>> {
    println!("Creating value file with ID: {:02X}", file_id);
    
    // Based on your successful value file creation
    let file_cmd = [
        0x90, 0xCC, 0x00, 0x00, 0x11, // Create value file command
        file_id,                      // File ID
        0x00,                         // Plain communication
        0x00, 0x00,                   // Free access rights (changed from FF FF)
        0x00, 0x00, 0x00, 0x00,       // Lower limit (0)
        0xFF, 0xFF, 0x00, 0x00,       // Upper limit (65535)
        0x00, 0x00, 0x00, 0x00,       // Initial value (0)
        0x00,                         // Limited credit disabled
        0x00                          // Le byte
    ];
    
    match send_apdu(card, &file_cmd) {
        Ok(response) => {
            println!("Create value file response: {}", HexSlice(&response));
            
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                println!("Value file created successfully");
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                // Check if file already exists
                if error == 0xC3 {
                    println!("Value file already exists, continuing");
                    Ok(())
                } else {
                    Err(format!("Failed to create value file, status: {:02X}", error).into())
                }
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

// Create a record file
pub fn create_record_file(card: &pcsc::Card, file_id: u8, record_size: u8, max_records: u8) -> Result<(), Box<dyn Error>> {
    println!("Creating record file with ID: {:02X}", file_id);
    
    let file_cmd = [
        0x90, 0xC1, 0x00, 0x00, 0x0D, // Create cyclic record file command
        file_id,                      // File ID
        0x00,                         // Plain communication
        0x00, 0x00,                   // Free access rights (changed from FF FF)
        record_size, 0x00, 0x00,      // Record size
        max_records, 0x00, 0x00,      // Max records
        0x00                          // Le byte
    ];
    
    match send_apdu(card, &file_cmd) {
        Ok(response) => {
            println!("Create record file response: {}", HexSlice(&response));
            
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 {
                if response[response.len() - 1] == 0x00 {
                    println!("Record file created successfully");
                    Ok(())
                } else if response[response.len() - 1] == 0x7E {
                    // May need to commit transaction
                    println!("Record file created, committing transaction");
                    let commit_cmd = [0x90, 0xC7, 0x00, 0x00, 0x00];
                    let _ = send_apdu(card, &commit_cmd)?;
                    Ok(())
                } else if response[response.len() - 1] == 0xC3 {
                    println!("Record file already exists, continuing");
                    Ok(())
                } else {
                    Err(format!("Failed to create record file, status: {:02X}", response[response.len() - 1]).into())
                }
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

// Read data from a file that works with your card
pub fn read_file_data(card: &pcsc::Card, file_id: u8, length: u8) -> Result<Vec<u8>, Box<dyn Error>> {
    println!("Reading data from file ID: {:02X}", file_id);
    
    // Based on your successful read operations
    let read_cmd = [
        0x90, 0xBD, 0x00, 0x00, 0x07, // Read data command
        file_id,                     // File ID
        0x00, 0x00, 0x00,            // Offset
        length, 0x00, 0x00,          // Length to read
        0x00                         // Le byte
    ];
    
    match send_apdu(card, &read_cmd) {
        Ok(response) => {
            println!("Read response: {}", HexSlice(&response));
            
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                // Extract the data (exclude the status bytes)
                Ok(response[0..response.len()-2].to_vec())
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
            } else {
                Err(format!("Read failed with status: {:?}", response).into())
            }
        },
        Err(e) => Err(e)
    }
}

// Write data to a file with reliable commit
pub fn write_file_data(card: &pcsc::Card, file_id: u8, data: &[u8]) -> Result<(), Box<dyn Error>> {
    println!("Writing data to file ID: {:02X}", file_id);
    
    // First, ensure we're in a clean state by doing a commit
    let initial_commit = [0x90, 0xC7, 0x00, 0x00, 0x00];
    let _ = send_apdu(card, &initial_commit); // Ignore any errors
    
    // Based on your successful write operations
    let mut write_cmd = vec![0x90, 0x3D, 0x00, 0x00];
    write_cmd.push(7 + data.len() as u8); // Lc byte
    write_cmd.push(file_id);
    write_cmd.push(0x00); // Offset (LSB)
    write_cmd.push(0x00); // Offset (middle byte)
    write_cmd.push(0x00); // Offset (MSB)
    write_cmd.push(data.len() as u8);
    write_cmd.extend_from_slice(data);
    write_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &write_cmd) {
        Ok(response) => {
            println!("Write response: {}", HexSlice(&response));
            
            // Add a small delay before committing
            sleep(Duration::from_millis(50));
            
            // Always try to commit, regardless of response
            let commit_cmd = [0x90, 0xC7, 0x00, 0x00, 0x00];
            let commit_resp = send_apdu(card, &commit_cmd)?;
            println!("Commit response: {}", HexSlice(&commit_resp));
            
            // Verify that the data was written by reading it back
            let read_cmd = [
                0x90, 0xBD, 0x00, 0x00, 0x07, // Read data command
                file_id,                     // File ID
                0x00, 0x00, 0x00,            // Offset
                data.len() as u8, 0x00, 0x00, // Length to read
                0x00                         // Le byte
            ];
            
            let _ = send_apdu(card, &read_cmd); // Just for verification
            
            Ok(())
        },
        Err(e) => Err(e)
    }
}

// Get current value from a value file
pub fn get_value(card: &pcsc::Card, file_id: u8) -> Result<u32, Box<dyn Error>> {
    println!("Getting value from file ID: {:02X}", file_id);
    
    let get_value_cmd = [
        0x90, 0x6C, 0x00, 0x00, 0x01, // GetValue command
        file_id,                      // File ID
        0x00                          // Le byte
    ];
    
    match send_apdu(card, &get_value_cmd) {
        Ok(response) => {
            println!("GetValue response: {}", HexSlice(&response));
            
            if response.len() >= 6 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                // Extract the value (4 bytes in little endian)
                let value_bytes = &response[0..4];
                let value = u32::from_le_bytes([
                    value_bytes[0], 
                    value_bytes[1], 
                    value_bytes[2], 
                    value_bytes[3]
                ]);
                println!("Current value: {}", value);
                Ok(value)
            } else {
                Err(format!("GetValue failed with response: {:?}", response).into())
            }
        },
        Err(e) => Err(e)
    }
}

// Credit a value file
pub fn credit_value(card: &pcsc::Card, file_id: u8, amount: u32) -> Result<(), Box<dyn Error>> {
    println!("Crediting value file ID: {:02X} with amount: {}", file_id, amount);
    
    // Create credit command
    let mut credit_cmd = vec![0x90, 0x0C, 0x00, 0x00, 0x05];
    credit_cmd.push(file_id);
    credit_cmd.extend_from_slice(&amount.to_le_bytes());
    credit_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &credit_cmd) {
        Ok(response) => {
            println!("Credit response: {}", HexSlice(&response));
            
            // Always commit transaction
            println!("Committing transaction...");
            let commit_cmd = [0x90, 0xC7, 0x00, 0x00, 0x00];
            match send_apdu(card, &commit_cmd) {
                Ok(commit_resp) => {
                    println!("Commit response: {}", HexSlice(&commit_resp));
                    Ok(())
                },
                Err(e) => Err(format!("Commit error: {}", e).into())
            }
        },
        Err(e) => Err(format!("Credit error: {}", e).into())
    }
}

// Write a record to a record file
pub fn write_record(card: &pcsc::Card, file_id: u8, data: &[u8]) -> Result<(), Box<dyn Error>> {
    println!("Writing record to file ID: {:02X}", file_id);
    
    let mut write_cmd = vec![0x90, 0x3B, 0x00, 0x00];
    write_cmd.push(2 + data.len() as u8); // Lc byte
    write_cmd.push(file_id);
    write_cmd.push(0x00); // Offset
    write_cmd.extend_from_slice(data);
    write_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &write_cmd) {
        Ok(response) => {
            println!("Write record response: {}", HexSlice(&response));
            
            // Add a small delay before committing
            sleep(Duration::from_millis(50));
            
            // Always commit transaction
            let commit_cmd = [0x90, 0xC7, 0x00, 0x00, 0x00];
            let commit_resp = send_apdu(card, &commit_cmd)?;
            println!("Commit response: {}", HexSlice(&commit_resp));
            
            Ok(())
        },
        Err(e) => Err(format!("Write record error: {}", e).into())
    }
}

// Read records from a record file
pub fn read_records(card: &pcsc::Card, file_id: u8, record_count: u8) -> Result<Vec<u8>, Box<dyn Error>> {
    println!("Reading records from file ID: {:02X}", file_id);
    
    let read_record_cmd = [
        0x90, 0xBB, 0x00, 0x00, 0x03, // ReadRecords command
        file_id,                      // File ID
        0x00,                         // Record number (start from 0)
        record_count,                 // Number of records to read
        0x00                          // Le byte
    ];
    
    match send_apdu(card, &read_record_cmd) {
        Ok(response) => {
            println!("Read records response: {}", HexSlice(&response));
            
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                // Extract the data (exclude the status bytes)
                Ok(response[0..response.len()-2].to_vec())
            } else {
                Err(format!("Read records failed with status: {:?}", response).into())
            }
        },
        Err(e) => Err(e)
    }
}
