// src/bin/raw_command.rs
//
// Utility for sending raw commands to DESFire cards

use std::io::{self, BufRead, Write};
use std::error::Error;
use pcsc::{Context, Protocols, Scope, ShareMode};
use std::ffi::CStr;

use ndef_explorer::commands::ndef_commands::send_apdu;
use ndef_explorer::util::ndef_util::hex_string;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Raw APDU Command Sender");
    println!("======================\n");

    // Establish PC/SC context
    let ctx = Context::establish(Scope::User)?;
    let mut reader_buffer = [0; 2048];
    let readers = ctx.list_readers(&mut reader_buffer)?;
    
    let mut reader_list = Vec::new();
    for reader in readers {
        reader_list.push(reader.to_owned());
    }
    
    if reader_list.is_empty() {
        return Err("No card readers found.".into());
    }

    // Select the first reader
    let reader_name = &reader_list[0];
    let reader_display = unsafe {
        CStr::from_ptr(reader_name.as_ptr()).to_string_lossy()
    };
    
    println!("Using reader: {}", reader_display);
    println!("Place your card on the reader and press Enter...");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    // Connect to the card
    let card = ctx.connect(reader_name, ShareMode::Shared, Protocols::ANY)?;
    println!("Connected to card successfully\n");

    println!("Enter raw APDU commands (format: 00 A4 04 00 ...) or 'exit' to quit:");
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    
    loop {
        print!("> ");
        io::stdout().flush()?;
        
        match lines.next() {
            Some(Ok(line)) => {
                let trimmed = line.trim();
                if trimmed.eq_ignore_ascii_case("exit") || trimmed.eq_ignore_ascii_case("quit") {
                    break;
                }
                
                // Parse the hex string into bytes
                let bytes: Result<Vec<u8>, _> = trimmed
                    .split_whitespace()
                    .map(|s| u8::from_str_radix(s, 16))
                    .collect();
                
                match bytes {
                    Ok(command) => {
                        if command.is_empty() {
                            continue;
                        }
                        
                        println!("Sending command: {}", hex_string(&command));
                        match send_apdu(&card, &command, "Raw Command") {
                            Some(response) => {
                                println!("Response: {}", hex_string(&response));
                            },
                            None => {
                                println!("No response or communication error");
                            }
                        }
                    },
                    Err(e) => {
                        println!("Error parsing command: {}", e);
                    }
                }
            },
            Some(Err(e)) => {
                println!("Error reading input: {}", e);
            },
            None => break,
        }
    }

    println!("Goodbye!");
    Ok(())
}
