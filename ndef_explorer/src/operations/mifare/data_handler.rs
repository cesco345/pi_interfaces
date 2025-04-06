// Data handling operations for MIFARE writer

use std::error::Error;
use std::fs;
use std::io::{self, BufRead, Write};
use std::env;
use serde_json;
use crate::operations::ndef_operations_writer::CardExport;

/// Load card data from command line arguments or stdin
pub fn load_card_data() -> Result<(CardExport, bool), Box<dyn Error>> {
    // Check command-line arguments for input file
    let args: Vec<String> = env::args().collect();
    let mut json_data = String::new();
    let mut force_mode = false;
    
    // Process arguments
    for arg in &args[1..] {
        if arg == "--force" {
            force_mode = true;
            println!("Force mode enabled: Will attempt to write regardless of format type");
        } else {
            // Load JSON from file
            match fs::read_to_string(arg) {
                Ok(content) => {
                    json_data = content;
                    println!("Loaded data from file: {}", arg);
                },
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    return Err(e.into());
                }
            }
        }
    }
    
    if json_data.is_empty() {
        // Read JSON from stdin
        println!("Paste the exported JSON data (press Enter, then Ctrl+D when finished):");
        
        let stdin = io::stdin();
        let mut lines = stdin.lock().lines();
        
        while let Some(line) = lines.next() {
            match line {
                Ok(line) => json_data.push_str(&format!("{}\n", line)),
                Err(e) => {
                    eprintln!("Error reading input: {}", e);
                    return Err(e.into());
                }
            }
        }
    }
    
    // Parse the JSON data
    let card_data: CardExport = match serde_json::from_str(&json_data) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error parsing JSON data: {}", e);
            return Err(e.into());
        }
    };
    
    // Print card data
    println!("\nCard Information:");
    println!("  Name: {}", card_data.name);
    println!("  Application ID: {}", card_data.applicationId);
    println!("  File ID: {}", card_data.fileId);
    println!("  Data: {}", card_data.fileData);
    println!("  Format: {}", card_data.format);
    println!("  Export Date: {}", card_data.exportDate);
    
    Ok((card_data, force_mode))
}

/// Check if the card data format is compatible
pub fn check_format_compatibility(card_data: &CardExport, force_mode: bool) -> Result<(), Box<dyn Error>> {
    if !force_mode && card_data.format != "mifare_classic" {
        println!("\nWARNING: This JSON file has format '{}' instead of 'mifare_classic'", card_data.format);
        println!("It may not be compatible with MIFARE Classic cards.");
        println!("Use --force flag to attempt writing anyway.");
        return Err("Format incompatibility".into());
    }
    
    Ok(())
}

/// Prepare data for writing to a block (pad/truncate to 16 bytes)
pub fn prepare_block_data(data: &[u8]) -> Vec<u8> {
    let mut block_data = Vec::new();
    block_data.extend_from_slice(data);
    
    // Pad to 16 bytes
    while block_data.len() < 16 {
        block_data.push(0x00);
    }
    
    // Truncate if too long
    if block_data.len() > 16 {
        block_data.truncate(16);
    }
    
    block_data
}

/// Get user confirmation
pub fn get_user_confirmation(prompt: &str) -> Result<bool, Box<dyn Error>> {
    print!("{} (y/n): ", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    Ok(input.trim().to_lowercase() == "y")
}

/// Wait for user to place card on reader
pub fn wait_for_card() -> Result<(), Box<dyn Error>> {
    println!("\nPlace your MIFARE Classic card on the reader and press Enter to continue...");
    io::stdout().flush()?;
    let mut _wait = String::new();
    io::stdin().read_line(&mut _wait)?;
    
    Ok(())
}

