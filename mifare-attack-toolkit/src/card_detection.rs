// src/card_detection.rs
use std::error::Error;
use std::thread;
use std::time::Duration;

use crate::reader::MifareClassic;

// Import constants directly from reader module
use crate::reader::{MI_OK, PICC_REQIDL};

/// Enhanced card detection function - FIXED to match working code
pub fn detect_card(reader: &mut MifareClassic) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    // FIXED: Use simple approach from working code
    let (status, _) = reader.request_card(PICC_REQIDL)?;
    if status != MI_OK {
        return Ok(None);
    }
    
    // Anti-collision
    let (status, uid) = reader.anticoll()?;
    if status != MI_OK {
        return Ok(None);
    }
    
    Ok(Some(uid))
}

/// Wait for a card to be placed on the reader - FIXED with simpler approach
pub fn wait_for_card_enhanced(reader: &mut MifareClassic, timeout_secs: u64) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    println!("Hold a card near the reader...");
    println!("You have {} seconds to place a card", timeout_secs);
    
    // FIXED: Make sure reader is in a clean state with simpler approach
    reader.stop_crypto1()?;
    
    // FIXED: Simplified reset approach
    reader.antenna_off()?;
    thread::sleep(Duration::from_millis(50));
    reader.antenna_on()?;
    thread::sleep(Duration::from_millis(50));
    
    let start_time = std::time::Instant::now();
    let timeout_duration = Duration::from_secs(timeout_secs);
    
    // Try detection
    while start_time.elapsed() < timeout_duration {
        match detect_card(reader)? {
            Some(uid) => {
                println!("Card detected! UID: {}", reader.format_uid(&uid));
                return Ok(Some(uid));
            },
            None => {}
        }
        
        // Wait before next attempt
        thread::sleep(Duration::from_millis(100));
    }
    
    println!("No card detected in the given time frame.");
    Ok(None)
}
