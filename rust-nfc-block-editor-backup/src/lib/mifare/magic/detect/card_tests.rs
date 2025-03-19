// ---------- src/lib/mifare/magic/detect/card_tests.rs ----------
// Basic card behavior tests

use std::error::Error;
use rppal::spi::Spi;
use crate::lib::mfrc522::{
    mfrc522_read, mfrc522_auth, mfrc522_stop_crypto1,
    mfrc522_to_card, PICC_AUTHENT1A, PICC_AUTHENT1B, MI_OK, PCD_TRANSCEIVE
};
use super::types::TestResult;
use super::utils::format_data_as_hex;
use super::super::{reconnect_to_card};

/// Test various read methods to see if the card allows non-standard reads
pub fn test_read_methods(spi: &mut Spi, card_uid: &[u8]) -> Result<(TestResult, Option<Vec<u8>>), Box<dyn Error>> {
    println!("Test 1: Testing read capabilities...");
    
    let mut result = TestResult::new("Read capability test");
    
    // Try reading block 0 without authentication (shouldn't work on standard cards)
    println!("1.1: Direct read without authentication...");
    match mfrc522_read(spi, 0) {
        Ok(Some(data)) => {
            println!("✅ Successfully read block 0 without authentication!");
            println!("   Block 0 data: {}", format_data_as_hex(&data));
            result.set_passed(2);
            result.add_note("Card allows reading without authentication");
            
            if !reconnect_to_card(spi, card_uid)? {
                println!("Card disconnected after read test.");
            }
            
            return Ok((result, Some(data)));
        },
        _ => {
            println!("❌ Direct read failed (normal for secure cards)");
        }
    }
    
    // Try alternate read command
    if reconnect_to_card(spi, card_uid)? {
        println!("1.2: Testing alternative read command...");
        
        // Try Proxmark-style direct read command
        let read_cmd = [0x30, 0x00]; // READ block 0
        let (status, data, _) = match mfrc522_to_card(spi, PCD_TRANSCEIVE, &read_cmd) {
            Ok(result) => result,
            Err(_) => (0, Vec::new(), 0),
        };
        
        if status == MI_OK && data.len() >= 16 {
            println!("✅ Successfully read block 0 with alternative command!");
            println!("   Block 0 data: {}", format_data_as_hex(&data));
            result.set_passed(2);
            result.add_note("Card responds to non-standard read commands");
            
            if !reconnect_to_card(spi, card_uid)? {
                println!("Card disconnected after alternative read test.");
            }
            
            return Ok((result, Some(data)));
        } else {
            println!("❌ Alternative read command also failed");
        }
    }
    
    if !reconnect_to_card(spi, card_uid)? {
        println!("Card disconnected after read tests.");
    }
    
    Ok((result, None))
}

/// Test various authentication methods to detect unusual behavior
pub fn test_authentication(spi: &mut Spi, card_uid: &[u8]) -> Result<(TestResult, Option<Vec<u8>>), Box<dyn Error>> {
    println!("Test 2: Testing authentication responses...");
    
    let mut result = TestResult::new("Authentication test");
    
    let mut block0_data: Option<Vec<u8>> = None;
    let mut unusual_auth_count = 0;
    
    // Test standard auth
    if reconnect_to_card(spi, card_uid)? {
        let key_a = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]; // Default key
        
        match mfrc522_auth(spi, PICC_AUTHENT1A, 0, &key_a, card_uid) {
            Ok(status) if status == MI_OK => {
                println!("✅ Card accepts standard KEY A authentication");
                
                // Try to read block 0
                match mfrc522_read(spi, 0) {
                    Ok(Some(data)) => {
                        println!("   Block 0 data: {}", format_data_as_hex(&data));
                        block0_data = Some(data);
                    },
                    _ => println!("   Could not read block 0 with KEY A")
                }
                
                mfrc522_stop_crypto1(spi)?;
            },
            _ => {
                println!("❌ Card rejected standard KEY A authentication");
                unusual_auth_count += 1; // Unusual for a card to reject standard auth
                mfrc522_stop_crypto1(spi)?;
            }
        }
    }
    
    // Test KEY B auth on block 0 (shouldn't work on standard cards)
    if reconnect_to_card(spi, card_uid)? {
        let key_b = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]; // Default key
        
        match mfrc522_auth(spi, PICC_AUTHENT1B, 0, &key_b, card_uid) {
            Ok(status) if status == MI_OK => {
                println!("✅ Card accepts KEY B authentication for block 0!");
                result.score += 1; // Suspicious but not definitive
                result.add_note("Accepts KEY B for block 0 (unusual)");
                unusual_auth_count += 1;
                
                // Try to read block 0
                if block0_data.is_none() {
                    match mfrc522_read(spi, 0) {
                        Ok(Some(data)) => {
                            println!("   Block 0 data: {}", format_data_as_hex(&data));
                            block0_data = Some(data);
                        },
                        _ => println!("   Could not read block 0 with KEY B")
                    }
                }
                
                mfrc522_stop_crypto1(spi)?;
            },
            _ => {
                println!("❌ Card rejected KEY B authentication (normal behavior)");
                mfrc522_stop_crypto1(spi)?;
            }
        }
    }
    
    // Try unusual key combinations
    let unusual_keys = [
        [0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // Null key
        [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5], // Common magic backdoor key
        [0xD3, 0xF7, 0xD3, 0xF7, 0xD3, 0xF7], // Another common backdoor
    ];
    
    for key in unusual_keys.iter() {
        if reconnect_to_card(spi, card_uid)? {
            match mfrc522_auth(spi, PICC_AUTHENT1A, 0, key, card_uid) {
                Ok(status) if status == MI_OK => {
                    println!("✅ Card accepts non-standard key: {}", format_data_as_hex(key));
                    result.score += 1;
                    result.add_note(&format!("Accepts unusual key: {}", format_data_as_hex(key)));
                    unusual_auth_count += 1;
                    
                    if block0_data.is_none() {
                        match mfrc522_read(spi, 0) {
                            Ok(Some(data)) => {
                                println!("   Block 0 data: {}", format_data_as_hex(&data));
                                block0_data = Some(data);
                            },
                            _ => {}
                        }
                    }
                    
                    mfrc522_stop_crypto1(spi)?;
                },
                _ => {
                    mfrc522_stop_crypto1(spi)?;
                }
            }
        }
    }
    
    // If the card accepted multiple unusual auth methods, it's suspicious
    if unusual_auth_count > 1 {
        result.passed = true;
        result.score += 1; // Extra point for multiple unusual auth patterns
        result.add_note(&format!("Card responds to {} unusual auth methods", unusual_auth_count));
    }
    
    if !reconnect_to_card(spi, card_uid)? {
        println!("Card disconnected after auth tests.");
    }
    
    Ok((result, block0_data))
}

/// Test unusual command sequences that might activate magic cards
pub fn test_unusual_commands(spi: &mut Spi, card_uid: &[u8]) -> Result<TestResult, Box<dyn Error>> {
    println!("Test 3: Testing response to unusual commands...");
    
    let mut result = TestResult::new("Unusual command test");
    
    // Generate a range of command sequences that might trigger magic cards
    // We'll try simple, common magic card commands first
    let command_sequences = [
        vec![0x40, 0x00], // Common activation command
        vec![0x40, 0x43], // CUID command
        vec![0x40, 0x01], // FUID command
        vec![0x90, 0xF0], // Chinese magic command
        vec![0x72, 0x00], // Another variant
    ];
    
    let mut unusual_responses = 0;
    
    for (i, cmd) in command_sequences.iter().enumerate() {
        if reconnect_to_card(spi, card_uid)? {
            println!("   Sending command sequence {}...", i+1);
            
            // Send the command and check for unusual responses
            let (status, response, _) = match mfrc522_to_card(spi, PCD_TRANSCEIVE, cmd) {
                Ok(result) => result,
                Err(_) => (0, Vec::new(), 0),
            };
            
            // Interpret the response - magic cards often give unusual responses
            let unusual = match status {
                MI_OK => {
                    // Got a success response to an unusual command
                    if !response.is_empty() {
                        println!("✅ Unusual response to command sequence {}: {}", 
                                i+1, format_data_as_hex(&response));
                        true
                    } else {
                        false
                    }
                },
                // Some cards respond with specific error codes that are actually indications
                9 | 7 | 10 => {
                    println!("✅ Card returned special status code {} to command sequence {}", 
                            status, i+1);
                    true
                },
                _ => false
            };
            
            if unusual {
                unusual_responses += 1;
                result.add_note(&format!("Unusual response to command sequence {}", i+1));
            }
            
            // If we get any unusual response, try a simple read/write afterwards
            // as the card might be in an activated state
            if unusual {
                // Try reading block 0 directly after the unusual command
                match mfrc522_read(spi, 0) {
                    Ok(Some(data)) => {
                        println!("✅ Successfully read block 0 after command sequence {}!", i+1);
                        println!("   Block 0 data: {}", format_data_as_hex(&data));
                        result.set_passed(2);
                        result.add_note("Card allows reading after special command");
                    },
                    _ => {}
                }
            }
        }
    }
    
    // If we got multiple unusual responses, that's suspicious
    if unusual_responses > 0 && !result.passed {
        result.score += unusual_responses;
        if unusual_responses > 1 {
            result.passed = true;
        }
    }
    
    if !reconnect_to_card(spi, card_uid)? {
        println!("Card disconnected after command tests.");
    }
    
    Ok(result)
}
