// Core writing operations for MIFARE cards

use std::error::Error;
use pcsc::Card;
use crate::commands::ndef_commands::send_apdu;
use crate::util::ndef_util::hex_string;
use crate::operations::ndef_operations_writer::CardExport;
use super::authentication::authenticate_sector;

/// Simple direct write - just try to write to block 4 with default key
pub fn simple_direct_write(card: &Card, card_data: &CardExport) -> Result<(), Box<dyn Error>> {
    // Get text data to write
    let text_data = card_data.fileData.as_bytes();
    
    // Prepare data (pad or truncate to 16 bytes)
    let mut block_data = Vec::new();
    block_data.extend_from_slice(text_data);
    
    // Pad to 16 bytes
    while block_data.len() < 16 {
        block_data.push(0x00);
    }
    
    // Truncate if too long
    if block_data.len() > 16 {
        block_data.truncate(16);
    }
    
    // Try to authenticate with default key
    let load_key = [0xFF, 0x82, 0x00, 0x00, 0x06, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    
    if let Some(_) = send_apdu(card, &load_key, "Load Default Key") {
        // Try to authenticate block 4 (first data block in sector 1)
        let auth_cmd = [0xFF, 0x88, 0x00, 0x04, 0x60, 0x00];
        
        if let Some(_) = send_apdu(card, &auth_cmd, "Auth Block 4") {
            // Write data to block 4
            let mut write_cmd = vec![0xFF, 0xD6, 0x00, 0x04, 0x10];
            write_cmd.extend_from_slice(&block_data);
            
            if let Some(_) = send_apdu(card, &write_cmd, "Write Block 4") {
                return Ok(());
            }
        }
    }
    
    Err("Simple direct write failed".into())
}

/// Direct write to MIFARE without NDEF formatting - comprehensive method
pub fn direct_write_mifare(card: &Card, card_data: &CardExport) -> Result<(), Box<dyn Error>> {
    println!("\nTrying comprehensive direct data write to MIFARE Classic card...");
    
    // Convert data to bytes
    let data_str = &card_data.fileData;
    let data_bytes = data_str.as_bytes().to_vec();
    
    // Pad to 16 bytes
    let mut block_data = data_bytes.clone();
    while block_data.len() < 16 {
        block_data.push(0x00);
    }
    
    // If data exceeds 16 bytes, truncate and warn
    if block_data.len() > 16 {
        println!("Warning: Data too large, truncating to 16 bytes");
        block_data.truncate(16);
    }
    
    // Try writing to all data blocks in the first few sectors
    // Block 0 is manufacturer info, and every 4th block is a sector trailer
    let data_blocks = vec![
        // Sector 1
        4, 5, 6,
        // Sector 2
        8, 9, 10,
        // Sector 3
        12, 13, 14,
        // Try sector 0 as a last resort, but be careful - block 0 is read-only
        1, 2
    ];
    
    for block in &data_blocks {
        // Calculate which sector this block is in
        let sector = block / 4;
        println!("\nTrying to write to block {} (sector {})...", block, sector);
        
        // Using Key Type A (0x60)
        if authenticate_sector(card, sector, 0x60) {
            // Try to write the data
            let mut write_cmd = vec![0xFF, 0xD6, 0x00, *block, 0x10]; // Write 16 bytes
            write_cmd.extend_from_slice(&block_data);
            
            if let Some(_) = send_apdu(card, &write_cmd, &format!("Write Block {}", block)) {
                println!("Successfully wrote data to block {}", block);
                println!("\n✅ Data successfully written to MIFARE Classic card (Block {})!", block);
                return Ok(());
            } else {
                println!("Failed to write to block {}, trying next block...", block);
            }
        } else {
            // Try with Key Type B (0x61)
            if authenticate_sector(card, sector, 0x61) {
                // Try to write the data
                let mut write_cmd = vec![0xFF, 0xD6, 0x00, *block, 0x10]; // Write 16 bytes
                write_cmd.extend_from_slice(&block_data);
                
                if let Some(_) = send_apdu(card, &write_cmd, &format!("Write Block {} (Key B)", block)) {
                    println!("Successfully wrote data to block {} using Key B", block);
                    println!("\n✅ Data successfully written to MIFARE Classic card (Block {})!", block);
                    return Ok(());
                } else {
                    println!("Failed to write to block {} with Key B, trying next block...", block);
                }
            } else {
                println!("Could not authenticate with sector {}, trying next block...", sector);
            }
        }
    }
    
    // If all traditional methods failed, try another approach
    println!("\nStandard write methods failed. Trying direct command approach...");
    
    // Try direct write commands for ACR122U (Example: block 4)
    let block = 4; // First data block of sector 1
    let mut direct_cmd = vec![0xFF, 0x00, 0x00, 0x00, 0x05 + block_data.len() as u8, 
                             0xD4, 0x40, block as u8];
    direct_cmd.extend_from_slice(&block_data);
    
    if let Some(_) = send_apdu(card, &direct_cmd, "Direct Command Write") {
        println!("Successfully wrote data using direct command approach");
        println!("\n✅ Data successfully written to MIFARE Classic card using direct command!");
        return Ok(());
    }
    
    // If we made it here, all attempts failed
    Err("Failed to write data to any block".into())
}

/// Verify the card and get UID
pub fn verify_mifare_classic(card: &Card) -> Result<Vec<u8>, Box<dyn Error>> {
    println!("Getting card details...");
    
    // Get card UID
    let get_uid = [0xFF, 0xCA, 0x00, 0x00, 0x00];
    
    if let Some(response) = send_apdu(card, &get_uid, "Get UID") {
        println!("Card UID: {}", hex_string(&response));
        
        // Get card type (ATS)
        let get_ats = [0xFF, 0xCA, 0x01, 0x00, 0x00];
        if let Some(ats_response) = send_apdu(card, &get_ats, "Get ATS") {
            println!("Card ATS: {}", hex_string(&ats_response));
        }
        
        return Ok(response);
    } else {
        Err("Could not get card UID".into())
    }
}
