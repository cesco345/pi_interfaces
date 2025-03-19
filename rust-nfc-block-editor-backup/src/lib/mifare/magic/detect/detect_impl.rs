use std::error::Error;
use rppal::spi::Spi;

use super::types::{TestResult, DetectionResult};
use super::card_tests::{test_read_methods, test_authentication, test_unusual_commands};
use super::write_tests::{test_safe_write, test_bcc_modification};
use super::activation::test_activation_sequences;
use super::utils::{format_data_as_hex, select_card};

use crate::lib::ui_mod::common::{clear_screen, wait_for_input, countdown_for_card_placement};
use super::super::{reconnect_to_card};

/// Main function for detecting magic cards
pub fn detect_magic_card(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("DETECT MAGIC CARD");
    println!("================");
    println!("");
    println!("This function attempts to detect Magic Cards by testing their behavior.");
    println!("Connection issues may occur during testing - the tool will attempt to reconnect.");
    
    // Wait for card placement
    println!("Prepare your card. You have 5 seconds to place it on the reader...");
    countdown_for_card_placement(5)?;
    println!("Reading card now...");
    
    // Select the card and get UID
    let (card_uid, _) = match select_card(spi)? {
        Some(data) => data,
        None => return Ok(()),
    };
    
    println!("\nCard detected. UID: {}", crate::lib::utils::uid_to_string(&card_uid));
    println!("\nPerforming magic card detection tests...");
    println!("Testing various properties and behaviors that indicate a Magic Card.");
    
    // Initialize results
    let mut result = DetectionResult::new();
    
    // Block 0 data for reference in tests
    let mut block0_data: Option<[u8; 16]> = None;
    
    // ==================================================================================
    // PHASE 1: Non-invasive information gathering
    // ==================================================================================
    println!("\nPhase 1: Non-invasive tests...");
    
    // Test block read methods
    let (read_result, data) = test_read_methods(spi, &card_uid)?;
    result.add_test(&read_result);
    
    // Update block0_data if available
    if let Some(block_data) = data {
        let mut block0 = [0u8; 16];
        block0.copy_from_slice(&block_data);
        block0_data = Some(block0);
    }
    
    // Test key authentication
    let (auth_result, auth_data) = test_authentication(spi, &card_uid)?;
    result.add_test(&auth_result);
    
    // Update block0_data if now available
    if block0_data.is_none() && auth_data.is_some() {
        let mut block0 = [0u8; 16];
        block0.copy_from_slice(&auth_data.unwrap());
        block0_data = Some(block0);
    }
    
    // Test unusual command sequences
    let command_result = test_unusual_commands(spi, &card_uid)?;
    result.add_test(&command_result);
    
    // ==================================================================================
    // PHASE 2: Safe modification tests (using block 0 data we already have)
    // ==================================================================================
    println!("\nPhase 2: Safe modification tests...");
    
    if let Some(block0) = block0_data {
        // Perform safe write tests (writing same data back)
        let write_result = test_safe_write(spi, &card_uid, &block0)?;
        result.add_test(&write_result);
        
        // If write was successful, it's definitely a magic card
        if write_result.passed {
            result.magic_card = true;
        } else {
            // If normal tests didn't work, try magic card activation sequences
            let activation_result = test_activation_sequences(spi, &card_uid, &block0)?;
            result.add_test(&activation_result);
            
            // Set magic_card flag if any activation sequence worked
            if activation_result.passed {
                result.magic_card = true;
            }
        }
    } else {
        println!("Skipping modification tests - couldn't read block 0 data");
    }
    
    // ==================================================================================
    // PHASE 3: Small modification test - only if explicitly allowed by user
    // ==================================================================================
    // Only proceed with risky tests if needed and user agrees
    if !result.magic_card && result.total_score > 0 && block0_data.is_some() {
        println!("\nThis card shows some unusual behaviors but hasn't been definitively identified.");
        println!("A more conclusive test would involve making a small, temporary change to the card.");
        
        let confirmation = wait_for_input("Would you like to perform this test? (y/n): ")?.to_lowercase();
        if confirmation == "y" || confirmation == "yes" {
            let bcc_result = test_bcc_modification(spi, &card_uid, block0_data.unwrap())?;
            result.add_test(&bcc_result);
            
            if bcc_result.passed {
                result.magic_card = true;
            }
        }
    }
    
    // ==================================================================================
    // Display Results
    // ==================================================================================
    display_results(&card_uid, &result);
    
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}

/// Display detection results
fn display_results(card_uid: &[u8], result: &DetectionResult) {
    println!("\n================ DETECTION RESULTS ================");
    
    // Determine based on both score and explicit magic_card flag
    let is_magic = result.magic_card || result.total_score >= 4;
    
    if is_magic {
        println!("✅ This appears to be a MAGIC CARD!");
        println!("Magic score: {}/25", result.total_score);
        
        // Show findings
        if !result.get_all_notes().is_empty() {
            println!("\nDetected magic card capabilities:");
            for note in result.get_all_notes() {
                println!(" • {}", note);
            }
        }
        
        // Determine card type based on passing tests
        if result.has_passing_test("Safe write test") {
            println!("\nThis is a Gen1 Magic Card with direct write capabilities.");
        } else if result.has_passing_test("Activation sequence test") {
            println!("\nThis is a Gen2 Magic Card that requires activation sequences.");
        } else if result.has_passing_test("BCC modification test") {
            println!("\nThis is a Magic Card that allows direct modification of Block 0.");
        } else {
            println!("\nThis is likely a Magic Card based on its unusual behavior.");
        }
        
        println!("\nYou can use the 'Write Custom UID' function to attempt UID modification.");
        println!("You can also try the 'Clone Card' function to copy another card.");
    } 
    else {
        println!("❌ This appears to be a standard MIFARE card.");
        println!("Magic score: {}/25", result.total_score);
        
        if result.total_score > 0 {
            println!("\nHowever, this card showed some unusual behaviors:");
            for note in result.get_all_notes() {
                println!(" • {}", note);
            }
            println!("\nIf you want to test further, you can try the 'Write Custom UID' function.");
        } else {
            println!("\nNo Magic Card features were detected in standard tests.");
            println!("\nThis card follows normal MIFARE security protocols, including:");
            println!(" • Requires proper authentication before reading/writing");
            println!(" • Rejects non-standard keys");
            println!(" • Protects block 0 (UID) from modification");
        }
        
        println!("\nNOTE: Some advanced Magic Cards hide their capabilities until activated.");
        println!("If you believe this is a Magic Card, you can still try the");
        println!("'Write Custom UID' function, but proceed with caution.");
    }
}
