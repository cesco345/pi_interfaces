use pcsc::{Card, Context, Protocols, Scope, ShareMode};
use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use crate::util::{HexSlice, prompt_card_action};

/// Send APDU command to the card and return the response
pub fn send_apdu(card: &Card, apdu: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    println!("Sending APDU: {}", HexSlice(apdu));
    
    let mut recv_buffer = [0; 258]; // Max response size
    let result = card.transmit(apdu, &mut recv_buffer)?;
    
    println!("Response: {}", HexSlice(result));
    Ok(result.to_vec())
}

/// Connect to the first available card reader with a card present
pub fn connect_to_card() -> Result<(Context, Card), Box<dyn Error>> {
    // Establish context
    let ctx = Context::establish(Scope::User)?;
    
    // List readers
    let mut readers_buffer = [0; 2048];
    let mut readers = ctx.list_readers(&mut readers_buffer)?;
    
    if readers.clone().count() == 0 {
        return Err("No readers found!".into());
    }
    
    // Print available readers
    println!("Available readers:");
    let mut i = 0;
    let mut readers_buffer2 = [0; 2048];
    let mut readers2 = ctx.list_readers(&mut readers_buffer2)?;
    while let Some(reader) = readers2.next() {
        println!("  Reader {}: {}", i, reader.to_string_lossy());
        i += 1;
    }
    
    // Ask user to remove any cards first
    prompt_card_action("remove", 1000)?;
    
    // Then ask the user to place a card
    prompt_card_action("place", 2000)?;
    
    // Now connect to the first available reader
    if let Some(reader) = readers.next() {
        println!("Connecting to reader: {}", reader.to_string_lossy());
        
        // Try to connect a few times in case of timing issues
        let mut retry_count = 0;
        let max_retries = 3;
        
        while retry_count < max_retries {
            match ctx.connect(reader, ShareMode::Shared, Protocols::ANY) {
                Ok(card) => {
                    println!("Successfully connected to card!");
                    
                    // Verify it's a DESFire card
                    match verify_desfire_card(&card) {
                        Ok(_) => return Ok((ctx, card)),
                        Err(e) => {
                            println!("Card verification failed: {}", e);
                            if retry_count < max_retries - 1 {
                                println!("Retrying connection...");
                                sleep(Duration::from_millis(500));
                                retry_count += 1;
                            } else {
                                return Err("Card verification failed after retries".into());
                            }
                        }
                    }
                },
                Err(e) => {
                    println!("Connection attempt {}: {}", retry_count + 1, e);
                    if retry_count < max_retries - 1 {
                        println!("Waiting and retrying...");
                        sleep(Duration::from_millis(1000));
                        retry_count += 1;
                    } else {
                        return Err("Could not connect to card after multiple attempts".into());
                    }
                }
            }
        }
        
        Err("Failed to establish stable connection to card".into())
    } else {
        Err("No readers available".into())
    }
}

/// Verify that the connected card is a DESFire card
pub fn verify_desfire_card(card: &Card) -> Result<(), Box<dyn Error>> {
    // Get card version to verify it's a DESFire card
    let get_version_apdu = [0x90, 0x60, 0x00, 0x00, 0x00];
    println!("\nVerifying card is a DESFire card...");
    
    match send_apdu(card, &get_version_apdu) {
        Ok(response) => {
            if response.len() > 2 {
                // Check if more data is available (91 AF response)
                if response.len() >= 2 && 
                   response[response.len() - 2] == 0x91 && 
                   response[response.len() - 1] == 0xAF {
                    println!("Card is a DESFire card.");
                    return Ok(());
                }
            }
            Err("Card doesn't appear to be a DESFire card.".into())
        },
        Err(e) => Err(e)
    }
}

/// Select the master application (PICC level)
pub fn select_master_application(card: &Card) -> Result<(), Box<dyn Error>> {
    println!("\nSelecting master application (PICC)");
    
    // Reset card state by selecting PICC (master application)
    let select_picc = [0x90, 0x5A, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00];
    match send_apdu(card, &select_picc) {
        Ok(response) => {
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                println!("Master application selected successfully");
                sleep(Duration::from_millis(100));
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                Err(format!("Failed to select master application: {:02X}", error).into())
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}
