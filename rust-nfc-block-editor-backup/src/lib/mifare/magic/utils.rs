// ---------- src/lib/mifare/magic/utils.rs ----------
// Common utility functions for magic card operations

use std::error::Error;
use std::thread::sleep;
use std::time::Duration;
use rppal::spi::Spi;

use crate::lib::mfrc522::{
    mfrc522_request, mfrc522_anticoll, mfrc522_select_tag, 
    PICC_REQIDL, MI_OK
};

// Constants for operations
pub const MAX_RETRIES: u8 = 3;
pub const DELAY_BETWEEN_OPS: u64 = 50; // milliseconds

/// Format a magic card key for display (convert bytes to hex string)
pub fn format_magic_key(key: &[u8]) -> String {
    key.iter()
       .map(|byte| format!("{:02X}", byte))
       .collect::<Vec<String>>()
       .join(" ")
}

/// Retry an operation multiple times
pub fn retry_operation<T, F>(mut operation: F, max_retries: u8) -> Result<T, Box<dyn Error>>
where
    F: FnMut() -> Result<T, Box<dyn Error>>,
{
    let mut last_error = None;
    
    for _ in 0..max_retries {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                sleep(Duration::from_millis(DELAY_BETWEEN_OPS));
            }
        }
    }
    
    Err(last_error.unwrap_or_else(|| "Unknown error during retry".into()))
}

/// Attempt to reconnect to a card after operations
pub fn reconnect_to_card(spi: &mut Spi, card_uid: &[u8]) -> Result<bool, Box<dyn Error>> {
    // Small pause to allow the card to settle
    sleep(Duration::from_millis(DELAY_BETWEEN_OPS));
    
    // Request card again
    let (status, _) = match retry_operation(|| mfrc522_request(spi, PICC_REQIDL), MAX_RETRIES) {
        Ok(result) => result,
        Err(_) => return Ok(false),
    };
    
    if status != MI_OK {
        return Ok(false);
    }
    
    // Get UID again
    let (status, uid) = match retry_operation(|| mfrc522_anticoll(spi), MAX_RETRIES) {
        Ok(result) => result,
        Err(_) => return Ok(false),
    };
    
    if status != MI_OK {
        return Ok(false);
    }
    
    // Verify it's the same card
    if uid.len() != card_uid.len() {
        return Ok(false);
    }
    
    for (i, byte) in uid.iter().enumerate() {
        if i >= card_uid.len() || *byte != card_uid[i] {
            return Ok(false);
        }
    }
    
    // Select the card again
    let size = match retry_operation(|| mfrc522_select_tag(spi, card_uid), MAX_RETRIES) {
        Ok(result) => result,
        Err(_) => return Ok(false),
    };
    
    if size == 0 {
        return Ok(false);
    }
    
    Ok(true)
}

/// Handle UID write failures with appropriate error messages
pub fn handle_uid_write_failure(status: u8, error_msg: &str) -> Result<(), Box<dyn Error>> {
    match status {
        MI_OK => Ok(()),
        _ => {
            println!("Error: {} (status: {})", error_msg, status);
            Err(format!("{} (status: {})", error_msg, status).into())
        }
    }
}
