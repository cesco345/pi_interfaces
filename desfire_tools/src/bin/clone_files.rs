use std::error::Error;
use std::io;
use std::thread::sleep;
use std::time::Duration;

use desfire_tools::desfire_common::{
    send_apdu, HexSlice
};

use crate::clone_auth::{authenticate_card, select_application};
use crate::TEST_APP_ID;
use crate::TEST_FILE_ID;

pub fn format_card(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("WARNING: This will ERASE ALL DATA on the card!");
    println!("Are you sure you want to format the card? (y/n)");
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() != "y" {
        println!("Format cancelled.");
        return Ok(());
    }
    
    println!("Attempting to format card...");
    
    // Format PICC command
    let format_cmd = [0x90, 0xFC, 0x00, 0x00, 0x00];
    
    match send_apdu(card, &format_cmd) {
        Ok(response) => {
            println!("Format response: {}", HexSlice(&response));
            
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                println!("Card formatted successfully!");
            } else {
                println!("Format returned status: {:?}", response);
                println!("Trying re-authentication and format again...");
                
                if let Ok(_) = authenticate_card(card) {
                    match send_apdu(card, &format_cmd) {
                        Ok(resp2) => {
                            println!("Second format response: {}", HexSlice(&resp2));
                        },
                        Err(e) => println!("Second format attempt error: {}", e)
                    }
                }
            }
            
            // Try alternative format approach for clone cards
            println!("\nTrying alternative format method...");
            
            // Some clones use a different format/reset command
            let alt_format_commands = [
                // Different INS/P1/P2 combinations
                [0x90, 0xFC, 0xFF, 0x00, 0x00],
                [0x90, 0xFC, 0x00, 0xFF, 0x00], 
                [0xFF, 0xFC, 0x00, 0x00, 0x00],
                // GetVersion command to reset card state
                [0x90, 0x60, 0x00, 0x00, 0x00]
            ];
            
            for (i, cmd) in alt_format_commands.iter().enumerate() {
                println!("Trying alternative format method #{}", i+1);
                match send_apdu(card, cmd) {
                    Ok(resp) => println!("Response: {}", HexSlice(&resp)),
                    Err(e) => println!("Error: {}", e)
                }
                sleep(Duration::from_millis(200));
            }
            
            println!("Format attempts completed.");
            Ok(())
        },
        Err(e) => {
            println!("Format command error: {}", e);
            println!("Trying to refresh authentication...");
            
            if let Ok(_) = authenticate_card(card) {
                match send_apdu(card, &format_cmd) {
                    Ok(resp) => {
                        println!("Format response after re-auth: {}", HexSlice(&resp));
                        Ok(())
                    },
                    Err(e) => Err(e)
                }
            } else {
                Err("Failed to re-authenticate for format".into())
            }
        }
    }
}

pub fn create_test_variants(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nCreating test application variants...");
    
    // Re-authenticate to ensure we're in a clean state
    authenticate_card(card)?;
    
    // First, create the application
    let mut create_app_cmd = vec![0x90, 0xCA, 0x00, 0x00, 0x05];
    create_app_cmd.extend_from_slice(&TEST_APP_ID);
    create_app_cmd.push(0x0F); // All permissions enabled
    create_app_cmd.push(0x01); // 1 key
    create_app_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &create_app_cmd) {
        Ok(response) => {
            println!("Create app response: {}", HexSlice(&response));
            
            // Try to select the application
            select_application(card, &TEST_APP_ID)?;
            
            // Re-authenticate with application
            authenticate_card(card)?;
            
            // Now try creating different file types
            println!("\nCreating different file types:");
            
            // File type variants to try - WITH TYPE ANNOTATION
            let file_variants: [(&str, fn(&pcsc::Card) -> Result<(), Box<dyn Error>>); 5] = [
                ("Standard file with free access", create_standard_file_free),
                ("Standard file with no access", create_standard_file_none),
                ("Value file", create_value_file),
                ("Record file", create_record_file),
                ("Backup file", create_backup_file),
            ];
            
            // Try each file variant
            for (name, create_fn) in file_variants.iter() {
                println!("\nTrying to create: {}", name);
                match create_fn(card) {
                    Ok(_) => println!("Successfully created {}", name),
                    Err(e) => println!("Failed to create {}: {}", name, e)
                }
                sleep(Duration::from_millis(200));
            }
            
            println!("\nFile creation tests completed");
            Ok(())
        },
        Err(e) => Err(format!("Failed to create application: {}", e).into())
    }
}

// Create standard file with free access
fn create_standard_file_free(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    let file_cmd = [
        0x90, 0xCD, 0x00, 0x00, 0x07, // Create standard file command
        TEST_FILE_ID,                  // File ID
        0x00,                          // Plain communication
        0xFF, 0xFF,                    // Free access rights
        0x20, 0x00, 0x00,              // 32 bytes size
        0x00                           // Le byte
    ];
    
    match send_apdu(card, &file_cmd) {
        Ok(response) => {
            println!("Create standard file (free) response: {}", HexSlice(&response));
            Ok(())
        },
        Err(e) => Err(e)
    }
}

// Create standard file with no access restrictions
fn create_standard_file_none(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    let file_cmd = [
        0x90, 0xCD, 0x00, 0x00, 0x07, // Create standard file command
        0x02,                          // Different file ID
        0x00,                          // Plain communication
        0x00, 0x00,                    // No access rights
        0x20, 0x00, 0x00,              // 32 bytes size
        0x00                           // Le byte
    ];
    
    match send_apdu(card, &file_cmd) {
        Ok(response) => {
            println!("Create standard file (none) response: {}", HexSlice(&response));
            Ok(())
        },
        Err(e) => Err(e)
    }
}

// Create value file
fn create_value_file(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    let file_cmd = [
        0x90, 0xCC, 0x00, 0x00, 0x11, // Create value file command
        0x03,                          // File ID
        0x00,                          // Plain communication
        0xFF, 0xFF,                    // Free access rights
        0x00, 0x00, 0x00, 0x00,        // Lower limit (0)
        0xFF, 0xFF, 0x00, 0x00,        // Upper limit (65535)
        0x00, 0x00, 0x00, 0x00,        // Initial value (0)
        0x00,                          // Limited credit disabled
        0x00                           // Le byte
    ];
    
    match send_apdu(card, &file_cmd) {
        Ok(response) => {
            println!("Create value file response: {}", HexSlice(&response));
            Ok(())
        },
        Err(e) => Err(e)
    }
}

// Create record file
fn create_record_file(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    let file_cmd = [
        0x90, 0xC1, 0x00, 0x00, 0x0D, // Create cyclic record file command
        0x04,                          // File ID
        0x00,                          // Plain communication
        0xFF, 0xFF,                    // Free access rights
        0x04, 0x00, 0x00,              // Record size (4 bytes)
        0x04, 0x00, 0x00,              // Max records (4)
        0x00                           // Le byte
    ];
    
    match send_apdu(card, &file_cmd) {
        Ok(response) => {
            println!("Create record file response: {}", HexSlice(&response));
            Ok(())
        },
        Err(e) => Err(e)
    }
}

// Create backup file (some clone cards have this type)
fn create_backup_file(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    let file_cmd = [
        0x90, 0xCB, 0x00, 0x00, 0x07, // Create backup file command
        0x05,                          // File ID
        0x00,                          // Plain communication
        0xFF, 0xFF,                    // Free access rights
        0x10, 0x00, 0x00,              // 16 bytes size
        0x00                           // Le byte
    ];
    
    match send_apdu(card, &file_cmd) {
        Ok(response) => {
            println!("Create backup file response: {}", HexSlice(&response));
            Ok(())
        },
        Err(e) => Err(e)
    }
}
