use std::error::Error;
use std::io;
use std::thread::sleep;
use std::time::Duration;

// Import from the main crate
use desfire_tools::desfire_common::{
    connect_to_card, send_apdu, HexSlice, DEFAULT_MASTER_KEY,
    print_desfire_error, des_encrypt, des_decrypt
};
use openssl::rand::rand_bytes;

// Basic card constants
const ACCESS_APP_ID: [u8; 3] = [0xA0, 0xB0, 0xC0]; // Different AID to avoid conflicts
const TEST_FILE_ID: u8 = 0x01;

fn main() -> Result<(), Box<dyn Error>> {
    // Connect to a card
    let (_, card) = connect_to_card()?;
    
    // Try to authenticate
    println!("Authenticating with card...");
    match authenticate_card(&card) {
        Ok(_) => println!("Authentication successful!"),
        Err(e) => {
            println!("Authentication failed: {}", e);
            return Err("Cannot proceed without authentication".into());
        }
    }
    
    // Display options to the user
    println!("\nBasic Card Test Options:");
    println!("1. Format card (RESET ALL DATA)");
    println!("2. Create simple test application");
    println!("3. Write simple test data");
    println!("4. Read test data");
    println!("5. Exit");
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    match input.trim() {
        "1" => format_card(&card)?,
        "2" => create_test_app(&card)?,
        "3" => write_test_data(&card)?,
        "4" => read_test_data(&card)?,
        _ => println!("Exiting"),
    }
    
    Ok(())
}

// Custom authentication function that works directly
fn authenticate_card(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("Attempting direct authentication...");
    
    // Direct authentication command with key 0
    let auth_cmd = [0x90, 0x0A, 0x00, 0x00, 0x01, 0x00, 0x00];
    
    // Try up to 3 times due to occasional CA errors
    for attempt in 1..=3 {
        println!("Auth attempt {}/3...", attempt);
        
        match send_apdu(card, &auth_cmd) {
            Ok(response) => {
                if response.len() >= 10 && 
                   response[response.len() - 2] == 0x91 && 
                   response[response.len() - 1] == 0xAF {
                    
                    println!("Received challenge from card: {}", HexSlice(&response[0..8]));
                    
                    // 1. Decrypt RndB using DES
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
                                println!("Card rejected challenge: {:?}", challenge_resp);
                                if attempt < 3 {
                                    println!("Retrying...");
                                    sleep(Duration::from_millis(500));
                                    continue;
                                }
                                return Err("Card rejected authentication challenge".into());
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
                } else if response.len() >= 2 && 
                         response[response.len() - 2] == 0x91 && 
                         response[response.len() - 1] == 0xCA {
                    // Command aborted - try again
                    println!("Got 'command aborted' (CA), retrying...");
                    sleep(Duration::from_millis(500));
                    continue;
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
                println!("Auth command error: {}", e);
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

fn format_card(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("WARNING: This will ERASE ALL DATA on the card!");
    println!("Are you sure you want to format the card? (y/n)");
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() != "y" {
        println!("Format cancelled.");
        return Ok(());
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
            } else if response.len() >= 2 && 
                     response[response.len() - 2] == 0x91 {
                
                let error = response[response.len() - 1];
                println!("Format returned status: {:02X}", error);
                
                // Re-authenticate and try again
                println!("Re-authenticating and trying again...");
                match authenticate_card(card) {
                    Ok(_) => {
                        // Try format again
                        match send_apdu(card, &format_cmd) {
                            Ok(resp2) => {
                                println!("Second format response: {}", HexSlice(&resp2));
                                println!("Format procedure completed.");
                                Ok(())
                            },
                            Err(e) => Err(format!("Second format attempt failed: {}", e).into())
                        }
                    },
                    Err(e) => Err(format!("Re-authentication failed: {}", e).into())
                }
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(format!("Format command failed: {}", e).into())
    }
}

fn create_test_app(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nCreating simple test application...");
    
    // Create application with the simplest possible settings
    let mut create_app_cmd = vec![0x90, 0xCA, 0x00, 0x00, 0x05];
    create_app_cmd.extend_from_slice(&ACCESS_APP_ID);
    create_app_cmd.push(0x0F); // All permissions enabled
    create_app_cmd.push(0x01); // Just use 1 key
    create_app_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &create_app_cmd) {
        Ok(response) => {
            println!("Create app response: {}", HexSlice(&response));
            
            let app_created = response.len() >= 2 && 
                              response[response.len() - 2] == 0x91 && 
                              (response[response.len() - 1] == 0x00 || 
                               response[response.len() - 1] == 0xDE); // DE = already exists
            
            if !app_created {
                return Err(format!("Failed to create application: {:?}", response).into());
            }
            
            println!("Application created/already exists. Selecting it...");
            
            // Select the application
            select_test_app(card)?;
            
            // Authenticate with the application
            authenticate_card(card)?;
            
            // Create a very simple file with open access
            println!("Creating test file...");
            
            // Create standard file with completely open access
            // Access rights 0xFF 0xFF means any authenticated state can read/write
            let create_file_cmd = [
                0x90, 0xCD, 0x00, 0x00, 0x07, // Create standard file command
                TEST_FILE_ID,                  // File ID
                0x00,                          // Plain communication
                0xFF, 0xFF,                    // Full access rights
                0x10, 0x00, 0x00,              // 16 bytes size
                0x00                           // Le byte
            ];
            
            match send_apdu(card, &create_file_cmd) {
                Ok(file_response) => {
                    println!("Create file response: {}", HexSlice(&file_response));
                    
                    let file_created = file_response.len() >= 2 && 
                                      file_response[file_response.len() - 2] == 0x91 && 
                                      (file_response[file_response.len() - 1] == 0x00 || 
                                       file_response[file_response.len() - 1] == 0xC3); // C3 = already exists
                    
                    if file_created {
                        println!("Test file created successfully!");
                    } else {
                        println!("Warning: File creation may have failed: {:?}", file_response);
                    }
                    
                    println!("Test application setup completed.");
                    Ok(())
                },
                Err(e) => Err(format!("Failed to create file: {}", e).into())
            }
        },
        Err(e) => Err(format!("Failed to create application: {}", e).into())
    }
}

fn select_test_app(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("Selecting test application: {}", HexSlice(&ACCESS_APP_ID));
    
    // Create the select application command
    let mut select_app_cmd = vec![0x90, 0x5A, 0x00, 0x00, 0x03];
    select_app_cmd.extend_from_slice(&ACCESS_APP_ID);
    select_app_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &select_app_cmd) {
        Ok(response) => {
            println!("Select app response: {}", HexSlice(&response));
            
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                println!("Application selected successfully");
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

fn write_test_data(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nWriting test data...");
    
    // Select the application first
    select_test_app(card)?;
    
    // Authenticate with the application
    authenticate_card(card)?;
    
    // Simple test data - just 4 bytes
    let test_data = b"TEST";
    
    // Write data command - use very small data
    let mut write_cmd = vec![0x90, 0x3D, 0x00, 0x00];
    write_cmd.push(7 + test_data.len() as u8); // Lc byte
    write_cmd.push(TEST_FILE_ID);
    write_cmd.push(0x00); // Offset (LSB)
    write_cmd.push(0x00); // Offset (middle byte)
    write_cmd.push(0x00); // Offset (MSB)
    write_cmd.push(test_data.len() as u8);
    write_cmd.extend_from_slice(test_data);
    write_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &write_cmd) {
        Ok(response) => {
            println!("Write response: {}", HexSlice(&response));
            
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 {
                
                // Handle different response codes
                match response[response.len() - 1] {
                    0x00 => {
                        println!("Test data written successfully!");
                        Ok(())
                    },
                    0x7E => {
                        println!("More data available, completing transaction...");
                        
                        // Try commit transaction
                        let commit_cmd = [0x90, 0xC7, 0x00, 0x00, 0x00];
                        match send_apdu(card, &commit_cmd) {
                            Ok(commit_resp) => {
                                println!("Commit response: {}", HexSlice(&commit_resp));
                                println!("Write operation completed.");
                                Ok(())
                            },
                            Err(e) => Err(format!("Commit failed: {}", e).into())
                        }
                    },
                    _ => {
                        println!("Warning: Write returned status {:02X}", response[response.len() - 1]);
                        // Try to continue anyway
                        println!("Write operation completed with warnings.");
                        Ok(())
                    }
                }
            } else {
                Err(format!("Invalid write response: {:?}", response).into())
            }
        },
        Err(e) => Err(e)
    }
}

fn read_test_data(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nReading test data...");
    
    // Select the application first
    select_test_app(card)?;
    
    // Authenticate with the application
    authenticate_card(card)?;
    
    // Try different read approaches in sequence to determine what works
    let mut success = false;
    
    // Try standard read first
    println!("\nTrying standard read approach...");
    match read_standard(card) {
        Ok(data) => {
            println!("Standard read approach succeeded!");
            println!("Data read: {:?}", data);
            if let Ok(text) = std::str::from_utf8(&data) {
                println!("Data as text: \"{}\"", text);
            }
            success = true;
        },
        Err(e) => {
            println!("Standard read approach failed: {}", e);
        }
    }
    
    // If standard read failed, try GetValue approach
    if !success {
        println!("\nTrying GetValue read approach...");
        match read_get_value(card) {
            Ok(data) => {
                println!("GetValue approach succeeded!");
                println!("Data read: {:?}", data);
                if let Ok(text) = std::str::from_utf8(&data) {
                    println!("Data as text: \"{}\"", text);
                }
                success = true;
            },
            Err(e) => {
                println!("GetValue approach failed: {}", e);
            }
        }
    }
    
    // If both previous approaches failed, try chunked read
    if !success {
        println!("\nTrying chunked read approach...");
        match read_chunked(card) {
            Ok(data) => {
                println!("Chunked read approach succeeded!");
                println!("Data read: {:?}", data);
                if let Ok(text) = std::str::from_utf8(&data) {
                    println!("Data as text: \"{}\"", text);
                }
                success = true;
            },
            Err(e) => {
                println!("Chunked read approach failed: {}", e);
            }
        }
    }
    
    if success {
        println!("\nSuccessfully read data from card!");
        Ok(())
    } else {
        Err("All read approaches failed".into())
    }
}

// Standard read approach
fn read_standard(card: &pcsc::Card) -> Result<Vec<u8>, Box<dyn Error>> {
    let read_cmd = [
        0x90, 0xBD, 0x00, 0x00, 0x07, // Read data command
        TEST_FILE_ID,                  // File ID
        0x00, 0x00, 0x00,             // Offset
        0x04, 0x00, 0x00,             // Length (4 bytes)
        0x00                          // Le byte
    ];
    
    match send_apdu(card, &read_cmd) {
        Ok(response) => {
            println!("Standard read response: {}", HexSlice(&response));
            
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                // Extract the data (exclude the status bytes)
                Ok(response[0..response.len()-2].to_vec())
            } else {
                Err(format!("Standard read failed with status: {:?}", response).into())
            }
        },
        Err(e) => Err(e)
    }
}

// GetValue approach (try a different command)
fn read_get_value(card: &pcsc::Card) -> Result<Vec<u8>, Box<dyn Error>> {
    // GetValue command (might work if file is interpreted differently)
    let get_value_cmd = [
        0x90, 0x6C, 0x00, 0x00, 0x01, // GetValue command
        TEST_FILE_ID,                  // File ID
        0x00                          // Le byte
    ];
    
    match send_apdu(card, &get_value_cmd) {
        Ok(response) => {
            println!("GetValue response: {}", HexSlice(&response));
            
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                // Extract the data (exclude the status bytes)
                Ok(response[0..response.len()-2].to_vec())
            } else {
                Err(format!("GetValue failed with status: {:?}", response).into())
            }
        },
        Err(e) => Err(e)
    }
}

// Chunked read approach (byte by byte)
fn read_chunked(card: &pcsc::Card) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut result = Vec::new();
    
    // Try to read one byte at a time
    for offset in 0..4 {
        let read_cmd = [
            0x90, 0xBD, 0x00, 0x00, 0x07, // Read data command
            TEST_FILE_ID,                  // File ID
            offset, 0x00, 0x00,            // Offset
            0x01, 0x00, 0x00,             // Length (1 byte)
            0x00                          // Le byte
        ];
        
        match send_apdu(card, &read_cmd) {
            Ok(response) => {
                println!("Chunked read at offset {}: {}", offset, HexSlice(&response));
                
                if response.len() >= 3 && 
                   response[response.len() - 2] == 0x91 && 
                   response[response.len() - 1] == 0x00 {
                    // Add the byte to our result
                    result.push(response[0]);
                } else {
                    println!("Failed to read byte at offset {}", offset);
                    // Continue trying other bytes
                }
            },
            Err(e) => {
                println!("Error reading byte at offset {}: {}", offset, e);
                // Continue with next byte
            }
        }
    }
    
    if result.is_empty() {
        Err("Chunked read failed to get any data".into())
    } else {
        Ok(result)
    }
}
