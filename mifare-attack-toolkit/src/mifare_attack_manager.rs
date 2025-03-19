// src/mifare_attack_manager.rs
use std::error::Error;
use std::io::{self, Write};

use crate::reader::MifareClassic;
use crate::attacks;
use crate::operations;
use crate::utils::{wait_for_enter, get_user_confirmation};

pub struct MifareAttackManager<'a> {
    reader: &'a mut MifareClassic,
}

impl<'a> MifareAttackManager<'a> {
    pub fn new(reader: &'a mut MifareClassic) -> Self {
        Self { reader }
    }
    
    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            self.display_menu();
            
            print!("Enter choice: ");
            io::stdout().flush()?;
            
            let mut choice = String::new();
            io::stdin().read_line(&mut choice)?;
            
            match choice.trim() {
                "1" => self.read_uid()?,
                "2" => self.try_default_keys()?,
                "3" => self.run_nested_attack()?,
                "4" => self.run_darkside_attack()?,
                "5" => self.detect_magic_card()?,
                "6" => self.write_custom_uid()?,
                "7" => self.dump_card()?,
                "8" => self.clone_card()?,
                "9" | "q" | "exit" | "quit" => {
                    println!("Exiting...");
                    break;
                },
                _ => {
                    println!("Invalid choice. Please try again.");
                }
            }
        }
        
        Ok(())
    }
    
    fn display_menu(&self) {
        println!("\n\nSelect an option:");
        println!("1. Read card UID");
        println!("2. Try default keys");
        println!("3. Run Nested Attack (requires a known key)");
        println!("4. Run Darkside Attack");
        println!("5. Detect Magic Card");
        println!("6. Write custom UID (requires Magic Card)");
        println!("7. Dump card contents");
        println!("8. Clone card to Magic Card");
        println!("9. Exit");
    }
    
    fn read_uid(&mut self) -> Result<(), Box<dyn Error>> {
        operations::read::read_uid(self.reader)
    }
    
    fn try_default_keys(&mut self) -> Result<(), Box<dyn Error>> {
        attacks::default_keys::run_default_key_search(self.reader)
    }
    
    fn run_nested_attack(&mut self) -> Result<(), Box<dyn Error>> {
        attacks::nested::run_nested_attack(self.reader)
    }
    
    fn run_darkside_attack(&mut self) -> Result<(), Box<dyn Error>> {
        attacks::darkside::run_darkside_attack(self.reader)
    }
    
    fn detect_magic_card(&mut self) -> Result<(), Box<dyn Error>> {
        operations::magic_card::detect_card_type(self.reader)
    }
    
    fn write_custom_uid(&mut self) -> Result<(), Box<dyn Error>> {
        operations::magic_card::write_custom_uid(self.reader)
    }
    
    fn dump_card(&mut self) -> Result<(), Box<dyn Error>> {
        operations::read::dump_card(self.reader)
    }
    
    fn clone_card(&mut self) -> Result<(), Box<dyn Error>> {
        operations::clone::clone_card(self.reader)
    }
}

// Helper function to run the menu
pub fn run_menu(reader: &mut MifareClassic) {
    let mut manager = MifareAttackManager::new(reader);
    
    if let Err(e) = manager.run() {
        println!("Error: {}", e);
    }
}
