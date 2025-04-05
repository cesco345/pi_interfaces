// File: src/commands/raw_commands.rs
use std::io::{self, Write};
use crate::util::ndef_util::{parse_hex_string, hex_string};

pub fn send_raw_command(card: &pcsc::Card) {
    println!("\nSend Raw APDU Command");
    println!("=====================");
    println!("Enter hex bytes separated by spaces (e.g., 00 A4 04 00 07 D2 76 00 00 85 01 01 00)");
    print!("Command > ");
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    
    let command_bytes = parse_hex_string(input.trim());
    
    if command_bytes.is_empty() {
        println!("Invalid command format");
        return;
    }
    
    println!("\nSending command: {}", command_bytes.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<String>>()
        .join(" "));
    
    let mut recv_buffer = [0; 258];
    match card.transmit(&command_bytes, &mut recv_buffer) {
        Ok(response) => {
            println!("Response: {}", response.iter()
                .map(|b| format!("{:02X}", *b))
                .collect::<Vec<String>>()
                .join(" "));
            
            if response.len() >= 2 {
                let sw1 = response[response.len() - 2];
                let sw2 = response[response.len() - 1];
                println!("Status: {:02X} {:02X}", sw1, sw2);
                
                match (sw1, sw2) {
                    (0x90, 0x00) => println!("Status meaning: Success"),
                    (0x6A, 0x82) => println!("Status meaning: File not found"),
                    (0x6A, 0x86) => println!("Status meaning: Incorrect parameters P1-P2"),
                    (0x6F, 0x00) => println!("Status meaning: Command not supported or invalid"),
                    (0x69, 0x86) => println!("Status meaning: Command not allowed"),
                    (0x6A, 0x87) => println!("Status meaning: Lc inconsistent with P1-P2"),
                    (0x61, _) => println!("Status meaning: More data available, {} bytes", sw2),
                    _ => println!("Status meaning: Unknown code"),
                }
                
                // If there's data before the status words, display it
                if response.len() > 2 {
                    let data = &response[..response.len() - 2];
                    println!("Data: {}", hex_string(data));
                    
                    // Try to interpret as ASCII if possible
                    if data.iter().all(|&b| b >= 32 && b <= 126) {
                        println!("ASCII: {}", String::from_utf8_lossy(data));
                    }
                    
                    // If this looks like the beginning of a multipart response with AF status
                    if !data.is_empty() && data[0] == 0xAF {
                        println!("\nThis appears to be a multi-part response. You can send an 'AF' command");
                        println!("to get the next part. To do this, select option 7 again and enter: AF");
                    }
                }
            }
        },
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}
