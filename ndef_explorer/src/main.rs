use std::io::{self, Write};
use std::ffi::CStr;

use pcsc::{Context, Protocols, Scope, ShareMode};

// Import from our library
use ndef_explorer::{
    hex_string, 
    protocol_to_string, 
    transmit_apdu,
    select_ndef_application,
    read_capability_container,
    read_ndef_length,
    read_ndef_message,
    write_ndef_message,
    scan_memory
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("NDEF Card Explorer Tool");
    println!("======================\n");

    // Establish PC/SC context
    let ctx = Context::establish(Scope::User)?;

    // List available readers
    let mut reader_buffer = [0; 2048];
    let readers = ctx.list_readers(&mut reader_buffer)?;
    
    // Convert readers to a vector we can safely iterate
    let mut reader_list = Vec::new();
    for reader in readers {
        reader_list.push(reader.to_owned());
    }
    
    if reader_list.is_empty() {
        return Err("No card readers found.".into());
    }

    println!("Available readers:");
    for (i, reader) in reader_list.iter().enumerate() {
        let reader_name = unsafe { 
            CStr::from_ptr(reader.as_ptr()).to_string_lossy()
        };
        println!("  Reader {}: {}", i, reader_name);
    }

    // Select the first reader by default
    let reader_name = &reader_list[0];
    let reader_display = unsafe {
        CStr::from_ptr(reader_name.as_ptr()).to_string_lossy()
    };
    
    println!("\n------------------------------------------------------");
    println!("Ready to connect to reader: {}", reader_display);
    println!("PLEASE PLACE YOUR CARD on the reader now");
    print!("Press ENTER when ready to continue...");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    println!("Connecting to card...");
    
    // Connect to the card
    let card = match ctx.connect(reader_name, ShareMode::Shared, Protocols::ANY) {
        Ok(card) => card,
        Err(e) => return Err(format!("Failed to connect to card: {}", e).into()),
    };
    
    println!("Successfully connected to card!");

    // Get card status
    let mut status_atr_buf = [0u8; 36];
    let mut status_names_buf = [0u8; 256];
    let status = card.status2(&mut status_names_buf, &mut status_atr_buf)?;
    
    println!("\nCard Status:");
    println!("  Protocol: {}", protocol_to_string(status.protocol()));
    println!("  ATR: {}", hex_string(status.atr()));

    // Get card UID
    println!("\nGetting card UID:");
    if let Some(uid) = transmit_apdu(&card, &[0xFF, 0xCA, 0x00, 0x00, 0x00], "Get UID command") {
        println!("  Card UID: {}", hex_string(&uid));
    } else {
        println!("  Could not get card UID");
    }

    // Interactive menu
    loop {
        println!("\n======= NDEF Explorer Menu =======");
        println!("1. Select NDEF Application");
        println!("2. Read NDEF Capability Container (CC)");
        println!("3. Read NDEF Message Length");
        println!("4. Read NDEF Message");
        println!("5. Write Sample NDEF Message");
        println!("6. Scan Memory");
        println!("7. Exit");
        print!("\nChoose an option (1-7): ");
        io::stdout().flush()?;
        
        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;
        
        match choice.trim() {
            "1" => select_ndef_application(&card),
            "2" => read_capability_container(&card),
            "3" => read_ndef_length(&card),
            "4" => read_ndef_message(&card),
            "5" => write_ndef_message(&card),
            "6" => scan_memory(&card),
            "7" => break,
            _ => println!("Invalid option, please try again."),
        }
    }

    println!("\nExiting NDEF Explorer.");
    Ok(())
}
