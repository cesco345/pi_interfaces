use std::error::Error;
use std::thread::sleep;
use std::time::Duration;
use desfire_tools::desfire_common::{
    connect_to_card, authenticate_des, send_apdu, 
    DEFAULT_MASTER_KEY, HexSlice
};

fn main() -> Result<(), Box<dyn Error>> {
    // Connect to card
    println!("Connecting to card...");
    let (_, card) = connect_to_card()?;
    
    // Try to reset the card to a known state
    println!("Attempting to reset card state...");
    
    // First try to select master application (may fail, but that's okay)
    let select_master = [0x90, 0x5A, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00];
    let select_result = send_apdu(&card, &select_master);
    println!("Select master result: {:?}", select_result);
    
    // Try a direct authentication attempt
    println!("Direct authentication attempt...");
    
    // Authentication command (may fail, but let's see the response)
    let auth_cmd = [0x90, 0x0A, 0x00, 0x00, 0x01, 0x00, 0x00];
    let auth_result = send_apdu(&card, &auth_cmd);
    println!("Auth command result: {:?}", auth_result);
    
    // Try to get card version (should work even without authentication)
    println!("\nGetting card version:");
    let get_version = [0x90, 0x60, 0x00, 0x00, 0x00];
    match send_apdu(&card, &get_version) {
        Ok(response) => {
            println!("Version response: {}", HexSlice(&response));
            
            // If we get more data available response, continue
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0xAF {
                
                println!("More version data available, continuing...");
                let continue_cmd = [0x90, 0xAF, 0x00, 0x00, 0x00];
                match send_apdu(&card, &continue_cmd) {
                    Ok(more_data) => println!("Additional version data: {}", HexSlice(&more_data)),
                    Err(e) => println!("Error getting more version data: {}", e)
                }
            }
        },
        Err(e) => println!("Error getting version: {}", e)
    }
    
    // Try a format operation to reset the card
    println!("\nAttempting card format operation...");
    let format_cmd = [0x90, 0xFC, 0x00, 0x00, 0x00];
    match send_apdu(&card, &format_cmd) {
        Ok(response) => println!("Format response: {}", HexSlice(&response)),
        Err(e) => println!("Format error: {}", e)
    }
    
    // Sleep after format attempt
    sleep(Duration::from_millis(1000));
    
    // Try authentication again after format attempt
    println!("\nTrying authentication after format attempt...");
    let auth_cmd = [0x90, 0x0A, 0x00, 0x00, 0x01, 0x00, 0x00];
    match send_apdu(&card, &auth_cmd) {
        Ok(response) => {
            println!("Auth response: {}", HexSlice(&response));
            
            // Check if we got the expected challenge
            if response.len() >= 10 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0xAF {
                
                println!("Received authentication challenge!");
                // Here we would normally complete the authentication, but
                // let's just confirm we can get this far
            }
        },
        Err(e) => println!("Auth error: {}", e)
    }
    
    // List applications (should be empty on a new or formatted card)
    println!("\nListing applications:");
    let get_apps_cmd = [0x90, 0x6A, 0x00, 0x00, 0x00];
    match send_apdu(&card, &get_apps_cmd) {
        Ok(response) => println!("Applications response: {}", HexSlice(&response)),
        Err(e) => println!("Error listing applications: {}", e)
    }
    
    println!("\nCard test complete!");
    Ok(())
}
