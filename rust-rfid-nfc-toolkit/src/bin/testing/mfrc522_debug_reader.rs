use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::{thread, time::Duration};
use std::error::Error;

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
const VERSION_REG: u8 = 0x37;

// MFRC522 Command Set
const CMD_IDLE: u8 = 0x00;
const CMD_TRANSCEIVE: u8 = 0x0C;

// MIFARE Commands
const PICC_REQA: u8 = 0x26;
const PICC_WUPA: u8 = 0x52;
const PICC_ANTICOLL: u8 = 0x93;

// Status flags
const IRQ_RX: u8 = 0x20;
const IRQ_IDLE: u8 = 0x10;
const IRQ_TIMER: u8 = 0x01;

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== MFRC522 Debug Card Reader ===");
    println!("This program will show detailed debug information");
    println!("Hold a card near the RFID module to read its ID");
    println!("Press Ctrl+C to exit\n");
    
    // Open SPI with the configuration we found works best
    let mut spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 100_000, Mode::Mode0)?;
    
    // Show version information
    let version = read_register(&mut spi, VERSION_REG)?;
    println!("MFRC522 Version: 0x{:02X}", version);
    
    // Initialize the MFRC522
    initialize_mfrc522(&mut spi)?;
    
    println!("MFRC522 initialized successfully");
    println!("Starting debug card scan...\n");
    
    // Main loop - look for cards continuously with detailed logging
    let mut scan_count = 0;
    loop {
        scan_count += 1;
        println!("Scan #{} - Testing for cards...", scan_count);
        
        // Read status registers
        let status1 = read_register(&mut spi, STATUS1_REG)?;
        let status2 = read_register(&mut spi, STATUS2_REG)?;
        println!("Status registers: STATUS1=0x{:02X}, STATUS2=0x{:02X}", status1, status2);
        
        // Try different card detection methods
        println!("\nMethod 1: Standard REQA");
        if let Some(uid) = try_read_card_uid(&mut spi, PICC_REQA, true)? {
            println!("🎉 SUCCESS! Card detected with UID: {}", uid_to_string(&uid));
            println!("Waiting for card to be removed...");
            
            // Wait for card to be removed (simple delay)
            thread::sleep(Duration::from_secs(3));
            println!("Ready for next card.\n");
        } else {
            println!("No card detected with REQA.\n");
        }
        
        println!("Method 2: Wake-Up (WUPA)");
        if let Some(uid) = try_read_card_uid(&mut spi, PICC_WUPA, true)? {
            println!("🎉 SUCCESS! Card detected with UID: {}", uid_to_string(&uid));
            println!("Waiting for card to be removed...");
            
            // Wait for card to be removed (simple delay)
            thread::sleep(Duration::from_secs(3));
            println!("Ready for next card.\n");
        } else {
            println!("No card detected with WUPA.\n");
        }
        
        // Try to detect card presence without getting UID - simpler test
        println!("Method 3: Simple card presence check");
        if detect_card_presence(&mut spi)? {
            println!("🎉 Card presence detected but couldn't read UID!");
            thread::sleep(Duration::from_secs(1));
        } else {
            println!("No card presence detected.\n");
        }
        
        // Let's also try lowering the detection threshold
        println!("Method 4: Low power detection");
        write_register(&mut spi, TX_ASK_REG, 0x40)?; // Ensure 100% ASK
        let result = try_read_card_uid(&mut spi, PICC_WUPA, false)?;
        write_register(&mut spi, TX_ASK_REG, 0x00)?; // Reset ASK
        
        if let Some(uid) = result {
            println!("🎉 SUCCESS! Card detected with low power setting. UID: {}", uid_to_string(&uid));
            thread::sleep(Duration::from_secs(3));
            println!("Ready for next card.\n");
        } else {
            println!("No card detected with low power setting.\n");
        }
        
        // Pause between scan cycles
        thread::sleep(Duration::from_millis(500));
        println!("--------------------------------------------------");
    }
}

// Initialize the MFRC522
fn initialize_mfrc522(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    println!("Initializing MFRC522...");
    
    // Soft reset
    println!("- Sending soft reset");
    write_register(spi, COMMAND_REG, 0x0F)?;
    thread::sleep(Duration::from_millis(50));
    
    // Read command register after reset
    let cmd = read_register(spi, COMMAND_REG)?;
    println!("- Command register after reset: 0x{:02X}", cmd);
    
    // Initialize configuration
    println!("- Setting up TX mode");
    write_register(spi, TX_MODE_REG, 0x00)?;    // Turn off CRC and use 106kbps
    
    println!("- Setting up RX mode");
    write_register(spi, RX_MODE_REG, 0x00)?;    // Turn off CRC and use 106kbps
    
    println!("- Setting up MODE register");
    write_register(spi, MODE_REG, 0x3D)?;       // Set defaults for CRC, polarity, etc.
    
    println!("- Setting CONTROL register");
    write_register(spi, CONTROL_REG, 0x10)?;    // Stop timer
    
    println!("- Turning on antenna");
    write_register(spi, TX_CONTROL_REG, 0x83)?; // Set Tx1RFEn and Tx2RFEn (antenna on)
    
    // Verify antenna is on
    let tx_control = read_register(spi, TX_CONTROL_REG)?;
    println!("- TX_CONTROL register: 0x{:02X} (should have 0x03 set)", tx_control);
    
    if (tx_control & 0x03) == 0x03 {
        println!("- Antenna is ON");
    } else {
        println!("- WARNING: Antenna might not be ON properly!");
    }
    
    println!("Initialization complete");
    
    Ok(())
}

// Simplified function to detect card presence without getting UID
fn detect_card_presence(spi: &mut Spi) -> Result<bool, Box<dyn Error>> {
    // Clear interrupts
    write_register(spi, COM_IRQ_REG, 0x7F)?;
    
    // Prepare the REQA command
    write_register(spi, FIFO_LEVEL_REG, 0x80)?; // Clear FIFO
    write_register(spi, FIFO_DATA_REG, PICC_REQA)?; // REQA command
    
    // Set bit framing for 7 bits (REQA uses 7 bits)
    write_register(spi, BIT_FRAMING_REG, 0x07)?;
    
    // Start transceive command
    write_register(spi, COMMAND_REG, CMD_TRANSCEIVE)?;
    write_register(spi, BIT_FRAMING_REG, 0x87)?; // Start transmission (0x80) + 7 bits
    
    // Wait for completion or timeout (short timeout)
    let mut irq_value: u8 = 0;
    let timeout = 10; // Reduced timeout for quicker checks
    let mut counter = 0;
    
    while (irq_value & (IRQ_RX | IRQ_IDLE | IRQ_TIMER)) == 0 && counter < timeout {
        counter += 1;
        thread::sleep(Duration::from_millis(5)); // Shorter sleep
        irq_value = read_register(spi, COM_IRQ_REG)?;
    }
    
    // Stop the transceive command
    write_register(spi, COMMAND_REG, CMD_IDLE)?;
    
    // Just check if we received something
    let success = (irq_value & IRQ_RX) != 0;
    
    let error = read_register(spi, ERROR_REG)?;
    let fifo_level = read_register(spi, FIFO_LEVEL_REG)?;
    
    println!("Card presence check results:");
    println!("- IRQ value: 0x{:02X}", irq_value);
    println!("- Error register: 0x{:02X}", error);
    println!("- FIFO level: {}", fifo_level);
    
    Ok(success)
}

// Try to read a card's UID with detailed logging
fn try_read_card_uid(spi: &mut Spi, command: u8, verbose: bool) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    // Step 1: Send request command to detect a card
    if !request_card(spi, command, verbose)? {
        if verbose {
            println!("No card detected during request phase");
        }
        return Ok(None);
    }
    
    if verbose {
        println!("Card detected! Attempting to read UID...");
    }
    
    // Step 2: Perform anticollision to get card UID
    match get_card_uid(spi, verbose)? {
        Some(uid) => {
            if verbose {
                println!("Successfully read card UID");
            }
            Ok(Some(uid))
        },
        None => {
            if verbose {
                println!("Failed to read card UID during anticollision");
            }
            Ok(None)
        }
    }
}

// Send the Request command (REQA/WUPA) to detect cards with detailed logging
fn request_card(spi: &mut Spi, command: u8, verbose: bool) -> Result<bool, Box<dyn Error>> {
    if verbose {
        println!("Sending card request command 0x{:02X}...", command);
    }
    
    // Clear interrupts
    write_register(spi, COM_IRQ_REG, 0x7F)?;
    
    // Prepare the request command
    write_register(spi, FIFO_LEVEL_REG, 0x80)?; // Clear FIFO
    write_register(spi, FIFO_DATA_REG, command)?; // REQA or WUPA command
    
    // Set bit framing for 7 bits (REQA/WUPA uses 7 bits)
    write_register(spi, BIT_FRAMING_REG, 0x07)?;
    
    // Start transceive command
    write_register(spi, COMMAND_REG, CMD_TRANSCEIVE)?;
    write_register(spi, BIT_FRAMING_REG, 0x87)?; // Start transmission (0x80) + 7 bits
    
    // Wait for completion or timeout
    let mut irq_value: u8 = 0;
    let timeout = 20; // Timeout counter
    let mut counter = 0;
    
    while (irq_value & (IRQ_RX | IRQ_IDLE | IRQ_TIMER)) == 0 && counter < timeout {
        counter += 1;
        thread::sleep(Duration::from_millis(10));
        irq_value = read_register(spi, COM_IRQ_REG)?;
    }
    
    // Stop the transceive command
    write_register(spi, COMMAND_REG, CMD_IDLE)?;
    
    let error = read_register(spi, ERROR_REG)?;
    let fifo_level = read_register(spi, FIFO_LEVEL_REG)?;
    
    if verbose {
        println!("Request command results:");
        println!("- IRQ value: 0x{:02X}", irq_value);
        println!("- Error register: 0x{:02X}", error);
        println!("- FIFO level: {}", fifo_level);
        
        if (irq_value & IRQ_TIMER) != 0 {
            println!("- Timeout occurred");
        }
        if (irq_value & IRQ_RX) != 0 {
            println!("- Data received");
        }
    }
    
    let success = (irq_value & IRQ_RX) != 0 && error == 0 && fifo_level > 0;
    
    if success && verbose {
        // Read and display the response data
        let mut response = Vec::new();
        for _ in 0..fifo_level {
            response.push(read_register(spi, FIFO_DATA_REG)?);
        }
        println!("- Response data: {}", bytes_to_hex(&response));
    }
    
    Ok(success)
}

// Get the card's UID using anticollision procedure with detailed logging
fn get_card_uid(spi: &mut Spi, verbose: bool) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    if verbose {
        println!("Starting anticollision procedure to get UID...");
    }
    
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
    
    while (irq_value & (IRQ_RX | IRQ_IDLE | IRQ_TIMER)) == 0 && counter < timeout {
        counter += 1;
        thread::sleep(Duration::from_millis(10));
        irq_value = read_register(spi, COM_IRQ_REG)?;
    }
    
    // Stop the transceive command
    write_register(spi, COMMAND_REG, CMD_IDLE)?;
    
    let error = read_register(spi, ERROR_REG)?;
    let fifo_level = read_register(spi, FIFO_LEVEL_REG)?;
    
    if verbose {
        println!("Anticollision command results:");
        println!("- IRQ value: 0x{:02X}", irq_value);
        println!("- Error register: 0x{:02X}", error);
        println!("- FIFO level: {}", fifo_level);
    }
    
    if (irq_value & IRQ_RX) != 0 && error == 0 && fifo_level >= 5 {
        // Read the UID (5 bytes: 4 UID bytes + 1 BCC)
        let mut uid = Vec::with_capacity(5);
        for _ in 0..fifo_level {
            uid.push(read_register(spi, FIFO_DATA_REG)?);
        }
        
        if verbose {
            println!("- Raw data read: {}", bytes_to_hex(&uid));
        }
        
        // Check if we got at least 5 bytes (4 UID + BCC)
        if uid.len() >= 5 {
            // Verify BCC (checksum)
            let bcc = uid[4];
            let calc_bcc = uid[0] ^ uid[1] ^ uid[2] ^ uid[3];
            
            if verbose {
                println!("- BCC received: 0x{:02X}, calculated: 0x{:02X}", bcc, calc_bcc);
            }
            
            if bcc == calc_bcc {
                // Return UID without BCC
                if verbose {
                    println!("- BCC check passed, valid UID");
                }
                Ok(Some(uid[0..4].to_vec()))
            } else {
                if verbose {
                    println!("- BCC check failed, invalid UID");
                }
                Ok(None)
            }
        } else {
            if verbose {
                println!("- Not enough data received for a complete UID");
            }
            Ok(None)
        }
    } else {
        if verbose {
            println!("- Failed to receive valid data from anticollision");
        }
        Ok(None)
    }
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
