// Enhanced Magic Card detection implementation
// Add this to your src/lib/mifare/magic/detect directory

use std::error::Error;
use std::thread::sleep;
use std::time::Duration;
use rppal::spi::Spi;

use crate::lib::mfrc522::{
    mfrc522_request, mfrc522_anticoll, mfrc522_select_tag, mfrc522_auth,
    mfrc522_stop_crypto1, mfrc522_read, mfrc522_write, mfrc522_to_card, 
    read_register, write_register, set_bit_mask, clear_bit_mask,
    PICC_REQIDL, PICC_AUTHENT1A, PICC_AUTHENT1B, MI_OK, PCD_TRANSCEIVE,
    STATUS2_REG, COMMAND_REG, BIT_FRAMING_REG, CONTROL_REG
};
use crate::lib::utils::{uid_to_string, bytes_to_hex};
use crate::lib::ui_mod::common::{clear_screen, wait_for_input, countdown_for_card_placement};
use super::types::{TestResult, DetectionResult};
use super::utils::{format_data_as_hex, select_card};
use super::super::{reconnect_to_card};

/// Advanced detection for Magic Cards
/// This function implements additional detection tests targeting specific Magic Card types
pub fn advanced_detect_magic_card(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("ADVANCED MAGIC CARD DETECTION");
    println!("============================");
    println!("");
    println!("This function uses specialized tests for detecting various Magic Card types.");
    println!("It combines non-invasive tests with targeted detection for specific cards.");
    
    // Wait for card placement
    println!("Prepare your card. You have 5 seconds to place it on the reader...");
    countdown_for_card_placement(5)?;
    println!("Reading card now...");
    
    // Select the card and get UID
    let (card_uid, _) = match select_card(spi)? {
        Some(data) => data,
        None => return Ok(()),
    };
    
    println!("\nCard detected. UID: {}", uid_to_string(&card_uid));
    println!("\nPerforming advanced magic card detection tests...");
    
    // Initialize results
    let mut result = DetectionResult::new();
    
    // Try to read block 0 for reference
    let mut block0_data: Option<[u8; 16]> = None;
    
    // ==================================================================================
    // PHASE 1: Basic card information and standard tests
    // ==================================================================================
    println!("\nPhase 1: Card information analysis...");
    
    // Basic test: Does the card allow direct read of block 0?
    if reconnect_to_card(spi, &card_uid)? {
        match mfrc522_read(spi, 0) {
            Ok(Some(data)) => {
                println!("✅ Card allows direct read of block 0 without authentication!");
                println!("   Block 0 data: {}", format_data_as_hex(&data));
                let mut test_result = TestResult::new("Direct read test");
                test_result.set_passed(3);
                test_result.add_note("Allows reading block 0 without authentication");
                result.add_test(&test_result);
                
                // Store block 0 data
                let mut block0 = [0u8; 16];
                block0.copy_from_slice(&data);
                block0_data = Some(block0);
            },
            _ => {
                println!("Card requires authentication to read block 0 (normal behavior)");
            }
        }
    }
    
    // Test standard authentication to get block 0 if we don't have it
    if block0_data.is_none() && reconnect_to_card(spi, &card_uid)? {
        let standard_key = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        
        match mfrc522_auth(spi, PICC_AUTHENT1A, 0, &standard_key, &card_uid) {
            Ok(status) if status == MI_OK => {
                match mfrc522_read(spi, 0) {
                    Ok(Some(data)) => {
                        println!("Read block 0 with standard key: {}", format_data_as_hex(&data));
                        let mut block0 = [0u8; 16];
                        block0.copy_from_slice(&data);
                        block0_data = Some(block0);
                    },
                    _ => {}
                }
                mfrc522_stop_crypto1(spi)?;
            },
            _ => {
                mfrc522_stop_crypto1(spi)?;
            }
        }
    }
    
    // ==================================================================================
    // PHASE 2: Targeted specific Magic Card type tests
    // ==================================================================================
    println!("\nPhase 2: Specific Magic Card type detection...");
    
    // Test for Gen1 Magic Cards (Chinese Magic Cards/CUID)
    if reconnect_to_card(spi, &card_uid)? {
        let gen1_test = test_gen1_magic_card(spi, &card_uid, block0_data.as_ref())?;
        result.add_test(&gen1_test);
        
        if gen1_test.passed {
            println!("✅ Gen1 Magic Card (CUID) detected!");
            result.magic_card = true;
        }
    }
    
    // Test for Gen2 Magic Cards (FUID)
    if !result.magic_card && reconnect_to_card(spi, &card_uid)? {
        let gen2_test = test_gen2_magic_card(spi, &card_uid, block0_data.as_ref())?;
        result.add_test(&gen2_test);
        
        if gen2_test.passed {
            println!("✅ Gen2 Magic Card (FUID) detected!");
            result.magic_card = true;
        }
    }
    
    // Test for Gen3 Magic Cards (UID)
    if !result.magic_card && reconnect_to_card(spi, &card_uid)? {
        let gen3_test = test_gen3_magic_card(spi, &card_uid, block0_data.as_ref())?;
        result.add_test(&gen3_test);
        
        if gen3_test.passed {
            println!("✅ Gen3 Magic Card (UID) detected!");
            result.magic_card = true;
        }
    }
    
    // Test for special variant Magic Cards
    if !result.magic_card && reconnect_to_card(spi, &card_uid)? {
        let special_test = test_special_magic_card(spi, &card_uid)?;
        result.add_test(&special_test);
        
        if special_test.passed {
            println!("✅ Special variant Magic Card detected!");
            result.magic_card = true;
        }
    }
    
    // ==================================================================================
    // PHASE 3: Direct write test (if nothing else detected)
    // ==================================================================================
    if !result.magic_card && block0_data.is_some() && reconnect_to_card(spi, &card_uid)? {
        println!("\nPhase 3: Direct write capability test...");
        
        let confirm = wait_for_input("Perform direct write test? This is the most reliable but potentially risky test (y/n): ")?.to_lowercase();
        
        if confirm == "y" || confirm == "yes" {
            let direct_write_test = test_direct_write(spi, &card_uid, block0_data.unwrap())?;
            result.add_test(&direct_write_test);
            
            if direct_write_test.passed {
                println!("✅ Card allows direct block 0 modification!");
                result.magic_card = true;
            }
        }
    }
    
    // ==================================================================================
    // Display final results
    // ==================================================================================
    println!("\n================ DETECTION RESULTS ================");
    
    if result.magic_card || result.total_score >= 3 {
        println!("✅ MAGIC CARD DETECTED!");
        println!("Magic score: {}/25", result.total_score);
        
        // Show card capabilities
        println!("\nDetected capabilities:");
        for note in result.get_all_notes() {
            println!(" • {}", note);
        }
        
        // Identify most likely card type
        if result.has_passing_test("Gen1 Magic Card test") {
            println!("\nCard type: Gen1 Magic Card (CUID)");
            println!("This card likely requires the 0x40-0x43 command sequence for activation.");
        } else if result.has_passing_test("Gen2 Magic Card test") {
            println!("\nCard type: Gen2 Magic Card (FUID)");
            println!("This card likely requires the 0x40-0x01 command sequence for activation.");
        } else if result.has_passing_test("Gen3 Magic Card test") {
            println!("\nCard type: Gen3 Magic Card (UID)");
            println!("This card likely uses the newer protocol with 0x90 commands.");
        } else if result.has_passing_test("Direct write test") {
            println!("\nCard type: Direct-writeable Magic Card");
            println!("This card allows direct block 0 modification without special activation.");
        } else {
            println!("\nCard type: Unknown Magic Card variant");
            println!("This card shows Magic Card behavior but couldn't be precisely identified.");
        }
    } else {
        println!("❌ No definitive Magic Card features detected.");
        println!("Magic score: {}/25", result.total_score);
        
        if result.total_score > 0 {
            println!("\nSome unusual behaviors were detected:");
            for note in result.get_all_notes() {
                println!(" • {}", note);
            }
            println!("\nThis might be:");
            println!(" • A standard card with unusual features");
            println!(" • A Magic Card that requires a specific activation sequence");
            println!(" • A newer generation Magic Card not covered by these tests");
        } else {
            println!("\nThis appears to be a standard MIFARE Classic card.");
        }
    }
    
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}

/// Test for Gen1 Magic Cards (CUID)
fn test_gen1_magic_card(spi: &mut Spi, card_uid: &[u8], block0: Option<&[u8; 16]>) -> Result<TestResult, Box<dyn Error>> {
    let mut result = TestResult::new("Gen1 Magic Card test");
    
    // Try Gen1 activation sequence
    println!("Testing for Gen1 Magic Card (CUID)...");
    
    // Special CUID command sequence
    let cuid_cmd = [0x40, 0x43];
    let (status, resp, _) = match mfrc522_to_card(spi, PCD_TRANSCEIVE, &cuid_cmd) {
        Ok(result) => result,
        Err(_) => (0, Vec::new(), 0),
    };
    
    let activated = status == MI_OK || (status != 0 && resp.len() > 0);
    
    if activated {
        println!("✅ Card responded to CUID activation sequence");
        result.score += 2;
        result.add_note("Responds to CUID activation (Gen1)");
        
        // If we have block0 data, try writing it back after activation
        if let Some(block0_data) = block0 {
            match mfrc522_write(spi, 0, block0_data) {
                Ok(status) if status == MI_OK => {
                    println!("✅ Successfully wrote to block 0 after CUID activation!");
                    result.set_passed(8);
                    result.add_note("Allows writing to block 0 after CUID activation");
                    return Ok(result);
                },
                _ => {}
            }
        }
        
        // Try direct write with modified data
        let mut test_data = [0xAA; 16];
        if let Some(block0_data) = block0 {
            test_data.copy_from_slice(block0_data);
            test_data[5] ^= 0x01; // Small change that shouldn't affect functionality
        }
        
        match mfrc522_write(spi, 1, &test_data) {
            Ok(status) if status == MI_OK => {
                println!("✅ Successfully wrote to block 1 after CUID activation!");
                result.set_passed(5);
                result.add_note("Allows writing to block 1 after CUID activation");
                
                // Try to restore original data if we have it
                if let Some(block0_data) = block0 {
                    let _ = mfrc522_write(spi, 1, block0_data);
                }
                
                return Ok(result);
            },
            _ => {}
        }
    } else {
        // Try alternate Gen1 command
        let alt_cmd = [0x40, 0x40];
        let (status, resp, _) = match mfrc522_to_card(spi, PCD_TRANSCEIVE, &alt_cmd) {
            Ok(result) => result,
            Err(_) => (0, Vec::new(), 0),
        };
        
        if status == MI_OK || (status != 0 && resp.len() > 0) {
            println!("✅ Card responded to alternate Gen1 activation");
            result.score += 1;
            result.add_note("Responds to alternate Gen1 activation");
        }
    }
    
    Ok(result)
}

/// Test for Gen2 Magic Cards (FUID)
fn test_gen2_magic_card(spi: &mut Spi, card_uid: &[u8], block0: Option<&[u8; 16]>) -> Result<TestResult, Box<dyn Error>> {
    let mut result = TestResult::new("Gen2 Magic Card test");
    
    // Try Gen2 activation sequence
    println!("Testing for Gen2 Magic Card (FUID)...");
    
    // FUID activation sequence is typically 0x40, 0x01
    let fuid_cmd = [0x40, 0x01];
    let (status, resp, _) = match mfrc522_to_card(spi, PCD_TRANSCEIVE, &fuid_cmd) {
        Ok(result) => result,
        Err(_) => (0, Vec::new(), 0),
    };
    
    let activated = status == MI_OK || (status != 0 && resp.len() > 0);
    
    if activated {
        println!("✅ Card responded to FUID activation sequence");
        result.score += 2;
        result.add_note("Responds to FUID activation (Gen2)");
        
        // If we have block0 data, try writing it back after activation
        if let Some(block0_data) = block0 {
            match mfrc522_write(spi, 0, block0_data) {
                Ok(status) if status == MI_OK => {
                    println!("✅ Successfully wrote to block 0 after FUID activation!");
                    result.set_passed(8);
                    result.add_note("Allows writing to block 0 after FUID activation");
                    return Ok(result);
                },
                _ => {}
            }
        }
        
        // Try the secondary FUID command
        let fuid_cmd2 = [0x43, 0x01];
        mfrc522_to_card(spi, PCD_TRANSCEIVE, &fuid_cmd2).ok();
        
        if let Some(block0_data) = block0 {
            match mfrc522_write(spi, 0, block0_data) {
                Ok(status) if status == MI_OK => {
                    println!("✅ Successfully wrote to block 0 after secondary FUID activation!");
                    result.set_passed(8);
                    result.add_note("Responds to secondary FUID activation sequence");
                    return Ok(result);
                },
                _ => {}
            }
        }
    }
    
    // Try alternate Gen2 pattern
    if !result.passed {
        // Some Gen2 cards use 0x72 command
        let alt_cmd = [0x72, 0x00];
        let (status, resp, _) = match mfrc522_to_card(spi, PCD_TRANSCEIVE, &alt_cmd) {
            Ok(result) => result,
            Err(_) => (0, Vec::new(), 0),
        };
        
        if status == MI_OK || (status != 0 && resp.len() > 0) {
            println!("✅ Card responded to alternate Gen2 activation");
            result.score += 1;
            result.add_note("Responds to alternate Gen2 activation");
            
            // If we have block0 data, try writing it back after activation
            if let Some(block0_data) = block0 {
                match mfrc522_write(spi, 0, block0_data) {
                    Ok(status) if status == MI_OK => {
                        println!("✅ Successfully wrote to block 0 after alternate Gen2 activation!");
                        result.set_passed(6);
                        result.add_note("Allows writing to block 0 after alternate Gen2 activation");
                        return Ok(result);
                    },
                    _ => {}
                }
            }
        }
    }
    
    Ok(result)
}

/// Test for Gen3 Magic Cards (UID)
fn test_gen3_magic_card(spi: &mut Spi, card_uid: &[u8], block0: Option<&[u8; 16]>) -> Result<TestResult, Box<dyn Error>> {
    let mut result = TestResult::new("Gen3 Magic Card test");
    
    // Try Gen3 activation sequence
    println!("Testing for Gen3 Magic Card...");
    
    // Gen3 cards often use 0x90 commands
    let gen3_cmds = [
        [0x90, 0xF0], // Read signature
        [0x90, 0xFB], // Enter backdoor mode
        [0x90, 0xFC], // Special mode
    ];
    
    let mut success = false;
    
    for cmd in gen3_cmds.iter() {
        let (status, resp, _) = match mfrc522_to_card(spi, PCD_TRANSCEIVE, cmd) {
            Ok(result) => result,
            Err(_) => (0, Vec::new(), 0),
        };
        
        if status == MI_OK && resp.len() > 1 {
            println!("✅ Card responded to Gen3 command: {:02X}{:02X}", cmd[0], cmd[1]);
            println!("   Response: {}", format_data_as_hex(&resp));
            result.score += 2;
            result.add_note(&format!("Responds to Gen3 command {:02X}{:02X}", cmd[0], cmd[1]));
            success = true;
            
            // If we got a response from 0x90 0xF0, check if it matches known signatures
            if cmd[0] == 0x90 && cmd[1] == 0xF0 && resp.len() >= 8 {
                let sig_start = format_data_as_hex(&resp[0..2]);
                if sig_start == "04 49" {
                    println!("✅ Detected NXP Gen3 Magic Card signature!");
                    result.score += 3;
                    result.set_passed(8);
                    result.add_note("Has NXP Gen3 Magic Card signature");
                    return Ok(result);
                }
            }
            
            // Try writing to block 0 after command
            if let Some(block0_data) = block0 {
                match mfrc522_write(spi, 0, block0_data) {
                    Ok(status) if status == MI_OK => {
                        println!("✅ Successfully wrote to block 0 after Gen3 command!");
                        result.set_passed(8);
                        result.add_note(&format!("Allows writing to block 0 after Gen3 command {:02X}{:02X}", cmd[0], cmd[1]));
                        return Ok(result);
                    },
                    _ => {}
                }
            }
        }
    }
    
    if success && !result.passed {
        // If we saw some Gen3 behavior but no write success, still flag it
        result.score += 1;
    }
    
    Ok(result)
}

/// Test for special variant Magic Cards
fn test_special_magic_card(spi: &mut Spi, card_uid: &[u8]) -> Result<TestResult, Box<dyn Error>> {
    let mut result = TestResult::new("Special Magic Card test");
    
    // Test for special cards that need unusual command sequences
    println!("Testing for special Magic Card variants...");
    
    // Some cards need custom commands with UID bytes
    if card_uid.len() >= 4 {
        // Create a card-specific command using UID bytes
        let custom_cmd = [card_uid[0], card_uid[1]];
        let (status, resp, _) = match mfrc522_to_card(spi, PCD_TRANSCEIVE, &custom_cmd) {
            Ok(result) => result,
            Err(_) => (0, Vec::new(), 0),
        };
        
        if status == MI_OK || (status != 0 && resp.len() > 0) {
            println!("✅ Card responded to UID-based custom command");
            result.score += 2;
            result.add_note("Responds to UID-based command sequence");
        }
    }
    
    // Check for backdoor authentication
    let backdoor_keys = [
        [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5], // Common magic backdoor
        [0xD3, 0xF7, 0xD3, 0xF7, 0xD3, 0xF7], // Another backdoor key
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55], // Variant backdoor
    ];
    
    for key in backdoor_keys.iter() {
        if !reconnect_to_card(spi, card_uid)? {
            continue;
        }
        
        match mfrc522_auth(spi, PICC_AUTHENT1A, 0, key, card_uid) {
            Ok(status) if status == MI_OK => {
                println!("✅ Card accepts backdoor key: {}", format_data_as_hex(key));
                result.score += 2;
                result.add_note(&format!("Accepts backdoor key: {}", format_data_as_hex(key)));
                
                // Try to read with this key
                match mfrc522_read(spi, 0) {
                    Ok(Some(data)) => {
                        println!("   Block 0 data: {}", format_data_as_hex(&data));
                        result.score += 1;
                    },
                    _ => {}
                }
                
                mfrc522_stop_crypto1(spi)?;
                
                // If we found a backdoor key, flag as special magic card
                result.set_passed(4);
                return Ok(result);
            },
            _ => {
                mfrc522_stop_crypto1(spi)?;
            }
        }
    }
    
    // Check for unusual register behavior
    // Some Magic Cards respond differently to register operations
    if reconnect_to_card(spi, card_uid)? {
        // Read the current value of a register
        let orig_value = read_register(spi, STATUS2_REG)?;
        
        // Write a different value and read it back
        write_register(spi, STATUS2_REG, orig_value ^ 0x0F)?;
        let new_value = read_register(spi, STATUS2_REG)?;
        
        // Restore the original value
        write_register(spi, STATUS2_REG, orig_value)?;
        
        // Check if behavior was unusual
        if new_value != orig_value && new_value != (orig_value ^ 0x0F) {
            println!("✅ Card shows unusual register behavior");
            result.score += 2;
            result.add_note("Shows unusual register behavior (potential Magic Card)");
            
            // This is suspicious but not definitive
            if !result.passed && result.score >= 3 {
                result.passed = true;
            }
        }
    }
    
    Ok(result)
}

/// Test direct write capability
fn test_direct_write(spi: &mut Spi, card_uid: &[u8], block0: [u8; 16]) -> Result<TestResult, Box<dyn Error>> {
    let mut result = TestResult::new("Direct write test");
    
    println!("Testing direct write capability...");
    
    // Create a modified version that's harmless (e.g., change manufacturer byte)
    let mut test_block = block0.clone();
    
    // Make a small change to SAK byte (byte 5)
    // This is safer than changing the actual UID bytes
    let original_sak = test_block[5];
    test_block[5] = original_sak ^ 0x01; // Toggle just one bit
    
    println!("Original SAK: {:02X}, Test SAK: {:02X}", original_sak, test_block[5]);
    
    // Try direct write
    match mfrc522_write(spi, 0, &test_block) {
        Ok(status) if status == MI_OK => {
            println!("✅ Successfully modified block 0 with direct write!");
            result.set_passed(10);
            result.add_note("Allows direct block 0 writing without authentication");
            
            // Restore original data
            println!("Restoring original data...");
            let _ = mfrc522_write(spi, 0, &block0);
            
            return Ok(result);
        },
        _ => {
            println!("❌ Direct write failed (normal for standard cards)");
        }
    }
    
    // Try with various keys
    let test_keys = [
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], // Default key
        [0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // All zeros
        [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5], // Magic backdoor
        [0xD3, 0xF7, 0xD3, 0xF7, 0xD3, 0xF7], // Another backdoor
    ];
    
    for key in test_keys.iter() {
        if !reconnect_to_card(spi, card_uid)? {
            continue;
        }
        
        match mfrc522_auth(spi, PICC_AUTHENT1A, 0, key, card_uid) {
            Ok(status) if status == MI_OK => {
                match mfrc522_write(spi, 0, &test_block) {
                    Ok(status) if status == MI_OK => {
                        println!("✅ Successfully modified block 0 with key: {}", format_data_as_hex(key));
                        result.set_passed(8);
                        result.add_note(&format!("Allows block 0 writing with key: {}", format_data_as_hex(key)));
                        
                        // Restore original data
                        println!("Restoring original data...");
                        let _ = mfrc522_write(spi, 0, &block0);
                        
                        mfrc522_stop_crypto1(spi)?;
                        return Ok(result);
                    },
                    _ => {
                        mfrc522_stop_crypto1(spi)?;
                    }
                }
            },
            _ => {
                mfrc522_stop_crypto1(spi)?;
            }
        }
    }
    
    println!("❌ All write tests failed (normal for standard cards)");
    
    Ok(result)
}
