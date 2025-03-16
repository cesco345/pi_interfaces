#!/bin/bash

# Path to your file
ORIGINAL_FILE="src/bin/mfrc522_enhanced.rs"
NEW_FILE="src/bin/mfrc522_clean.rs"

# Check if file exists
if [ ! -f "$ORIGINAL_FILE" ]; then
    echo "Error: File $ORIGINAL_FILE not found!"
    exit 1
fi

# Create a new file with the imports and constants
cat > "$NEW_FILE" << 'EOF'
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::{thread, time::Duration};
use std::error::Error;
use std::io::{self, Write};
use std::fmt;

// MFRC522 Commands
const PCD_IDLE: u8 = 0x00;
const PCD_AUTHENT: u8 = 0x0E;
const PCD_RECEIVE: u8 = 0x08;
const PCD_TRANSMIT: u8 = 0x04;
const PCD_TRANSCEIVE: u8 = 0x0C;
const PCD_RESETPHASE: u8 = 0x0F;
const PCD_CALCCRC: u8 = 0x03;

// MIFARE Commands
const PICC_REQIDL: u8 = 0x26;
const PICC_REQALL: u8 = 0x52;
const PICC_ANTICOLL: u8 = 0x93;
const PICC_SELECTTAG: u8 = 0x93;
const PICC_AUTHENT1A: u8 = 0x60;
const PICC_AUTHENT1B: u8 = 0x61;
const PICC_READ: u8 = 0x30;
const PICC_WRITE: u8 = 0xA0;
const PICC_DECREMENT: u8 = 0xC0;
const PICC_INCREMENT: u8 = 0xC1;
const PICC_RESTORE: u8 = 0xC2;
const PICC_TRANSFER: u8 = 0xB0;
const PICC_HALT: u8 = 0x50;

// Status codes
const MI_OK: u8 = 0;
const MI_NOTAGERR: u8 = 1;
const MI_ERR: u8 = 2;

// MFRC522 Registers
const COMMAND_REG: u8 = 0x01;
const COM_IEN_REG: u8 = 0x02;
const DIV_IEN_REG: u8 = 0x03;
const COM_IRQ_REG: u8 = 0x04;
const DIV_IRQ_REG: u8 = 0x05;
const ERROR_REG: u8 = 0x06;
const STATUS1_REG: u8 = 0x07;
const STATUS2_REG: u8 = 0x08;
const FIFO_DATA_REG: u8 = 0x09;
const FIFO_LEVEL_REG: u8 = 0x0A;
const WATER_LEVEL_REG: u8 = 0x0B;
const CONTROL_REG: u8 = 0x0C;
const BIT_FRAMING_REG: u8 = 0x0D;
const COLL_REG: u8 = 0x0E;

const MODE_REG: u8 = 0x11;
const TX_MODE_REG: u8 = 0x12;
const RX_MODE_REG: u8 = 0x13;
const TX_CONTROL_REG: u8 = 0x14;
const TX_AUTO_REG: u8 = 0x15;
const TX_SEL_REG: u8 = 0x16;
const RX_SEL_REG: u8 = 0x17;
const RX_THRESHOLD_REG: u8 = 0x18;
const DEMOD_REG: u8 = 0x19;
const MIFARE_REG: u8 = 0x1C;
const SERIAL_SPEED_REG: u8 = 0x1F;

const CRC_RESULT_REG_M: u8 = 0x21;
const CRC_RESULT_REG_L: u8 = 0x22;
const MOD_WIDTH_REG: u8 = 0x24;
const RF_CFG_REG: u8 = 0x26;
const GS_N_REG: u8 = 0x27;
const CW_GS_P_REG: u8 = 0x28;
const MOD_GS_P_REG: u8 = 0x29;
const T_MODE_REG: u8 = 0x2A;
const T_PRESCALER_REG: u8 = 0x2B;
const T_RELOAD_REG_H: u8 = 0x2C;
const T_RELOAD_REG_L: u8 = 0x2D;

const VERSION_REG: u8 = 0x37;

const MAX_LEN: usize = 16;

// Access bit configurations
struct AccessBits {
    c1: [bool; 4],  // Access conditions for C1 (least significant bit)
    c2: [bool; 4],  // Access conditions for C2
    c3: [bool; 4],  // Access conditions for C3 (most significant bit)
}

impl AccessBits {
    // Create access bits from raw bytes
    fn from_bytes(access_bytes: &[u8; 4]) -> Self {
        let mut c1 = [false; 4];
        let mut c2 = [false; 4];
        let mut c3 = [false; 4];
        
        // The access bits are in a weird order in the sector trailer
        // Byte 6 - b7 = C3b, b6 = C3a, b5 = C2b, b4 = C2a, b3 = C1b, b2 = C1a, b1 = C0b, b0 = C0a
        // Byte 7 - b7 = C1c, b6 = C1f, b5 = C1e, b4 = C1d, b3 = C0c, b2 = C0f, b1 = C0e, b0 = C0d
        // Byte 8 - b7 = C3c, b6 = C3f, b5 = C3e, b4 = C3d, b3 = C2c, b2 = C2f, b1 = C2e, b0 = C2d
        
        // Extract C1 bits
        c1[0] = (access_bytes[0] & 0b00000100) != 0; // C1a from byte 6 bit 2
        c1[1] = (access_bytes[0] & 0b00001000) != 0; // C1b from byte 6 bit 3
        c1[2] = (access_bytes[1] & 0b10000000) != 0; // C1c from byte 7 bit 7
        c1[3] = (access_bytes[1] & 0b01000000) != 0; // C1f from byte 7 bit 6
        
        // Extract C2 bits
        c2[0] = (access_bytes[0] & 0b00010000) != 0; // C2a from byte 6 bit 4
        c2[1] = (access_bytes[0] & 0b00100000) != 0; // C2b from byte 6 bit 5
        c2[2] = (access_bytes[2] & 0b00001000) != 0; // C2c from byte 8 bit 3
        c2[3] = (access_bytes[2] & 0b00000100) != 0; // C2f from byte 8 bit 2
        
        // Extract C3 bits
        c3[0] = (access_bytes[0] & 0b01000000) != 0; // C3a from byte 6 bit 6
        c3[1] = (access_bytes[0] & 0b10000000) != 0; // C3b from byte 6 bit 7
        c3[2] = (access_bytes[2] & 0b10000000) != 0; // C3c from byte 8 bit 7
        c3[3] = (access_bytes[2] & 0b01000000) != 0; // C3f from byte 8 bit 6
        
        Self {
            c1,
            c2,
            c3,
        }
    }
    
    // Convert access bits to raw bytes for writing to card
    fn to_bytes(&self) -> [u8; 4] {
        let mut access_bytes = [0u8; 4];
        
        // Byte 6
        if self.c1[0] { access_bytes[0] |= 0b00000100; } // C1a -> bit 2
        if self.c1[1] { access_bytes[0] |= 0b00001000; } // C1b -> bit 3
        if self.c2[0] { access_bytes[0] |= 0b00010000; } // C2a -> bit 4
        if self.c2[1] { access_bytes[0] |= 0b00100000; } // C2b -> bit 5
        if self.c3[0] { access_bytes[0] |= 0b01000000; } // C3a -> bit 6
        if self.c3[1] { access_bytes[0] |= 0b10000000; } // C3b -> bit 7
        
        // Byte 7
        if self.c1[2] { access_bytes[1] |= 0b10000000; } // C1c -> bit 7
        if self.c1[3] { access_bytes[1] |= 0b01000000; } // C1f -> bit 6
        
        // Byte 8
        if self.c2[2] { access_bytes[2] |= 0b00001000; } // C2c -> bit 3
        if self.c2[3] { access_bytes[2] |= 0b00000100; } // C2f -> bit 2
        if self.c3[2] { access_bytes[2] |= 0b10000000; } // C3c -> bit 7
        if self.c3[3] { access_bytes[2] |= 0b01000000; } // C3f -> bit 6
        
        // Byte 9 (usually 0x69 or some combination for user data byte)
        access_bytes[3] = 0x69;
        
        access_bytes
    }
    
    // Get a predefined access configuration
    fn get_predefined_config(config_type: &str) -> Self {
        match config_type {
            "transport" => {
                // Transport configuration - Everything accessible with Key A
                let c1 = [false, false, false, false];
                let c2 = [false, false, false, false];
                let c3 = [false, false, false, false];
                Self { c1, c2, c3 }
            },
            "secure" => {
                // Secure configuration - Data blocks read with Key A, write with Key B
                // Key A can only authenticate, Key B can read/write/auth
                let c1 = [false, false, true, false];
                let c2 = [false, true, false, false];
                let c3 = [true, true, false, true];
                Self { c1, c2, c3 }
            },
            "readonly" => {
                // Read-only configuration - No writes allowed
                let c1 = [false, true, true, false];
                let c2 = [false, false, false, true];
                let c3 = [true, true, false, true];
                Self { c1, c2, c3 }
            },
            _ => {
                // Default to transport configuration
                let c1 = [false, false, false, false];
                let c2 = [false, false, false, false];
                let c3 = [false, false, false, false];
                Self { c1, c2, c3 }
            }
        }
    }
    
    // Interpret the access conditions for a specific block
    fn interpret_access(&self, block_type: &str, block_index: usize) -> String {
        let index = match block_type {
            "data" => {
                if block_index >= 3 { return "Invalid block index".to_string(); }
                block_index
            },
            "trailer" => 3,
            _ => return "Invalid block type".to_string()
        };
        
        let c1 = self.c1[index];
        let c2 = self.c2[index];
        let c3 = self.c3[index];
        
        match block_type {
            "data" => {
                match (c1, c2, c3) {
                    (false, false, false) => "R/W: Key A|B".to_string(),
                    (false, false, true) => "R: Key A|B, W: Never".to_string(),
                    (true, false, false) => "R: Key A|B, W: Key B".to_string(),
                    (true, false, true) => "R: Key B, W: Key B".to_string(),
                    (false, true, false) => "R: Key A|B, W: Never".to_string(),
                    (false, true, true) => "R: Key B, W: Never".to_string(),
                    (true, true, false) => "R: Key A|B, W: Key B".to_string(),
                    (true, true, true) => "R: Never, W: Never".to_string(),
                }
            },
            "trailer" => {
                let key_a_access = match (c1, c2) {
                    (false, false) => "R: Never, W: Key A",
                    (false, true) => "R: Never, W: Never",
                    (true, false) => "R: Never, W: Key B",
                    (true, true) => "R: Never, W: Never",
                };
                
                let access_bits_access = match (c1, c3) {
                    (false, false) => "R: Key A|B, W: Key A",
                    (true, false) => "R: Key A|B, W: Never",
                    (false, true) => "R: Key A|B, W: Key B",
                    (true, true) => "R: Key A|B, W: Never",
                };
                
                let key_b_access = match (c2, c3) {
                    (false, false) => "R: Key A|B, W: Key A",
                    (true, false) => "R: Key A|B, W: Key B",
                    (false, true) => "R: Never, W: Key A",
                    (true, true) => "R: Never, W: Never",
                };
                
                format!("Key A: {}\nAccess Bits: {}\nKey B: {}", key_a_access, access_bits_access, key_b_access)
            },
            _ => "Invalid block type".to_string()
        }
    }
}

impl fmt::Display for AccessBits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Block 0: {}\n", self.interpret_access("data", 0))?;
        write!(f, "Block 1: {}\n", self.interpret_access("data", 1))?;
        write!(f, "Block 2: {}\n", self.interpret_access("data", 2))?;
        write!(f, "Block 3 (Trailer): \n{}", self.interpret_access("trailer", 0))
    }
}
EOF

# Add all the original functions from the first part
sed -n '/^fn main() -> Result<(), Box<dyn Error>> {/q;/^fn/,/^}/p' "$ORIGINAL_FILE" | grep -v "^fn main" >> "$NEW_FILE"

# Add the enhanced functions from the second part
sed -n '/^\/\/ Convert a hex string to bytes/,/^}/p' "$ORIGINAL_FILE" >> "$NEW_FILE"
sed -n '/^\/\/ Read all blocks in a sector/,/^}/p' "$ORIGINAL_FILE" >> "$NEW_FILE"
sed -n '/^\/\/ Write data to a specific block$/,/^}/p' "$ORIGINAL_FILE" >> "$NEW_FILE"
sed -n '/^\/\/ Write data to a specific block with a provided key/,/^}/p' "$ORIGINAL_FILE" >> "$NEW_FILE"
sed -n '/^\/\/ Modify access conditions for a sector$/,/^}/p' "$ORIGINAL_FILE" >> "$NEW_FILE"
sed -n '/^\/\/ Change keys for a sector/,/^}/p' "$ORIGINAL_FILE" >> "$NEW_FILE"
sed -n '/^\/\/ Format a card to factory defaults/,/^}/p' "$ORIGINAL_FILE" >> "$NEW_FILE"
sed -n '/^\/\/ Dump all card data .* with multiple keys/,/^}/p' "$ORIGINAL_FILE" >> "$NEW_FILE"

# Add the enhanced main function last
cat >> "$NEW_FILE" << 'EOF'
fn main() -> Result<(), Box<dyn Error>> {
    println!("=== MFRC522 RFID Reader/Writer (Enhanced Version) ===");
    println!("This implementation adds key and access management features");
    println!("Press Ctrl+C to exit\n");
    
    // Open SPI with the configuration we found works best
    let mut spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 1_000_000, Mode::Mode0)?;
    
    // Show version information
    let version = read_register(&mut spi, VERSION_REG)?;
    println!("MFRC522 Version: 0x{:02X}", version);
    
    // Initialize the MFRC522
    mfrc522_init(&mut spi)?;
    
    println!("MFRC522 initialized successfully");
    
    // Menu system
    loop {
        println!("\nSelect an option:");
        println!("1. Read card UID");
        println!("2. Read card data");
        println!("3. Write data to card");
        println!("4. Dump card contents");
        println!("5. Modify sector access");
        println!("6. Change keys");
        println!("7. Format card (reset to defaults)");
        println!("8. Write to specific block");
        println!("9. Exit");
        print!("Enter choice: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim();
        
        match choice {
            "1" => {
                println!("\n=== Reading Card UID ===");
                
                match wait_for_card(&mut spi, 15, read_card_uid)? {
                    Some(uid) => {
                        println!("UID: {}", uid_to_string(&uid));
                        println!("UID as decimal: {}", uid_to_num(&uid));
                        wait_for_card_removal(&mut spi)?;
                    },
                    None => println!("No card detected during the timeout period."),
                }
            },
            "2" => {
                println!("\n=== Reading Card Data ===");
                
                print!("Enter sector number (0-15): ");
                io::stdout().flush()?;
                let mut sector_input = String::new();
                io::stdin().read_line(&mut sector_input)?;
                let sector = match sector_input.trim().parse::<u8>() {
                    Ok(s) if s < 16 => s,
                    _ => {
                        println!("Invalid sector number. Using sector 1.");
                        1
                    }
                };
                
                match wait_for_card(&mut spi, 15, |spi| read_sector_data(spi, sector))? {
                    Some((uid, blocks)) => {
                        println!("UID: {}", uid_to_string(&uid));
                        println!("Data from sector {}:", sector);
                        
                        for (i, block_data) in blocks.iter().enumerate() {
                            let block_addr = sector * 4 + i as u8;
                            match block_data {
                                Some(data) => {
                                    println!("  Block {}: {}", block_addr, bytes_to_hex(data));
                                    
                                    // For non-sector trailer blocks, also show ASCII
                                    if i != 3 {
                                        println!("          ASCII: {}", bytes_to_ascii(data));
                                    } else {
                                        // Sector trailer - display keys and access bits
                                        println!("          Key A: {}", bytes_to_hex(&data[0..6]));
                                        println!("          Access Bits: {}", bytes_to_hex(&data[6..10]));
                                        println!("          Key B: {}", bytes_to_hex(&data[10..16]));
                                        
                                        // Interpret access bits
                                        let access_bytes = [data[6], data[7], data[8], data[9]];
                                        let access_bits = AccessBits::from_bytes(&access_bytes);
                                        println!("\n--- Access Conditions ---");
                                        println!("{}", access_bits);
                                    }
                                },
                                None => println!("  Block {}: (Read failed)", block_addr),
                            }
                        }
                        
                        wait_for_card_removal(&mut spi)?;
                    },
                    None => println!("No card detected during the timeout period."),
                }
            },
            "3" => {
                println!("\n=== Writing to Card ===");
                
                print!("Enter sector number (0-15): ");
                io::stdout().flush()?;
                let mut sector_input = String::new();
                io::stdin().read_line(&mut sector_input)?;
                let sector = match sector_input.trim().parse::<u8>() {
                    Ok(s) if s < 16 => s,
                    _ => {
                        println!("Invalid sector number. Using sector 1.");
                        1
                    }
                };
                
                print!("Enter block number within sector (0-2, not 3): ");
                io::stdout().flush()?;
                let mut block_input = String::new();
                io::stdin().read_line(&mut block_input)?;
                let block_offset = match block_input.trim().parse::<u8>() {
                    Ok(b) if b < 3 => b,
                    _ => {
                        println!("Invalid block number. Using block 0.");
                        0
                    }
                };
                
                println!("Enter the text to write to the block:");
                
                let mut text_input = String::new();
                io::stdin().read_line(&mut text_input)?;
                let text_to_write = text_input.trim();
                
                println!("\nText to write: \"{}\"", text_to_write);
                println!("Target: Sector {}, Block {}", sector, block_offset);
                
                // Countdown before trying to read card
                countdown_for_card_placement(10)?;
                
                let block_addr = sector * 4 + block_offset;
                let result = write_block_data(&mut spi, block_addr, text_to_write)?;
                
                match result {
                    Some((uid, text)) => {
                        println!("✅ Block write successful!");
                        println!("UID: {}", uid_to_string(&uid));
                        println!("Data written: \"{}\"", text);
                        wait_for_card_removal(&mut spi)?;
                    },
                    None => {
                        println!("❌ Write failed. The card may have been removed too soon or there might be permissions issues.");
                        println!("Try again with a fresh card or make sure you're using the correct keys.");
                    },
                }
            },
            "4" => {
                println!("\n=== Dumping Card Contents ===");
                println!("This will try to read all sectors with default keys.");
                
                match wait_for_card(&mut spi, 15, dump_card)? {
                    Some(uid) => {
                        println!("Card dump complete for UID: {}", uid_to_string(&uid));
                        wait_for_card_removal(&mut spi)?;
                    },
                    None => println!("No card detected during the timeout period."),
                }
            },
            "5" => {
                println!("\n=== Modify Sector Access Rights ===");
                
                print!("Enter sector number to modify (0-15): ");
                io::stdout().flush()?;
                let mut sector_input = String::new();
                io::stdin().read_line(&mut sector_input)?;
                let sector = match sector_input.trim().parse::<u8>() {
                    Ok(s) if s < 16 => s,
                    _ => {
                        println!("Invalid sector number. Using sector 1.");
                        1
                    }
                };
                
                println!("Select access configuration:");
                println!("1. Transport (Default, all open)");
                println!("2. Secure (Read with Key A, Write with Key B)");
                println!("3. Read-only (No writes allowed)");
                println!("4. Custom (Advanced users only)");
                
                print!("Enter choice: ");
                io::stdout().flush()?;
                let mut config_input = String::new();
                io::stdin().read_line(&mut config_input)?;
                
                let access_bits = match config_input.trim() {
                    "1" => AccessBits::get_predefined_config("transport"),
                    "2" => AccessBits::get_predefined_config("secure"),
                    "3" => AccessBits::get_predefined_config("readonly"),
                    "4" => {
                        println!("Custom configuration not implemented in this version.");
                        println!("Using transport configuration instead.");
                        AccessBits::get_predefined_config("transport")
                    },
                    _ => {
                        println!("Invalid choice. Using transport configuration.");
                        AccessBits::get_predefined_config("transport")
                    }
                };
                
                println!("\nSelected access configuration:");
                println!("{}", access_bits);
                
                // Countdown before trying to read card
                countdown_for_card_placement(10)?;
                
                let result = modify_sector_access(&mut spi, sector, &access_bits)?;
                
                match result {
                    true => {
                        println!("✅ Access conditions successfully changed!");
                        wait_for_card_removal(&mut spi)?;
                    },
                    false => {
                        println!("❌ Failed to change access conditions.");
                        println!("This may be due to incorrect keys or access restrictions.");
                    },
                }
            },
            "6" => {
                println!("\n=== Change Sector Keys ===");
                
                print!("Enter sector number (0-15): ");
                io::stdout().flush()?;
                let mut sector_input = String::new();
                io::stdin().read_line(&mut sector_input)?;
                let sector = match sector_input.trim().parse::<u8>() {
                    Ok(s) if s < 16 => s,
                    _ => {
                        println!("Invalid sector number. Using sector 1.");
                        1
                    }
                };
                
                println!("Which key do you want to change?");
                println!("1. Key A");
                println!("2. Key B");
                println!("3. Both keys");
                
                print!("Enter choice: ");
                io::stdout().flush()?;
                let mut key_choice_input = String::new();
                io::stdin().read_line(&mut key_choice_input)?;
                let key_choice = key_choice_input.trim();
                
                // Get current key to authenticate
                println!("Enter current Key A (6 bytes in hex, e.g. FFFFFFFFFFFF for default):");
                print!("Current Key A: ");
                io::stdout().flush()?;
                let mut current_key_input = String::new();
                io::stdin().read_line(&mut current_key_input)?;
                let current_key_str = current_key_input.trim();
                
                let current_key = match hex_string_to_bytes(current_key_str) {
                    Some(k) if k.len() == 6 => k,
                    _ => {
                        println!("Invalid key format. Using default key FFFFFFFFFFFF.");
                        vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
                    }
                };
                
                // Get new keys
                let mut new_key_a = Vec::new();
                let mut new_key_b = Vec::new();
                
                if key_choice == "1" || key_choice == "3" {
                    println!("Enter new Key A (6 bytes in hex, e.g. AABBCCDDEEFF):");
                    print!("New Key A: ");
                    io::stdout().flush()?;
                    let mut new_key_a_input = String::new();
                    io::stdin().read_line(&mut new_key_a_input)?;
                    let new_key_a_str = new_key_a_input.trim();
                    
                    new_key_a = match hex_string_to_bytes(new_key_a_str) {
                        Some(k) if k.len() == 6 => k,
                        _ => {
                            println!("Invalid key format. Using 000000000000.");
                            vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
                        }
                    };
                }
                
                if key_choice == "2" || key_choice == "3" {
                    println!("Enter new Key B (6 bytes in hex, e.g. AABBCCDDEEFF):");
                    print!("New Key B: ");
                    io::stdout().flush()?;
                    let mut new_key_b_input = String::new();
                    io::stdin().read_line(&mut new_key_b_input)?;
                    let new_key_b_str = new_key_b_input.trim();
                    
                    new_key_b = match hex_string_to_bytes(new_key_b_str) {
                        Some(k) if k.len() == 6 => k,
                        _ => {
                            println!("Invalid key format. Using FFFFFFFFFFFF.");
                            vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
                        }
                    };
                }
                
                // Countdown before trying to read card
                countdown_for_card_placement(10)?;
                
                // Execute key change
                let result = change_sector_keys(&mut spi, sector, &current_key, 
                                               key_choice == "1" || key_choice == "3", &new_key_a,
                                               key_choice == "2" || key_choice == "3", &new_key_b)?;
                
                match result {
                    true => {
                        println!("✅ Keys successfully changed!");
                        println!("⚠️ IMPORTANT: Keep your new keys safe! If lost, you will lose access to this sector.");
                        if key_choice == "1" || key_choice == "3" {
                            println!("New Key A: {}", bytes_to_hex(&new_key_a));
                        }
                        if key_choice == "2" || key_choice == "3" {
                            println!("New Key B: {}", bytes_to_hex(&new_key_b));
                        }
                        wait_for_card_removal(&mut spi)?;
                    },
                    false => {
                        println!("❌ Failed to change keys.");
                        println!("This may be due to incorrect current key or access restrictions.");
                    },
                }
            },
            "7" => {
                println!("\n=== Format Card (Reset to Defaults) ===");
                println!("⚠️ WARNING: This will reset ALL sectors to default transport configuration!");
                println!("⚠️ All data will be erased and keys will be set to factory defaults.");
                println!("⚠️ This operation can't be reversed!");
                
                print!("Are you sure? (y/n): ");
                io::stdout().flush()?;
                let mut confirm_input = String::new();
                io::stdin().read_line(&mut confirm_input)?;
                
                if confirm_input.trim().to_lowercase() != "y" {
                    println!("Format operation cancelled.");
                    continue;
                }
                
                // Countdown before trying to read card
                countdown_for_card_placement(10)?;
                
                let result = format_card(&mut spi)?;
                
                match result {
                    true => {
                        println!("✅ Card successfully formatted to factory defaults!");
                        wait_for_card_removal(&mut spi)?;
                    },
                    false => {
                        println!("❌ Format operation failed.");
                        println!("Some sectors may have been reset while others remain unchanged.");
                    },
                }
            },
            "8" => {
                println!("\n=== Write to Specific Block ===");
                
                print!("Enter block number (0-63): ");
                io::stdout().flush()?;
                let mut block_input = String::new();
                io::stdin().read_line(&mut block_input)?;
                let block = match block_input.trim().parse::<u8>() {
                    Ok(b) if b < 64 => b,
                    _ => {
                        println!("Invalid block number. Using block 4.");
                        4  // Block 0 is dangerous to write, use block 4 as default
                    }
                };
                
                let sector = block / 4;
                let is_trailer = block % 4 == 3;
                
                if is_trailer {
                    println!("⚠️ WARNING: You are about to write to a sector trailer (block {}).", block);
                    println!("⚠️ Incorrect data can permanently lock your card.");
                    print!("Are you sure? (y/n): ");
                    io::stdout().flush()?;
                    
                    let mut confirm_input = String::new();
                    io::stdin().read_line(&mut confirm_input)?;
                    
                    if confirm_input.trim().to_lowercase() != "y" {
                        println!("Operation cancelled.");
                        continue;
                    }
                }
                
                println!("Enter 16 bytes as hex string (e.g. 00112233445566778899AABBCCDDEEFF):");
                print!("Data: ");
                io::stdout().flush()?;
                
                let mut data_input = String::new();
                io::stdin().read_line(&mut data_input)?;
                let data_str = data_input.trim();
                
                let data = match hex_string_to_bytes(data_str) {
                    Some(d) if d.len() == 16 => d,
                    _ => {
                        println!("Invalid data format. Using zeroes.");
                        vec![0; 16]
                    }
                };
                
                println!("Enter key for authenticating sector {} (6 bytes in hex, e.g. FFFFFFFFFFFF for default):", sector);
                print!("Key: ");
                io::stdout().flush()?;
                
                let mut key_input = String::new();
                io::stdin().read_line(&mut key_input)?;
                let key_str = key_input.trim();
                
                let key = match hex_string_to_bytes(key_str) {
                    Some(k) if k.len() == 6 => k,
                    _ => {
                        println!("Invalid key format. Using default key FFFFFFFFFFFF.");
                        vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
                    }
                };
                
                println!("\nTarget: Block {} (Sector {})", block, sector);
                println!("Data to write: {}", bytes_to_hex(&data));
                
                // Countdown before trying to read card
                countdown_for_card_placement(10)?;
                
                let result = write_block_raw(&mut spi, block, &key, &data)?;
                
                match result {
                    true => {
                        println!("✅ Block write successful!");
                        wait_for_card_removal(&mut spi)?;
                    },
                    false => {
                        println!("❌ Write failed. The card may have been removed too soon or there might be permissions issues.");
                        println!("Try again with a fresh card or make sure you're using the correct keys.");
                    },
                }
            },
            "9" => {
                println!("\nExiting program. Goodbye!");
                break;
            },
            _ => println!("Invalid choice, please try again")
        }
    }
    
    Ok(())
}
EOF

echo "Fixed file created at $NEW_FILE"
echo "Now update your Cargo.toml with:"
echo "[[bin]]"
echo "name = \"rfid\""
echo "path = \"src/bin/mfrc522_clean.rs\""
echo
echo "Then compile with: cargo build --bin rfid"
