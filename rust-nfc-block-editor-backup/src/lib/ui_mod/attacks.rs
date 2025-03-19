use std::error::Error;
use rppal::spi::Spi;

use crate::lib::mfrc522::{PICC_AUTHENT1A, PICC_AUTHENT1B};
use crate::lib::mifare::{
    default_keys_attack, nested_authentication_attack, darkside_attack, save_recovered_keys,
    KeyResult
};
use crate::lib::utils::bytes_to_hex;
use super::common::{clear_screen, wait_for_input, countdown_for_card_placement};

/// Card Attacks Menu
pub fn attacks_menu(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    loop {
        clear_screen();
        println!("CARD ATTACKS");
        println!("============");
        
        println!("");
        println!("These attacks attempt to recover keys from MIFARE Classic cards.");
        println!("Some attacks may take time and might not work on all cards.");
        
        println!("");
        println!("Select attack type:");
        println!("1. Default Keys Attack");
        println!("2. Nested Authentication Attack");
        println!("3. Darkside Attack");
        println!("4. Run All Attacks");
        println!("0. Return to Main Menu");
        
        let choice = wait_for_input("\nEnter choice: ")?;
        
        match choice.as_str() {
            "1" => {
                clear_screen();
                println!("DEFAULT KEYS ATTACK");
                println!("==================");
                println!("");
                println!("This attack will try common keys against all sectors.");
                
                let confirm = wait_for_input("\nContinue? (y/n): ")?.to_lowercase();
                if confirm != "y" {
                    continue;
                }
                
                countdown_for_card_placement(5)?;
                
                match default_keys_attack(spi) {
                    Ok(results) => {
                        if !results.is_empty() {
                            save_recovered_keys(&results)?;
                        }
                    },
                    Err(e) => println!("Attack failed: {}", e),
                }
            },
            "2" => {
                clear_screen();
                println!("NESTED AUTHENTICATION ATTACK");
                println!("===========================");
                println!("");
                println!("This attack uses known keys to find additional keys.");
                println!("It first runs the default keys attack to find at least one key.");
                
                let confirm = wait_for_input("\nContinue? (y/n): ")?.to_lowercase();
                if confirm != "y" {
                    continue;
                }
                
                countdown_for_card_placement(5)?;
                
                match nested_authentication_attack(spi) {
                    Ok(results) => {
                        if !results.is_empty() {
                            save_recovered_keys(&results)?;
                        }
                    },
                    Err(e) => println!("Attack failed: {}", e),
                }
            },
            "3" => {
                clear_screen();
                println!("DARKSIDE ATTACK");
                println!("==============");
                println!("");
                println!("This attack exploits a weakness in MIFARE Classic authentication.");
                println!("Warning: This can take several minutes and may not work on all cards/readers.");
                
                let confirm = wait_for_input("\nContinue? (y/n): ")?.to_lowercase();
                if confirm != "y" {
                    continue;
                }
                
                countdown_for_card_placement(5)?;
                
                match darkside_attack(spi) {
                    Ok(results) => {
                        if !results.is_empty() {
                            save_recovered_keys(&results)?;
                        }
                    },
                    Err(e) => println!("Attack failed: {}", e),
                }
            },
            "4" => {
                clear_screen();
                println!("RUNNING ALL ATTACKS");
                println!("==================");
                println!("");
                println!("This will run all attacks in sequence, from fastest to slowest.");
                println!("The process may take several minutes.");
                
                let confirm = wait_for_input("\nContinue? (y/n): ")?.to_lowercase();
                if confirm != "y" {
                    continue;
                }
                
                countdown_for_card_placement(5)?;
                
                let mut all_results = Vec::new();
                
                // Start with default keys attack (fastest)
                println!("");
                println!("Running Default Keys Attack...");
                match default_keys_attack(spi) {
                    Ok(results) => {
                        all_results.extend(results);
                    },
                    Err(e) => println!("Default keys attack failed: {}", e),
                }
                
                // Then nested authentication attack
                if !all_results.is_empty() {
                    println!("");
                    println!("Running Nested Authentication Attack...");
                    match nested_authentication_attack(spi) {
                        Ok(results) => {
                            // Only add new results (avoid duplicates)
                            for result in results {
                                let already_found = all_results.iter().any(|r: &KeyResult| 
                                    r.sector == result.sector && r.key_type == result.key_type);
                                
                                if !already_found {
                                    all_results.push(result);
                                }
                            }
                        },
                        Err(e) => println!("Nested attack failed: {}", e),
                    }
                }
                
                // Finally darkside attack (slowest)
                println!("");
                println!("Running Darkside Attack...");
                match darkside_attack(spi) {
                    Ok(results) => {
                        // Only add new results (avoid duplicates)
                        for result in results {
                            let already_found = all_results.iter().any(|r: &KeyResult| 
                                r.sector == result.sector && r.key_type == result.key_type);
                            
                            if !already_found {
                                all_results.push(result);
                            }
                        }
                    },
                    Err(e) => println!("Darkside attack failed: {}", e),
                }
                
                // Summarize all found keys
                if all_results.is_empty() {
                    println!("");
                    println!("No keys were recovered from any attack.");
                } else {
                    println!("");
                    println!("RECOVERED KEYS SUMMARY");
                    println!("=====================");
                    println!("Found {} total keys:", all_results.len());
                    
                    // Sort by sector and key type for better readability
                    all_results.sort_by(|a, b| {
                        if a.sector != b.sector {
                            a.sector.cmp(&b.sector)
                        } else {
                            a.key_type.cmp(&b.key_type)
                        }
                    });
                    
                    for result in &all_results {
                        let key_type_str = if result.key_type == PICC_AUTHENT1A { "A" } else { "B" };
                        println!("  Sector {}: Key {}: {}", 
                                result.sector, key_type_str, bytes_to_hex(&result.key));
                    }
                    
                    save_recovered_keys(&all_results)?;
                }
            },
            "0" => {
                return Ok(());
            },
            _ => {
                println!("Invalid choice. Please try again.");
            }
        }
        
        wait_for_input("\nPress Enter to continue...")?;
    }
}
