use std::error::Error;
use std::io;
use std::io::Write;
use std::thread::sleep;
use std::time::Duration;

use desfire_tools::desfire_common::{
    send_apdu, HexSlice
};

// Try direct memory access methods (last resort for clone cards)
pub fn try_direct_access(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("Attempting direct memory access methods (clone card specific)");
    
    // Some clone cards support direct memory access with custom commands
    // These are completely non-standard and vendor-specific
    
    // Some possible commands to try
    let direct_commands = [
        // GET_DATA with various parameters
        [0x00, 0xCA, 0x00, 0x00, 0x00],        // ISO 7816-4 GET DATA
        [0x00, 0xB0, 0x00, 0x00, 0x04],        // ISO 7816-4 READ BINARY
        [0xFF, 0xB0, 0x00, 0x00, 0x04],        // Custom READ BINARY
        [0xFF, 0xCA, 0x00, 0x00, 0x00],        // Factory test command
        
        // Various get card data commands
        [0x80, 0xCA, 0x9F, 0x7F, 0x00],        // Get entire TLV tree
        [0x80, 0xCA, 0x00, 0x00, 0x00],        // Get basic data
        [0x80, 0x60, 0x00, 0x00, 0x00],        // GetVersion alternative
        
        // Non-standard memory access
        [0xFF, 0xF3, 0x00, 0x00, 0x04],        // Direct memory read
        [0xFF, 0xF5, 0x00, 0x00, 0x04],        // Another direct read variant
        
        // GetUID alternative methods
        [0xFF, 0xCA, 0x00, 0x00, 0x00],        // Get UID (common in clone cards)
        [0x90, 0x5A, 0x00, 0x00, 0x04],        // Another Get UID variant
        
        // Memory dump commands
        [0x90, 0xFB, 0x00, 0x00, 0x00],        // Memory dump (some clones)
        [0xFF, 0xFB, 0x00, 0x00, 0x00],        // Alternative dump
    ];
    
    let mut found_working = false;
    
    for (i, cmd) in direct_commands.iter().enumerate() {
        println!("Trying direct command #{}: {}", i+1, HexSlice(cmd));
        match send_apdu(card, cmd) {
            Ok(response) => {
                println!("Response: {}", HexSlice(&response));
                if response.len() > 2 {
                    println!("Command #{} returned data!", i+1);
                    found_working = true;
                    
                    // Try a few more offsets if this worked
                    if cmd[0] == 0x00 && cmd[1] == 0xB0 {
                        println!("Trying to read more with this command...");
                        for offset in 1..5 {
                            let mut read_more = cmd.clone();
                            read_more[2] = offset; // P1 = offset
                            match send_apdu(card, &read_more) {
                                Ok(more_resp) => println!("Offset {}: {}", offset, HexSlice(&more_resp)),
                                Err(_) => {}
                            }
                        }
                    }
                }
            },
            Err(e) => println!("Command #{} error: {}", i+1, e)
        }
        sleep(Duration::from_millis(100));
    }
    
    if found_working {
        println!("Found at least one working direct access command!");
    } else {
        println!("No direct access commands worked.");
    }
    
    Ok(())
}

// Raw command mode for manual experimentation
pub fn raw_command_mode(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nRaw command mode - enter APDU commands manually");
    println!("Enter commands as hex bytes separated by spaces (e.g. '90 CA 00 00 00')");
    println!("Enter 'exit' to quit");
    
    loop {
        print!("\nEnter command> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let input = input.trim();
        if input.to_lowercase() == "exit" {
            break;
        }
        
        // Parse hex bytes
        let mut bytes = Vec::new();
        for hex_byte in input.split_whitespace() {
            match u8::from_str_radix(hex_byte, 16) {
                Ok(byte) => bytes.push(byte),
                Err(_) => {
                    println!("Invalid hex byte: {}", hex_byte);
                    continue;
                }
            }
        }
        
        if bytes.is_empty() {
            println!("No valid bytes entered");
            continue;
        }
        
        println!("Sending: {}", HexSlice(&bytes));
        match send_apdu(card, &bytes) {
            Ok(response) => println!("Response: {}", HexSlice(&response)),
            Err(e) => println!("Error: {}", e)
        }
    }
    
    println!("Exiting raw command mode");
    Ok(())
}

// Helper function to test a specific command repeatedly with different parameters
pub fn explore_command(card: &pcsc::Card, base_cmd: &[u8], param_byte_index: usize) -> Result<(), Box<dyn Error>> {
    println!("Exploring command variations: {}", HexSlice(base_cmd));
    
    // Try all possible values for the parameter byte
    for value in 0..=255u8 {
        let mut cmd = base_cmd.to_vec();
        if param_byte_index < cmd.len() {
            cmd[param_byte_index] = value;
            
            println!("Trying with parameter {:02X}: {}", value, HexSlice(&cmd));
            match send_apdu(card, &cmd) {
                Ok(response) => {
                    println!("Response: {}", HexSlice(&response));
                    if response.len() > 2 && 
                       response[response.len() - 2] == 0x90 && 
                       response[response.len() - 1] == 0x00 {
                        println!("SUCCESS with parameter {:02X}!", value);
                    }
                },
                Err(e) => println!("Error: {}", e)
            }
            
            sleep(Duration::from_millis(50));
        }
    }
    
    Ok(())
}
