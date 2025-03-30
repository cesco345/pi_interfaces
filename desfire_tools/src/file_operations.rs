use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use crate::card::send_apdu;
use crate::util::HexSlice;
use crate::error::{print_desfire_error, is_operation_success};

/// Create a standard data file
pub fn create_standard_file(
    card: &pcsc::Card, 
    file_id: u8, 
    size: u16, 
    access_rights: [u8; 2]
) -> Result<(), Box<dyn Error>> {
    println!("Creating standard file with ID: {:02X}, size: {} bytes", file_id, size);
    
    // File parameters
    let comm_settings = 0x00; // Plain communication
    let file_size = [
        (size & 0xFF) as u8,
        ((size >> 8) & 0xFF) as u8,
        0x00
    ]; // Convert size to little endian
    
    // Create standard file command
    let mut create_file_cmd = vec![0x90, 0xCD, 0x00, 0x00, 0x07];
    create_file_cmd.push(file_id);
    create_file_cmd.push(comm_settings);
    create_file_cmd.extend_from_slice(&access_rights);
    create_file_cmd.extend_from_slice(&file_size);
    create_file_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &create_file_cmd) {
        Ok(response) => {
            if is_operation_success(&response) {
                println!("File created successfully with ID: {:02X}", file_id);
                sleep(Duration::from_millis(200)); // Add delay
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                if error == 0xC3 { // File already exists
                    println!("File with ID {:02X} already exists, skipping creation", file_id);
                    Ok(())
                } else {
                    Err(format!("Failed to create file: {} ({})",
                        print_desfire_error(error), error).into())
                }
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

/// Write data to a file
pub fn write_data(card: &pcsc::Card, file_id: u8, data: &[u8]) -> Result<(), Box<dyn Error>> {
    println!("Writing {} bytes to file ID: {:02X}", data.len(), file_id);
    
    // Write data command
    let offset = [0x00, 0x00, 0x00]; // Start at offset 0
    let length = data.len() as u8;
    
    let mut write_cmd = vec![0x90, 0x3D, 0x00, 0x00];
    write_cmd.push(7 + length); // Lc byte (file_id + offset + length + data.len)
    write_cmd.push(file_id);
    write_cmd.extend_from_slice(&offset);
    write_cmd.push(length);
    write_cmd.extend_from_slice(data);
    write_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &write_cmd) {
        Ok(response) => {
            if is_operation_success(&response) {
                println!("Data written successfully");
                sleep(Duration::from_millis(200)); // Add delay
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                Err(format!("Failed to write data: {} ({})",
                    print_desfire_error(error), error).into())
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

/// Read data from a file
pub fn read_data(card: &pcsc::Card, file_id: u8, length: u8) -> Result<Vec<u8>, Box<dyn Error>> {
    println!("Reading {} bytes from file ID: {:02X}", length, file_id);
    
    let offset = [0x00, 0x00, 0x00]; // Start at offset 0
    let read_length = [length, 0x00, 0x00]; // Length to read
    
    // Read data command
    let mut read_cmd = vec![0x90, 0xBD, 0x00, 0x00, 0x07];
    read_cmd.push(file_id);
    read_cmd.extend_from_slice(&offset);
    read_cmd.extend_from_slice(&read_length);
    read_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &read_cmd) {
        Ok(response) => {
            if is_operation_success(&response) {
                // Extract the data (exclude the status bytes)
                let data = response[0..response.len()-2].to_vec();
                println!("Read {} bytes of data", data.len());
                Ok(data)
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                Err(format!("Failed to read data: {} ({})",
                    print_desfire_error(error), error).into())
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

/// Create a value file (for storing integer values)
pub fn create_value_file(
    card: &pcsc::Card,
    file_id: u8,
    access_rights: [u8; 2],
    lower_limit: i32,
    upper_limit: i32,
    initial_value: i32,
    limited_credit_enabled: bool
) -> Result<(), Box<dyn Error>> {
    println!("Creating value file with ID: {:02X}", file_id);
    
    // Convert integers to byte arrays (little endian)
    let lower_limit_bytes = lower_limit.to_le_bytes();
    let upper_limit_bytes = upper_limit.to_le_bytes();
    let value_bytes = initial_value.to_le_bytes();
    
    // File parameters
    let comm_settings = 0x00; // Plain communication
    let limited_credit = if limited_credit_enabled { 0x01 } else { 0x00 };
    
    // Create value file command
    let mut create_file_cmd = vec![0x90, 0xCC, 0x00, 0x00, 0x11];
    create_file_cmd.push(file_id);
    create_file_cmd.push(comm_settings);
    create_file_cmd.extend_from_slice(&access_rights);
    create_file_cmd.extend_from_slice(&lower_limit_bytes);
    create_file_cmd.extend_from_slice(&upper_limit_bytes);
    create_file_cmd.extend_from_slice(&value_bytes);
    create_file_cmd.push(limited_credit);
    create_file_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &create_file_cmd) {
        Ok(response) => {
            if is_operation_success(&response) {
                println!("Value file created successfully with ID: {:02X}", file_id);
                sleep(Duration::from_millis(200)); // Add delay
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                if error == 0xC3 { // File already exists
                    println!("File with ID {:02X} already exists, skipping creation", file_id);
                    Ok(())
                } else {
                    Err(format!("Failed to create value file: {} ({})",
                        print_desfire_error(error), error).into())
                }
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

/// Create a record file (for storing multiple fixed-size records)
pub fn create_record_file(
    card: &pcsc::Card,
    file_id: u8,
    access_rights: [u8; 2],
    record_size: u16,
    max_records: u16,
    cyclic: bool
) -> Result<(), Box<dyn Error>> {
    println!("Creating {} record file with ID: {:02X}", 
             if cyclic { "cyclic" } else { "linear" }, file_id);
    
    // Convert integers to byte arrays (little endian)
    let record_size_bytes = [
        (record_size & 0xFF) as u8,
        ((record_size >> 8) & 0xFF) as u8,
        0x00
    ];
    
    let max_records_bytes = [
        (max_records & 0xFF) as u8,
        ((max_records >> 8) & 0xFF) as u8,
        0x00
    ];
    
    // File parameters
    let comm_settings = 0x00; // Plain communication
    
    // Command code: 0xC1 for cyclic, 0xC0 for linear
    let command_code = if cyclic { 0xC1 } else { 0xC0 };
    
    // Create record file command
    let mut create_file_cmd = vec![0x90, command_code, 0x00, 0x00, 0x0D];
    create_file_cmd.push(file_id);
    create_file_cmd.push(comm_settings);
    create_file_cmd.extend_from_slice(&access_rights);
    create_file_cmd.extend_from_slice(&record_size_bytes);
    create_file_cmd.extend_from_slice(&max_records_bytes);
    create_file_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &create_file_cmd) {
        Ok(response) => {
            if is_operation_success(&response) {
                println!("Record file created successfully with ID: {:02X}", file_id);
                sleep(Duration::from_millis(200)); // Add delay
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                if error == 0xC3 { // File already exists
                    println!("File with ID {:02X} already exists, skipping creation", file_id);
                    Ok(())
                } else {
                    Err(format!("Failed to create record file: {} ({})",
                        print_desfire_error(error), error).into())
                }
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}
