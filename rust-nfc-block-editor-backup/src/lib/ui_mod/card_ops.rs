use std::error::Error;
use rppal::spi::Spi;

use crate::lib::mifare::{read_card_uid, dump_card, format_card};
use crate::lib::utils::uid_to_string;
use super::common::{clear_screen, wait_for_input, countdown_for_card_placement};

/// Read Card UID Menu
pub fn read_uid_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("READ CARD UID");
    println!("=============");
    
    countdown_for_card_placement(5)?;
    
    match read_card_uid(spi)? {
        Some(uid) => {
            println!("");
            println!("Card UID: {}", uid_to_string(&uid));
            println!("UID as decimal: {}", crate::lib::utils::uid_to_num(&uid));
        },
        None => {
            println!("");
            println!("No card detected or error reading card.");
        }
    }
    
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}

/// Dump Card Menu
pub fn dump_card_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("DUMP CARD");
    println!("=========");
    
    let confirm = wait_for_input("\nDump entire card? This may take a while. Continue? (y/n): ")?.to_lowercase();
    if confirm != "y" {
        return Ok(());
    }
    
    countdown_for_card_placement(5)?;
    
    match dump_card(spi)? {
        Some(_) => {
            // Card dump was successful, output is already printed by the dump_card function
        },
        None => {
            println!("");
            println!("Error dumping card.");
        }
    }
    
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}

/// Format Card Menu
pub fn format_card_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("FORMAT CARD");
    println!("===========");
    
    println!("");
    println!("WARNING: This will reset all sectors to default transport configuration.");
    println!("All data will be lost. Sector 0 (manufacturer block) will not be modified.");
    
    let confirm = wait_for_input("\nAre you sure you want to format the card? (type FORMAT to confirm): ")?;
    if confirm != "FORMAT" {
        println!("Operation cancelled.");
        wait_for_input("\nPress Enter to continue...")?;
        return Ok(());
    }
    
    countdown_for_card_placement(5)?;
    
    if format_card(spi)? {
        println!("");
        println!("Card formatted successfully.");
    } else {
        println!("");
        println!("Error formatting card.");
    }
    
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}

/// Test Keys Menu
pub fn test_keys_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_screen();
    println!("TEST KEYS");
    println!("=========");
    
    println!("This will test multiple keys against all sectors of your card.");
    println!("This process may take some time.");
    
    let confirm = wait_for_input("\nProceed? (y/n): ")?.to_lowercase();
    if confirm != "y" {
        return Ok(());
    }
    
    countdown_for_card_placement(5)?;
    
    match crate::lib::mifare::dump::test_keys(spi) {
        Ok(results) => {
            println!("");
            println!("Key Testing Results:");
            println!("====================");
            
            if results.is_empty() {
                println!("No working keys found for any sector.");
            } else {
                for (sector, key) in results {
                    println!("Sector {}: Key {}", sector, crate::lib::utils::bytes_to_hex(&key));
                }
            }
        },
        Err(e) => {
            println!("Error testing keys: {}", e);
        }
    }
    
    wait_for_input("\nPress Enter to continue...")?;
    Ok(())
}
