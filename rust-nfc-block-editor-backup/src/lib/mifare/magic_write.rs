use std::error::Error;
use std::thread::sleep;
use std::time::Duration;
use rppal::spi::Spi;

use crate::lib::mfrc522::{
    mfrc522_request, mfrc522_anticoll, mfrc522_select_tag, 
    mfrc522_auth, mfrc522_stop_crypto1, mfrc522_read, mfrc522_write,
    PICC_REQIDL, PICC_AUTHENT1A, MI_OK
};
use crate::lib::utils::{uid_to_string, bytes_to_hex, hex_string_to_bytes};
use crate::lib::ui_mod::common::{clear_screen, wait_for_input, countdown_for_card_placement};
use super::magic::{retry_operation, reconnect_to_card, handle_uid_write_failure};

// Constants for timing and retries
const DELAY_BETWEEN_OPS: u64 = 50; // milliseconds
const MAX_RETRIES: usize = 3;

/// Write a custom UID to a Magic Card
pub fn write_custom_uid(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("WRITE CUSTOM UID");
    println!("===============");
    println!("");
    println!("WARNING: This only works with Magic Cards that support UID changing!");
    println!("Using this on non-Magic Cards may DAMAGE your card permanently.");
    println!("");
    println!("This function will attempt direct write to block 0.");
    
    // Get the new UID
    let new_uid_str = wait_for_input("\nEnter new UID in hex (e.g., 11:22:33:44): ")?;
    
    let new_uid = match hex_string_to_bytes(&new_uid_str) {
        Some(bytes) => {
            if bytes.len() != 4 && bytes.len() != 7 && bytes.len() != 10 {
                println!("Invalid UID length. Must be 4, 7, or 10 bytes.");
                wait_for_input("\nPress Enter to continue...")?;
                return Ok(());
            }
            bytes
        },
        None => {
            println!("Invalid hex format.");
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
    };
    
    println!("\nNew UID will be: {}", bytes_to_hex(&new_uid));
    println!("\nWARNING: This operation may PERMANENTLY DAMAGE non-Magic Cards!");
    let confirm = wait_for_input("Are you ABSOLUTELY sure you want to proceed? (type YES in capital letters): ")?;
    
    if confirm != "YES" {
        println!("Operation cancelled.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    countdown_for_card_placement(5)?;
    
    // Request tag with retries
    let (status, _) = match retry_operation(|| mfrc522_request(spi, PICC_REQIDL), MAX_RETRIES) {
        Ok(result) => result,
        Err(e) => {
            println!("\nError detecting card: {:?}", e);
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
    };
    
    if status != MI_OK {
        println!("");
        println!("Error: Could not detect card after multiple attempts.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Anti-collision and get UID with retries
    let (status, uid) = match retry_operation(|| mfrc522_anticoll(spi), MAX_RETRIES) {
        Ok(result) => result,
        Err(e) => {
            println!("\nError during anticollision: {:?}", e);
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
    };
    
    if status != MI_OK {
        println!("");
        println!("Error: Could not read card UID after multiple attempts.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    println!("");
    println!("Card detected. Current UID: {}", uid_to_string(&uid));
    
    // First read block 0 to get its current content
    println!("\nReading current block 0 data...");
    
    // Select card with retries
    let size = match retry_operation(|| mfrc522_select_tag(spi, &uid), MAX_RETRIES) {
        Ok(result) => result,
        Err(e) => {
            println!("\nError selecting card: {:?}", e);
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
    };
    
    if size == 0 {
        println!("Error: Could not select card.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Add delay after selection
    sleep(Duration::from_millis(DELAY_BETWEEN_OPS));
    
    // Try to read block 0 (first with standard key)
    let standard_key = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let mut current_block0_data = [0u8; 16];
    let mut block0_read = false;
    
    match mfrc522_auth(spi, PICC_AUTHENT1A, 0, &standard_key, &uid) {
        Ok(status) if status == MI_OK => {
            match mfrc522_read(spi, 0) {
                Ok(Some(data)) => {
                    println!("Current block 0: {}", bytes_to_hex(&data));
                    current_block0_data.copy_from_slice(&data);
                    block0_read = true;
                },
                _ => {
                    println!("Could not read block 0 with standard key.");
                }
            }
            
            // Stop crypto
            mfrc522_stop_crypto1(spi)?;
        },
        _ => {
            println!("Standard key authentication failed. Trying other methods...");
            // Stop crypto in case partial authentication occurred
            mfrc522_stop_crypto1(spi)?;
        }
    }
    
    sleep(Duration::from_millis(DELAY_BETWEEN_OPS));
    
    // If standard key failed, try with null key
    if !block0_read {
        // Reconnect to the card
        if !reconnect_to_card(spi, &uid)? {
            println!("Card disconnected. Cancelling operation.");
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
        
        let null_key = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        match mfrc522_auth(spi, PICC_AUTHENT1A, 0, &null_key, &uid) {
            Ok(status) if status == MI_OK => {
                match mfrc522_read(spi, 0) {
                    Ok(Some(data)) => {
                        println!("Current block 0: {}", bytes_to_hex(&data));
                        current_block0_data.copy_from_slice(&data);
                        block0_read = true;
                    },
                    _ => {
                        println!("Could not read block 0 with null key.");
                    }
                }
                
                // Stop crypto
                mfrc522_stop_crypto1(spi)?;
            },
            _ => {
                println!("Null key authentication failed.");
                // Stop crypto in case partial authentication occurred
                mfrc522_stop_crypto1(spi)?;
            }
        }
        
        sleep(Duration::from_millis(DELAY_BETWEEN_OPS));
    }
    
    // If we still couldn't read block 0, try direct read (some Magic Cards allow this)
    if !block0_read {
        // Reconnect to the card
        if !reconnect_to_card(spi, &uid)? {
            println!("Card disconnected. Cancelling operation.");
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
        
        // Try direct read without authentication
        match mfrc522_read(spi, 0) {
            Ok(Some(data)) => {
                println!("Current block 0: {}", bytes_to_hex(&data));
                current_block0_data.copy_from_slice(&data);
                block0_read = true;
            },
            _ => {
                println!("Could not read block 0 with any method.");
                println!("Will attempt direct write anyway.");
            }
        }
        
        sleep(Duration::from_millis(DELAY_BETWEEN_OPS));
    }
    
    // Prepare new block 0 data
    let mut new_block0_data = [0u8; 16];
    
    // If we read block 0, use its data as a template
    if block0_read {
        new_block0_data.copy_from_slice(&current_block0_data);
    } else {
        // Default values for a typical MIFARE Classic
        new_block0_data[5] = 0x08; // SAK
        new_block0_data[6] = 0x04; // ATQA (byte 1)
        new_block0_data[7] = 0x00; // ATQA (byte 2)
    }
    
    // Set the new UID
    if new_uid.len() == 4 {
        // 4-byte UID
        for i in 0..4 {
            new_block0_data[i] = new_uid[i];
        }
        
        // Calculate BCC
        new_block0_data[4] = new_uid[0] ^ new_uid[1] ^ new_uid[2] ^ new_uid[3];
    } else if new_uid.len() == 7 {
        // 7-byte UID: First byte is typically 0x04 (NXP manufacturer ID)
        new_block0_data[0] = 0x04;
        
        // Copy the next 6 bytes
        for i in 0..6 {
            new_block0_data[i+1] = new_uid[i+1];
        }
        
        // For 7-byte UIDs, SAK/ATQA positions are different
        new_block0_data[7] = 0x08; // SAK
        new_block0_data[8] = 0x44; // ATQA (byte 1)
        new_block0_data[9] = 0x00; // ATQA (byte 2)
    }
    
    println!("\nNew block 0 data: {}", bytes_to_hex(&new_block0_data));
    println!("\nAttempting to write new UID...");
    
    // Reconnect to the card before writing
    if !reconnect_to_card(spi, &uid)? {
        println!("Card disconnected. Cancelling operation.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // First try direct write without authentication (Gen2/CUID method)
    match mfrc522_write(spi, 0, &new_block0_data) {
        Ok(status) if status == MI_OK => {
            println!("\n✅ UID successfully changed using direct write!");
            println!("This confirms this is a Magic Card (likely Gen2/CUID type).");
            println!("\nNew UID should be: {}", if new_uid.len() == 4 {
                bytes_to_hex(&new_uid)
            } else {
                // For 7-byte UID, the actual UID is constructed differently
                let mut full_uid = Vec::new();
                full_uid.push(0x04); // First byte is manufacturer ID
                full_uid.extend_from_slice(&new_uid[1..7]);
                bytes_to_hex(&full_uid)
            });
            
            wait_for_input("\nRemove the card and place it again to verify the new UID.");
            return Ok(());
        },
        _ => {
            println!("Direct write failed. Trying other methods...");
        }
    }
    
    sleep(Duration::from_millis(DELAY_BETWEEN_OPS));
    
    // If direct write failed, try with authentication first
    // Reconnect to the card
    if !reconnect_to_card(spi, &uid)? {
        println!("Card disconnected. Cancelling operation.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Try standard key
    let standard_key = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    match mfrc522_auth(spi, PICC_AUTHENT1A, 0, &standard_key, &uid) {
        Ok(status) if status == MI_OK => {
            match mfrc522_write(spi, 0, &new_block0_data) {
                Ok(status) if status == MI_OK => {
                    println!("\n✅ UID successfully changed using standard key authentication!");
                    println!("This confirms this is a Magic Card that allows block 0 writing.");
                    println!("\nNew UID should be: {}", if new_uid.len() == 4 {
                        bytes_to_hex(&new_uid)
                    } else {
                        // For 7-byte UID, the actual UID is constructed differently
                        let mut full_uid = Vec::new();
                        full_uid.push(0x04); // First byte is manufacturer ID
                        full_uid.extend_from_slice(&new_uid[1..7]);
                        bytes_to_hex(&full_uid)
                    });
                    
                    // Stop crypto
                    mfrc522_stop_crypto1(spi)?;
                    
                    wait_for_input("\nRemove the card and place it again to verify the new UID.");
                    return Ok(());
                },
                _ => {
                    println!("Write with standard key failed.");
                    // Stop crypto
                    mfrc522_stop_crypto1(spi)?;
                }
            }
        },
        _ => {
            println!("Standard key authentication failed.");
            // Stop crypto in case partial authentication occurred
            mfrc522_stop_crypto1(spi)?;
        }
    }
    
    sleep(Duration::from_millis(DELAY_BETWEEN_OPS));
    
    // If standard key failed, try null key
    // Reconnect to the card
    if !reconnect_to_card(spi, &uid)? {
        println!("Card disconnected. Cancelling operation.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    let null_key = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    match mfrc522_auth(spi, PICC_AUTHENT1A, 0, &null_key, &uid) {
        Ok(status) if status == MI_OK => {
            match mfrc522_write(spi, 0, &new_block0_data) {
                Ok(status) if status == MI_OK => {
                    println!("\n✅ UID successfully changed using null key authentication!");
                    println!("This confirms this is a Magic Card that allows block 0 writing.");
                    println!("\nNew UID should be: {}", if new_uid.len() == 4 {
                        bytes_to_hex(&new_uid)
                    } else {
                        // For 7-byte UID, the actual UID is constructed differently
                        let mut full_uid = Vec::new();
                        full_uid.push(0x04); // First byte is manufacturer ID
                        full_uid.extend_from_slice(&new_uid[1..7]);
                        bytes_to_hex(&full_uid)
                    });
                    
                    // Stop crypto
                    mfrc522_stop_crypto1(spi)?;
                    
                    wait_for_input("\nRemove the card and place it again to verify the new UID.");
                    return Ok(());
                },
                _ => {
                    println!("Write with null key failed.");
                    // Stop crypto
                    mfrc522_stop_crypto1(spi)?;
                }
            }
        },
        _ => {
            println!("Null key authentication failed.");
            // Stop crypto in case partial authentication occurred
            mfrc522_stop_crypto1(spi)?;
        }
    }
    
    return handle_uid_write_failure(spi);
}
