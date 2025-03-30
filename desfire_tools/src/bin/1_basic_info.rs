use pcsc::{Card, Context, Protocols, Scope, ShareMode};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Establish context and connect to the reader
    let ctx = Context::establish(Scope::User)?;
    
    // List readers (in 2.9.0, we need a different approach)
    let mut readers_buffer = [0; 2048]; // Buffer for reader names
    let mut readers = ctx.list_readers(&mut readers_buffer)?;
    
    if readers.clone().count() == 0 {
        println!("No readers found!");
        return Ok(());
    }
    
    // Print available readers
    println!("Available readers:");
    let mut i = 0;
    let mut readers_buffer2 = [0; 2048]; // Need a new buffer for another iterator
    let mut readers2 = ctx.list_readers(&mut readers_buffer2)?;
    while let Some(reader) = readers2.next() {
        println!("  Reader {}: {}", i, reader.to_string_lossy());
        i += 1;
    }
    
    // Get the first reader
    if let Some(reader) = readers.next() {
        println!("\nConnecting to reader: {}", reader.to_string_lossy());
        let card = ctx.connect(reader, ShareMode::Shared, Protocols::ANY)?;
        
        println!("Card connected!");
        println!("Place your DESFire card on the reader...");
        
        // Send GetVersion command (ISO 7816-4 wrapping mode)
        let get_version_apdu = [0x90, 0x60, 0x00, 0x00, 0x00];
        println!("\nSending GetVersion command (first part)...");
        match send_apdu(&card, &get_version_apdu) {
            Ok(response) => {
                println!("Response: {:02X?}", response);
                
                // Check if more data is available
                if response.len() >= 2 && response[response.len() - 2] == 0x91 && response[response.len() - 1] == 0xAF {
                    println!("\nMore data available, sending GetAdditionalFrame...");
                    
                    // Send GetAdditionalFrame command
                    let get_additional_frame = [0x90, 0xAF, 0x00, 0x00, 0x00];
                    match send_apdu(&card, &get_additional_frame) {
                        Ok(response2) => {
                            println!("Response: {:02X?}", response2);
                            
                            // If more data is available, get the final frame
                            if response2.len() >= 2 && response2[response2.len() - 2] == 0x91 && response2[response2.len() - 1] == 0xAF {
                                println!("\nSending final GetAdditionalFrame...");
                                match send_apdu(&card, &get_additional_frame) {
                                    Ok(response3) => {
                                        println!("Final response: {:02X?}", response3);
                                        print_card_info(&response, &response2, &response3);
                                    },
                                    Err(e) => println!("Error sending final GetAdditionalFrame: {}", e)
                                }
                            }
                        },
                        Err(e) => println!("Error sending GetAdditionalFrame: {}", e)
                    }
                }
            },
            Err(e) => println!("Error sending GetVersion command: {}", e)
        }
        
        // Try to get card UID (if possible without authentication)
        let get_uid_apdu = [0x90, 0x51, 0x00, 0x00, 0x00];
        println!("\nAttempting to read card UID...");
        match send_apdu(&card, &get_uid_apdu) {
            Ok(response) => {
                println!("UID Response: {:02X?}", response);
                if response.len() > 2 {
                    let uid = &response[0..response.len()-2];
                    println!("Card UID: {:02X?}", uid);
                } else {
                    println!("UID may require authentication");
                }
            },
            Err(e) => println!("Error reading UID (may require authentication): {}", e)
        }
    } else {
        println!("Failed to access the first reader");
    }
    
    Ok(())
}

fn send_apdu(card: &Card, apdu: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut recv_buffer = [0; 258]; // Max response size
    
    // Call transmit and get a slice reference back
    let result = card.transmit(apdu, &mut recv_buffer)?;
    
    // Convert the slice to a Vec<u8> as required by the return type
    Ok(result.to_vec())
}


fn print_card_info(resp1: &[u8], resp2: &[u8], resp3: &[u8]) {
    // Extract hardware info
    if resp1.len() >= 7 {
        println!("\n--- Card Information ---");
        println!("Hardware Vendor ID: {:02X?}", &resp1[0]);
        println!("Hardware Type: {:02X?}", &resp1[1]);
        println!("Hardware Subtype: {:02X?}", &resp1[2]);
        println!("Hardware Version Major: {:02X?}", &resp1[3]);
        println!("Hardware Version Minor: {:02X?}", &resp1[4]);
        println!("Hardware Storage Size: {:02X?}", &resp1[5]);
        println!("Hardware Protocol: {:02X?}", &resp1[6]);
    }
    
    // Extract software info
    if resp2.len() >= 7 {
        println!("\nSoftware Vendor ID: {:02X?}", &resp2[0]);
        println!("Software Type: {:02X?}", &resp2[1]);
        println!("Software Subtype: {:02X?}", &resp2[2]);
        println!("Software Version Major: {:02X?}", &resp2[3]);
        println!("Software Version Minor: {:02X?}", &resp2[4]);
        println!("Software Storage Size: {:02X?}", &resp2[5]);
        println!("Software Protocol: {:02X?}", &resp2[6]);
    }
    
    // Extract additional info if available
    if resp3.len() > 10 {
        println!("\nAdditional UID Info: {:02X?}", &resp3[0..7]);
        println!("Production Batch: {:02X?}", &resp3[7..11]);
        println!("Week of Production: {:02X?}", resp3[11]);
        println!("Year of Production: {:02X?}", resp3[12]);
    }
}
