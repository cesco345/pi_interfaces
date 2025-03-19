use std::error::Error;
use rppal::spi::Spi;

use crate::lib::mfrc522::{
    mfrc522_request, mfrc522_anticoll, mfrc522_select_tag, 
    mfrc522_auth, mfrc522_stop_crypto1, mfrc522_read, mfrc522_write,
    PICC_REQIDL, PICC_AUTHENT1A, PICC_AUTHENT1B, MI_OK
};

use crate::lib::utils::{bytes_to_hex, bytes_to_ascii, uid_to_string};
use crate::lib::mifare::access::AccessBits;

// Common authentication keys to try
pub const DEFAULT_KEYS: [[u8; 6]; 4] = [
    [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],  // Default key
    [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5],  // Common key
    [0xD3, 0xF7, 0xD3, 0xF7, 0xD3, 0xF7],  // Common key
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00],  // All zeroes
];

// Read card UID
pub fn read_card_uid(spi: &mut Spi) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    // Request tag
    let (status, _) = mfrc522_request(spi, PICC_REQIDL)?;
    if status != MI_OK {
        return Ok(None);
    }
    
    // Anti-collision
    let (status, uid) = mfrc522_anticoll(spi)?;
    if status != MI_OK {
        return Ok(None);
    }
    
    Ok(Some(uid))
}

// Wait for a card to be removed
pub fn wait_for_card_removal(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    println!("Waiting for card to be removed...");
    
    while read_card_uid(spi)?.is_some() {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    
    println!("Card removed");
    Ok(())
}

// Read all blocks in a sector
pub fn read_sector_data(spi: &mut Spi, sector: u8) -> Result<Option<(Vec<u8>, [Option<Vec<u8>>; 4])>, Box<dyn Error>> {
    if sector >= 16 {
        return Err("Invalid sector number".into());
    }
    
    // Request tag
    let (status, _) = mfrc522_request(spi, PICC_REQIDL)?;
    if status != MI_OK {
        return Ok(None);
    }
    
    // Anti-collision
    let (status, uid) = mfrc522_anticoll(spi)?;
    if status != MI_OK {
        return Ok(None);
    }
    
    // Select the tag
    let size = mfrc522_select_tag(spi, &uid)?;
    if size == 0 {
        return Ok(None);
    }
    
    // Try to authenticate with different keys
    let mut authenticated = false;
    let mut auth_key_type = PICC_AUTHENT1A;
    let mut auth_key = &DEFAULT_KEYS[0];
    
    // First try with Key A
    for key in &DEFAULT_KEYS {
        let trailer_block = sector * 4 + 3;
        let status = mfrc522_auth(spi, PICC_AUTHENT1A, trailer_block, key, &uid)?;
        if status == MI_OK {
            authenticated = true;
            auth_key_type = PICC_AUTHENT1A;
            auth_key = key;
            break;
        }
    }
    
    // If Key A fails, try Key B
    if !authenticated {
        for key in &DEFAULT_KEYS {
            let trailer_block = sector * 4 + 3;
            let status = mfrc522_auth(spi, PICC_AUTHENT1B, trailer_block, key, &uid)?;
            if status == MI_OK {
                authenticated = true;
                auth_key_type = PICC_AUTHENT1B;
                auth_key = key;
                break;
            }
        }
    }
    
    if !authenticated {
        println!("Failed to authenticate sector {}. Try with custom keys.", sector);
        mfrc522_stop_crypto1(spi)?;
        return Ok(None);
    }
    
    // Read data from blocks
    let mut blocks: [Option<Vec<u8>>; 4] = [None, None, None, None];
    
    for block_offset in 0..4 {
        let block_addr = sector * 4 + block_offset;
        
        // Re-authenticate for each block if needed
        if block_offset > 0 {
            let status = mfrc522_auth(spi, auth_key_type, block_addr, auth_key, &uid)?;
            if status != MI_OK {
                continue;  // Skip this block if authentication fails
            }
        }
        
        if let Some(block_data) = mfrc522_read(spi, block_addr)? {
            blocks[block_offset as usize] = Some(block_data);
        }
    }
    
    mfrc522_stop_crypto1(spi)?;
    
    Ok(Some((uid, blocks)))
}

// Write data to a specific block
pub fn write_block_data(spi: &mut Spi, block_addr: u8, text: &str) -> Result<Option<(Vec<u8>, String)>, Box<dyn Error>> {
    let sector = block_addr / 4;
    let is_trailer = block_addr % 4 == 3;
    
    if is_trailer {
        return Err("Cannot write to sector trailer using this function".into());
    }
    
    // Request tag
    let (status, _) = mfrc522_request(spi, PICC_REQIDL)?;
    if status != MI_OK {
        return Ok(None);
    }
    
    // Anti-collision
    let (status, uid) = mfrc522_anticoll(spi)?;
    if status != MI_OK {
        return Ok(None);
    }
    
    // Select the tag
    let size = mfrc522_select_tag(spi, &uid)?;
    if size == 0 {
        return Ok(None);
    }
    
    // Try to authenticate with different keys
    let mut authenticated = false;
    
    // For write access, always try Key B first, then Key A
    for &auth_type in &[PICC_AUTHENT1B, PICC_AUTHENT1A] {
        for &key in &DEFAULT_KEYS {
            let status = mfrc522_auth(spi, auth_type, sector * 4 + 3, &key, &uid)?;
            if status == MI_OK {
                authenticated = true;
                break;
            }
        }
        if authenticated {
            break;
        }
    }
    
    if !authenticated {
        mfrc522_stop_crypto1(spi)?;
        return Ok(None);
    }
    
    // Prepare data: text + padding to fill 16 bytes
    let mut data = Vec::from(text.as_bytes());
    data.resize(16, 0); // Pad with zeros
    
    // Write data to the block
    if mfrc522_write(spi, block_addr, &data)? != MI_OK {
        mfrc522_stop_crypto1(spi)?;
        return Ok(None);
    }
    
    mfrc522_stop_crypto1(spi)?;
    
    // Return written text (trimmed to what actually fits)
    let written_text = String::from_utf8_lossy(&data).trim_end_matches('\0').to_string();
    
    Ok(Some((uid, written_text)))
}

// Write data to a specific block with a provided key
pub fn write_block_raw(spi: &mut Spi, block_addr: u8, key: &[u8], data: &[u8]) -> Result<bool, Box<dyn Error>> {
    if key.len() != 6 || data.len() != 16 {
        return Err("Invalid key or data length".into());
    }
    
    let sector = block_addr / 4;
    let trailer_block = sector * 4 + 3;
    
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
    
    // Try authentication with both key types
    let mut authenticated = false;
    
    for &auth_type in &[PICC_AUTHENT1A, PICC_AUTHENT1B] {
        let status = mfrc522_auth(spi, auth_type, trailer_block, key, &uid)?;
        if status == MI_OK {
            authenticated = true;
            break;
        }
    }
    
    if !authenticated {
        mfrc522_stop_crypto1(spi)?;
        return Ok(false);
    }
    
    // Write data to the block
    let result = mfrc522_write(spi, block_addr, data)? == MI_OK;
    
    mfrc522_stop_crypto1(spi)?;
    
    Ok(result)
}
