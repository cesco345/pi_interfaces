use rppal::gpio::Gpio;
use rppal::uart::{Uart, Parity};
use std::{thread, time::Duration, io::{self, Write}};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("PN532 NFC HAT - RFID Reader");
    
    // Initialize UART
    let mut uart = Uart::new(115200, Parity::None, 8, 1)?;
    
    // Clear any pending data
    let mut buffer = [0u8; 100];
    let _ = uart.read(&mut buffer);
    
    // Wake up the PN532
    println!("Sending wake-up sequence");
    let wake_up = [
        0x55, 0x55, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x03, 0xFD, 0xD4,
        0x14, 0x01, 0x17, 0x00
    ];
    uart.write(&wake_up)?;
    
    // Read response
    thread::sleep(Duration::from_millis(100));
    let mut response = [0u8; 30];
    let n = uart.read(&mut response)?;
    println!("Wake-up response: {:02X?}", &response[..n]);
    
    // Get firmware version
    println!("\nSending firmware version command");
    let firmware_cmd = [
        0x00, 0x00, 0xFF, 0x02, 0xFE, 0xD4, 0x02, 0x2A, 0x00
    ];
    uart.write(&firmware_cmd)?;
    
    // Read response (combined ACK and data)
    thread::sleep(Duration::from_millis(100));
    let mut ack = [0u8; 30];
    let n = uart.read(&mut ack)?;
    
    if n > 0 {
        println!("Firmware response: {:02X?}", &ack[..n]);
        
        // Parse firmware version if available
        if n >= 12 && ack[9] == 0x06 && ack[11] == 0x03 {
            println!("Found PN532 with firmware version: {}.{}", ack[14], ack[15]);
        }
    } else {
        println!("No firmware response received");
    }
    
    // Configure SAM (Secure Access Module)
    println!("\nConfiguring SAM");
    let sam_cmd = [
        0x00, 0x00, 0xFF, 0x05, 0xFB, 0xD4, 0x14, 0x01, 0x00, 0x00, 0xE9, 0x00
    ];
    uart.write(&sam_cmd)?;
    
    // Read SAM response
    thread::sleep(Duration::from_millis(100));
    let mut sam_response = [0u8; 30];
    let n = uart.read(&mut sam_response)?;
    
    if n > 0 {
        println!("SAM configuration response: {:02X?}", &sam_response[..n]);
    } else {
        println!("No SAM configuration response");
    }
    
    // Card detection loop
    println!("\nWaiting for an NFC card... (Press Ctrl+C to exit)");
    
    let scan_cmd = [
        0x00, 0x00, 0xFF, 0x04, 0xFC, 0xD4, 0x4A, 0x01, 0x00, 0xE1, 0x00
    ];
    
    loop {
        // Send card detection command
        uart.write(&scan_cmd)?;
        
        // Read response
        thread::sleep(Duration::from_millis(100));
        let mut card_response = [0u8; 30];
        let n = match uart.read(&mut card_response) {
            Ok(n) => n,
            Err(_) => {
                print!(".");
                io::stdout().flush()?;
                thread::sleep(Duration::from_millis(500));
                continue;
            }
        };
        
        if n > 0 {
            println!("\nReceived response: {:02X?}", &card_response[..n]);
            
            // Parse for card detection
            // Look for pattern indicating card found
            for i in 0..n.saturating_sub(9) {
                if card_response[i..].starts_with(&[0x00, 0x00, 0xFF]) && 
                   i + 8 < n && card_response[i+6] == 0xD5 && card_response[i+7] == 0x4B {
                    
                    // Check if a card was found (NUM > 0 at offset 8)
                    if i + 8 < n && card_response[i+8] > 0 {
                        // UID length is at offset 12
                        if i + 12 < n {
                            let uid_len = card_response[i+12] as usize;
                            
                            // Check if we have enough bytes for the UID
                            if i + 13 + uid_len <= n {
                                let uid = &card_response[i+13..i+13+uid_len];
                                println!("\nFound card with UID: {:02X?}", uid);
                                
                                // Wait a bit before scanning again
                                thread::sleep(Duration::from_secs(1));
                            }
                        }
                    }
                    break;
                }
            }
        }
        
        print!(".");
        io::stdout().flush()?;
        thread::sleep(Duration::from_millis(500));
    }
}
