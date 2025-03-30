use std::error::Error;
use std::io;
use std::thread::sleep;
use std::time::Duration;

// Import from the main crate
use desfire_tools::desfire_common::{
    connect_to_card, authenticate_des, 
    send_apdu, HexSlice, DEFAULT_MASTER_KEY,
    print_desfire_error
};

fn main() -> Result<(), Box<dyn Error>> {
    // Connect to a card
    let (ctx, card) = connect_to_card()?;
    
    // Try to authenticate with retries
    let mut success = false;
    for attempt in 1..=3 {
        println!("\nAttempting authentication (try {}/3)...", attempt);
        match authenticate_des(&card, 0, &DEFAULT_MASTER_KEY) {
            Ok(_) => {
                success = true;
                break;
            },
            Err(e) => {
                println!("Authentication attempt {} failed: {}", attempt, e);
                if attempt < 3 {
                    println!("Waiting before retry...");
                    sleep(Duration::from_millis(500));
                }
            }
        }
    }
    
    if !success {
        return Err("Authentication failed after multiple attempts".into());
    }
    
    // Display options to the user
    println!("\nCard setup options:");
    println!("1. Create a new application");
    println!("2. List existing applications");
    println!("3. Exit");
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    match input.trim() {
        "1" => create_application(&card)?,
        "2" => list_applications(&card)?,
        _ => println!("Exiting"),
    }
    
    Ok(())
}

// Create a new application on the card
fn create_application(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nCreating a new application...");
    
    // Ask for the application ID (3 bytes)
    println!("Enter application ID (6 hex digits, e.g., 112233):");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    // Parse the hex string to bytes
    let input = input.trim();
    if input.len() != 6 {
        return Err("Application ID must be 6 hex digits (3 bytes)".into());
    }
    
    let app_id = [
        u8::from_str_radix(&input[0..2], 16)?,
        u8::from_str_radix(&input[2..4], 16)?,
        u8::from_str_radix(&input[4..6], 16)?,
    ];
    
    // Ask for number of keys
    println!("Enter number of keys for this application (1-14):");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let num_keys: u8 = input.trim().parse()?;
    
    if num_keys < 1 || num_keys > 14 {
        return Err("Number of keys must be between 1 and 14".into());
    }
    
    // Set application settings
    // Bit 0: Allow changing master key
    // Bit 1: Require authentication for directory listing
    // Bit 2: Allow creating files without master key authentication
    // Bit 3: Allow configuration changes
    let settings = 0x0F; // All permissions enabled
    
    // Create the application
    let mut create_app_cmd = vec![0x90, 0xCA, 0x00, 0x00, 0x05];
    create_app_cmd.extend_from_slice(&app_id);
    create_app_cmd.push(settings);
    create_app_cmd.push(num_keys);
    create_app_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &create_app_cmd) {
        Ok(response) => {
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                println!("Application created successfully with ID: {}", HexSlice(&app_id));
                
                // Add delay before selecting
                sleep(Duration::from_millis(200));
                
                // Select the application
                select_application(card, &app_id)?;
                
                // Create a sample file in the application
                create_sample_file(card)?;
                
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                Err(format!("Failed to create application: {} ({})", 
                        print_desfire_error(error), error).into())
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

// Select an application by AID
fn select_application(card: &pcsc::Card, app_id: &[u8; 3]) -> Result<(), Box<dyn Error>> {
    println!("\nSelecting application: {}", HexSlice(app_id));
    
    // Create the select application command
    let mut select_app_cmd = vec![0x90, 0x5A, 0x00, 0x00, 0x03];
    select_app_cmd.extend_from_slice(app_id);
    select_app_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &select_app_cmd) {
        Ok(response) => {
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                println!("Application selected successfully");
                // Add delay after selection
                sleep(Duration::from_millis(200));
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

// Create a standard data file within the application
fn create_sample_file(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nCreating a standard data file...");
    
    // File parameters
    let file_id = 0x01; // File ID (0-31)
    let comm_settings = 0x00; // Plain communication
    let access_rights = [0xEE, 0xEE]; // Read/write with key 0, all other operations with key 0
    let file_size = [0x20, 0x00, 0x00]; // 32 bytes (little endian)
    
    // Create standard file command
    let mut create_file_cmd = vec![0x90, 0xCD, 0x00, 0x00, 0x07];
    create_file_cmd.push(file_id);
    create_file_cmd.push(comm_settings);
    create_file_cmd.extend_from_slice(&access_rights);
    create_file_cmd.extend_from_slice(&file_size);
    create_file_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &create_file_cmd) {
        Ok(response) => {
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                println!("File created successfully with ID: {:02X}", file_id);
                
                // Add delay before writing
                sleep(Duration::from_millis(200));
                
                // Write sample data to the file
                write_to_file(card, file_id)?;
                
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                Err(format!("Failed to create file: {} ({})",
                       print_desfire_error(error), error).into())
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

// Write data to the file
fn write_to_file(card: &pcsc::Card, file_id: u8) -> Result<(), Box<dyn Error>> {
    println!("\nWriting data to file...");
    
    // Sample data to write
    let data = b"Hello, DESFire Card!";
    let offset = [0x00, 0x00, 0x00]; // Start at offset 0
    let length = data.len() as u8;
    
    // Write data command
    let mut write_cmd = vec![0x90, 0x3D, 0x00, 0x00];
    write_cmd.push(7 + length); // Lc byte (file_id + offset + length + data.len)
    write_cmd.push(file_id);
    write_cmd.extend_from_slice(&offset);
    write_cmd.push(length);
    write_cmd.extend_from_slice(data);
    write_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &write_cmd) {
        Ok(response) => {
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                println!("Data written successfully");
                
                // Add delay before reading
                sleep(Duration::from_millis(200));
                
                // Read back the data to verify
                read_from_file(card, file_id)?;
                
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                Err(format!("Failed to write data: {} ({})",
                       print_desfire_error(error), error).into())
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

// Read data from the file
fn read_from_file(card: &pcsc::Card, file_id: u8) -> Result<(), Box<dyn Error>> {
    println!("\nReading data from file...");
    
    let offset = [0x00, 0x00, 0x00]; // Start at offset 0
    let length = [0x20, 0x00, 0x00]; // Read up to 32 bytes
    
    // Read data command
    let mut read_cmd = vec![0x90, 0xBD, 0x00, 0x00, 0x07];
    read_cmd.push(file_id);
    read_cmd.extend_from_slice(&offset);
    read_cmd.extend_from_slice(&length);
    read_cmd.push(0x00); // Le byte
    
    match send_apdu(card, &read_cmd) {
        Ok(response) => {
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                // Extract the data (exclude the status bytes)
                let data = &response[0..response.len()-2];
                println!("Read data: {}", HexSlice(data));
                
                // Convert to text if possible
                match std::str::from_utf8(data) {
                    Ok(text) => println!("Data as text: \"{}\"", text),
                    Err(_) => println!("Data is not valid UTF-8 text")
                }
                
                Ok(())
            } else if response.len() >= 2 {
                let error = response[response.len() - 1];
                Err(format!("Failed to read data: {} ({})",
                       print_desfire_error(error), error).into())
            } else {
                Err("Invalid response format".into())
            }
        },
        Err(e) => Err(e)
    }
}

// List applications on the card
fn list_applications(card: &pcsc::Card) -> Result<(), Box<dyn Error>> {
    println!("\nListing applications on card...");
    
    // GetApplications command
    let get_apps_cmd = [0x90, 0x6A, 0x00, 0x00, 0x00];
    
    match send_apdu(card, &get_apps_cmd) {
        Ok(response) => {
            if response.len() >= 2 && 
               response[response.len() - 2] == 0x91 && 
               response[response.len() - 1] == 0x00 {
                
                // Check if there are any applications (no data = no apps)
                if response.len() <= 2 {
                    println!("No applications found on the card.");
                } else {
                    println!("Applications found:");
                    
                    // Each AID is 3 bytes
                    let data = &response[0..response.len()-2];
                    for i in (0..data.len()).step_by(3) {
                        if i + 3 <= data.len() {
                            let app_id = &data[i..i+3];
                            println!("  Application ID: {}", HexSlice(app_id));
                        }
                    }
                }
                
                Ok(())
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
