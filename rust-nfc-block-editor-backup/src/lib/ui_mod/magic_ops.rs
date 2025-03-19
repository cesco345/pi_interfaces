use std::error::Error;
use rppal::spi::Spi;

use crate::lib::mifare::magic::{detect_magic_card, write_custom_uid, clone_card, format_magic_key};
use crate::lib::mfrc522::{mfrc522_request, mfrc522_anticoll, PICC_REQIDL, MI_OK};
use crate::lib::ui_mod::common::{clear_screen, wait_for_input};

/// Magic Card Operations Menu
pub fn magic_card_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    loop {
        clear_screen();
        println!("MAGIC CARD OPERATIONS");
        println!("====================");
        
        println!("");
        println!("1. Detect Magic Card");
        println!("2. Write Custom UID");
        println!("3. Clone Card");
        println!("4. Generate Magic Key for Card");
        println!("0. Return to Main Menu");
        
        let choice = wait_for_input("\nEnter choice: ")?;
        
        match choice.as_str() {
            "1" => detect_magic_card(spi)?,
            "2" => write_custom_uid(spi)?,
            "3" => clone_card(spi)?,
            "4" => generate_magic_key_ui(spi)?,
            "0" => return Ok(()),
            _ => {
                println!("Invalid choice. Please try again.");
                wait_for_input("\nPress Enter to continue...")?;
            }
        }
    }
}

/// UI for generating a Magic Key based on card UID
fn generate_magic_key_ui(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("GENERATE MAGIC KEY");
    println!("=================");
    println!("");
    println!("This function generates a potential key for Magic Cards based on their UID.");
    println!("Place your card on the reader to generate keys.");
    
    wait_for_input("\nPress Enter when ready to scan a card...")?;
    
    // Request tag
    let (status, _) = mfrc522_request(spi, PICC_REQIDL)?;
    if status != MI_OK {
        println!("\nError: Could not detect card.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    // Get UID
    let (status, uid) = mfrc522_anticoll(spi)?;
    if status != MI_OK {
        println!("\nError: Could not read card UID.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    println!("\nCard detected!");
    
    // Format UID for display
    let uid_str = uid.iter()
        .map(|&byte| format!("{:02X}", byte))
        .collect::<Vec<String>>()
        .join("");
    
    println!("UID: {}", uid_str);
    
    // Generate and display the Magic key
    // FIXED: Removed the match statement since format_magic_key returns a String directly
    let key = format_magic_key(&uid);
    println!("\nGenerated Magic Key: {}", key);
    println!("\nThis key might work with some types of Magic Cards.");
    println!("Try using this key for authentication if standard keys fail.");
    
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}
