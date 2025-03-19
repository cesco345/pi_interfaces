use std::error::Error;
use rppal::spi::Spi;

use crate::lib::mfrc522::{
    mfrc522_request, mfrc522_anticoll, mfrc522_select_tag, 
    mfrc522_auth, mfrc522_stop_crypto1, mfrc522_read, mfrc522_write,
    PICC_REQIDL, PICC_AUTHENT1A, PICC_AUTHENT1B, MI_OK
};

use crate::lib::utils::{bytes_to_hex, bytes_to_ascii, uid_to_string};
use crate::lib::mifare::access::AccessBits;
use crate::lib::mifare::operations::DEFAULT_KEYS;

// Modify access conditions for a sector
pub fn modify_sector_access(spi: &mut Spi, sector: u8, access_bits: &AccessBits) -> Result<bool, Box<dyn Error>> {
    if sector >= 16 {
        return Err("Invalid sector number".into());
    }
    
    // Request tag
    let (status, _) = mfrc522_request(spi, PICC_REQIDL)?;
    if status != MI_OK {
        return Ok(false);
    }
    
    // Anti-collision
    let (status, uid) = mfrc522_anticoll(spi)?;
    if status != MI_OK {
        return Ok(false);
    }
    
    // Select the tag
    let size = mfrc522_select_tag(spi, &uid)?;
    if size == 0 {
        return Ok(false);
    }
    
    // Try to authenticate with different keys
    let mut authenticated = false;
    let mut auth_key = &DEFAULT_KEYS[0];
    
    for key in &DEFAULT_KEYS {
        let trailer_block = sector * 4 + 3;
        let status = mfrc522_auth(spi, PICC_AUTHENT1A, trailer_block, key, &uid)?;
        if status == MI_OK {
            authenticated = true;
            auth_key = key;
            break;
        }
    }
    
    if !authenticated {
        mfrc522_stop_crypto1(spi)?;
        return Ok(false);
    }
    
    // Read the current trailer to preserve the keys
    let trailer_block = sector * 4 + 3;
    let trailer_data_opt = mfrc522_read(spi, trailer_block)?;
    
    if trailer_data_opt.is_none() {
        mfrc522_stop_crypto1(spi)?;
        return Ok(false);
    }
    
    let trailer_data = trailer_data_opt.unwrap();
    
    // Create new trailer data with updated access bits
    let mut new_trailer = [0u8; 16];
    
    // Copy Key A (first 6 bytes)
    new_trailer[0..6].copy_from_slice(&trailer_data[0..6]);
    
    // Set the new access bits (bytes 6-9)
    let access_bytes = access_bits.to_bytes();
    new_trailer[6..10].copy_from_slice(&access_bytes);
    
    // Copy Key B (last 6 bytes)
    new_trailer[10..16].copy_from_slice(&trailer_data[10..16]);
    
    // Write the updated trailer
    if mfrc522_write(spi, trailer_block, &new_trailer)? != MI_OK {
        mfrc522_stop_crypto1(spi)?;
        return Ok(false);
    }
    
    mfrc522_stop_crypto1(spi)?;
    return Ok(true);
}

// Change keys for a sector
pub fn change_sector_keys(spi: &mut Spi, sector: u8, current_key: &[u8], 
                     change_key_a: bool, new_key_a: &[u8],
                     change_key_b: bool, new_key_b: &[u8]) -> Result<bool, Box<dyn Error>> {
    if sector >= 16 {
        return Err("Invalid sector number".into());
    }
    
    if current_key.len() != 6 || (change_key_a && new_key_a.len() != 6) || (change_key_b && new_key_b.len() != 6) {
        return Err("Invalid key length (must be 6 bytes)".into());
    }
    
    // Request tag
    let (status, _) = mfrc522_request(spi, PICC_REQIDL)?;
    if status != MI_OK {
        return Ok(false);
    }
    
    // Anti-collision
    let (status, uid) = mfrc522_anticoll(spi)?;
    if status != MI_OK {
        return Ok(false);
    }
    
    // Select the tag
    let size = mfrc522_select_tag(spi, &uid)?;
    if size == 0 {
        return Ok(false);
    }
    
    // Authenticate with current key
    let trailer_block = sector * 4 + 3;
    let status = mfrc522_auth(spi, PICC_AUTHENT1A, trailer_block, current_key, &uid)?;
    
    if status != MI_OK {
        mfrc522_stop_crypto1(spi)?;
        return Ok(false);
    }
    
    // Read current trailer
    let trailer_data_opt = mfrc522_read(spi, trailer_block)?;
    
    if trailer_data_opt.is_none() {
        mfrc522_stop_crypto1(spi)?;
        return Ok(false);
    }
    
    let trailer_data = trailer_data_opt.unwrap();
    
    // Create new trailer data
    let mut new_trailer = [0u8; 16];
    
    // Copy current trailer
    new_trailer.copy_from_slice(&trailer_data);
    
    // Update keys as needed
    if change_key_a {
        new_trailer[0..6].copy_from_slice(new_key_a);
    }
    
    if change_key_b {
        new_trailer[10..16].copy_from_slice(new_key_b);
    }
    
    // Write the updated trailer
    if mfrc522_write(spi, trailer_block, &new_trailer)? != MI_OK {
        mfrc522_stop_crypto1(spi)?;
        return Ok(false);
    }
    
    mfrc522_stop_crypto1(spi)?;
    return Ok(true);
}

// Format a card to factory defaults (all sectors to transport configuration)
pub fn format_card(spi: &mut Spi) -> Result<bool, Box<dyn Error>> {
    // Default trailer data (all 0xFF for Key A, default transport access bits, all 0xFF for Key B)
    let default_trailer = [
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, // Key A
        0xFF, 0x07, 0x80, 0x69,             // Access bits
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF  // Key B
    ];
    
    // Default data block (all zeros)
    let default_data = [0u8; 16];
    
    // Request tag
    let (status, _) = mfrc522_request(spi, PICC_REQIDL)?;
    if status != MI_OK {
        return Ok(false);
    }
    
    // Anti-collision
    let (status, uid) = mfrc522_anticoll(spi)?;
    if status != MI_OK {
        return Ok(false);
    }
    
    // Select the tag
    let size = mfrc522_select_tag(spi, &uid)?;
    if size == 0 {
        return Ok(false);
    }
    
    let mut success_count = 0;
    
    // Format each sector
    for sector in 1..16 {  // Skip sector 0 to avoid damaging manufacturer data
        println!("Formatting sector {}...", sector);
        
        // Try to authenticate with different keys
        let mut authenticated = false;
        
        for &auth_type in &[PICC_AUTHENT1A, PICC_AUTHENT1B] {
            for &key in &DEFAULT_KEYS {
                let trailer_block = sector * 4 + 3;
                let status = mfrc522_auth(spi, auth_type, trailer_block, &key, &uid)?;
                if status == MI_OK {
                    authenticated = true;
                    
                    // Write default data to all data blocks
                    for block_offset in 0..3 {
                        let block_addr = sector * 4 + block_offset;
                        if mfrc522_write(spi, block_addr, &default_data)? == MI_OK {
                            println!("  Block {} reset to zeros", block_addr);
                        } else {
                            println!("  Failed to reset block {}", block_addr);
                        }
                    }
                    
                    // Write default trailer to trailer block
                    if mfrc522_write(spi, trailer_block, &default_trailer)? == MI_OK {
                        println!("  Sector trailer reset to factory defaults");
                        success_count += 1;
                    } else {
                        println!("  Failed to reset sector trailer");
                    }
                    
                    // Stop after successful formatting of this sector
                    break;
                }
            }
            
            if authenticated {
                break;
            }
        }
        
        if !authenticated {
            println!("  Could not authenticate sector {} with any key", sector);
        }
        
        // Always stop crypto before trying next sector
        mfrc522_stop_crypto1(spi)?;
    }
    
    println!("Format complete. Successfully reset {}/15 sectors.", success_count);
    return Ok(success_count > 0);
}
