// File: src/commands/ndef_commands.rs
use crate::util::ndef_util::{hex_string, interpret_status_code};

// Send APDU command to card with logging and response parsing
pub fn send_apdu(card: &pcsc::Card, apdu: &[u8], description: &str) -> Option<Vec<u8>> {
    println!("\n{}", description);
    println!("  → Sending: {}", hex_string(apdu));
    
    let mut recv_buffer = [0; 258]; // Max APDU response
    match card.transmit(apdu, &mut recv_buffer) {
        Ok(response) => {
            println!("  ← Response: {}", hex_string(response));
            
            // Parse SW1SW2 status bytes if response length >= 2
            if response.len() >= 2 {
                let sw1 = response[response.len() - 2];
                let sw2 = response[response.len() - 1];
                let status = format!("{:02X}{:02X}", sw1, sw2);
                
                println!("  ← Status: {} - {}", status, interpret_status_code(sw1, sw2));
                
                // Return data without status bytes if successful
                if sw1 == 0x90 && sw2 == 0x00 {
                    if response.len() > 2 {
                        // Return data without status bytes
                        Some(response[..response.len() - 2].to_vec())
                    } else {
                        // Success but no data
                        Some(Vec::new())
                    }
                } else if sw1 == 0x61 {
                    // More data available, try to fetch it with GET RESPONSE
                    println!("  ← More data available ({}  bytes), fetching with GET RESPONSE", sw2);
                    return get_response(card, sw2);
                } else {
                    None
                }
            } else {
                None
            }
        },
        Err(e) => {
            println!("  ✗ Error: {}", e);
            None
        }
    }
}

// Silent version of send_apdu that doesn't print details unless data is found
pub fn send_apdu_silent(card: &pcsc::Card, apdu: &[u8], _description: &str) -> Option<Vec<u8>> {
    let mut recv_buffer = [0; 258];
    
    match card.transmit(apdu, &mut recv_buffer) {
        Ok(response) => {
            if response.len() >= 2 {
                let sw1 = response[response.len() - 2];
                let sw2 = response[response.len() - 1];
                
                if sw1 == 0x90 && sw2 == 0x00 && response.len() > 2 {
                    // Success with data
                    return Some(response[..response.len() - 2].to_vec());
                }
            }
            None
        },
        Err(_) => None
    }
}

// Retrieve additional data using GET RESPONSE APDU
fn get_response(card: &pcsc::Card, length: u8) -> Option<Vec<u8>> {
    let apdu = [0x00, 0xC0, 0x00, 0x00, length];
    let mut recv_buffer = [0; 258];
    
    match card.transmit(&apdu, &mut recv_buffer) {
        Ok(response) => {
            println!("  ← GET RESPONSE: {}", hex_string(response));
            
            if response.len() >= 2 {
                let sw1 = response[response.len() - 2];
                let sw2 = response[response.len() - 1];
                
                if sw1 == 0x90 && sw2 == 0x00 && response.len() > 2 {
                    return Some(response[..response.len() - 2].to_vec());
                }
            }
            None
        },
        Err(e) => {
            println!("  ✗ GET RESPONSE Error: {}", e);
            None
        }
    }
}
