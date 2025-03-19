use std::error::Error;
use rppal::spi::Spi;

use super::common::{clear_screen, wait_for_input};
use super::card_ops::{read_uid_menu, dump_card_menu, format_card_menu, test_keys_menu};
use super::block_ops::{read_block_menu, write_block_menu, block_editor_menu};
use super::sector_ops::{access_bits_menu, change_keys_menu};
use super::attacks::attacks_menu;
use super::magic_ops::magic_card_menu;

/// UI Main Menu
pub fn main_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    loop {
        clear_screen();
        println!("==========================");
        println!("  NFC/RFID BLOCK EDITOR  ");
        println!("==========================");
        
        println!("");
        println!("MAIN MENU:");
        println!("1. Read Card UID");
        println!("2. Read Block");
        println!("3. Write Block");
        println!("4. Dump Card");
        println!("5. Format Card");
        println!("6. Change Keys");
        println!("7. Modify Access Bits");
        println!("8. Block Editor (Interactive)");
        println!("9. Test Keys");
        println!("10. Card Attacks");
        println!("11. Magic Card Operations");
        println!("0. Exit");
        
        let choice = wait_for_input("\nEnter your choice: ")?;
        
        match choice.as_str() {
            "1" => read_uid_menu(spi)?,
            "2" => read_block_menu(spi)?,
            "3" => write_block_menu(spi)?,
            "4" => dump_card_menu(spi)?,
            "5" => format_card_menu(spi)?,
            "6" => change_keys_menu(spi)?,
            "7" => access_bits_menu(spi)?,
            "8" => block_editor_menu(spi)?,
            "9" => test_keys_menu(spi)?,
            "10" => attacks_menu(spi)?,
            "11" => magic_card_menu(spi)?,
            "0" => {
                println!("Exiting...");
                break;
            },
            _ => {
                println!("Invalid choice. Press Enter to continue...");
                wait_for_input("")?;
            }
        }
    }
    
    Ok(())
}
