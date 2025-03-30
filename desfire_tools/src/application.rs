use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use crate::card::send_apdu;
use crate::util::HexSlice;
use crate::error::{print_desfire_error, is_operation_success};

/// List applications on the card
pub fn list_applications(card: &pcsc::Card) -> Result<Vec<[u8; 3]>, Box<dyn Error>> {
    println!("\nListing applications on card...");
    
    // GetApplications command
    let get_apps_cmd = [0x90, 0x6A, 0x00, 0x00, 0x00];
    
    match send_apdu(card, &get_apps_cmd) {
        Ok(response) => {
            if is_operation_success(&response) {
                // Check if there are any applications (no data = no apps)
                if response.len() <= 2 {
                    println!("No applications found on the card.");
                    return Ok(Vec::new());
                }
                
                // Each AID is 3 bytes
                let data = &response[0..response.len()-2];
                let mut applications = Vec::new();
                
                println!("Applications found:");
                for i in (0..data.len()).step_by(3) {
                    if i + 3 <= data.len() {
                        let mut app_id = [0u8; 3];
                        app_id.copy_from_slice(&data[i..i+3]);
                        println!("  Application ID: {}", HexSlice(&app_id));
                        applications.push(app_id);
                    }
                }
                
                Ok(applications)
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                Err(format!("Failed to get applications: {} ({})",
                    print_desfire_error(error), error).into())
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

/// Create a new application on the card
pub fn create_application(
    card: &pcsc::Card, 
    app_id: &[u8; 3], 
    settings: u8, 
    num_keys: u8
) -> Result<(), Box<dyn Error>> {
    println!("\nCreating application with ID: {}", HexSlice(app_id));
    
    // Create the application
    let mut create_app_cmd = vec![0x90, 0xCA, 0x00, 0x00, 0x05];
    create_app_cmd.extend_from_slice(app_id);
    create_app_cmd.push(settings);
    create_app_cmd.push(num_keys);
    create_app_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &create_app_cmd) {
        Ok(response) => {
            if is_operation_success(&response) {
                println!("Application created successfully");
                sleep(Duration::from_millis(200)); // Add delay
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                
                // Special case: app already exists
                if error == 0xDE {
                    println!("Application already exists");
                    Ok(())
                } else {
                    Err(format!("Failed to create application: {} ({})",
                        print_desfire_error(error), error).into())
                }
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

/// Select an application by its ID
pub fn select_application(card: &pcsc::Card, app_id: &[u8; 3]) -> Result<(), Box<dyn Error>> {
    println!("\nSelecting application: {}", HexSlice(app_id));
    
    // Create the select application command
    let mut select_app_cmd = vec![0x90, 0x5A, 0x00, 0x00, 0x03];
    select_app_cmd.extend_from_slice(app_id);
    select_app_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &select_app_cmd) {
        Ok(response) => {
            if is_operation_success(&response) {
                println!("Application selected successfully");
                sleep(Duration::from_millis(200)); // Add delay after selection
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                Err(format!("Failed to select application: {} ({})",
                    print_desfire_error(error), error).into())
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

/// Delete an application from the card
pub fn delete_application(card: &pcsc::Card, app_id: &[u8; 3]) -> Result<(), Box<dyn Error>> {
    println!("\nDeleting application: {}", HexSlice(app_id));
    
    // Create the delete application command
    let mut delete_app_cmd = vec![0x90, 0xDA, 0x00, 0x00, 0x03];
    delete_app_cmd.extend_from_slice(app_id);
    delete_app_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &delete_app_cmd) {
        Ok(response) => {
            if is_operation_success(&response) {
                println!("Application deleted successfully");
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                Err(format!("Failed to delete application: {} ({})",
                    print_desfire_error(error), error).into())
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}
