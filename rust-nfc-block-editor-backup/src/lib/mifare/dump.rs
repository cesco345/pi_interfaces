use std::error::Error;
use rppal::spi::Spi;

use crate::lib::mfrc522::{
    mfrc522_request, mfrc522_anticoll, mfrc522_select_tag, 
    mfrc522_auth, mfrc522_stop_crypto1, mfrc522_read,
    PICC_REQIDL, PICC_AUTHENT1A, PICC_AUTHENT1B, MI_OK
};

use crate::lib::utils::{bytes_to_hex, bytes_to_ascii, uid_to_string};
use crate::lib::mifare::access::AccessBits;

// Dump all card data (Classic 1K) using the method from the working code
pub fn dump_card(spi: &mut Spi) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    // Key to use for authentication
    let key = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    
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
    
    println!("Card selected. UID: {}  Size: {}", uid_to_string(&uid), size);
    println!("\nDumping card data...");
    
    // Classic 1K has 16 sectors with 4 blocks each
    for sector in 0..16 {
        println!("\nSector {}", sector);
        println!("------------------");
        
        for block in 0..4 {
            let block_addr = sector * 4 + block;
            
            // Authenticate for the block
            let status = mfrc522_auth(spi, PICC_AUTHENT1A, block_addr, &key, &uid)?;
            
            if status == MI_OK {
                if let Some(data) = mfrc522_read(spi, block_addr)? {
                    println!("  Block {}: {}", block_addr, bytes_to_hex(&data));
                    
                    // For non-sector trailer blocks, also show ASCII
                    if block != 3 {
                        println!("          ASCII: {}", bytes_to_ascii(&data));
                    } else {
                        // Sector trailer - display keys and access bits
                        println!("          Key A: {}", bytes_to_hex(&data[0..6]));
                        println!("          Access Bits: {}", bytes_to_hex(&data[6..10]));
                        println!("          Key B: {}", bytes_to_hex(&data[10..16]));
                        
                        // Show interpreted access conditions
                        let access_bytes = [data[6], data[7], data[8], data[9]];
                        let access_bits = AccessBits::from_bytes(&access_bytes);
                        println!("\n          Access Conditions:");
                        println!("          Block {}: {}", block_addr-3, access_bits.interpret_access("data", 0));
                        println!("          Block {}: {}", block_addr-2, access_bits.interpret_access("data", 1));
                        println!("          Block {}: {}", block_addr-1, access_bits.interpret_access("data", 2));
                        println!("          Block {} (Trailer): Key A: {}", block_addr, 
                                access_bits.interpret_access("trailer", 0).split('\n').next().unwrap_or(""));
                    }
                } else {
                    println!("  Block {}: (Read failed)", block_addr);
                }
            } else {
                println!("  Authentication failed for Block {}", block_addr);
                break; // Can't read more blocks in this sector
            }
        }
    }
    
    // Only stop crypto once at the end
    mfrc522_stop_crypto1(spi)?;
    
    Ok(Some(uid))
}

// Simple dump of a specific card sector
pub fn dump_sector(spi: &mut Spi, sector: u8) -> Result<bool, Box<dyn Error>> {
    if sector >= 16 {
        return Err("Invalid sector number (must be 0-15)".into());
    }
    
    // Request tag
    let (status, _) = mfrc522_request(spi, PICC_REQIDL)?;
    if status != MI_OK {
        println!("No card detected");
        return Ok(false);
    }
    
    // Anti-collision
    let (status, uid) = mfrc522_anticoll(spi)?;
    if status != MI_OK {
        println!("Failed to get card UID");
        return Ok(false);
    }
    
    // Select the tag
    let size = mfrc522_select_tag(spi, &uid)?;
    if size == 0 {
        println!("Failed to select card");
        return Ok(false);
    }
    
    println!("Card selected. UID: {}", uid_to_string(&uid));
    println!("\nDumping sector {}:", sector);
    println!("------------------");
    
    // Just use the default key
    let key = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    
    for block_offset in 0..4 {
        let block_addr = sector * 4 + block_offset;
        
        // Authenticate directly for each block
        let status = mfrc522_auth(spi, PICC_AUTHENT1A, block_addr, &key, &uid)?;
        if status != MI_OK {
            println!("  Block {}: (Authentication failed)", block_addr);
            break; // Stop at first authentication failure
        }
        
        if let Some(data) = mfrc522_read(spi, block_addr)? {
            println!("  Block {}: {}", block_addr, bytes_to_hex(&data));
            
            if block_offset == 3 {
                // Sector trailer - display keys and access bits
                println!("    Key A: {}", bytes_to_hex(&data[0..6]));
                println!("    Access Bits: {}", bytes_to_hex(&data[6..10]));
                println!("    Key B: {}", bytes_to_hex(&data[10..16]));
                
                // Show interpreted access conditions
                let access_bytes = [data[6], data[7], data[8], data[9]];
                let access_bits = AccessBits::from_bytes(&access_bytes);
                println!("\n    Access Conditions:");
                println!("    Block {}: {}", block_addr-3, access_bits.interpret_access("data", 0));
                println!("    Block {}: {}", block_addr-2, access_bits.interpret_access("data", 1));
                println!("    Block {}: {}", block_addr-1, access_bits.interpret_access("data", 2));
                println!("    Block {} (Trailer): {}", block_addr, 
                         access_bits.interpret_access("trailer", 0).replace("\n", "\n    "));
            } else {
                println!("    ASCII: {}", bytes_to_ascii(&data));
            }
        } else {
            println!("  Block {}: (Read failed)", block_addr);
        }
    }
    
    // Only stop crypto once at the end
    mfrc522_stop_crypto1(spi)?;
    
    Ok(true)
}

// Function to test various keys against a card
pub fn test_keys(spi: &mut Spi) -> Result<Vec<(u8, [u8; 6])>, Box<dyn Error>> {
    let keys = [
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],  // Default key
        [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5],  // Common key
        [0xD3, 0xF7, 0xD3, 0xF7, 0xD3, 0xF7],  // Common key
        [0x00, 0x00, 0x00, 0x00, 0x00, 0x00],  // All zeroes
        [0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5],  // Another common key
    ];
    
    // Request tag
    let (status, _) = mfrc522_request(spi, PICC_REQIDL)?;
    if status != MI_OK {
        return Err("No card detected".into());
    }
    
    // Anti-collision
    let (status, uid) = mfrc522_anticoll(spi)?;
    if status != MI_OK {
        return Err("Failed to get card UID".into());
    }
    
    // Select the tag
    let size = mfrc522_select_tag(spi, &uid)?;
    if size == 0 {
        return Err("Failed to select card".into());
    }
    
    println!("Card selected. UID: {}  Size: {}", uid_to_string(&uid), size);
    println!("\nTesting keys...");
    
    let mut results = Vec::new();
    
    // Test keys for each sector
    for sector in 0..16 {
        println!("Sector {}: ", sector);
        
        let first_block = sector * 4;
        
        for auth_type in &[PICC_AUTHENT1A, PICC_AUTHENT1B] {
            for key in &keys {
                // Make sure to stop crypto from previous attempts
                mfrc522_stop_crypto1(spi)?;
                
                // Fresh card detection
                let (status, _) = mfrc522_request(spi, PICC_REQIDL)?;
                if status != MI_OK {
                    continue;
                }
                
                let (status, new_uid) = mfrc522_anticoll(spi)?;
                if status != MI_OK {
                    continue;
                }
                
                mfrc522_select_tag(spi, &new_uid)?;
                
                // Try authentication with this key
                let status = mfrc522_auth(spi, *auth_type, first_block, key, &new_uid)?;
                if status == MI_OK {
                    // This key works!
                    let key_type = if *auth_type == PICC_AUTHENT1A { "A" } else { "B" };
                    println!("  Found working Key {}: {}", key_type, bytes_to_hex(key));
                    
                    let mut key_copy = [0u8; 6];
                    key_copy.copy_from_slice(key);
                    results.push((sector, key_copy));
                    
                    // Clean up
                    mfrc522_stop_crypto1(spi)?;
                    break;
                }
            }
        }
    }
    
    Ok(results)
}
