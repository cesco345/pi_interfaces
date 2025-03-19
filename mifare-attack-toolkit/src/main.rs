mod reader;
mod cards;
mod attacks;
mod operations;
mod ui;
mod utils;
mod crypto1;
mod reader_adapter;
mod mifare_attack_manager;
mod attack_manager;
mod card_detection;

// Make functions available
pub use card_detection::{detect_card, wait_for_card_enhanced};
use reader::MifareClassic;

fn main() {
    println!("=== MIFARE Attack Toolkit ===");
    println!("Based on Proxmark3 algorithms ported to Rust");
    println!("Compatible with MFRC522 on Raspberry Pi");
    
    // Initialize the MFRC522 reader
    let mut mifare = match MifareClassic::new() {
        Ok(m) => m,
        Err(e) => {
            println!("Error initializing MFRC522: {}", e);
            return;
        }
    };
    
    println!("=== Mifare Attack Manager ===");
    println!("Based on Proxmark3 algorithms and 'Tears For Fears' approach");
    println!("Press Ctrl+C to exit\n");
    
    // Use the existing menu function 
    mifare_attack_manager::run_menu(&mut mifare);
}
