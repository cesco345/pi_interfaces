// ---------- src/lib/mifare/magic/detect/util.rs ----------
// Utility functions for magic card detection

use std::error::Error;
use rppal::spi::Spi;

use crate::lib::mfrc522::{
    mfrc522_request, mfrc522_anticoll, mfrc522_select_tag, 
    PICC_REQIDL, MI_OK
};
use super::super::utils::{retry_operation, MAX_RETRIES};

/// Format data as a hex string for display
pub fn format_data_as_hex(data: &[u8]) -> String {
    data.iter()
       .map(|byte| format!("{:02X}", byte))
       .collect::<Vec<String>>()
       .join(" ")
}

/// Select the card and get its UID
pub fn select_card(spi: &mut Spi) -> Result<Option<(Vec<u8>, u8)>, Box<dyn Error>> {
    // Request tag
    let (status, _) = match retry_operation(|| mfrc522_request(spi, PICC_REQIDL), MAX_RETRIES) {
        Ok(result) => result,
        Err(e) => {
            println!("\nError detecting card: {:?}", e);
            return Ok(None);
        }
    };
    
    if status != MI_OK {
        println!("Error: Could not detect card.");
        return Ok(None);
    }
    
    // Anti-collision and get UID
    let (status, card_uid_slice) = match retry_operation(|| mfrc522_anticoll(spi), MAX_RETRIES) {
        Ok(result) => result,
        Err(e) => {
            println!("\nError during anticollision: {:?}", e);
            return Ok(None);
        }
    };
    
    if status != MI_OK {
        println!("Error: Could not read card UID.");
        return Ok(None);
    }
    
    // Convert to a sized type (Vec<u8>) from slice
    let card_uid = card_uid_slice.to_vec();
    
    // Select card
    let size = match retry_operation(|| mfrc522_select_tag(spi, &card_uid), MAX_RETRIES) {
        Ok(result) => result,
        Err(e) => {
            println!("\nError selecting card: {:?}", e);
            return Ok(None);
        }
    };
    
    if size == 0 {
        println!("Error: Could not select card.");
        return Ok(None);
    }
    
    Ok(Some((card_uid, size)))
}
