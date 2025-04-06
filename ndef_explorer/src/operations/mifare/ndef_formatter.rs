// NDEF formatting operations for MIFARE cards

use std::error::Error;
use pcsc::Card;
use crate::commands::ndef_commands::send_apdu;
use crate::util::ndef_util::hex_string;
use crate::operations::ndef_operations_writer::CardExport;
use super::authentication::authenticate_sector;

/// Format MIFARE Classic card for NDEF
pub fn format_mifare_classic(card: &Card, card_data: &CardExport) -> Result<(), Box<dyn Error>> {
    println!("\nFormatting MIFARE Classic card for NDEF...");
    
    // Convert data to bytes
    let data_str = &card_data.fileData;
    let data_bytes = data_str.as_bytes().to_vec();
    
    // Create a simple NFC Data Exchange Format (NDEF) message
    println!("Creating NDEF message...");
    
    // NDEF record header: MB=1, ME=1, SR=1, IL=0, TNF=1 (Well Known Type)
    let mut ndef_message = vec![0xD1];
    // Type length (1 byte)
    ndef_message.push(0x01);
    // Payload length (data length + status byte + language code length)
    ndef_message.push((data_bytes.len() + 3) as u8);
    // Type: 'T' for Text record
    ndef_message.push(0x54);
    // Status byte (UTF-8 encoding, 2-byte language code)
    ndef_message.push(0x02);
    // Language code: 'en'
    ndef_message.push(0x65); // 'e'
    ndef_message.push(0x6E); // 'n'
    // Text content
    ndef_message.extend_from_slice(&data_bytes);
    
    // Calculate TLV structures
    // NDEF Message TLV
    let mut tlv_data = vec![0x03]; // NDEF Message TLV tag
    let tlv_length = ndef_message.len();
    tlv_data.push(tlv_length as u8); // Length (assuming length < 255 bytes)
    tlv_data.extend_from_slice(&ndef_message); // Value
    // Terminator TLV
    tlv_data.push(0xFE); // Terminator TLV tag
    
    println!("NDEF TLV data prepared: {}", hex_string(&tlv_data));
    
    // Try to authenticate with sector 1 (NDEF data usually goes here)
    println!("\nAttempting to authenticate with sector 1...");
    if authenticate_sector(card, 1, 0x60) { // 0x60 = Key A
        println!("Authentication with sector 1 successful");
        
        // Write NDEF data to block 4
        let mut ndef_block = Vec::new();
        ndef_block.extend_from_slice(&tlv_data);
        
        // Pad to 16 bytes
        while ndef_block.len() < 16 {
            ndef_block.push(0x00);
        }
        
        // If data exceeds 16 bytes, truncate and warn
        if ndef_block.len() > 16 {
            println!("Warning: NDEF data too large, truncating to 16 bytes");
            ndef_block.truncate(16);
        }
        
        let mut write_ndef = vec![0xFF, 0xD6, 0x00, 0x04, 0x10]; // Write to block 4
        write_ndef.extend_from_slice(&ndef_block);
        
        if let Some(_) = send_apdu(card, &write_ndef, "Write NDEF") {
            println!("Successfully wrote NDEF data to block 4");
            println!("\n✅ MIFARE Classic card successfully written with NDEF data!");
            return Ok(());
        } else {
            return Err("Failed to write NDEF data".into());
        }
    } else {
        println!("Could not authenticate with sector 1");
        return Err("Authentication failed for all keys".into());
    }
}

/// Create a properly formatted NDEF message for Text content
pub fn create_text_ndef_message(text: &str, language_code: &str) -> Vec<u8> {
    // Convert data to bytes
    let text_bytes = text.as_bytes();
    
    // NDEF record header: MB=1, ME=1, SR=1, IL=0, TNF=1 (Well Known Type)
    let mut ndef_message = vec![0xD1];
    
    // Type length (1 byte for 'T')
    ndef_message.push(0x01);
    
    // Payload length (text length + status byte + language code length)
    let lang_code_bytes = language_code.as_bytes();
    ndef_message.push((text_bytes.len() + 1 + lang_code_bytes.len()) as u8);
    
    // Type: 'T' for Text record
    ndef_message.push(0x54); // ASCII 'T'
    
    // Status byte (UTF-8 encoding, language code length)
    ndef_message.push(lang_code_bytes.len() as u8);
    
    // Language code
    ndef_message.extend_from_slice(lang_code_bytes);
    
    // Text content
    ndef_message.extend_from_slice(text_bytes);
    
    ndef_message
}

/// Create a full NDEF data package with TLV structure
pub fn create_ndef_tlv(ndef_message: &[u8]) -> Vec<u8> {
    let mut tlv_data = Vec::new();
    
    // NDEF Message TLV
    tlv_data.push(0x03); // NDEF Message TLV tag
    
    // Length field - handle both short and long form
    if ndef_message.len() < 255 {
        // Short form
        tlv_data.push(ndef_message.len() as u8);
    } else {
        // Long form (3-byte length field)
        tlv_data.push(0xFF); // Indicator for 3-byte length
        tlv_data.push((ndef_message.len() >> 8) as u8); // High byte
        tlv_data.push((ndef_message.len() & 0xFF) as u8); // Low byte
    }
    
    // Value (the NDEF message)
    tlv_data.extend_from_slice(ndef_message);
    
    // Terminator TLV
    tlv_data.push(0xFE);
    
    tlv_data
}

/// Write NDEF data to multiple blocks if needed
pub fn write_ndef_data_to_card(card: &Card, ndef_data: &[u8]) -> Result<(), Box<dyn Error>> {
    println!("Writing NDEF data ({} bytes) to card...", ndef_data.len());
    
    // First, try to authenticate sector 1 (standard location for NDEF)
    if !authenticate_sector(card, 1, 0x60) && !authenticate_sector(card, 1, 0x61) {
        println!("Could not authenticate with sector 1, trying other sectors...");
        return Err("Authentication failed for sector 1".into());
    }
    
    // Break data into 16-byte chunks
    for (i, chunk) in ndef_data.chunks(16).enumerate() {
        // Calculate block number - start with block 4 (first data block in sector 1)
        let block = 4 + i as u8;
        
        // Skip sector trailers (every 4th block)
        let adjusted_block = block + (block / 3) as u8;
        
        // Create a 16-byte block
        let mut block_data = Vec::with_capacity(16);
        block_data.extend_from_slice(chunk);
        
        // Pad to 16 bytes if needed
        while block_data.len() < 16 {
            block_data.push(0x00);
        }
        
        // Write command
        let mut write_cmd = vec![0xFF, 0xD6, 0x00, adjusted_block, 0x10];
        write_cmd.extend_from_slice(&block_data);
        
        if let Some(_) = send_apdu(card, &write_cmd, &format!("Write Block {}", adjusted_block)) {
            println!("  Wrote data chunk {} to block {}", i, adjusted_block);
        } else {
            return Err(format!("Failed to write to block {}", adjusted_block).into());
        }
    }
    
    println!("Successfully wrote NDEF data to card");
    Ok(())
}
