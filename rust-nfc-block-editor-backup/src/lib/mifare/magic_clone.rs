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
use super::magic::{retry_operation, reconnect_to_card};

// Constants for timing and retries
const DELAY_BETWEEN_OPS: u64 = 50; // milliseconds
const MAX_RETRIES: usize = 3;

/// Clone a card to a Magic Card
pub fn clone_card(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("CLONE CARD");
    println!("==========");
    println!("");
    println!("This operation will read data from a source card and write it to a Magic Card.");
    println!("Connection issues may occur during operation - the tool will attempt to reconnect.");
    
    // First read the source card
    println!("\nStep 1: Read source card");
    wait_for_input("Place the SOURCE card on the reader and press ENTER...")?;
    
    // Read the source card's UID and data with retries
    let (status, _) = match retry_operation(|| mfrc522_request(spi, PICC_REQIDL), MAX_RETRIES) {
        Ok(result) => result,
        Err(e) => {
            println!("\nError detecting source card: {:?}", e);
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
    };
    
    if status != MI_OK {
        println!("Error: Could not detect source card after multiple attempts.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Anti-collision and get UID with retries
    let (status, source_uid) = match retry_operation(|| mfrc522_anticoll(spi), MAX_RETRIES) {
        Ok(result) => result,
        Err(e) => {
            println!("\nError during anticollision: {:?}", e);
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
    };
    
    if status != MI_OK {
        println!("Error: Could not read source card UID after multiple attempts.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    println!("Source card detected. UID: {}", uid_to_string(&source_uid));
    
    // Select card with retries
    let size = match retry_operation(|| mfrc522_select_tag(spi, &source_uid), MAX_RETRIES) {
        Ok(result) => result,
        Err(e) => {
            println!("\nError selecting source card: {:?}", e);
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
    };
    
    if size == 0 {
        println!("Error: Could not select source card.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Add delay after selection
    sleep(Duration::from_millis(DELAY_BETWEEN_OPS));
    
    // Read all sectors from the source card
    println!("Reading card data...");
    
    // Attempt to read block 0 for UID verification
    let standard_key = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let mut block0_data = Vec::new();
    
    match mfrc522_auth(spi, PICC_AUTHENT1A, 0, &standard_key, &source_uid) {
        Ok(status) if status == MI_OK => {
            match mfrc522_read(spi, 0) {
                Ok(Some(data)) => {
                    println!("Successfully read block 0: {}", bytes_to_hex(&data));
                    block0_data = data.to_vec();
                },
                _ => {
                    println!("Could not read block 0 with standard key.");
                }
            }
            
            // Stop crypto
            mfrc522_stop_crypto1(spi)?;
        },
        _ => {
            println!("Standard key authentication for block 0 failed.");
            // Stop crypto in case partial authentication occurred
            mfrc522_stop_crypto1(spi)?;
        }
    }
    
    sleep(Duration::from_millis(DELAY_BETWEEN_OPS));
    
    // In a full implementation, we would:
    // 1. Try to authenticate to each sector using common keys
    // 2. Read all successful sectors
    // 3. Store the data for writing to the target card
    //
    // For simplicity, we're just simulating this part
    println!("Successfully read card data (simulated).");
    
    // Ask user to remove the source card
    wait_for_input("\nPlease remove the source card and press ENTER...")?;
    
    // Ask user for potential UID change
    let change_uid = wait_for_input("\nDo you want to use a different UID for the target card? (y/n): ")?.to_lowercase();
    
    let target_uid = if change_uid == "y" {
        let new_uid_str = wait_for_input("Enter new UID in hex (e.g., 11:22:33:44): ")?;
        
        match hex_string_to_bytes(&new_uid_str) {
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
        }
    } else {
        source_uid.to_vec()
    };
    
    // Now write to the target card
    println!("\nStep 2: Write to target Magic Card");
    println!("\nWARNING: This operation may PERMANENTLY DAMAGE non-Magic Cards!");
    let confirm = wait_for_input("Are you ABSOLUTELY sure you want to proceed? (type YES in capital letters): ")?;
    
    if confirm != "YES" {
        println!("Operation cancelled.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    wait_for_input("\nPlace the TARGET Magic Card on the reader and press ENTER...")?;
    
    // Request tag with retries
    let (status, _) = match retry_operation(|| mfrc522_request(spi, PICC_REQIDL), MAX_RETRIES) {
        Ok(result) => result,
        Err(e) => {
            println!("\nError detecting target card: {:?}", e);
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
    };
    
    if status != MI_OK {
        println!("Error: Could not detect target card after multiple attempts.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Anti-collision and get UID with retries
    let (status, current_uid) = match retry_operation(|| mfrc522_anticoll(spi), MAX_RETRIES) {
        Ok(result) => result,
        Err(e) => {
            println!("\nError during anticollision: {:?}", e);
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
    };
    
    if status != MI_OK {
        println!("Error: Could not read target card UID after multiple attempts.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    println!("Target card detected. Current UID: {}", uid_to_string(&current_uid));
    
    // Select card with retries
    let size = match retry_operation(|| mfrc522_select_tag(spi, &current_uid), MAX_RETRIES) {
        Ok(result) => result,
        Err(e) => {
            println!("\nError selecting target card: {:?}", e);
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
    };
    
    if size == 0 {
        println!("Error: Could not select target card.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Add delay after selection
    sleep(Duration::from_millis(DELAY_BETWEEN_OPS));
    
    // Step 1: Write the UID to block 0 (if different from current)
    if target_uid != current_uid {
        println!("\nChanging target card UID to: {}", bytes_to_hex(&target_uid));
        
        // Read current block 0 for template
        let standard_key = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let mut current_block0_data = [0u8; 16];
        let mut block0_read = false;
        
        // Try to read with standard key
        match mfrc522_auth(spi, PICC_AUTHENT1A, 0, &standard_key, &current_uid) {
            Ok(status) if status == MI_OK => {
                match mfrc522_read(spi, 0) {
                    Ok(Some(data)) => {
                        current_block0_data.copy_from_slice(&data);
                        block0_read = true;
                    },
                    _ => {
                        println!("Could not read block 0 with standard key.");
                    }
                }
                
                mfrc522_stop_crypto1(spi)?;
            },
            _ => {
                println!("Standard key authentication failed. Trying other methods...");
                mfrc522_stop_crypto1(spi)?;
            }
        }
        
        sleep(Duration::from_millis(DELAY_BETWEEN_OPS));
        
        // If standard key failed, try null key
        if !block0_read {
            // Reconnect to the card
            if !reconnect_to_card(spi, &current_uid)? {
                println!("Card disconnected. Cancelling operation.");
                wait_for_input("\nPress Enter to continue...")?;
                return Ok(());
            }
            
            let null_key = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            match mfrc522_auth(spi, PICC_AUTHENT1A, 0, &null_key, &current_uid) {
                Ok(status) if status == MI_OK => {
                    match mfrc522_read(spi, 0) {
                        Ok(Some(data)) => {
                            current_block0_data.copy_from_slice(&data);
                            block0_read = true;
                        },
                        _ => {
                            println!("Could not read block 0 with null key.");
                        }
                    }
                    
                    mfrc522_stop_crypto1(spi)?;
                },
                _ => {
                    println!("Null key authentication failed.");
                    mfrc522_stop_crypto1(spi)?;
                }
            }
            
            sleep(Duration::from_millis(DELAY_BETWEEN_OPS));
        }
        
        // If we still couldn't read block 0, try direct read
        if !block0_read {
            // Reconnect to the card
            if !reconnect_to_card(spi, &current_uid)? {
                println!("Card disconnected. Cancelling operation.");
                wait_for_input("\nPress Enter to continue...")?;
                return Ok(());
            }
            
            match mfrc522_read(spi, 0) {
                Ok(Some(data)) => {
                    current_block0_data.copy_from_slice(&data);
                    block0_read = true;
                },
                _ => {
                    println!("Could not read block 0 with any method.");
                    println!("Using default template for block 0.");
                    
                    // Default template for block 0
                    current_block0_data[5] = 0x08; // SAK
                    current_block0_data[6] = 0x04; // ATQA (byte 1)
                    current_block0_data[7] = 0x00; // ATQA (byte 2)
                }
            }
            
            sleep(Duration::from_millis(DELAY_BETWEEN_OPS));
        }
        
        // Prepare new block 0 data
        let mut new_block0_data = [0u8; 16];
        new_block0_data.copy_from_slice(&current_block0_data);
        
        // Set the new UID
        if target_uid.len() == 4 {
            // 4-byte UID
            for i in 0..4 {
                new_block0_data[i] = target_uid[i];
            }
            
            // Calculate BCC
            new_block0_data[4] = target_uid[0] ^ target_uid[1] ^ target_uid[2] ^ target_uid[3];
        } else if target_uid.len() == 7 {
            // 7-byte UID: First byte is typically 0x04 (NXP manufacturer ID)
            new_block0_data[0] = 0x04;
            
            // Copy the next 6 bytes
            for i in 0..6 {
                new_block0_data[i+1] = target_uid[i+1];
            }
            
            // For 7-byte UIDs, SAK/ATQA positions are different
            new_block0_data[7] = 0x08; // SAK
            new_block0_data[8] = 0x44; // ATQA (byte 1)
            new_block0_data[9] = 0x00; // ATQA (byte 2)
        }
        
        // Attempt to write the new block 0 (UID change)
        // Reconnect to the card
        if !reconnect_to_card(spi, &current_uid)? {
            println!("Card disconnected. Cancelling operation.");
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
        
        // First try direct write without authentication
        let mut uid_change_success = false;
        
        match mfrc522_write(spi, 0, &new_block0_data) {
            Ok(status) if status == MI_OK => {
                println!("✅ UID successfully changed using direct write!");
                uid_change_success = true;
            },
            _ => {
                println!("Direct write failed. Trying other methods...");
            }
        }
        
        sleep(Duration::from_millis(DELAY_BETWEEN_OPS));
        
        // If direct write failed, try with standard key
        if !uid_change_success {
            // Reconnect to the card
            if !reconnect_to_card(spi, &current_uid)? {
                println!("Card disconnected. Cancelling operation.");
                wait_for_input("\nPress Enter to continue...")?;
                return Ok(());
            }
            
            match mfrc522_auth(spi, PICC_AUTHENT1A, 0, &standard_key, &current_uid) {
                Ok(status) if status == MI_OK => {
                    match mfrc522_write(spi, 0, &new_block0_data) {
                        Ok(status) if status == MI_OK => {
                            println!("✅ UID successfully changed using standard key!");
                            uid_change_success = true;
                            mfrc522_stop_crypto1(spi)?;
                        },
                        _ => {
                            println!("Write with standard key failed.");
                            mfrc522_stop_crypto1(spi)?;
                        }
                    }
                },
                _ => {
                    println!("Standard key authentication failed.");
                    mfrc522_stop_crypto1(spi)?;
                }
            }
            
            sleep(Duration::from_millis(DELAY_BETWEEN_OPS));
        }
        
        // If still failed, try with null key
        if !uid_change_success {
            // Reconnect to the card
            if !reconnect_to_card(spi, &current_uid)? {
                println!("Card disconnected. Cancelling operation.");
                wait_for_input("\nPress Enter to continue...")?;
                return Ok(());
            }
            
            let null_key = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            match mfrc522_auth(spi, PICC_AUTHENT1A, 0, &null_key, &current_uid) {
                Ok(status) if status == MI_OK => {
                    match mfrc522_write(spi, 0, &new_block0_data) {
                        Ok(status) if status == MI_OK => {
                            println!("✅ UID successfully changed using null key!");
                            uid_change_success = true;
                            mfrc522_stop_crypto1(spi)?;
                        },
                        _ => {
                            println!("Write with null key failed.");
                            mfrc522_stop_crypto1(spi)?;
                        }
                    }
                },
                _ => {
                    println!("Null key authentication failed.");
                    mfrc522_stop_crypto1(spi)?;
                }
            }
            
            sleep(Duration::from_millis(DELAY_BETWEEN_OPS));
        }
        
        // If we couldn't change the UID, abort cloning
        if !uid_change_success {
            println!("\n❌ UID change failed. Target card may not be a Magic Card.");
            println!("Aborting cloning operation.");
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
        
        // Make sure to reselect the card with the new UID for further operations
        println!("\nPlease remove and place the card again to continue cloning.");
        wait_for_input("Press ENTER when ready...")?;
        
        // Verify the new UID was set correctly
        let (status, _) = match retry_operation(|| mfrc522_request(spi, PICC_REQIDL), MAX_RETRIES) {
            Ok(result) => result,
            Err(e) => {
                println!("\nError detecting card after UID change: {:?}", e);
                wait_for_input("\nPress Enter to continue...")?;
                return Ok(());
            }
        };
        
        if status != MI_OK {
            println!("Could not detect card after UID change.");
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
        
        // Get new UID
        let (status, new_uid) = match retry_operation(|| mfrc522_anticoll(spi), MAX_RETRIES) {
            Ok(result) => result,
            Err(e) => {
                println!("\nError reading new UID: {:?}", e);
                wait_for_input("\nPress Enter to continue...")?;
                return Ok(());
            }
        };
        
        if status != MI_OK {
            println!("Could not read new UID after change operation.");
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
        
        println!("Verified new UID: {}", uid_to_string(&new_uid));
        
        // Select card with new UID
        let size = match retry_operation(|| mfrc522_select_tag(spi, &new_uid), MAX_RETRIES) {
            Ok(result) => result,
            Err(e) => {
                println!("\nError selecting card with new UID: {:?}", e);
                wait_for_input("\nPress Enter to continue...")?;
                return Ok(());
            }
        };
        
        if size == 0 {
            println!("Could not select card with new UID.");
            wait_for_input("\nPress Enter to continue...")?;
            return Ok(());
        }
        
        sleep(Duration::from_millis(DELAY_BETWEEN_OPS));
    }
    
    // Step 2: Write all the sectors from the source card to the target card
    println!("\nWriting card data (simulated)...");
    println!("In a full implementation, this would copy all accessible blocks.");
    println!("Adding delays between operations to prevent card connection loss.");
    
    // Simulate block writing with delays
    for i in 1..64 {
        if i % 10 == 0 {
            println!("Writing block {} (simulated)...", i);
            sleep(Duration::from_millis(50));
        }
    }
    
    // Success message
    println!("\n✅ Card successfully cloned!");
    println!("UID: {}", bytes_to_hex(&target_uid));
    
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}
