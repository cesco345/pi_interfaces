use openssl::symm::{Cipher, Crypter, Mode};
use openssl::rand::rand_bytes;
use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use crate::card::{send_apdu, select_master_application};
use crate::util::HexSlice;
use crate::error::print_desfire_error;

// DESFire default key (all zeros)
pub const DEFAULT_MASTER_KEY: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

/// Authenticate with the card using DES encryption
pub fn authenticate_des(card: &pcsc::Card, key_no: u8, key: &[u8; 8]) -> Result<(), Box<dyn Error>> {
    println!("\nAuthenticating with DES key: {}", HexSlice(key));
    
    // Reset card state by selecting PICC (master application)
    select_master_application(card)?;
    sleep(Duration::from_millis(100));
    
    // 1. Send authentication command for DES
    let auth_cmd = [0x90, 0x0A, 0x00, 0x00, 0x01, key_no, 0x00];
    
    match send_apdu(card, &auth_cmd) {
        Ok(response) => {
            if response.len() >= 2 && response[response.len() - 2] == 0x91 {
                if response[response.len() - 1] == 0xAF {
                    // Authentication started, card returned challenge (RndB)
                    println!("Received challenge from card: {}", HexSlice(&response[0..8]));
                    
                    // Small delay after receiving challenge
                    sleep(Duration::from_millis(50));
                    
                    // 2. Decrypt RndB
                    let enc_rnd_b = &response[0..8];
                    let rnd_b = des_decrypt(key, enc_rnd_b)?;
                    println!("Decrypted RndB: {}", HexSlice(&rnd_b));
                    
                    // 3. Rotate RndB left
                    let rotated_rnd_b = rotate_left(&rnd_b);
                    println!("Rotated RndB: {}", HexSlice(&rotated_rnd_b));
                    
                    // 4. Generate random RndA
                    let mut rnd_a = [0u8; 8];
                    rand_bytes(&mut rnd_a)?;
                    println!("Generated RndA: {}", HexSlice(&rnd_a));
                    
                    // 5. Concatenate RndA + rotated RndB
                    let mut challenge_response = Vec::with_capacity(16);
                    challenge_response.extend_from_slice(&rnd_a);
                    challenge_response.extend_from_slice(&rotated_rnd_b);
                    
                    // 6. Encrypt the challenge response
                    let enc_challenge = des_encrypt(key, &challenge_response)?;
                    
                    // Small delay before sending challenge response
                    sleep(Duration::from_millis(50));
                    
                    // 7. Send the encrypted challenge to the card
                    let mut send_challenge = vec![0x90, 0xAF, 0x00, 0x00, 0x10];
                    send_challenge.extend_from_slice(&enc_challenge);
                    send_challenge.push(0x00);
                    
                    println!("Sending encrypted challenge response");
                    match send_apdu(card, &send_challenge) {
                        Ok(challenge_resp) => {
                            if challenge_resp.len() >= 2 && 
                               challenge_resp[challenge_resp.len() - 2] == 0x91 && 
                               challenge_resp[challenge_resp.len() - 1] == 0x00 {
                                
                                println!("DES Authentication successful with key_no {}!", key_no);
                                return Ok(());
                            } else if challenge_resp.len() >= 2 {
                                let error = challenge_resp[challenge_resp.len() - 1];
                                return Err(format!("Card rejected authentication with status: {:02X} ({})",
                                         error, print_desfire_error(error)).into());
                            }
                        },
                        Err(e) => return Err(e)
                    }
                } else {
                    let error = response[response.len() - 1];
                    return Err(format!("Authentication error: {:02X} ({})", 
                              error, print_desfire_error(error)).into());
                }
            }
            Err("Unexpected response format".into())
        },
        Err(e) => Err(e)
    }
}

/// Helper function to rotate bytes left by one position
fn rotate_left(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }
    
    let mut result = Vec::with_capacity(data.len());
    result.extend_from_slice(&data[1..]);
    result.push(data[0]);
    
    result
}

/// DES encryption using OpenSSL
pub fn des_encrypt(key: &[u8], data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    if key.len() != 8 {
        return Err("DES key must be exactly 8 bytes".into());
    }
    
    // Make sure data is a multiple of 8 bytes (DES block size)
    let padded_data = if data.len() % 8 != 0 {
        let mut padded = data.to_vec();
        padded.resize((data.len() / 8 + 1) * 8, 0);
        padded
    } else {
        data.to_vec()
    };
    
    let cipher = Cipher::des_cbc();
    let iv = [0u8; 8]; // DESFire uses zero IV
    
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, Some(&iv))?;
    crypter.pad(false); // DESFire does not use padding
    
    let mut output = vec![0; padded_data.len() + cipher.block_size()];
    let count = crypter.update(&padded_data, &mut output)?;
    let rest = crypter.finalize(&mut output[count..])?;
    output.truncate(count + rest);
    
    Ok(output)
}

/// DES decryption using OpenSSL
pub fn des_decrypt(key: &[u8], data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    if key.len() != 8 {
        return Err("DES key must be exactly 8 bytes".into());
    }
    
    if data.len() % 8 != 0 {
        return Err("DES data must be a multiple of 8 bytes".into());
    }
    
    let cipher = Cipher::des_cbc();
    let iv = [0u8; 8]; // DESFire uses zero IV
    
    let mut crypter = Crypter::new(cipher, Mode::Decrypt, key, Some(&iv))?;
    crypter.pad(false); // DESFire does not use padding
    
    let mut output = vec![0; data.len() + cipher.block_size()];
    let count = crypter.update(data, &mut output)?;
    let rest = crypter.finalize(&mut output[count..])?;
    output.truncate(count + rest);
    
    Ok(output)
}
