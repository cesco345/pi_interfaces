// src/bin/read_ntag.rs
use std::error::Error;
use std::io::{self, Write};

use pcsc::{Card, Context, Scope, ShareMode, Protocols};
use ndef_explorer::commands::ndef_commands::send_apdu;
use ndef_explorer::util::ndef_util::hex_string;

fn main() -> Result<(), Box<dyn Error>> {
    println!("NTAG213 Tag Reader");
    println!("================\n");
    
    // Connect to the card
    println!("Place your NTAG213 card on the reader and press Enter to continue...");
    io::stdout().flush()?;
    let mut _wait = String::new();
    io::stdin().read_line(&mut _wait)?;
    
    let (_ctx, card) = connect_to_card()?;
    
    // Read the tag
    read_ntag213(&card)?;
    
    println!("\nPlease remove the card from the reader when finished.");
    
    Ok(())
}

fn connect_to_card() -> Result<(Context, Card), Box<dyn Error>> {
    let ctx = Context::establish(Scope::User)?;
    
    let mut readers_buf = [0; 2048];
    let readers = ctx.list_readers(&mut readers_buf)?;
    
    let mut reader_found = false;
    let mut selected_reader = None;
    
    for reader in readers {
        reader_found = true;
        selected_reader = Some(reader);
        println!("Found reader: {}", reader.to_string_lossy());
        break;
    }
    
    if !reader_found {
        return Err("No smart card readers found".into());
    }
    
    let reader = selected_reader.ok_or("Failed to get reader")?;
    println!("Using reader: {}", reader.to_string_lossy());
    
    let card = ctx.connect(reader, ShareMode::Shared, Protocols::ANY)?;
    println!("Successfully connected to card");
    
    Ok((ctx, card))
}

fn read_ntag213(card: &Card) -> Result<(), Box<dyn Error>> {
    println!("Reading NTAG213 tag...");
    
    // Get card UID
    let get_uid = [0xFF, 0xCA, 0x00, 0x00, 0x00];
    
    if let Some(response) = send_apdu(card, &get_uid, "Get UID") {
        println!("Card UID: {}", hex_string(&response));
        
        // Read pages 4-39 (user memory)
        println!("\nUser Memory Content:");
        println!("=====================");
        
        for page in 4..40 {
            let read_cmd = [0xFF, 0xB0, 0x00, page, 0x04];
            if let Some(page_data) = send_apdu(card, &read_cmd, &format!("Read Page {}", page)) {
                print!("Page {:02}: ", page);
                
                // Print hex representation
                for byte in &page_data {
                    print!("{:02X} ", byte);
                }
                
                // Try to print ASCII representation if printable
                print!(" | ");
                for &byte in &page_data {
                    if byte >= 32 && byte <= 126 {
                        print!("{}", byte as char);
                    } else {
                        print!(".");
                    }
                }
                
                println!();
            } else {
                println!("Page {:02}: Failed to read", page);
            }
        }
        
        Ok(())
    } else {
        Err("Failed to read card UID".into())
    }
}
