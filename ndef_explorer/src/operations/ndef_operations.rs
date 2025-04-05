// File: src/operations/ndef_operations.rs
// Functions for NDEF card operations - Basic operations only

use crate::commands::ndef_commands::send_apdu;
use crate::interpreter::ndef_interpreter::parse_capability_container;

// Select NDEF application on the card
pub fn select_ndef_application(card: &pcsc::Card) {
    println!("\nSelecting NDEF Application...");
    
    // Try primary NDEF AID
    if let Some(_) = send_apdu(card, &[0x00, 0xA4, 0x04, 0x00, 0x07, 0xD2, 0x76, 0x00, 0x00, 0x85, 0x01, 0x01, 0x00], 
                  "Select NDEF Application (Standard)") {
        println!("NDEF Application selected successfully!");
        return;
    }
    
    // Try alternative NDEF AID
    if let Some(_) = send_apdu(card, &[0x00, 0xA4, 0x04, 0x00, 0x07, 0xD2, 0x76, 0x00, 0x00, 0x85, 0x01, 0x00, 0x00], 
                  "Select NDEF Application (Alternative)") {
        println!("Alternative NDEF Application selected successfully!");
        return;
    }
    
    // Try NFC Forum Tag application
    if let Some(_) = send_apdu(card, &[0x00, 0xA4, 0x04, 0x00, 0x07, 0xD2, 0x76, 0x00, 0x00, 0x85, 0x01, 0x01], 
                  "Select NFC Forum Tag Application") {
        println!("NFC Forum Tag Application selected successfully!");
        return;
    }
    
    println!("Failed to select any NDEF Application!");
}

// Read NDEF capability container
pub fn read_capability_container(card: &pcsc::Card) {
    println!("\nReading NDEF Capability Container...");
    
    // Try to read CC file - location may vary by card type
    if let Some(cc_data) = send_apdu(card, &[0x00, 0xB0, 0x00, 0x00, 0x0F], 
                             "Read NDEF Capability Container (First 15 bytes)") {
        if !cc_data.is_empty() {
            parse_capability_container(&cc_data);
        } else {
            // Try another common offset
            if let Some(cc_data) = send_apdu(card, &[0x00, 0xB0, 0x00, 0x03, 0x0F], 
                                   "Read NDEF CC (Offset 3, 15 bytes)") {
                if !cc_data.is_empty() {
                    parse_capability_container(&cc_data);
                }
            }
        }
    }
}

// Read NDEF message length
pub fn read_ndef_length(card: &pcsc::Card) {
    println!("\nReading NDEF Message Length...");
    
    // Common offsets for NDEF length in different cards
    let possible_offsets = [0x0F, 0x00, 0x03, 0x04, 0x10];
    
    for offset in possible_offsets.iter() {
        println!("\nTrying offset 0x{:02X}...", offset);
        if let Some(length_data) = send_apdu(card, &[0x00, 0xB0, 0x00, *offset, 0x02], 
                                   &format!("Read NDEF Length at offset 0x{:02X}", offset)) {
            if length_data.len() >= 2 {
                let length = (u16::from(length_data[0]) << 8) | u16::from(length_data[1]);
                println!("  NDEF Message Length: {} bytes", length);
                
                // Try to read a few bytes of the actual message
                let ndef_offset = offset + 2;
                let read_size = if length < 16 { length } else { 16 } as u8;
                
                println!("\nReading first {} bytes of NDEF message at offset 0x{:02X}...", read_size, ndef_offset);
                send_apdu(card, &[0x00, 0xB0, 0x00, ndef_offset, read_size], 
                             &format!("Read NDEF Data Preview (offset 0x{:02X})", ndef_offset));
                
                return;
            }
        }
    }
    
    println!("Could not find NDEF message length at common offsets.");
}
