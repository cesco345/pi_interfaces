// ---------- src/lib/mifare/magic/detect/activation.rs ----------
// Magic card activation sequence tests

use std::error::Error;
use std::thread::sleep;
use std::time::Duration;
use rppal::spi::Spi;

use crate::lib::mfrc522::{
    mfrc522_write, mfrc522_to_card, 
    MI_OK, PCD_TRANSCEIVE
};

use super::types::TestResult;
use super::utils::format_data_as_hex;
use super::super::{reconnect_to_card};

/// Test various activation sequences for different magic cards
pub fn test_activation_sequences(spi: &mut Spi, card_uid: &[u8], block0: &[u8; 16]) -> Result<TestResult, Box<dyn Error>> {
    println!("Test 5: Testing activation sequences...");
    
    let mut result = TestResult::new("Activation sequence test");
    
    // Common activation commands for different magic card types
    let activation_commands = [
        vec![0x40, 0x43], // CUID activation
        vec![0x40, 0x00], // Direct write command
        vec![0x40, 0x01], // FUID activation start
        vec![0x90, 0xFB], // Gen3 mode command
        vec![0x72, 0x00], // FUID activation command
    ];
    
    for (i, cmd) in activation_commands.iter().enumerate() {
        if reconnect_to_card(spi, card_uid)? {
            println!("   Trying activation sequence {}...", i+1);
            
            // Send the activation command
            match mfrc522_to_card(spi, PCD_TRANSCEIVE, cmd) {
                Ok(_) => {},
                Err(_) => continue,
            };
            sleep(Duration::from_millis(50));
            
            // Try to write after activation
            match mfrc522_write(spi, 0, block0) {
                Ok(status) if status == MI_OK => {
                    println!("✅ Successfully wrote to block 0 after activation sequence {}!", i+1);
                    println!("   This confirms it's a Magic Card that requires activation!");
                    result.set_passed(8);
                    result.add_note(&format!("Responds to activation sequence {}", i+1));
                    return Ok(result);
                },
                _ => {}
            }
        }
    }
    
    // If standard activation failed, try card-specific activation
    if card_uid.len() >= 2 && reconnect_to_card(spi, card_uid)? {
        println!("   Testing card-specific activation...");
        
        // Create a card-specific command using the UID
        let specific_cmd = vec![0x40, card_uid[1]];
        mfrc522_to_card(spi, PCD_TRANSCEIVE, &specific_cmd).ok();
        sleep(Duration::from_millis(50));
        
        match mfrc522_write(spi, 0, block0) {
            Ok(status) if status == MI_OK => {
                println!("✅ Successfully wrote to block 0 after card-specific activation!");
                result.set_passed(8);
                result.add_note("Responds to card-specific activation command");
                return Ok(result);
            },
            _ => {}
        }
    }
    
    println!("❌ Card did not respond to activation sequences");
    
    if !reconnect_to_card(spi, card_uid)? {
        println!("Card disconnected after activation tests.");
    }
    
    Ok(result)
}
