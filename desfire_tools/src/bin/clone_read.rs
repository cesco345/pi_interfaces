use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use desfire_tools::desfire_common::{
    send_apdu, HexSlice
};

use crate::clone_auth::{authenticate_card, select_application};
use crate::clone_low_level::try_direct_access;
use crate::TEST_APP_ID;

pub fn read_test_variants(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nTrying different read methods...");
    
    // Re-authenticate to ensure we're in a clean state
    authenticate_card(card)?;
    
    // Select the application
    select_application(card, &TEST_APP_ID)?;
    
    // Re-authenticate
    authenticate_card(card)?;
    
    // Try different read methods
    println!("\nTrying read methods on different files:");
    
    // Methods to try - WITH TYPE ANNOTATION
    let read_methods: [(&str, fn(&pcsc::Card, u8) -> Result<Vec<u8>, Box<dyn Error>>, u8); 8] = [
        ("Standard Read from File 01", read_standard, 0x01),
        ("Standard Read from File 02", read_standard, 0x02),
        ("Standard Read from File 05", read_standard, 0x05),
        ("GetValue from File 03", read_get_value, 0x03),
        ("Read Record from File 04", read_record, 0x04),
        ("Binary Read from File 01", read_binary, 0x01),
        ("Chunk Read from File 01", read_chunk, 0x01),
        ("Modified Read from File 01", read_modified, 0x01),
    ];
    
    let mut any_success = false;
    
    for (name, read_fn, file_id) in read_methods.iter() {
        println!("\nTrying: {} from file {:02X}", name, file_id);
        match read_fn(card, *file_id) {
            Ok(data) => {
                println!("Successfully read with {}: {:?}", name, data);
                if let Ok(text) = std::str::from_utf8(&data) {
                    println!("Data as text: \"{}\"", text);
                }
                any_success = true;
            },
            Err(e) => println!("Failed with {}: {}", name, e)
        }
        sleep(Duration::from_millis(300));
    }
    
    if any_success {
        println!("\nAt least one read method succeeded!");
    } else {
        println!("\nAll read methods failed. Trying direct memory access methods...");
        try_direct_access(card)?;
    }
    
    println!("\nRead tests completed");
    Ok(())
}

// Standard read approach
pub fn read_standard(card: &pcsc::Card, file_id: u8) -> Result<Vec<u8>, Box<dyn Error>> {
    let read_cmd = [
        0x90, 0xBD, 0x00, 0x00, 0x07, // Read data command
        file_id,                      // File ID
        0x00, 0x00, 0x00,             // Offset
        0x04, 0x00, 0x00,             // Length (4 bytes)
        0x00                          // Le byte
    ];
    
    match send_apdu(card, &read_cmd) {
        Ok(response) => {
            println!("Standard read response: {}", HexSlice(&response));
            
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                // Extract the data (exclude the status bytes)
                Ok(response[0..response.len()-2].to_vec())
            } else {
                Err(format!("Standard read failed with status: {:?}", response).into())
            }
        },
        Err(e) => Err(e)
    }
}

// GetValue approach (for value files)
pub fn read_get_value(card: &pcsc::Card, file_id: u8) -> Result<Vec<u8>, Box<dyn Error>> {
    let get_value_cmd = [
        0x90, 0x6C, 0x00, 0x00, 0x01, // GetValue command
        file_id,                      // File ID
        0x00                          // Le byte
    ];
    
    match send_apdu(card, &get_value_cmd) {
        Ok(response) => {
            println!("GetValue response: {}", HexSlice(&response));
            
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                // Extract the data (exclude the status bytes)
                Ok(response[0..response.len()-2].to_vec())
            } else {
                Err(format!("GetValue failed with status: {:?}", response).into())
            }
        },
        Err(e) => Err(e)
    }
}

// Read Record (for record files)
pub fn read_record(card: &pcsc::Card, file_id: u8) -> Result<Vec<u8>, Box<dyn Error>> {
    let read_record_cmd = [
        0x90, 0xBB, 0x00, 0x00, 0x03, // ReadRecords command
        file_id,                      // File ID
        0x00,                         // Record number
        0x04,                         // Number of records
        0x00                          // Le byte
    ];
    
    match send_apdu(card, &read_record_cmd) {
        Ok(response) => {
            println!("Read record response: {}", HexSlice(&response));
            
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                // Extract the data (exclude the status bytes)
                Ok(response[0..response.len()-2].to_vec())
            } else {
                Err(format!("Read record failed with status: {:?}", response).into())
            }
        },
        Err(e) => Err(e)
    }
}

// Binary read (some clones support this)
pub fn read_binary(card: &pcsc::Card, file_id: u8) -> Result<Vec<u8>, Box<dyn Error>> {
    // Use ISO 7816-4 READ BINARY instead of DESFire ReadData
    let read_binary_cmd = [
        0x00, 0xB0, file_id, 0x00, 0x04 // INS=B0 for READ BINARY
    ];
    
    match send_apdu(card, &read_binary_cmd) {
        Ok(response) => {
            println!("Read binary response: {}", HexSlice(&response));
            
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x90 && 
               response[response.len() - 1] == 0x00 {
                // Extract the data (exclude the status bytes)
                Ok(response[0..response.len()-2].to_vec())
            } else {
                Err(format!("Read binary failed with status: {:?}", response).into())
            }
        },
        Err(e) => Err(e)
    }
}

// Chunked read (1 byte at a time)
pub fn read_chunk(card: &pcsc::Card, file_id: u8) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut result = Vec::new();
    
    // Try to read one byte at a time
    for offset in 0..4 {
        let read_cmd = [
            0x90, 0xBD, 0x00, 0x00, 0x07, // Read data command
            file_id,                      // File ID
            offset, 0x00, 0x00,           // Offset
            0x01, 0x00, 0x00,             // Length (1 byte)
            0x00                          // Le byte
        ];
        
        match send_apdu(card, &read_cmd) {
            Ok(response) => {
                println!("Chunked read at offset {}: {}", offset, HexSlice(&response));
                
                if response.len() >= 3 && 
                   response[response.len() - 2] == 0x91 && 
                   response[response.len() - 1] == 0x00 {
                    // Add the byte to our result
                    result.push(response[0]);
                } else {
                    println!("Failed to read byte at offset {}: {:?}", offset, response);
                }
            },
            Err(e) => {
                println!("Error reading byte at offset {}: {}", offset, e);
            }
        }
    }
    
    if result.is_empty() {
        Err("Chunked read failed to get any data".into())
    } else {
        Ok(result)
    }
}

// Modified read command (some clones use non-standard commands)
pub fn read_modified(card: &pcsc::Card, file_id: u8) -> Result<Vec<u8>, Box<dyn Error>> {
    // Modified command with different INS byte
    let read_cmd = [
        0x90, 0xBE, 0x00, 0x00, 0x07, // 0xBE instead of 0xBD
        file_id,                      // File ID
        0x00, 0x00, 0x00,             // Offset
        0x04, 0x00, 0x00,             // Length (4 bytes)
        0x00                          // Le byte
    ];
    
    match send_apdu(card, &read_cmd) {
        Ok(response) => {
            println!("Modified read response: {}", HexSlice(&response));
            
            if response.len() >= 2 {
                // Return whatever we got
                if response.len() > 2 {
                    Ok(response[0..response.len()-2].to_vec())
                } else {
                    Err(format!("Modified read returned no data: {:?}", response).into())
                }
            } else {
                Err(format!("Modified read failed: {:?}", response).into())
            }
        },
        Err(e) => Err(e)
    }
}
