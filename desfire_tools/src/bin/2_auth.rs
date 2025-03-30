use pcsc::{Card, Context, Protocols, Scope, ShareMode};
use std::error::Error;
use std::fmt;
use std::io;
use std::thread::sleep;
use std::time::Duration;
use openssl::symm::{Cipher, Crypter, Mode};
use openssl::rand::rand_bytes;

fn main() -> Result<(), Box<dyn Error>> {
    // Establish context
    let ctx = Context::establish(Scope::User)?;
    
    // List readers
    let mut readers_buffer = [0; 2048]; // Buffer for reader names
    let mut readers = ctx.list_readers(&mut readers_buffer)?;
    
    if readers.clone().count() == 0 {
        println!("No readers found!");
        return Ok(());
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
    
    // Ask user to place card on reader BEFORE connecting
    println!("\n------------------------------------------------------");
    println!("PLEASE REMOVE ANY CARDS from the reader now.");
    println!("Press ENTER when ready to continue...");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    println!("\nNow PLACE YOUR CARD on the reader...");
    println!("Press ENTER after placing the card on the reader.");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    // Add a small delay to ensure card is ready
    println!("Waiting for card to stabilize...");
    sleep(Duration::from_millis(1000));
    
    // Now connect to the reader with card in place
    if let Some(reader) = readers.next() {
        println!("Connecting to reader: {}", reader.to_string_lossy());
        
        // Connect with card already on reader
        let card = match ctx.connect(reader, ShareMode::Shared, Protocols::ANY) {
            Ok(card) => {
                println!("Successfully connected to card!");
                card
            },
            Err(e) => {
                println!("Error connecting to card: {}", e);
                println!("Please try repositioning the card and run again.");
                return Err("Card connection failed".into());
            }
        };
        
        // First try to get card version to verify connection
        let get_version_apdu = [0x90, 0x60, 0x00, 0x00, 0x00];
        println!("\nVerifying card connection...");
        match send_apdu(&card, &get_version_apdu) {
            Ok(response) => {
                if response.len() > 2 {
                    println!("Card connection verified!");
                    
                    // Check if more data is available (91 AF response)
                    if response.len() >= 2 && 
                       response[response.len() - 2] == 0x91 && 
                       response[response.len() - 1] == 0xAF {
                        // There's more data, but we don't need it for connection test
                        println!("Card is a DESFire card.");
                    }
                } else {
                    println!("Card response too short. Try repositioning the card.");
                    return Err("Card communication error".into());
                }
            },
            Err(e) => {
                println!("Error communicating with card: {}", e);
                println!("Please make sure the card is properly placed on the reader.");
                return Err(e);
            }
        }
        
        // Choose which authentication method to try
        println!("\nChoose authentication method to try:");
        println!("1. DES Authentication (Single DES)");
        println!("2. 3DES Authentication (Triple DES)");
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim();
        
        match choice {
            "1" => try_des_auth(&card)?,
            "2" => try_3des_auth(&card)?,
            _ => {
                println!("Invalid choice, defaulting to 3DES Authentication");
                try_3des_auth(&card)?
            }
        }
    } else {
        println!("No readers available");
    }
    
    Ok(())
}

fn try_des_auth(card: &Card) -> Result<(), Box<dyn Error>> {
    // Get card UID for possible key diversification
    let card_uid = match get_card_uid(card) {
        Ok(uid) => uid,
        Err(_) => {
            println!("Could not retrieve card UID, using default value");
            vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        }
    };
    
    println!("Card UID: {}", HexSlice(&card_uid));
    
    // Array of common DES key patterns
    let des_keys = [
        // Default/transport keys
        [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // All zeros
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], // All FF
        [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0], // NXP default
        
        // Common patterns
        [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7],
        [0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7],
        [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11], // Maintenance key
        [0xA0, 0xB0, 0xC0, 0xD0, 0xE0, 0xF0, 0xA1, 0xB1], // Known mifare key
        
        // Additional default keys
        [0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47],
        [0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F],
        [0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57],
        
        // DESFire EV1 default MAD key
        [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7],
        
        // Sequences
        [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
        [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88],
    ];
    
    // Additional derived keys
    let mut all_keys = Vec::with_capacity(50); // Reserve space for original + derived keys
    
    // Add the original keys
    for key in &des_keys {
        all_keys.push(key.clone());
    }
    
    // Add reversed keys
    for key in &des_keys {
        let mut reversed = [0u8; 8];
        for i in 0..8 {
            reversed[i] = key[7-i];
        }
        all_keys.push(reversed);
    }
    
    // Add inverted keys
    for key in &des_keys {
        let mut inverted = [0u8; 8];
        for i in 0..8 {
            inverted[i] = !key[i];
        }
        all_keys.push(inverted);
    }
    
    // Try to create diversified keys from UID if it's 7 bytes long
    if card_uid.len() == 7 {
        // Create a simple diversified key by XORing first 7 bytes of key with UID
        for key in &des_keys {
            let mut diversified = [0u8; 8];
            for i in 0..7 {
                diversified[i] = key[i] ^ card_uid[i];
            }
            diversified[7] = key[7]; // Keep last byte
            all_keys.push(diversified);
        }
    }
    
    println!("\nTrying DES authentication with {} different keys...", all_keys.len());
    
    // First focus only on key #0 which showed most promise
    println!("\n--- Testing with key number 0 ---");
    
    // Try each key
    for (i, key) in all_keys.iter().enumerate() {
        println!("\nAttempting authentication with key {}: {}", i+1, HexSlice(key));
        
        // 1. Send authentication command for DES with key number 0
        let auth_cmd = [0x90, 0x0A, 0x00, 0x00, 0x01, 0x00, 0x00];
        
        match send_apdu(card, &auth_cmd) {
            Ok(response) => {
                if response.len() >= 2 && response[response.len() - 2] == 0x91 {
                    if response[response.len() - 1] == 0xAF {
                        // Authentication started, card returned challenge (RndB)
                        println!("Received challenge from card: {}", HexSlice(&response[0..8]));
                        
                        // 2. Decrypt RndB
                        let enc_rnd_b = &response[0..8];
                        let rnd_b = match des_decrypt(key, enc_rnd_b) {
                            Ok(data) => {
                                println!("Decrypted RndB: {}", HexSlice(&data));
                                data
                            },
                            Err(e) => {
                                println!("Error decrypting RndB: {}", e);
                                continue; // Try next key
                            }
                        };
                        
                        // 3. Rotate RndB left
                        let rotated_rnd_b = rotate_left(&rnd_b);
                        println!("Rotated RndB: {}", HexSlice(&rotated_rnd_b));
                        
                        // 4. Generate random RndA
                        let mut rnd_a = [0u8; 8];
                        rand_bytes(&mut rnd_a)?;
                        println!("Generated RndA: {}", HexSlice(&rnd_a));
                        
                        // 5. Concatenate RndA + rotated RndB
                        let mut challenge_response = Vec::with_capacity(16);
                        challenge_response.extend_from_slice(&rnd_a);
                        challenge_response.extend_from_slice(&rotated_rnd_b);
                        
                        // 6. Encrypt the challenge response
                        let enc_challenge = match des_encrypt(key, &challenge_response) {
                            Ok(data) => data,
                            Err(e) => {
                                println!("Error encrypting challenge response: {}", e);
                                continue; // Try next key
                            }
                        };
                        
                        // 7. Send the encrypted challenge to the card
                        let mut send_challenge = vec![0x90, 0xAF, 0x00, 0x00, 0x10];
                        send_challenge.extend_from_slice(&enc_challenge);
                        send_challenge.push(0x00);
                        
                        println!("Sending encrypted challenge response");
                        match send_apdu(card, &send_challenge) {
                            Ok(challenge_resp) => {
                                if challenge_resp.len() >= 2 && 
                                   challenge_resp[challenge_resp.len() - 2] == 0x91 && 
                                   challenge_resp[challenge_resp.len() - 1] == 0x00 {
                                    
                                    // 8. Card responds with encrypted RndA (rotated left)
                                    if challenge_resp.len() >= 10 {
                                        let enc_rnd_a_resp = &challenge_resp[0..8];
                                        
                                        // 9. Decrypt to get rotated RndA
                                        match des_decrypt(key, enc_rnd_a_resp) {
                                            Ok(rnd_a_from_card) => {
                                                // 10. Compare with rotated RndA
                                                let rotated_rnd_a = rotate_left(&rnd_a);
                                                
                                                if rnd_a_from_card == rotated_rnd_a {
                                                    println!("DES Authentication successful with key {} and key number 0!", i+1);
                                                    println!("Key: {}", HexSlice(key));
                                                    
                                                    // Try to get a list of applications
                                                    println!("\nTrying to get applications list...");
                                                    let get_apps_cmd = [0x90, 0x6A, 0x00, 0x00, 0x00];
                                                    match send_apdu(card, &get_apps_cmd) {
                                                        Ok(apps_resp) => {
                                                            println!("Applications response: {}", HexSlice(&apps_resp));
                                                            if apps_resp.len() > 2 && apps_resp[apps_resp.len() - 2] == 0x91 {
                                                                println!("Card has applications!");
                                                                // Parse the application IDs
                                                                let mut app_ids = Vec::new();
                                                                let mut i = 0;
                                                                while i < apps_resp.len() - 2 {
                                                                    if i + 3 <= apps_resp.len() - 2 {
                                                                        let app_id = &apps_resp[i..i+3];
                                                                        app_ids.push(app_id);
                                                                        println!("  App ID: {}", HexSlice(app_id));
                                                                        i += 3;
                                                                    } else {
                                                                        break;
                                                                    }
                                                                }
                                                            } else {
                                                                println!("No applications found or error.");
                                                            }
                                                        },
                                                        Err(e) => println!("Error getting applications: {}", e)
                                                    }
                                                    
                                                    return Ok(());
                                                } else {
                                                    println!("Authentication response verification failed");
                                                    println!("Expected: {}", HexSlice(&rotated_rnd_a));
                                                    println!("Received: {}", HexSlice(&rnd_a_from_card));
                                                }
                                            },
                                            Err(e) => println!("Error decrypting card response: {}", e)
                                        }
                                    } else {
                                        println!("DES Authentication successful with key {} and key number 0!", i+1);
                                        println!("Key: {}", HexSlice(key));
                                        return Ok(());
                                    }
                                } else if challenge_resp.len() >= 2 {
                                    println!("Card rejected authentication with status: {:02X} {:02X}",
                                             challenge_resp[challenge_resp.len() - 2],
                                             challenge_resp[challenge_resp.len() - 1]);
                                    print_desfire_error(challenge_resp[challenge_resp.len() - 1]);
                                }
                            },
                            Err(e) => println!("Error sending challenge: {}", e)
                        }
                    } else {
                        println!("Authentication error: {:02X}", response[response.len() - 1]);
                        print_desfire_error(response[response.len() - 1]);
                    }
                }
            },
            Err(e) => println!("Error starting authentication: {}", e)
        }
    }
    
    // If we're here, then key #0 authentication failed with all keys
    // Let's also try with key #1 through #4, but only with the original key set (not the derived ones)
    for key_num in 1..5 {
        println!("\n--- Testing with key number {} ---", key_num);
        
        // Only try the original keys, not all the derived ones
        for (i, key) in des_keys.iter().enumerate() {
            println!("\nAttempting authentication with key {}: {}", i+1, HexSlice(key));
            
            // 1. Send authentication command for DES with varying key number
            let auth_cmd = [0x90, 0x0A, 0x00, 0x00, 0x01, key_num, 0x00];
            
            match send_apdu(card, &auth_cmd) {
                Ok(response) => {
                    if response.len() >= 2 && response[response.len() - 2] == 0x91 {
                        if response[response.len() - 1] == 0xAF {
                            // Handle authentication challenge response (same as above)
                            println!("Received challenge from card: {}", HexSlice(&response[0..8]));
                            
                            // Complete the authentication process as above...
                            let enc_rnd_b = &response[0..8];
                            let rnd_b = match des_decrypt(key, enc_rnd_b) {
                                Ok(data) => {
                                    println!("Decrypted RndB: {}", HexSlice(&data));
                                    data
                                },
                                Err(e) => {
                                    println!("Error decrypting RndB: {}", e);
                                    continue; // Try next key
                                }
                            };
                            
                            let rotated_rnd_b = rotate_left(&rnd_b);
                            println!("Rotated RndB: {}", HexSlice(&rotated_rnd_b));
                            
                            let mut rnd_a = [0u8; 8];
                            rand_bytes(&mut rnd_a)?;
                            println!("Generated RndA: {}", HexSlice(&rnd_a));
                            
                            let mut challenge_response = Vec::with_capacity(16);
                            challenge_response.extend_from_slice(&rnd_a);
                            challenge_response.extend_from_slice(&rotated_rnd_b);
                            
                            let enc_challenge = match des_encrypt(key, &challenge_response) {
                                Ok(data) => data,
                                Err(e) => {
                                    println!("Error encrypting challenge response: {}", e);
                                    continue; // Try next key
                                }
                            };
                            
                            let mut send_challenge = vec![0x90, 0xAF, 0x00, 0x00, 0x10];
                            send_challenge.extend_from_slice(&enc_challenge);
                            send_challenge.push(0x00);
                            
                            println!("Sending encrypted challenge response");
                            match send_apdu(card, &send_challenge) {
                                Ok(challenge_resp) => {
                                    if challenge_resp.len() >= 2 && 
                                       challenge_resp[challenge_resp.len() - 2] == 0x91 && 
                                       challenge_resp[challenge_resp.len() - 1] == 0x00 {
                                        
                                        println!("DES Authentication successful with key {} and key number {}!", i+1, key_num);
                                        println!("Key: {}", HexSlice(key));
                                        return Ok(());
                                    } else if challenge_resp.len() >= 2 {
                                        println!("Card rejected authentication with status: {:02X} {:02X}",
                                                 challenge_resp[challenge_resp.len() - 2],
                                                 challenge_resp[challenge_resp.len() - 1]);
                                        print_desfire_error(challenge_resp[challenge_resp.len() - 1]);
                                    }
                                },
                                Err(e) => println!("Error sending challenge: {}", e)
                            }
                        } else {
                            println!("Authentication error: {:02X}", response[response.len() - 1]);
                            print_desfire_error(response[response.len() - 1]);
                        }
                    }
                },
                Err(e) => println!("Error starting authentication: {}", e)
            }
        }
    }
    
    println!("\nNone of the DES keys worked with any key number.");
    
    Err("DES authentication failed with all keys".into())
}

fn try_3des_auth(card: &Card) -> Result<(), Box<dyn Error>> {
    // Get card UID for possible key diversification
    let card_uid = match get_card_uid(card) {
        Ok(uid) => uid,
        Err(_) => {
            println!("Could not retrieve card UID, using default value");
            vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        }
    };
    
    println!("Card UID: {}", HexSlice(&card_uid));
    
    // Array of common 3DES key patterns (16 bytes each for 3DES)
    let des3_keys = [
        // Common keys
        [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // All zeros
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], // All FF
        [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0], // NXP default
        
        // Transport keys
        [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F],
        [0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F],
        
        // Application & PICC master keys
        [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7], // App master
        [0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7], // PICC master
        [0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7], // Known transport
        
        // Other known keys
        [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11], // Backup key
        [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], // Mixed zeros/FF
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // Mixed FF/zeros
        
        // DESFire EV1 default
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
        
        // Common sequences
        [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
    ];
    
    // Additional derived keys
    let mut all_keys = Vec::with_capacity(50);
    
    // Add the original keys
    for key in &des3_keys {
        all_keys.push(key.clone());
    }
    
    // Add key variants (first half and second half swapped)
    for key in &des3_keys {
        let mut swapped = [0u8; 16];
        for i in 0..8 {
            swapped[i] = key[i+8];
            swapped[i+8] = key[i];
        }
        all_keys.push(swapped);
    }
    
    // If we have a UID, try to create diversified keys
    if card_uid.len() == 7 {
        // Create diversified keys by XORing with UID
        for key in &des3_keys {
            let mut diversified = [0u8; 16];
            // XOR first 7 bytes with UID
            for i in 0..7 {
                diversified[i] = key[i] ^ card_uid[i];
                diversified[i+8] = key[i+8] ^ card_uid[i];
            }
            // Copy remaining bytes
            diversified[7] = key[7];
            diversified[15] = key[15];
            all_keys.push(diversified);
        }
    }
    
    println!("\nTrying 3DES authentication with {} different keys...", all_keys.len());
    
    // First focus on key #0 which showed most promise
    println!("\n--- Testing 3DES with key number 0 ---");
    
    // Try each key with key number 0
    for (i, key) in all_keys.iter().enumerate() {
        println!("\nAttempting 3DES authentication with key {}: {}", i+1, HexSlice(key));
        
        // 1. Send authentication command for 3DES with key number 0
        let auth_cmd = [0x90, 0x1A, 0x00, 0x00, 0x01, 0x00, 0x00];
        
        match send_apdu(card, &auth_cmd) {
            Ok(response) => {
                if response.len() >= 2 && response[response.len() - 2] == 0x91 {
                    if response[response.len() - 1] == 0xAF {
                        // Authentication started, card returned challenge (RndB)
                        println!("Received 3DES challenge from card: {}", HexSlice(&response[0..8]));
                        
                        // 2. Decrypt RndB using 3DES
                        let enc_rnd_b = &response[0..8];
                        let rnd_b = match des3_decrypt(key, enc_rnd_b) {
                            Ok(data) => {
                                println!("Decrypted RndB: {}", HexSlice(&data));
                                data
                            },
                            Err(e) => {
                                println!("Error decrypting RndB: {}", e);
                                continue; // Try next key
                            }
                        };
                        
                        // 3. Rotate RndB left
                        let rotated_rnd_b = rotate_left(&rnd_b);
                        println!("Rotated RndB: {}", HexSlice(&rotated_rnd_b));
                        
                        // 4. Generate random RndA
                        let mut rnd_a = [0u8; 8];
                        rand_bytes(&mut rnd_a)?;
                        println!("Generated RndA: {}", HexSlice(&rnd_a));
                        
                        // 5. Concatenate RndA + rotated RndB
                        let mut challenge_response = Vec::with_capacity(16);
                        challenge_response.extend_from_slice(&rnd_a);
                        challenge_response.extend_from_slice(&rotated_rnd_b);
                        
                        // 6. Encrypt the challenge response with 3DES
                        let enc_challenge = match des3_encrypt(key, &challenge_response) {
                            Ok(data) => data,
                            Err(e) => {
                                println!("Error encrypting challenge response: {}", e);
                                continue; // Try next key
                            }
                        };
                        
                        // 7. Send the encrypted challenge to the card
                        let mut send_challenge = vec![0x90, 0xAF, 0x00, 0x00, 0x10];
                        send_challenge.extend_from_slice(&enc_challenge);
                        send_challenge.push(0x00);
                        
                        println!("Sending encrypted 3DES challenge response");
                        match send_apdu(card, &send_challenge) {
                            Ok(challenge_resp) => {
                                if challenge_resp.len() >= 2 && 
                                   challenge_resp[challenge_resp.len() - 2] == 0x91 && 
                                   challenge_resp[challenge_resp.len() - 1] == 0x00 {
                                    
                                    // 8. Card responds with encrypted RndA (rotated left)
                                    if challenge_resp.len() >= 10 {
                                        let enc_rnd_a_resp = &challenge_resp[0..8];
                                        
                                        // 9. Decrypt to get rotated RndA using 3DES
                                        match des3_decrypt(key, enc_rnd_a_resp) {
                                            Ok(rnd_a_from_card) => {
                                                // 10. Compare with rotated RndA
                                                let rotated_rnd_a = rotate_left(&rnd_a);
                                                
                                                if rnd_a_from_card == rotated_rnd_a {
                                                    println!("3DES Authentication successful with key {} and key number 0!", i+1);
                                                    println!("Key: {}", HexSlice(key));
                                                    
                                                    // Try to get a list of applications after successful auth
                                                    println!("\nTrying to get applications list...");
                                                    let get_apps_cmd = [0x90, 0x6A, 0x00, 0x00, 0x00];
                                                    match send_apdu(card, &get_apps_cmd) {
                                                        Ok(apps_resp) => {
                                                            println!("Applications response: {}", HexSlice(&apps_resp));
                                                            if apps_resp.len() > 2 && apps_resp[apps_resp.len() - 2] == 0x91 {
                                                                println!("Card has applications!");
                                                                // Parse the application IDs
                                                                let mut app_ids = Vec::new();
                                                                let mut i = 0;
                                                                while i < apps_resp.len() - 2 {
                                                                    if i + 3 <= apps_resp.len() - 2 {
                                                                        let app_id = &apps_resp[i..i+3];
                                                                        app_ids.push(app_id);
                                                                        println!("  App ID: {}", HexSlice(app_id));
                                                                        i += 3;
                                                                    } else {
                                                                        break;
                                                                    }
                                                                }
                                                            } else {
                                                                println!("No applications found or error.");
                                                            }
                                                        },
                                                        Err(e) => println!("Error getting applications: {}", e)
                                                    }
                                                    
                                                    return Ok(());
                                                } else {
                                                    println!("Authentication response verification failed");
                                                    println!("Expected: {}", HexSlice(&rotated_rnd_a));
                                                    println!("Received: {}", HexSlice(&rnd_a_from_card));
                                                }
                                            },
                                            Err(e) => println!("Error decrypting card response: {}", e)
                                        }
                                    } else {
                                        println!("3DES Authentication successful with key {} and key number 0!", i+1);
                                        println!("Key: {}", HexSlice(key));
                                        return Ok(());
                                    }
                                } else if challenge_resp.len() >= 2 {
                                    println!("Card rejected authentication with status: {:02X} {:02X}",
                                             challenge_resp[challenge_resp.len() - 2],
                                             challenge_resp[challenge_resp.len() - 1]);
                                    print_desfire_error(challenge_resp[challenge_resp.len() - 1]);
                                }
                            },
                            Err(e) => println!("Error sending challenge: {}", e)
                        }
                    } else {
                        println!("Authentication error: {:02X}", response[response.len() - 1]);
                        print_desfire_error(response[response.len() - 1]);
                    }
                }
            },
            Err(e) => println!("Error starting authentication: {}", e)
        }
    }
    
    // If key #0 authentication failed with all keys, try other key numbers but with a smaller key set
    for key_num in 1..5 {
        println!("\n--- Testing 3DES with key number {} ---", key_num);
        
        // Only try the original keys, not all the derived ones
        for (i, key) in des3_keys.iter().enumerate() {
            println!("\nAttempting 3DES authentication with key {}: {}", i+1, HexSlice(key));
            
            // 1. Send 3DES authentication command with varying key number
            let auth_cmd = [0x90, 0x1A, 0x00, 0x00, 0x01, key_num, 0x00];
            
            match send_apdu(card, &auth_cmd) {
                Ok(response) => {
                    if response.len() >= 2 && response[response.len() - 2] == 0x91 {
                        if response[response.len() - 1] == 0xAF {
                            // Handle authentication challenge response same as above
                            println!("Received 3DES challenge from card: {}", HexSlice(&response[0..8]));
                            
                            // Complete the authentication process as above...
                            let enc_rnd_b = &response[0..8];
                            let rnd_b = match des3_decrypt(key, enc_rnd_b) {
                                Ok(data) => {
                                    println!("Decrypted RndB: {}", HexSlice(&data));
                                    data
                                },
                                Err(e) => {
                                    println!("Error decrypting RndB: {}", e);
                                    continue; // Try next key
                                }
                            };
                            
                            let rotated_rnd_b = rotate_left(&rnd_b);
                            let mut rnd_a = [0u8; 8];
                            rand_bytes(&mut rnd_a)?;
                            
                            let mut challenge_response = Vec::with_capacity(16);
                            challenge_response.extend_from_slice(&rnd_a);
                            challenge_response.extend_from_slice(&rotated_rnd_b);
                            
                            let enc_challenge = match des3_encrypt(key, &challenge_response) {
                                Ok(data) => data,
                                Err(e) => {
                                    println!("Error encrypting challenge response: {}", e);
                                    continue; // Try next key
                                }
                            };
                            
                            let mut send_challenge = vec![0x90, 0xAF, 0x00, 0x00, 0x10];
                            send_challenge.extend_from_slice(&enc_challenge);
                            send_challenge.push(0x00);
                            
                            match send_apdu(card, &send_challenge) {
                                Ok(challenge_resp) => {
                                    if challenge_resp.len() >= 2 && 
                                       challenge_resp[challenge_resp.len() - 2] == 0x91 && 
                                       challenge_resp[challenge_resp.len() - 1] == 0x00 {
                                        
                                        println!("3DES Authentication successful with key {} and key number {}!", i+1, key_num);
                                        println!("Key: {}", HexSlice(key));
                                        return Ok(());
                                    } else if challenge_resp.len() >= 2 {
                                        println!("Card rejected 3DES authentication with status: {:02X} {:02X}",
                                                 challenge_resp[challenge_resp.len() - 2],
                                                 challenge_resp[challenge_resp.len() - 1]);
                                        print_desfire_error(challenge_resp[challenge_resp.len() - 1]);
                                    }
                                },
                                Err(e) => println!("Error sending 3DES challenge: {}", e)
                            }
                        } else {
                            println!("Authentication error: {:02X}", response[response.len() - 1]);
                            print_desfire_error(response[response.len() - 1]);
                        }
                    }
                },
                Err(e) => println!("Error starting 3DES authentication: {}", e)
            }
        }
    }
    
    println!("\nNone of the 3DES keys worked with any key number.");
    
    Err("3DES authentication failed with all keys".into())
}

// Function to get the card UID
fn get_card_uid(card: &Card) -> Result<Vec<u8>, Box<dyn Error>> {
    println!("Attempting to read card UID...");
    
    // Send GetCardUID command
    let get_uid_cmd = [0x90, 0x60, 0x00, 0x00, 0x00];
    
    match send_apdu(card, &get_uid_cmd) {
        Ok(response) => {
            if response.len() >= 9 && response[response.len() - 2] == 0x91 {
                // Extract UID from version info response
                // DESFire version info format: hw_vendor(1) | hw_type(1) | hw_subtype(1) | hw_version(1) | 
                //                            hw_storage(1) | hw_proto(1) | sw_vendor(1) | sw_type(1) | 
                //                            sw_subtype(1) | sw_version(1) | sw_storage(1) | uid(7) | 
                //                            batch_no(5) | cwProd(1) | yearProd(1)
                
                // If response has more data available (91 AF)
                if response[response.len() - 1] == 0xAF {
                    // Get additional frame
                    let get_more_cmd = [0x90, 0xAF, 0x00, 0x00, 0x00];
                    match send_apdu(card, &get_more_cmd) {
                        Ok(more_resp) => {
                            if more_resp.len() >= 9 && more_resp[more_resp.len() - 2] == 0x91 {
                                // Try to extract UID - typically at offset 7 in this frame
                                if more_resp.len() >= 14 { // 7 bytes UID + 2 byte status
                                    let uid = more_resp[0..7].to_vec();
                                    return Ok(uid);
                                }
                            }
                        },
                        Err(_) => {}
                    }
                }
                
                // If we couldn't get it from version info, try the GetCardUID command
                let get_uid_apdu = [0x90, 0x51, 0x00, 0x00, 0x00];
                match send_apdu(card, &get_uid_apdu) {
                    Ok(uid_resp) => {
                        if uid_resp.len() >= 9 && uid_resp[uid_resp.len() - 2] == 0x91 {
                            let uid = uid_resp[0..7].to_vec();
                            return Ok(uid);
                        }
                    },
                    Err(_) => {}
                }
            }
        },
        Err(_) => {}
    }
    
    Err("Could not retrieve card UID".into())
}

// Helper function to rotate bytes left by one position
fn rotate_left(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }
    
    let mut result = Vec::with_capacity(data.len());
    result.extend_from_slice(&data[1..]);
    result.push(data[0]);
    
    result
}

// DES encryption using OpenSSL
fn des_encrypt(key: &[u8], data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    if key.len() != 8 {
        return Err("DES key must be exactly 8 bytes".into());
    }
    
    // Make sure data is a multiple of 8 bytes (DES block size)
    let padded_data = if data.len() % 8 != 0 {
        let mut padded = data.to_vec();
        padded.resize((data.len() / 8 + 1) * 8, 0);
        padded
    } else {
        data.to_vec()
    };
    
    let cipher = Cipher::des_cbc();
    let iv = [0u8; 8]; // DESFire uses zero IV
    
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, Some(&iv))?;
    crypter.pad(false); // DESFire does not use padding
    
    let mut output = vec![0; padded_data.len() + cipher.block_size()];
    let count = crypter.update(&padded_data, &mut output)?;
    let rest = crypter.finalize(&mut output[count..])?;
    output.truncate(count + rest);
    
    Ok(output)
}

// DES decryption using OpenSSL
fn des_decrypt(key: &[u8], data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    if key.len() != 8 {
        return Err("DES key must be exactly 8 bytes".into());
    }
    
    if data.len() % 8 != 0 {
        return Err("DES data must be a multiple of 8 bytes".into());
    }
    
    let cipher = Cipher::des_cbc();
    let iv = [0u8; 8]; // DESFire uses zero IV
    
    let mut crypter = Crypter::new(cipher, Mode::Decrypt, key, Some(&iv))?;
    crypter.pad(false); // DESFire does not use padding
    
    let mut output = vec![0; data.len() + cipher.block_size()];
    let count = crypter.update(data, &mut output)?;
    let rest = crypter.finalize(&mut output[count..])?;
    output.truncate(count + rest);
    
    Ok(output)
}

// 3DES encryption using OpenSSL (16-byte key)
fn des3_encrypt(key: &[u8], data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    if key.len() != 16 {
        return Err("3DES key must be exactly 16 bytes (2 keys)".into());
    }
    
    // Make sure data is a multiple of 8 bytes (DES block size)
    let padded_data = if data.len() % 8 != 0 {
        let mut padded = data.to_vec();
        padded.resize((data.len() / 8 + 1) * 8, 0);
        padded
    } else {
        data.to_vec()
    };
    
    // Convert 16-byte key to 24-byte key for 3DES by repeating first 8 bytes
    let mut extended_key = Vec::with_capacity(24);
    extended_key.extend_from_slice(&key[0..16]);  // Add the original 16 bytes
    extended_key.extend_from_slice(&key[0..8]);   // Add the first 8 bytes again
    
    // For 3DES, we use the DES-EDE3-CBC mode with extended key
    let cipher = Cipher::des_ede3_cbc();
    let iv = [0u8; 8]; // DESFire uses zero IV
    
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, &extended_key, Some(&iv))?;
    crypter.pad(false); // DESFire does not use padding
    
    let mut output = vec![0; padded_data.len() + cipher.block_size()];
    let count = crypter.update(&padded_data, &mut output)?;
    let rest = crypter.finalize(&mut output[count..])?;
    output.truncate(count + rest);
    
    Ok(output)
}

// 3DES decryption using OpenSSL (16-byte key)
fn des3_decrypt(key: &[u8], data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    if key.len() != 16 {
        return Err("3DES key must be exactly 16 bytes (2 keys)".into());
    }
    
    if data.len() % 8 != 0 {
        return Err("3DES data must be a multiple of 8 bytes".into());
    }
    
    // Convert 16-byte key to 24-byte key for 3DES by repeating first 8 bytes
    let mut extended_key = Vec::with_capacity(24);
    extended_key.extend_from_slice(&key[0..16]);  // Add the original 16 bytes
    extended_key.extend_from_slice(&key[0..8]);   // Add the first 8 bytes again
    
    // For 3DES, we use the DES-EDE3-CBC mode with extended key
    let cipher = Cipher::des_ede3_cbc();
    let iv = [0u8; 8]; // DESFire uses zero IV
    
    let mut crypter = Crypter::new(cipher, Mode::Decrypt, &extended_key, Some(&iv))?;
    crypter.pad(false); // DESFire does not use padding
    
    let mut output = vec![0; data.len() + cipher.block_size()];
    let count = crypter.update(data, &mut output)?;
    let rest = crypter.finalize(&mut output[count..])?;
    output.truncate(count + rest);
    
    Ok(output)
}

fn send_apdu(card: &Card, apdu: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    println!("Sending APDU: {}", HexSlice(apdu));
    
    let mut recv_buffer = [0; 258]; // Max response size
    let result = card.transmit(apdu, &mut recv_buffer)?;
    
    // In PCSC 2.9.0, transmit returns a slice reference, not a length
    println!("Response: {}", HexSlice(result));
    Ok(result.to_vec())
}

// Helper struct to print byte slices as hex
struct HexSlice<'a>(&'a [u8]);

impl<'a> fmt::Display for HexSlice<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{:02X} ", byte)?;
        }
        Ok(())
    }
}

fn print_desfire_error(error_code: u8) {
    match error_code {
        0x00 => println!("Operation successful"),
        0x0C => println!("No changes made"),
        0x0E => println!("Out of EEPROM memory"),
        0x1C => println!("Illegal command code"),
        0x1E => println!("Integrity error"),
        0x40 => println!("No such key"),
        0x6E => println!("Error in authentication"),
        0x7E => println!("More data available"),
        0x9C => println!("Permission denied (authentication required)"),
        0x9E => println!("Parameter error"),
        0xA0 => println!("Application not found"),
        0xAE => println!("Authentication error"),
        0xDE => println!("Duplicate file/application"),
        0xEE => println!("File not found"),
        0xF0 => println!("File/application parameter error"),
        0xCA => println!("Command aborted"),
        _ => println!("Unknown error code: {:02X}", error_code),
    }
}
