// src/utils.rs
use std::error::Error;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use crate::reader::MifareClassic;

/// Wait for a card to be removed
pub fn wait_for_card_removal(reader: &mut MifareClassic) -> Result<(), Box<dyn Error>> {
    println!("Please remove the card from the reader...");
    
    let mut card_present = true;
    while card_present {
        // Attempt to detect card
        match reader.get_uid() {
            Ok(Some(_)) => {
                // Card still present, wait a moment
                thread::sleep(Duration::from_millis(100));
            },
            _ => {
                // Card removed
                card_present = false;
            }
        }
    }
    
    println!("Card removed");
    Ok(())
}

/// Wait for a card with simplified approach to avoid type parameter issues
pub fn wait_for_card(reader: &mut MifareClassic, timeout_secs: u64, _detect_fn: impl Fn(&mut MifareClassic) -> Result<Option<Vec<u8>>, Box<dyn Error>>) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    println!("Hold a card near the reader...");
    println!("You have {} seconds to place a card", timeout_secs);
    
    // Make sure reader is in a clean state
    reader.stop_crypto1()?;
    
    // Reset antenna
    reader.antenna_off()?;
    thread::sleep(Duration::from_millis(50));
    reader.antenna_on()?;
    thread::sleep(Duration::from_millis(50));
    
    let start_time = std::time::Instant::now();
    let timeout_duration = Duration::from_secs(timeout_secs);
    
    // Try detection - directly using get_uid instead of callback to avoid type issues
    while start_time.elapsed() < timeout_duration {
        match reader.get_uid()? {
            Some(uid) => {
                println!("Card detected! UID: {}", format_uid(&uid));
                return Ok(Some(uid.to_vec()));
            },
            None => {}
        }
        
        // Wait before next attempt
        thread::sleep(Duration::from_millis(100));
    }
    
    println!("No card detected in the given time frame.");
    Ok(None)
}

/// Format a byte slice to a hex string
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|byte| format!("{:02X}", byte))
        .collect::<Vec<String>>()
        .join(" ")
}

/// Format a byte slice to an ASCII string
pub fn bytes_to_ascii(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|&byte| {
            if byte >= 32 && byte <= 126 {
                byte as char
            } else {
                '.'
            }
        })
        .collect()
}

/// Format a card UID as a hex string
pub fn format_uid(uid: &[u8]) -> String {
    uid.iter()
        .map(|byte| format!("{:02X}", byte))
        .collect::<Vec<String>>()
        .join(":")
}

/// Convert a hex string to a byte vector
pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    let hex = hex.replace(" ", "").replace(":", "");
    if hex.len() % 2 != 0 {
        return Err("Invalid hex string length".to_string());
    }
    
    let mut result = Vec::with_capacity(hex.len() / 2);
    for i in (0..hex.len()).step_by(2) {
        let byte = u8::from_str_radix(&hex[i..i+2], 16)
            .map_err(|_| "Invalid hex character".to_string())?;
        result.push(byte);
    }
    
    Ok(result)
}

/// Get user confirmation (y/n)
pub fn get_user_confirmation(prompt: &str) -> bool {
    print!("{} (y/n): ", prompt);
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    
    input.trim().to_lowercase() == "y"
}

/// Wait for user to press Enter
pub fn wait_for_enter() {
    print!("Press Enter to continue...");
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
}
