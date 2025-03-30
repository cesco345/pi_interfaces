use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use desfire_tools::desfire_common::{
    send_apdu, HexSlice, DEFAULT_MASTER_KEY,
    des_encrypt, des_decrypt
};
use openssl::rand::rand_bytes;

// Custom authentication function
pub fn authenticate_card(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("Attempting direct authentication...");
    
    // Direct authentication command with key 0
    let auth_cmd = [0x90, 0x0A, 0x00, 0x00, 0x01, 0x00, 0x00];
    
    // Try up to 3 times
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
pub fn rotate_left(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }
    
    let mut result = Vec::with_capacity(data.len());
    result.extend_from_slice(&data[1..]);
    result.push(data[0]);
    
    result
}

// Helper function to select an application
pub fn select_application(card: &pcsc::Card, app_id: &[u8; 3]) -> Result<(), Box<dyn Error>> {
    println!("Selecting application: {}", HexSlice(app_id));
    
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
                sleep(Duration::from_millis(200)); // Add delay after selection
                Ok(())
            } else {
                println!("Non-success response for select: {:?}", response);
                // Continue anyway as some clone cards return non-standard responses
                sleep(Duration::from_millis(200));
                Ok(())
            }
        },
        Err(e) => Err(e)
    }
}
