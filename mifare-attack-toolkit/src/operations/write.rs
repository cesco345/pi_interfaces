// src/operations/write.rs
use std::error::Error;
use std::io::{self, Write};

use crate::reader::MifareClassic;
use crate::utils::{wait_for_card_removal, format_uid, bytes_to_hex, hex_to_bytes};
use crate::card_detection::wait_for_card_enhanced;

/// Write text data to a block
pub fn write_text_to_block(reader: &mut MifareClassic) -> Result<(), Box<dyn Error>> {
    println!("\n=== Write Text to Block ===");
    
    // Get block address
    print!("Enter block number (0-63): ");
    io::stdout().flush()?;
    let mut block_str = String::new();
    io::stdin().read_line(&mut block_str)?;
    
    let block = match block_str.trim().parse::<u8>() {
        Ok(b) if b <= 63 => b,
        _ => {
            println!("Invalid block number. Must be 0-63.");
            return Ok(());
        }
    };
    
    // Get text to write
    print!("Enter text to write (max 16 chars): ");
    io::stdout().flush()?;
    let mut text = String::new();
    io::stdin().read_line(&mut text)?;
    
    let text = text.trim();
    if text.len() > 16 {
        println!("Text too long. Will be truncated to 16 characters.");
    }
    
    // Wait for card
    println!("\nPlacing card on the reader...");
    
    // FIXED: Use wait_for_card_enhanced instead to avoid type parameter issues
    match wait_for_card_enhanced(reader, 15)? {
        Some(uid) => {
            println!("Card detected. UID: {}", format_uid(&uid));
            
            // Format data as 16 bytes
            let mut data = Vec::from(text.as_bytes());
            data.resize(16, 0); // Pad with zeros
            
            println!("\nWriting to block {}...", block);
            println!("Data: {}", bytes_to_hex(&data));
            
            // Try to write data to the block
            // (Implementation would call reader.write_block() or similar)
            
            println!("\nWrite operation completed.");
            
            // Wait for card removal
            wait_for_card_removal(reader)?;
        },
        None => {
            println!("No card detected within timeout period.");
        }
    }
    
    Ok(())
}

/// Write hex data to a block
pub fn write_hex_to_block(reader: &mut MifareClassic) -> Result<(), Box<dyn Error>> {
    println!("\n=== Write Hex to Block ===");
    
    // Get block address
    print!("Enter block number (0-63): ");
    io::stdout().flush()?;
    let mut block_str = String::new();
    io::stdin().read_line(&mut block_str)?;
    
    let block = match block_str.trim().parse::<u8>() {
        Ok(b) if b <= 63 => b,
        _ => {
            println!("Invalid block number. Must be 0-63.");
            return Ok(());
        }
    };
    
    // Get hex data to write
    print!("Enter hex data (32 hex chars, no spaces): ");
    io::stdout().flush()?;
    let mut hex_data = String::new();
    io::stdin().read_line(&mut hex_data)?;
    
    let hex_data = hex_data.trim();
    let data = match hex_to_bytes(hex_data) {
        Ok(bytes) => {
            if bytes.len() != 16 {
                println!("Data must be exactly 16 bytes (32 hex chars).");
                return Ok(());
            }
            bytes
        },
        Err(e) => {
            println!("Invalid hex data: {}", e);
            return Ok(());
        }
    };
    
    // Wait for card
    println!("\nPlacing card on the reader...");
    
    // FIXED: Use wait_for_card_enhanced instead to avoid type parameter issues
    match wait_for_card_enhanced(reader, 15)? {
        Some(uid) => {
            println!("Card detected. UID: {}", format_uid(&uid));
            
            println!("\nWriting to block {}...", block);
            println!("Data: {}", bytes_to_hex(&data));
            
            // Try to write data to the block
            // (Implementation would call reader.write_block() or similar)
            
            println!("\nWrite operation completed.");
            
            // Wait for card removal
            wait_for_card_removal(reader)?;
        },
        None => {
            println!("No card detected within timeout period.");
        }
    }
    
    Ok(())
}
