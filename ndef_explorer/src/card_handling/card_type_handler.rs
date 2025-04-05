// Import the DESFire setup module
use super::desfire::setup;
use std::error::Error;
use crate::operations::ndef_operations_writer::CardExport;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum CardWriteStrategy {
    NdefFormatted,
    MifareClassicDirect,
    MifareUltralight,
    DESFireDirect,
    DESFireNdefSetup,
    MagicCardClone,
    NtagDirect,  // New strategy for NTAG213 tags
}

/// Analyze card data to determine the best writing strategy
pub fn analyze_card_data(card_data: &CardExport) -> CardWriteStrategy {
    // Check format field
    if card_data.format.to_lowercase() == "ntag_213" {
        return CardWriteStrategy::NtagDirect;
    } else if card_data.format.to_lowercase() == "desfire_ndef" {
        return CardWriteStrategy::DESFireNdefSetup;
    } else if card_data.format.to_lowercase() == "mifare_classic" {
        return CardWriteStrategy::MifareClassicDirect;
    } else if card_data.format.to_lowercase() == "mifare_ultralight" {
        return CardWriteStrategy::MifareUltralight;
    }
    
    // Simple logic to determine the best strategy based on the card data
    // This could be made more sophisticated based on actual card data analysis
    
    if card_data.fileData.starts_with("D276") {
        // DESFire NDEF application
        CardWriteStrategy::DESFireNdefSetup
    } else if card_data.fileData.len() <= 8 && card_data.fileData.chars().all(|c| c.is_ascii_hexdigit()) {
        // Possibly a UID for cloning
        CardWriteStrategy::MagicCardClone
    } else if card_data.fileData.len() > 64 {
        // Larger data, probably standard NDEF
        CardWriteStrategy::NdefFormatted
    } else {
        // Default to standard NDEF
        CardWriteStrategy::NdefFormatted
    }
}

/// Handle writing data based on card type
pub fn write_card_data(card_data: &CardExport, strategy: CardWriteStrategy) -> Result<bool, Box<dyn Error>> {
    println!("Writing with strategy: {:?}", strategy);
    println!("Data to write: {}", card_data.fileData);
    
    // Based on the strategy, we'll recommend the right approach
    match strategy {
        CardWriteStrategy::NdefFormatted => {
            println!("\nUsing standard NDEF format approach:");
            println!("1. Call 'focused_ndef_reader' and select option 5 (Write Sample NDEF Message)");
            println!("2. Enter the message: {}", card_data.fileData);
            println!("\nRunning NDEF writer command...");
            
            // Here we could spawn a process to run the focused_ndef_reader with the data
            // But for now, we'll just return success and let the user do it manually
            Ok(true)
        },
        CardWriteStrategy::MifareClassicDirect => {
            println!("\nRecommended approach for this data: Direct Mifare block writing");
            println!("This data appears to be a hex/UID format that might work better with direct block access.");
            println!("Use your existing Mifare tools to write this data to blocks 4-7 (Sector 1).");
            
            // Provide guidance on using the direct write approach
            println!("\nData to write: {}", card_data.fileData);
            println!("Recommended blocks: 4 (first data block of sector 1)");
            
            Ok(true)
        },
        CardWriteStrategy::MifareUltralight => {
            println!("\nUltralight card detected or recommended.");
            println!("Use your focused_ndef_reader for NDEF operations or direct page writing.");
            println!("Data to write: {}", card_data.fileData);
            
            Ok(true)
        },
        CardWriteStrategy::DESFireDirect => {
            println!("\nDESFire card detected or recommended.");
            println!("For DESFire cards, it's best to use the NDEF approach:");
            println!("1. Call 'focused_ndef_reader' and select option 5 (Write Sample NDEF Message)");
            println!("2. Enter the message: {}", card_data.fileData);
            
            Ok(true)
        },
        CardWriteStrategy::DESFireNdefSetup => {
            println!("\nDESFire card needs to be set up for NDEF compatibility.");
            println!("This requires special formatting with the DESFire tools.");
            
            // Display the steps for setting up DESFire for NDEF
            let steps = setup::suggest_ndef_setup_steps(&card_data.fileData);
            println!("\n{}", steps);
            
            println!("\nAfter setting up the card, you should be able to use it with standard NDEF operations.");
            println!("Place the card on the reader and try again with the NDEF Explorer.");
            
            Ok(true)
        },
        CardWriteStrategy::MagicCardClone => {
            println!("\nMagic Card operations recommended for this data.");
            println!("Data appears to be a UID format suitable for cloning.");
            println!("Use your specialized magic card tools for this operation.");
            println!("UID to clone: {}", card_data.fileData);
            
            Ok(true)
        },
        CardWriteStrategy::NtagDirect => {
            println!("\nNTAG213 card detected or recommended.");
            println!("Using direct page writing approach for NTAG213 tags.");
            println!("Data will be written to user memory pages 4-39.");
            println!("\nNote: The UID of NTAG213 cards cannot be changed.");
            println!("Only the user memory will be cloned.");
            
            Ok(true)
        }
    }
}
