use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::{thread, time::Duration};
use std::error::Error;
use std::io::{self, Write};

// MFRC522 Register Addresses
const COMMAND_REG: u8 = 0x01;
const COM_IRQ_REG: u8 = 0x04;
const DIV_IRQ_REG: u8 = 0x05;
const ERROR_REG: u8 = 0x06;
const STATUS1_REG: u8 = 0x07;
const STATUS2_REG: u8 = 0x08;
const FIFO_DATA_REG: u8 = 0x09;
const FIFO_LEVEL_REG: u8 = 0x0A;
const CONTROL_REG: u8 = 0x0C;
const BIT_FRAMING_REG: u8 = 0x0D;
const MODE_REG: u8 = 0x11;
const TX_MODE_REG: u8 = 0x12;
const RX_MODE_REG: u8 = 0x13;
const TX_CONTROL_REG: u8 = 0x14;
const TX_ASK_REG: u8 = 0x15;
const T_MODE_REG: u8 = 0x2A;
const T_PRESCALER_REG: u8 = 0x2B;
const T_RELOAD_REG_H: u8 = 0x2C;
const T_RELOAD_REG_L: u8 = 0x2D;
const VERSION_REG: u8 = 0x37;

// MFRC522 Command Set
const CMD_IDLE: u8 = 0x00;
const CMD_MEM: u8 = 0x01;
const CMD_AUTH: u8 = 0x0E;
const CMD_RECEIVE: u8 = 0x08;
const CMD_TRANSCEIVE: u8 = 0x0C;
const CMD_CALC_CRC: u8 = 0x03;

// MIFARE Commands
const PICC_WUPA: u8 = 0x52;
const PICC_ANTICOLL: u8 = 0x93;
const PICC_SELECT: u8 = 0x93;
const PICC_AUTHENT1A: u8 = 0x60;
const PICC_AUTHENT1B: u8 = 0x61;
const PICC_READ: u8 = 0x30;
const PICC_WRITE: u8 = 0xA0;
const PICC_DECREMENT: u8 = 0xC0;
const PICC_INCREMENT: u8 = 0xC1;
const PICC_RESTORE: u8 = 0xC2;
const PICC_TRANSFER: u8 = 0xB0;
const PICC_HALT: u8 = 0x50;

// Status flags
const IRQ_RX: u8 = 0x20;
const IRQ_IDLE: u8 = 0x10;
const IRQ_TIMER: u8 = 0x01;

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== MFRC522 Card Reader/Writer ===");
    println!("This tool reads and writes MIFARE Classic cards");
    println!("Press Ctrl+C to exit\n");
    
    // Open SPI with the configuration we found works best
    let mut spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 100_000, Mode::Mode0)?;
    
    // Show version information
    let version = read_register(&mut spi, VERSION_REG)?;
    println!("MFRC522 Version: 0x{:02X}", version);
    
    // Initialize the MFRC522
    initialize_mfrc522(&mut spi)?;
    
    println!("MFRC522 initialized successfully");
    
    // Menu system
    loop {
        println!("\nSelect an option:");
        println!("1. Read card UID");
        println!("2. Read card data (block)");
        println!("3. Write data to card");
        println!("4. Dump entire card data");
        println!("5. Exit");
        print!("Enter choice: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim();
        
        match choice {
            "1" => {
                println!("\nHold a card near the reader...");
                if let Some(uid) = wait_for_card(&mut spi)? {
                    println!("Card detected!");
                    println!("UID: {}", uid_to_string(&uid));
                    wait_for_card_removal(&mut spi)?;
                } else {
                    println!("No card detected or operation timed out.");
                }
            },
            "2" => {
                println!("\nHold a card near the reader...");
                if let Some(uid) = wait_for_card(&mut spi)? {
                    println!("Card detected!");
                    println!("UID: {}", uid_to_string(&uid));
                    
                    // Ask which block to read
                    print!("Enter block number (0-63): ");
                    io::stdout().flush()?;
                    let mut block_input = String::new();
                    io::stdin().read_line(&mut block_input)?;
                    let block: u8 = match block_input.trim().parse() {
                        Ok(num) if num < 64 => num,
                        _ => {
                            println!("Invalid block number, using block 1");
                            1
                        }
                    };
                    
                    // Ask which key type to use
                    print!("Use Key A or Key B? (A/B, default: A): ");
                    io::stdout().flush()?;
                    let mut key_input = String::new();
                    io::stdin().read_line(&mut key_input)?;
                    let use_key_b = key_input.trim().to_uppercase() == "B";
                    
                    // Ask for key value
                    println!("Enter 6-byte key in hex (FF FF FF FF FF FF for default):");
                    io::stdout().flush()?;
                    let mut key_val_input = String::new();
                    io::stdin().read_line(&mut key_val_input)?;
                    let key_val_input = key_val_input.trim();
                    
                    let key = if key_val_input.is_empty() {
                        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
                    } else {
                        let bytes: Vec<u8> = key_val_input
                            .split_whitespace()
                            .filter_map(|s| u8::from_str_radix(s, 16).ok())
                            .collect();
                        
                        if bytes.len() != 6 {
                            println!("Invalid key format, using default FF FF FF FF FF FF");
                            [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
                        } else {
                            let mut key_arr = [0xFF; 6];
                            key_arr.copy_from_slice(&bytes[..6]);
                            key_arr
                        }
                    };
                    
                    // First select the card
                    if !select_card(&mut spi, &uid)? {
                        println!("Failed to select card");
                    } else {
                        // Authenticate
                        let sector = block / 4;
                        let auth_cmd = if use_key_b { PICC_AUTHENT1B } else { PICC_AUTHENT1A };
                        
                        if !authenticate(&mut spi, auth_cmd, sector, &key, &uid)? {
                            println!("Authentication failed with provided key");
                        } else {
                            // Read block data
                            match read_block(&mut spi, block)? {
                                Some(data) => {
                                    println!("Data in block {}: {}", block, bytes_to_hex(&data));
                                    println!("ASCII: {}", bytes_to_ascii(&data));
                                },
                                None => println!("Failed to read block data")
                            }
                        }
                    }
                    
                    wait_for_card_removal(&mut spi)?;
                } else {
                    println!("No card detected or operation timed out.");
                }
            },
            "3" => {
                println!("\nHold a card near the reader...");
                if let Some(uid) = wait_for_card(&mut spi)? {
                    println!("Card detected!");
                    println!("UID: {}", uid_to_string(&uid));
                    
                    // Ask which block to write
                    print!("Enter block number (1-62, avoid sector trailers): ");
                    io::stdout().flush()?;
                    let mut block_input = String::new();
                    io::stdin().read_line(&mut block_input)?;
                    let block: u8 = match block_input.trim().parse() {
                        Ok(num) if num > 0 && num < 63 && num % 4 != 3 => num,
                        _ => {
                            println!("Invalid block number, using block 1");
                            1
                        }
                    };
                    
                    // Ask for data to write
                    println!("Enter data to write (16 bytes max, ASCII):");
                    io::stdout().flush()?;
                    let mut data_input = String::new();
                    io::stdin().read_line(&mut data_input)?;
                    let data_input = data_input.trim();
                    
                    // Convert input to byte array
                    let mut data = [0u8; 16];
                    for (i, byte) in data_input.bytes().take(16).enumerate() {
                        data[i] = byte;
                    }
                    
                    println!("Data to write: {}", bytes_to_hex(&data));
                    
                    // First select the card
                    if !select_card(&mut spi, &uid)? {
                        println!("Failed to select card");
                    } else {
                        // Authenticate with Key A
                        let sector = block / 4;
                        let key = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]; // Default key
                        
                        if !authenticate(&mut spi, PICC_AUTHENT1A, sector, &key, &uid)? {
                            println!("Authentication failed");
                        } else {
                            // Write block data
                            if write_block(&mut spi, block, &data)? {
                                println!("Data written successfully to block {}", block);
                            } else {
                                println!("Failed to write data to block");
                            }
                        }
                    }
                    
                    wait_for_card_removal(&mut spi)?;
                } else {
                    println!("No card detected or operation timed out.");
                }
            },
            "4" => {
                println!("\nHold a card near the reader...");
                if let Some(uid) = wait_for_card(&mut spi)? {
                    println!("Card detected!");
                    println!("UID: {}", uid_to_string(&uid));
                    
                    // Dump all accessible blocks
                    dump_card(&mut spi, &uid)?;
                    
                    wait_for_card_removal(&mut spi)?;
                } else {
                    println!("No card detected or operation timed out.");
                }
            },
            "5" => break,
            _ => println!("Invalid choice")
        }
    }
    
    Ok(())
}

// Initialize the MFRC522 with settings optimized for your hardware
fn initialize_mfrc522(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    // Soft reset
    write_register(spi, COMMAND_REG, 0x0F)?;
    thread::sleep(Duration::from_millis(50));
    
    // Initialize configuration
    write_register(spi, TX_MODE_REG, 0x00)?;    // Turn off CRC and use 106kbps
    write_register(spi, RX_MODE_REG, 0x00)?;    // Turn off CRC and use 106kbps
    write_register(spi, MODE_REG, 0x3D)?;       // Set defaults for CRC, polarity, etc.
    write_register(spi, T_MODE_REG, 0x80)?;     // Auto timer start after transmission
    write_register(spi, T_PRESCALER_REG, 0xA9)?; // Timer prescaler
    write_register(spi, T_RELOAD_REG_H, 0x03)?; // Timer reload value high byte
    write_register(spi, T_RELOAD_REG_L, 0xE8)?; // Timer reload value low byte
    write_register(spi, TX_CONTROL_REG, 0x83)?; // Set Tx1RFEn and Tx2RFEn (antenna on)
    
    // Key setting for your specific hardware - 100% ASK modulation
    write_register(spi, TX_ASK_REG, 0x40)?;
    
    Ok(())
}

// Wait for a card to be presented or timeout after 10 seconds
fn wait_for_card(spi: &mut Spi) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    let timeout = 100; // 100 * 100ms = 10 seconds
    
    for _ in 0..timeout {
        if let Some(uid) = read_card_uid(spi)? {
            return Ok(Some(uid));
        }
        thread::sleep(Duration::from_millis(100));
    }
    
    Ok(None)
}

// Wait for a card to be removed
fn wait_for_card_removal(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    println!("Waiting for card to be removed...");
    
    while read_card_uid(spi)?.is_some() {
        thread::sleep(Duration::from_millis(100));
    }
    
    println!("Card removed");
    Ok(())
}

// Read a card's UID
fn read_card_uid(spi: &mut Spi) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    // Step 1: Send WUPA command to detect a card
    if !request_card(spi)? {
        return Ok(None);
    }
    
    // Step 2: Perform anticollision to get card UID
    match get_card_uid(spi)? {
        Some(uid) => Ok(Some(uid)),
        None => Ok(None),
    }
}

// Send the WUPA command to detect cards
fn request_card(spi: &mut Spi) -> Result<bool, Box<dyn Error>> {
    // Clear interrupts
    write_register(spi, COM_IRQ_REG, 0x7F)?;
    
    // Prepare the WUPA command
    write_register(spi, FIFO_LEVEL_REG, 0x80)?; // Clear FIFO
    write_register(spi, FIFO_DATA_REG, PICC_WUPA)?; // WUPA command
    
    // Set bit framing for 7 bits (WUPA uses 7 bits)
    write_register(spi, BIT_FRAMING_REG, 0x07)?;
    
    // Start transceive command
    write_register(spi, COMMAND_REG, CMD_TRANSCEIVE)?;
    write_register(spi, BIT_FRAMING_REG, 0x87)?; // Start transmission (0x80) + 7 bits
    
    // Wait for completion or timeout
    let mut irq_value: u8 = 0;
    let timeout = 30; // Longer timeout for better card detection
    let mut counter = 0;
    
    while (irq_value & IRQ_RX) == 0 && counter < timeout {
        counter += 1;
        thread::sleep(Duration::from_millis(10));
        irq_value = read_register(spi, COM_IRQ_REG)?;
    }
    
    // Stop the transceive command
    write_register(spi, COMMAND_REG, CMD_IDLE)?;
    
    let error = read_register(spi, ERROR_REG)?;
    let fifo_level = read_register(spi, FIFO_LEVEL_REG)?;
    
    let success = (irq_value & IRQ_RX) != 0 && error == 0;
    
    if success {
        // Read the response (2-byte ATQA) but we don't use it for now
        for _ in 0..fifo_level {
            let _data = read_register(spi, FIFO_DATA_REG)?;
        }
    }
    
    Ok(success)
}

// Get the card's UID using anticollision procedure
fn get_card_uid(spi: &mut Spi) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    // Clear interrupts
    write_register(spi, COM_IRQ_REG, 0x7F)?;
    
    // Prepare for anticollision command
    write_register(spi, FIFO_LEVEL_REG, 0x80)?; // Clear FIFO
    write_register(spi, FIFO_DATA_REG, PICC_ANTICOLL)?; // Anticollision command
    write_register(spi, FIFO_DATA_REG, 0x20)?; // NVB = 0x20 (no data sent yet)
    
    // Configure for full bytes
    write_register(spi, BIT_FRAMING_REG, 0x00)?;
    
    // Start transceive command
    write_register(spi, COMMAND_REG, CMD_TRANSCEIVE)?;
    write_register(spi, BIT_FRAMING_REG, 0x80)?; // Start transmission
    
    // Wait for completion or timeout
    let mut irq_value: u8 = 0;
    let timeout = 30; // Timeout counter
    let mut counter = 0;
    
    while (irq_value & IRQ_RX) == 0 && counter < timeout {
        counter += 1;
        thread::sleep(Duration::from_millis(10));
        irq_value = read_register(spi, COM_IRQ_REG)?;
    }
    
    // Stop the transceive command
    write_register(spi, COMMAND_REG, CMD_IDLE)?;
    
    let error = read_register(spi, ERROR_REG)?;
    let fifo_level = read_register(spi, FIFO_LEVEL_REG)?;
    
    if (irq_value & IRQ_RX) != 0 && error == 0 && fifo_level >= 5 {
        // Read the UID (5 bytes: 4 UID bytes + 1 BCC)
        let mut uid = Vec::with_capacity(5);
        for _ in 0..fifo_level {
            uid.push(read_register(spi, FIFO_DATA_REG)?);
        }
        
        // We need at least 5 bytes (4 UID + BCC)
        if uid.len() >= 5 {
            // Verify BCC (checksum)
            let bcc = uid[4];
            let calc_bcc = uid[0] ^ uid[1] ^ uid[2] ^ uid[3];
            
            if bcc == calc_bcc {
                // Return UID without BCC
                Ok(Some(uid[0..4].to_vec()))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

// Select a card for further operations
fn select_card(spi: &mut Spi, uid: &[u8]) -> Result<bool, Box<dyn Error>> {
    // Clear interrupts
    write_register(spi, COM_IRQ_REG, 0x7F)?;
    
    // Prepare for select command
    write_register(spi, FIFO_LEVEL_REG, 0x80)?; // Clear FIFO
    
    // Build SELECT command
    write_register(spi, FIFO_DATA_REG, PICC_SELECT)?; // Select command
    write_register(spi, FIFO_DATA_REG, 0x70)?; // NVB = 0x70 (all UID bytes)
    
    // Send UID
    for &byte in uid {
        write_register(spi, FIFO_DATA_REG, byte)?;
    }
    
    // Send BCC
    let bcc = uid[0] ^ uid[1] ^ uid[2] ^ uid[3];
    write_register(spi, FIFO_DATA_REG, bcc)?;
    
    // Set bit framing for full bytes
    write_register(spi, BIT_FRAMING_REG, 0x00)?;
    
    // Start transceive command
    write_register(spi, COMMAND_REG, CMD_TRANSCEIVE)?;
    write_register(spi, BIT_FRAMING_REG, 0x80)?; // Start transmission
    
    // Wait for completion or timeout
    let mut irq_value: u8 = 0;
    let timeout = 30;
    let mut counter = 0;
    
    while (irq_value & IRQ_RX) == 0 && counter < timeout {
        counter += 1;
        thread::sleep(Duration::from_millis(10));
        irq_value = read_register(spi, COM_IRQ_REG)?;
    }
    
    // Stop the transceive command
    write_register(spi, COMMAND_REG, CMD_IDLE)?;
    
    let error = read_register(spi, ERROR_REG)?;
    let fifo_level = read_register(spi, FIFO_LEVEL_REG)?;
    
    let success = (irq_value & IRQ_RX) != 0 && error == 0 && fifo_level > 0;
    
    if success {
        // Read the SAK (Select Acknowledge)
let mut sak = Vec::with_capacity(fifo_level as usize);
        for _ in 0..fifo_level {
            sak.push(read_register(spi, FIFO_DATA_REG)?);
        }
    }
    
    Ok(success)
}

// Authenticate with a sector using Key A or Key B
fn authenticate(spi: &mut Spi, auth_cmd: u8, sector: u8, key: &[u8; 6], uid: &[u8]) -> Result<bool, Box<dyn Error>> {
    // Calculate block address for the sector trailer
    let block_addr = sector * 4 + 3;
    
    // Clear interrupts
    write_register(spi, COM_IRQ_REG, 0x7F)?;
    
    // Prepare for authentication
    write_register(spi, FIFO_LEVEL_REG, 0x80)?; // Clear FIFO
    
    // Build authentication command
    write_register(spi, FIFO_DATA_REG, auth_cmd)?; // Auth command (Key A or Key B)
    write_register(spi, FIFO_DATA_REG, block_addr)?; // Block address
    
    // Write key to FIFO
    for &k in key {
        write_register(spi, FIFO_DATA_REG, k)?;
    }
    
    // Write card UID to FIFO (first 4 bytes of UID)
    for &id in &uid[0..4] {
        write_register(spi, FIFO_DATA_REG, id)?;
    }
    
    // Start authentication command
    write_register(spi, COMMAND_REG, CMD_AUTH)?;
    
    // Wait for completion or timeout
    let mut irq_value: u8 = 0;
    let timeout = 30;
    let mut counter = 0;
    
    while (irq_value & IRQ_IDLE) == 0 && counter < timeout {
        counter += 1;
        thread::sleep(Duration::from_millis(10));
        irq_value = read_register(spi, COM_IRQ_REG)?;
    }
    
    // Check if authentication was successful
    let status2 = read_register(spi, STATUS2_REG)?;
    let success = (status2 & 0x08) != 0; // Check MFCrypto1On bit
    
    Ok(success)
}

// Read a block of data from the card
fn read_block(spi: &mut Spi, block: u8) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    // Clear interrupts
    write_register(spi, COM_IRQ_REG, 0x7F)?;
    
    // Prepare for read command
    write_register(spi, FIFO_LEVEL_REG, 0x80)?; // Clear FIFO
    
    // Build READ command
    write_register(spi, FIFO_DATA_REG, PICC_READ)?; // Read command
    write_register(spi, FIFO_DATA_REG, block)?; // Block address
    
    // Calculate CRC
    calculate_crc(spi, &[PICC_READ, block])?;
    
    // Get CRC values
    let crc_h = read_register(spi, FIFO_DATA_REG)?;
    let crc_l = read_register(spi, FIFO_DATA_REG)?;
    
    // Send the CRC
    write_register(spi, FIFO_DATA_REG, crc_l)?;
    write_register(spi, FIFO_DATA_REG, crc_h)?;
    
    // Start transceive command
    write_register(spi, COMMAND_REG, CMD_TRANSCEIVE)?;
    write_register(spi, BIT_FRAMING_REG, 0x80)?; // Start transmission
    
    // Wait for completion or timeout
    let mut irq_value: u8 = 0;
    let timeout = 30;
    let mut counter = 0;
    
    while (irq_value & IRQ_RX) == 0 && counter < timeout {
        counter += 1;
        thread::sleep(Duration::from_millis(10));
        irq_value = read_register(spi, COM_IRQ_REG)?;
    }
    
    // Stop the transceive command
    write_register(spi, COMMAND_REG, CMD_IDLE)?;
    
    let error = read_register(spi, ERROR_REG)?;
    let fifo_level = read_register(spi, FIFO_LEVEL_REG)?;
    
    let success = (irq_value & IRQ_RX) != 0 && error == 0 && fifo_level > 0;
    
    if success {
        // Read the data block (16 bytes)
        let mut data = Vec::with_capacity(fifo_level as usize);
        for _ in 0..fifo_level {
            data.push(read_register(spi, FIFO_DATA_REG)?);
        }
        
        Ok(Some(data))
    } else {
        Ok(None)
    }
}

// Write a block of data to the card
fn write_block(spi: &mut Spi, block: u8, data: &[u8]) -> Result<bool, Box<dyn Error>> {
    if data.len() != 16 {
        return Ok(false); // MIFARE Classic blocks are always 16 bytes
    }
    
    // Clear interrupts
    write_register(spi, COM_IRQ_REG, 0x7F)?;
    
    // Prepare for write command
    write_register(spi, FIFO_LEVEL_REG, 0x80)?; // Clear FIFO
    
    // Build WRITE command
    write_register(spi, FIFO_DATA_REG, PICC_WRITE)?; // Write command
    write_register(spi, FIFO_DATA_REG, block)?; // Block address
    
    // Calculate CRC
    calculate_crc(spi, &[PICC_WRITE, block])?;
    
    // Get CRC values
    let crc_h = read_register(spi, FIFO_DATA_REG)?;
    let crc_l = read_register(spi, FIFO_DATA_REG)?;
    
    // Send the CRC
    write_register(spi, FIFO_DATA_REG, crc_l)?;
    write_register(spi, FIFO_DATA_REG, crc_h)?;
    
    // Start transceive command
    write_register(spi, COMMAND_REG, CMD_TRANSCEIVE)?;
    write_register(spi, BIT_FRAMING_REG, 0x80)?; // Start transmission
    
    // Wait for completion or timeout
    let mut irq_value: u8 = 0;
    let timeout = 30;
    let mut counter = 0;
    
    while (irq_value & IRQ_RX) == 0 && counter < timeout {
        counter += 1;
        thread::sleep(Duration::from_millis(10));
        irq_value = read_register(spi, COM_IRQ_REG)?;
    }
    
    // Check if card acknowledged the write command
    let error = read_register(spi, ERROR_REG)?;
    let fifo_level = read_register(spi, FIFO_LEVEL_REG)?;
    
    if (irq_value & IRQ_RX) == 0 || error != 0 || fifo_level != 1 {
        return Ok(false);
    }
    
    // Read and validate the ACK
    let ack = read_register(spi, FIFO_DATA_REG)?;
    if (ack & 0x0F) != 0x0A {
        return Ok(false); // Not an ACK
    }
    
    // Clear FIFO for data
    write_register(spi, FIFO_LEVEL_REG, 0x80)?;
    
    // Write data to FIFO
    for &byte in data {
        write_register(spi, FIFO_DATA_REG, byte)?;
    }
    
    // Calculate CRC for the data
    calculate_crc(spi, data)?;
    
    // Get CRC values
    let data_crc_h = read_register(spi, FIFO_DATA_REG)?;
    let data_crc_l = read_register(spi, FIFO_DATA_REG)?;
    
    // Send data CRC
    write_register(spi, FIFO_DATA_REG, data_crc_l)?;
    write_register(spi, FIFO_DATA_REG, data_crc_h)?;
    
    // Start transceive command again for data
    write_register(spi, COMMAND_REG, CMD_TRANSCEIVE)?;
    write_register(spi, BIT_FRAMING_REG, 0x80)?; // Start transmission
    
    // Wait for completion or timeout
    irq_value = 0;
    counter = 0;
    
    while (irq_value & IRQ_RX) == 0 && counter < timeout {
        counter += 1;
        thread::sleep(Duration::from_millis(10));
        irq_value = read_register(spi, COM_IRQ_REG)?;
    }
    
    // Stop the transceive command
    write_register(spi, COMMAND_REG, CMD_IDLE)?;
    
    // Check if write was successful
    let error = read_register(spi, ERROR_REG)?;
    let fifo_level = read_register(spi, FIFO_LEVEL_REG)?;
    
    let success = (irq_value & IRQ_RX) != 0 && error == 0 && fifo_level > 0;
    
    if success {
        // Read ACK/NAK
        let response = read_register(spi, FIFO_DATA_REG)?;
        Ok((response & 0x0F) == 0x0A) // Check for ACK (0x0A)
    } else {
        Ok(false)
    }
}

// Calculate CRC using the onboard CRC coprocessor
fn calculate_crc(spi: &mut Spi, data: &[u8]) -> Result<(), Box<dyn Error>> {
    // Stop any command
    write_register(spi, COMMAND_REG, CMD_IDLE)?;
    
    // Clear CRC_IRQ flag
    write_register(spi, DIV_IRQ_REG, 0x04)?;
    
    // Clear FIFO
    write_register(spi, FIFO_LEVEL_REG, 0x80)?;
    
    // Write data to FIFO
    for &byte in data {
        write_register(spi, FIFO_DATA_REG, byte)?;
    }
    
    // Start CRC calculation
    write_register(spi, COMMAND_REG, CMD_CALC_CRC)?;
    
    // Wait for CRC calculation to complete
    let mut irq_value: u8 = 0;
    let timeout = 30;
    let mut counter = 0;
    
    while (irq_value & 0x04) == 0 && counter < timeout {
        counter += 1;
        thread::sleep(Duration::from_millis(10));
        irq_value = read_register(spi, DIV_IRQ_REG)?;
    }
    
    // Stop CRC command
    write_register(spi, COMMAND_REG, CMD_IDLE)?;
    
    Ok(())
}

// Dump all blocks of the card
fn dump_card(spi: &mut Spi, uid: &[u8]) -> Result<(), Box<dyn Error>> {
    println!("Attempting to dump card data...");
    println!("Note: This will try to read all sectors with the default key.");
    
    // Default keys to try
    let keys = [
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], // Factory default
        [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5], // Common alternative
        [0xD3, 0xF7, 0xD3, 0xF7, 0xD3, 0xF7], // Common alternative
        [0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // All zeros
    ];
    
    // Select the card first
    if !select_card(spi, uid)? {
        println!("Failed to select card for reading");
        return Ok(());
    }
    
    // MIFARE Classic 1K has 16 sectors with 4 blocks each
    for sector in 0..16 {
        println!("\nSector {}", sector);
        println!("------------------");
        
        // Try to authenticate with different keys
        let mut sector_read = false;
        
        for key_type in &[PICC_AUTHENT1A, PICC_AUTHENT1B] {
            let key_name = if *key_type == PICC_AUTHENT1A { "A" } else { "B" };
            
            for (key_idx, key) in keys.iter().enumerate() {
                // Skip already read sector
                if sector_read {
                    break;
                }
                
                // Reauthenticate for every key attempt
                if select_card(spi, uid)? && authenticate(spi, *key_type, sector as u8, key, uid)? {
                    println!("  Authenticated with Key {}{}", key_name, key_idx);
                    
                    // Read all blocks in the sector
                    let first_block = sector as u8 * 4;
                    
                    for block in first_block..first_block + 4 {
                        match read_block(spi, block)? {
                            Some(data) => {
                                // Display block data
                                println!("  Block {}: {}", block, bytes_to_hex(&data));
                                
                                // For non-sector trailer blocks, also show ASCII
                                if block % 4 != 3 {
                                    println!("          ASCII: {}", bytes_to_ascii(&data));
                                } else {
                                    // Sector trailer - display keys and access bits
                                    println!("          Key A: {}", bytes_to_hex(&data[0..6]));
                                    println!("          Access Bits: {}", bytes_to_hex(&data[6..10]));
                                    println!("          Key B: {}", bytes_to_hex(&data[10..16]));
                                }
                            },
                            None => println!("  Block {}: (Read failed)", block)
                        }
                    }
                    
                    sector_read = true;
                    break;
                }
            }
            
            if sector_read {
                break;
            }
        }
        
        if !sector_read {
            println!("  Could not authenticate with any key");
        }
    }
    
    Ok(())
}

// Read from MFRC522 register
fn read_register(spi: &mut Spi, reg: u8) -> Result<u8, Box<dyn Error>> {
    let command = (reg << 1) | 0x80; // Read command: bit 7 set, bit 0 clear
    let mut rx_buf = [0u8, 0u8];
    
    spi.transfer(&mut rx_buf, &[command, 0x00])?;
    
    Ok(rx_buf[1])
}

// Write to MFRC522 register
fn write_register(spi: &mut Spi, reg: u8, value: u8) -> Result<(), Box<dyn Error>> {
    let command = reg << 1; // Write command: bit 7 clear, bit 0 clear
    let mut rx_buf = [0u8, 0u8];
    
    spi.transfer(&mut rx_buf, &[command, value])?;
    
    Ok(())
}

// Format UID as a hex string
fn uid_to_string(uid: &[u8]) -> String {
    uid.iter()
        .map(|byte| format!("{:02X}", byte))
        .collect::<Vec<String>>()
        .join(":")
}

// Format bytes as a hex string
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|byte| format!("{:02X}", byte))
        .collect::<Vec<String>>()
        .join(" ")
}

// Convert bytes to ASCII string (replacing non-printable chars with dots)
fn bytes_to_ascii(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|&byte| {
            if byte >= 32 && byte <= 126 {
                byte as char
            } else {
                '.'
            }
        })
        .collect()
}
