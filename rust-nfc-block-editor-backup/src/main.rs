pub mod lib {
    pub mod mfrc522;
    pub mod mifare;
    pub mod ui_mod;  // The new modular UI code
    pub mod ui_wrapper;  // Add this to include the wrapper
    pub mod utils;
}

use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::error::Error;
use std::process;
use crate::lib::ui_wrapper::main_menu::main_menu;

fn main() -> Result<(), Box<dyn Error>> {
    println!("NFC/RFID Block Editor");
    println!("=====================");
    println!("Initializing...");
    
    // Initialize SPI
    let mut spi = match Spi::new(Bus::Spi0, SlaveSelect::Ss0, 1_000_000, Mode::Mode0) {
        Ok(spi) => {
            println!("SPI interface initialized successfully.");
            spi
        },
        Err(e) => {
            eprintln!("Failed to initialize SPI: {}", e);
            eprintln!("Make sure SPI is enabled on your Raspberry Pi.");
            eprintln!("Run 'sudo raspi-config', go to 'Interface Options' > 'SPI' and enable it.");
            return Err(e.into());
        }
    };
    
    // Initialize MFRC522
    match crate::lib::mfrc522::mfrc522_init(&mut spi) {
        Ok(_) => {
            println!("MFRC522 RFID reader initialized successfully.");
        },
        Err(e) => {
            eprintln!("Failed to initialize MFRC522 RFID reader: {}", e);
            eprintln!("Check connections and ensure reader is properly connected.");
            return Err(e);
        }
    }
    
    // Start the main menu (using ui_wrapper for backward compatibility)
    if let Err(e) = main_menu(&mut spi) {
        eprintln!("Error in main menu: {}", e);
        process::exit(1);
    }
    
    println!("Exiting program. Goodbye!");
    Ok(())
}
