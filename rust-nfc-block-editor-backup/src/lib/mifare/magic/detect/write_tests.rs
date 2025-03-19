// ---------- src/lib/mifare/magic/detect/write_tests.rs ----------
// Write capability tests

use std::error::Error;
use rppal::spi::Spi;
use crate::lib::mfrc522::{
    mfrc522_auth, mfrc522_write, mfrc522_stop_crypto1,
    PICC_AUTHENT1A, MI_OK
};
use super::types::TestResult;
use super::utils::format_data_as_hex;
use super::super::{reconnect_to_card};

/// Test safe write operations (writing same data back)
pub fn test_safe_write(spi: &mut Spi, card_uid: &[u8], block0: &[u8; 16]) -> Result<TestResult, Box<dyn Error>> {
    println!("Test 4: Testing safe write operations...");
    
    let mut result = TestResult::new("Safe write test");
    
    // Try direct write without authentication
    if reconnect_to_card(spi, card_uid)? {
        println!("4.1: Attempting direct write without authentication...");
        
        match mfrc522_write(spi, 0, block0) {
            Ok(status) if status == MI_OK => {
                println!("✅ Successfully wrote to block 0 WITHOUT authentication!");
                println!("   This is a definitive indicator of a Magic Card!");
                result.set_passed(10); // Very high score - this is definitive
                result.add_note("Allows direct writing to block 0 without authentication");
                return Ok(result);
            },
            _ => {
                println!("❌ Direct write failed (normal for secure cards)");
            }
        }
    }
    
    // Try authenticated write
    if reconnect_to_card(spi, card_uid)? {
        println!("4.2: Attempting write with standard authentication...");
        
        let std_key = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        match mfrc522_auth(spi, PICC_AUTHENT1A, 0, &std_key, card_uid) {
            Ok(status) if status == MI_OK => {
                match mfrc522_write(spi, 0, block0) {
                    Ok(status) if status == MI_OK => {
                        println!("✅ Successfully wrote to block 0 with authentication!");
                        println!("   This is strong evidence of a Magic Card!");
                        result.set_passed(7);
                        result.add_note("Allows writing to block 0 with standard authentication");
                        mfrc522_stop_crypto1(spi)?;
                        return Ok(result);
                    },
                    _ => {
                        println!("❌ Authenticated write failed (normal for secure cards)");
                        mfrc522_stop_crypto1(spi)?;
                    }
                }
            },
            _ => {
                println!("❌ Authentication failed");
                mfrc522_stop_crypto1(spi)?;
            }
        }
    }
    
    if !reconnect_to_card(spi, card_uid)? {
        println!("Card disconnected after write tests.");
    }
    
    Ok(result)
}

/// Test BCC modification (more invasive but definitive)
pub fn test_bcc_modification(spi: &mut Spi, card_uid: &[u8], block0: [u8; 16]) -> Result<TestResult, Box<dyn Error>> {
    println!("Test 6: Testing BCC modification (advanced test)...");
    
    let mut result = TestResult::new("BCC modification test");
    
    // Create a modified version of block 0 with a changed BCC byte
    let mut modified_block0 = block0.clone();
    
    // The BCC byte is typically the 5th byte (index 4)
    // We'll try to toggle just one bit to minimize risk
    let original_bcc = modified_block0[4];
    modified_block0[4] = original_bcc ^ 0x01; // Toggle just the least significant bit
    
    println!("   Original BCC: {:02X}, Modified BCC: {:02X}", original_bcc, modified_block0[4]);
    
    // Try various methods to write the modified block
    
    // Direct write first
    if reconnect_to_card(spi, card_uid)? {
        match mfrc522_write(spi, 0, &modified_block0) {
            Ok(status) if status == MI_OK => {
                println!("✅ Successfully modified BCC byte with direct write!");
                result.set_passed(10);
                result.add_note("Allows direct modification of BCC byte");
                
                // Restore original BCC
                let _ = mfrc522_write(spi, 0, &block0);
                return Ok(result);
            },
            _ => {}
        }
    }
    
    // Try with authentication
    if reconnect_to_card(spi, card_uid)? {
        let std_key = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        match mfrc522_auth(spi, PICC_AUTHENT1A, 0, &std_key, card_uid) {
            Ok(status) if status == MI_OK => {
                match mfrc522_write(spi, 0, &modified_block0) {
                    Ok(status) if status == MI_OK => {
                        println!("✅ Successfully modified BCC byte with authenticated write!");
                        result.set_passed(8);
                        result.add_note("Allows authenticated modification of BCC byte");
                        
                        // Restore original BCC
                        let _ = mfrc522_write(spi, 0, &block0);
                        mfrc522_stop_crypto1(spi)?;
                        return Ok(result);
                    },
                    _ => {
                        mfrc522_stop_crypto1(spi)?;
                    }
                }
            },
            _ => {
                mfrc522_stop_crypto1(spi)?;
            }
        }
    }
    
    println!("❌ BCC modification tests failed (normal for secure cards)");
    
    Ok(result)
}
