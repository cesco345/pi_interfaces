// File: src/operations/ndef_operations_scanner.rs
// Functions for scanning memory areas of the card
use crate::util::ndef_util::hex_string;

// Scan memory to find data
pub fn scan_readable_memory(card: &pcsc::Card) {
    // Start with areas that are likely to be readable
    let scan_ranges = [
        (0x00, 0x20), // CC area
        (0x0F, 0x20), // NDEF length and beginning of content
        (0x10, 0x20), // Another possible NDEF content start
    ];
    
    for (start, length) in scan_ranges {
        println!("\nScanning range 0x{:02X}-0x{:02X}:", start, start + length - 1);
        for offset in (start..start+length).step_by(4) {
            let read_size = if offset + 4 > start + length { (start + length - offset) as u8 } else { 4 };
            
            // Silent reading - only report successful reads
            let mut recv_buffer = [0; 258];
            let apdu = [0x00, 0xB0, 0x00, offset as u8, read_size];
            
            match card.transmit(&apdu, &mut recv_buffer) {
                Ok(response) => {
                    if response.len() >= 2 {
                        let sw1 = response[response.len() - 2];
                        let sw2 = response[response.len() - 1];
                        
                        if sw1 == 0x90 && sw2 == 0x00 && response.len() > 2 {
                            let data = &response[..response.len() - 2];
                            println!("  Offset 0x{:02X}: {}", offset, hex_string(data));
                        }
                    }
                },
                Err(_) => {}
            }
        }
    }
}
