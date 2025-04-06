// src/bin/mifare_writer.rs
// Main entry point for the MIFARE writer application

use std::error::Error;

// Import the needed modules from our crate
use ndef_explorer::operations::mifare::{
    // Card connection functions
    connect_to_card,
    verify_mifare_classic,
    
    // Writing functions
    simple_direct_write,
    format_mifare_classic,
    direct_write_mifare,
    
    // Data handler functions
    load_card_data,
    check_format_compatibility,
    get_user_confirmation,
    wait_for_card
};

fn main() -> Result<(), Box<dyn Error>> {
    println!("MIFARE Classic Card Writer");
    println!("========================\n");
    
    // Load card data from command line or stdin
    let (card_data, force_mode) = load_card_data()?;
    
    // Check format compatibility
    if let Err(e) = check_format_compatibility(&card_data, force_mode) {
        return Err(e);
    }
    
    // Ask for confirmation
    if !get_user_confirmation("\nContinue with writing to MIFARE Classic card?")? {
        println!("Operation cancelled.");
        return Ok(());
    }
    
    // Wait for user to place card on reader
    wait_for_card()?;
    
    // Connect to card
    let (_ctx, card) = connect_to_card()?;
    
    // Verify the card is a MIFARE Classic
    let _ = verify_mifare_classic(&card)?;
    
    // First try: easy direct write method
    println!("\nAttempting simple direct write method...");
    if let Ok(()) = simple_direct_write(&card, &card_data) {
        println!("\n✅ Successfully wrote data using simple direct write method!");
        println!("\nPlease remove the card from the reader when finished.");
        return Ok(());
    }
    
    // Second try: proper NDEF formatting
    println!("\nAttempting NDEF formatting...");
    let result = format_mifare_classic(&card, &card_data);
    
    if result.is_err() {
        println!("\nTrying alternative method with sector scan and direct write...");
        // If regular formatting fails, try more extensive direct write
        match direct_write_mifare(&card, &card_data) {
            Ok(()) => {
                println!("\n✅ Successfully wrote data using alternative method!");
            },
            Err(e) => {
                println!("\n❌ All write methods failed. Error: {}", e);
                
                // Give some diagnostic information
                println!("\nDiagnostic Information:");
                println!("- The card may not be a genuine MIFARE Classic card");
                println!("- The card may be locked or using non-standard keys");
                println!("- Try using 'mifare_ndef_formatter' first to prepare the card");
                
                return Err("Failed to write data to card".into());
            }
        }
    }
    
    println!("\nPlease remove the card from the reader when finished.");
    
    Ok(())
}
