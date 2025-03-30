use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use desfire_tools::desfire_common::{
    send_apdu, HexSlice
};

use crate::clone_auth::{authenticate_card, select_application};
use crate::TEST_APP_ID;

pub fn write_test_variants(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nTrying different write methods...");
    
    // Re-authenticate to ensure we're in a clean state
    authenticate_card(card)?;
    
    // Select the application
    select_application(card, &TEST_APP_ID)?;
    
    // Re-authenticate
    authenticate_card(card)?;
    
    // Test data to write
    let test_data = b"ABCD";
    
    // Try different write methods
    println!("\nTrying write methods on different files:");
    
    // Methods to try - WITH TYPE ANNOTATION
    let write_methods: [(&str, fn(&pcsc::Card, u8, &[u8]) -> Result<(), Box<dyn Error>>, u8); 8] = [
        ("Standard Write to File 01", write_standard, 0x01),
        ("Standard Write to File 02", write_standard, 0x02),
        ("Standard Write to File 05", write_standard, 0x05),
        ("Write with no commit to File 01", write_no_commit, 0x01),
        ("Write with explicit commit to File 01", write_with_commit, 0x01),
        ("Credit Value to File 03", credit_value, 0x03),
        ("Write Record to File 04", write_record, 0x04),
        ("Write with modified command to File 01", write_modified, 0x01),
    ];
    
    for (name, write_fn, file_id) in write_methods.iter() {
        println!("\nTrying: {} to file {:02X}", name, file_id);
        match write_fn(card, *file_id, test_data) {
            Ok(_) => println!("Successfully completed {}", name),
            Err(e) => println!("Failed with {}: {}", name, e)
        }
        sleep(Duration::from_millis(300));
    }
    
    println!("\nWrite tests completed");
    Ok(())
}

// Standard write approach
pub fn write_standard(card: &pcsc::Card, file_id: u8, data: &[u8]) -> Result<(), Box<dyn Error>> {
    let mut write_cmd = vec![0x90, 0x3D, 0x00, 0x00];
    write_cmd.push(7 + data.len() as u8); // Lc byte
    write_cmd.push(file_id);
    write_cmd.push(0x00); // Offset (LSB)
    write_cmd.push(0x00); // Offset (middle byte)
    write_cmd.push(0x00); // Offset (MSB)
    write_cmd.push(data.len() as u8);
    write_cmd.extend_from_slice(data);
    write_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &write_cmd) {
        Ok(response) => {
            println!("Standard write response: {}", HexSlice(&response));
            
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x7E {
                
                // Try to commit
                let commit_cmd = [0x90, 0xC7, 0x00, 0x00, 0x00];
                let commit_resp = send_apdu(card, &commit_cmd)?;
                println!("Auto-commit response: {}", HexSlice(&commit_resp));
            }
            
            Ok(())
        },
        Err(e) => Err(e)
    }
}

// Write with no commit
fn write_no_commit(card: &pcsc::Card, file_id: u8, data: &[u8]) -> Result<(), Box<dyn Error>> {
    let mut write_cmd = vec![0x90, 0x3D, 0x00, 0x00];
    write_cmd.push(7 + data.len() as u8); // Lc byte
    write_cmd.push(file_id);
    write_cmd.push(0x00); // Offset (LSB)
    write_cmd.push(0x00); // Offset (middle byte)
    write_cmd.push(0x00); // Offset (MSB)
    write_cmd.push(data.len() as u8);
    write_cmd.extend_from_slice(data);
    write_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &write_cmd) {
        Ok(response) => {
            println!("Write no-commit response: {}", HexSlice(&response));
            Ok(()) // Don't commit
        },
        Err(e) => Err(e)
    }
}

// Write with explicit commit
fn write_with_commit(card: &pcsc::Card, file_id: u8, data: &[u8]) -> Result<(), Box<dyn Error>> {
    let mut write_cmd = vec![0x90, 0x3D, 0x00, 0x00];
    write_cmd.push(7 + data.len() as u8); // Lc byte
    write_cmd.push(file_id);
    write_cmd.push(0x00); // Offset (LSB)
    write_cmd.push(0x00); // Offset (middle byte)
    write_cmd.push(0x00); // Offset (MSB)
    write_cmd.push(data.len() as u8);
    write_cmd.extend_from_slice(data);
    write_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &write_cmd) {
        Ok(response) => {
            println!("Write response: {}", HexSlice(&response));
            
            // Always try to commit, regardless of response
            println!("Explicitly committing transaction...");
            let commit_cmd = [0x90, 0xC7, 0x00, 0x00, 0x00];
            let commit_resp = send_apdu(card, &commit_cmd)?;
            println!("Commit response: {}", HexSlice(&commit_resp));
            
            // Also try abort command, as some clone cards need this
            let abort_cmd = [0x90, 0xA7, 0x00, 0x00, 0x00];
            let abort_resp = send_apdu(card, &abort_cmd);
            if let Ok(resp) = abort_resp {
                println!("Abort response: {}", HexSlice(&resp));
            }
            
            Ok(())
        },
        Err(e) => Err(e)
    }
}

// Credit Value (for value files)
fn credit_value(card: &pcsc::Card, file_id: u8, data: &[u8]) -> Result<(), Box<dyn Error>> {
    // Convert up to 4 bytes of data to a 32-bit value
    let mut value = 0u32;
    for (i, &b) in data.iter().take(4).enumerate() {
        value |= (b as u32) << (8 * i);
    }
    
    // Credit command
    let mut credit_cmd = vec![0x90, 0x0C, 0x00, 0x00, 0x05];
    credit_cmd.push(file_id);
    credit_cmd.extend_from_slice(&value.to_le_bytes());
    credit_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &credit_cmd) {
        Ok(response) => {
            println!("Credit value response: {}", HexSlice(&response));
            
            // Try to commit
            let commit_cmd = [0x90, 0xC7, 0x00, 0x00, 0x00];
            let commit_resp = send_apdu(card, &commit_cmd)?;
            println!("Commit response: {}", HexSlice(&commit_resp));
            
            Ok(())
        },
        Err(e) => Err(e)
    }
}

// Write Record (for record files)
fn write_record(card: &pcsc::Card, file_id: u8, data: &[u8]) -> Result<(), Box<dyn Error>> {
    let mut write_cmd = vec![0x90, 0x3B, 0x00, 0x00];
    write_cmd.push(2 + data.len() as u8); // Lc byte
    write_cmd.push(file_id);
    write_cmd.push(0x00); // Offset
    write_cmd.extend_from_slice(data);
    write_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &write_cmd) {
        Ok(response) => {
            println!("Write record response: {}", HexSlice(&response));
            
            // Try to commit
            let commit_cmd = [0x90, 0xC7, 0x00, 0x00, 0x00];
            let commit_resp = send_apdu(card, &commit_cmd)?;
            println!("Commit response: {}", HexSlice(&commit_resp));
            
            Ok(())
        },
        Err(e) => Err(e)
    }
}

// Modified write command (some clones use non-standard commands)
fn write_modified(card: &pcsc::Card, file_id: u8, data: &[u8]) -> Result<(), Box<dyn Error>> {
    // Modified command with different INS byte
    let mut write_cmd = vec![0x90, 0xDD, 0x00, 0x00]; // 0xDD instead of 0x3D
    write_cmd.push(7 + data.len() as u8); // Lc byte
    write_cmd.push(file_id);
    write_cmd.push(0x00); // Offset (LSB)
    write_cmd.push(0x00); // Offset (middle byte)
    write_cmd.push(0x00); // Offset (MSB)
    write_cmd.push(data.len() as u8);
    write_cmd.extend_from_slice(data);
    write_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &write_cmd) {
        Ok(response) => {
            println!("Modified write response: {}", HexSlice(&response));
            
            // Try with different commit command too
            let alt_commit = [0x90, 0xDF, 0x00, 0x00, 0x00]; // Non-standard commit
            let alt_resp = send_apdu(card, &alt_commit);
            if let Ok(resp) = alt_resp {
                println!("Alternative commit response: {}", HexSlice(&resp));
            }
            
            Ok(())
        },
        Err(e) => {
            println!("Modified write error: {}", e);
            println!("This is expected if the card doesn't support non-standard commands");
            Ok(()) // Return Ok even if failed - this is experimental
        }
    }
}
