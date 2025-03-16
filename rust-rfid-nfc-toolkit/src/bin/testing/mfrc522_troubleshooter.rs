use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::{thread, time::Duration};
use std::error::Error;
use std::io::{self, Write};

// MFRC522 Register Addresses
const VERSION_REG: u8 = 0x37;
const COMMAND_REG: u8 = 0x01;
const STATUS1_REG: u8 = 0x07;
const TX_CONTROL_REG: u8 = 0x14;
const RF_CFG_REG: u8 = 0x26;
const FIFO_DATA_REG: u8 = 0x09;
const FIFO_LEVEL_REG: u8 = 0x0A;

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== MFRC522 Connection Troubleshooter ===");
    println!("This tool attempts to communicate with an MFRC522 RFID module using various SPI configurations");
    println!("---------------------------------------");
    
    // Ask which SPI bus to try
    print!("Which SPI bus would you like to test? (0, 1, or 'b' for both) [b]: ");
    io::stdout().flush()?;
    let mut input_string = String::new();
    io::stdin().read_line(&mut input_string)?;
    let input_str = input_string.trim();
    
    let test_spi0 = input_str.is_empty() || input_str == "b" || input_str == "0";
    let test_spi1 = input_str.is_empty() || input_str == "b" || input_str == "1";
    
    // Ask which chip select to try
    print!("Which chip select would you like to test? (0, 1, 2, or 'a' for all) [a]: ");
    io::stdout().flush()?;
    input_string.clear();
    io::stdin().read_line(&mut input_string)?;
    let input_str = input_string.trim();
    
    let mut cs_pins = Vec::new();
    if input_str.is_empty() || input_str == "a" || input_str == "0" { cs_pins.push(SlaveSelect::Ss0); }
    if input_str.is_empty() || input_str == "a" || input_str == "1" { cs_pins.push(SlaveSelect::Ss1); }
    if input_str.is_empty() || input_str == "a" || input_str == "2" { cs_pins.push(SlaveSelect::Ss2); }
    
    // Test speeds
    let speeds = [100_000, 1_000_000, 4_000_000];
    
    // Test all modes
    let modes = [Mode::Mode0, Mode::Mode1, Mode::Mode2, Mode::Mode3];
    
    // Track best configuration
    let mut best_config = None;
    let mut best_version = 0;
    
    println!("\nStarting comprehensive scan (this may take a moment)...");
    
    // Try each combination
    for &bus_id in &[Bus::Spi0, Bus::Spi1] {
        if (bus_id == Bus::Spi0 && !test_spi0) || (bus_id == Bus::Spi1 && !test_spi1) {
            continue;
        }
        
        let bus_name = if bus_id == Bus::Spi0 { "SPI0" } else { "SPI1" };
        
        for &cs in &cs_pins {
            let cs_name = match cs {
                SlaveSelect::Ss0 => "CS0",
                SlaveSelect::Ss1 => "CS1",
                SlaveSelect::Ss2 => "CS2",
                _ => "CS?",
            };
            
            for &speed in &speeds {
                for &mode in &modes {
                    print!("Testing {}/{}/{}/{:?}... ", bus_name, cs_name, speed, mode);
                    io::stdout().flush()?;
                    
                    // Try to open the SPI device
                    match Spi::new(bus_id, cs, speed, mode) {
                        Ok(mut spi) => {
                            // Test for MFRC522 by reading version register
                            let (status, version) = read_register(&mut spi, VERSION_REG);
                            
                            if status {
                                if version == 0x91 || version == 0x92 {
                                    println!("SUCCESS! MFRC522 detected (version: 0x{:02X})", version);
                                    
                                    // This is the correct device with proper version
                                    if version > best_version {
                                        best_config = Some((bus_id, cs, speed, mode));
                                        best_version = version;
                                    }
                                    
                                    // No need to test other configs for this bus/cs
                                    break;
                                } else {
                                    println!("Device found, but unexpected version: 0x{:02X}", version);
                                    
                                    // If we haven't found any device yet, keep this as potential candidate
                                    if best_version == 0 && version > 0 {
                                        best_config = Some((bus_id, cs, speed, mode));
                                        best_version = version;
                                    }
                                }
                            } else {
                                println!("Device found, but communication failed");
                            }
                        },
                        Err(e) => {
                            println!("FAILED ({})", e);
                        }
                    }
                }
            }
        }
    }
    
    if let Some((bus_id, cs, speed, mode)) = best_config {
        let bus_name = if bus_id == Bus::Spi0 { "SPI0" } else { "SPI1" };
        let cs_name = match cs {
            SlaveSelect::Ss0 => "CS0",
            SlaveSelect::Ss1 => "CS1",
            SlaveSelect::Ss2 => "CS2",
            _ => "CS?",
        };
        
        println!("\n=== Best configuration found ===");
        println!("Bus: {}", bus_name);
        println!("Chip Select: {}", cs_name);
        println!("Speed: {} Hz", speed);
        println!("Mode: {:?}", mode);
        println!("Version: 0x{:02X}", best_version);
        
        // Ask if user wants to run detailed tests with this configuration
        print!("\nWould you like to run detailed tests with this configuration? (y/n) [y]: ");
        io::stdout().flush()?;
        
        input_string.clear();
        io::stdin().read_line(&mut input_string)?;
        let answer = input_string.trim().to_lowercase();
        
        if answer.is_empty() || answer.starts_with('y') {
            run_detailed_tests(bus_id, cs, speed, mode)?;
        }
    } else {
        println!("\nNo MFRC522 device was detected on any SPI bus.");
        println!("Please check your wiring and try again.");
        println!("Make sure that:");
        println!("1. The MFRC522 module is properly connected to the Raspberry Pi");
        println!("2. SPI is enabled in raspi-config");
        println!("3. The correct SPI pins are being used for MOSI, MISO, SCLK, and CS");
    }
    
    Ok(())
}

fn read_register(spi: &mut Spi, reg: u8) -> (bool, u8) {
    let command = (reg << 1) | 0x80; // Read command: bit 7 set, bit 0 clear
    let mut rx_buf = [0u8, 0u8];
    
    match spi.transfer(&mut rx_buf, &[command, 0x00]) {
        Ok(_) => (true, rx_buf[1]),
        Err(_) => (false, 0),
    }
}

fn write_register(spi: &mut Spi, reg: u8, value: u8) -> bool {
    let command = reg << 1; // Write command: bit 7 clear, bit 0 clear
    let mut rx_buf = [0u8, 0u8];
    
    match spi.transfer(&mut rx_buf, &[command, value]) {
        Ok(_) => true,
        Err(_) => false,
    }
}

fn run_detailed_tests(bus: Bus, cs: SlaveSelect, speed: u32, mode: Mode) -> Result<(), Box<dyn Error>> {
    println!("\n=== Running detailed MFRC522 tests ===");
    
    // Open the SPI device with the best configuration
    let mut spi = Spi::new(bus, cs, speed, mode)?;
    
    // Step 1: Read version register again to confirm
    println!("\n[1. Version Check]");
    let (status, version) = read_register(&mut spi, VERSION_REG);
    if status {
        println!("Version register: 0x{:02X}", version);
        if version == 0x91 || version == 0x92 {
            println!("PASS - MFRC522 detected!");
        } else {
            println!("WARNING - Unexpected version value");
        }
    } else {
        println!("FAIL - Could not read version register");
    }
    
    // Step 2: Reset the device
    println!("\n[2. Device Reset]");
    if write_register(&mut spi, COMMAND_REG, 0x0F) {
        println!("Soft reset command sent");
        thread::sleep(Duration::from_millis(50)); // Give it time to reset
        
        // Check command register after reset
        let (status, cmd_reg) = read_register(&mut spi, COMMAND_REG);
        if status {
            println!("Command register after reset: 0x{:02X}", cmd_reg);
            if cmd_reg == 0x00 {
                println!("PASS - Device reset successful");
            } else {
                println!("WARNING - Command register not cleared after reset");
            }
        } else {
            println!("FAIL - Could not read command register after reset");
        }
    } else {
        println!("FAIL - Could not send reset command");
    }
    
    // Step 3: Turn on the antenna
    println!("\n[3. Antenna Control]");
    if write_register(&mut spi, TX_CONTROL_REG, 0x03) {
        println!("Antenna enabled (Tx1RFEn and Tx2RFEn bits set)");
        
        // Read back the register to confirm
        let (status, tx_ctrl) = read_register(&mut spi, TX_CONTROL_REG);
        if status {
            println!("TX Control register: 0x{:02X}", tx_ctrl);
            if (tx_ctrl & 0x03) == 0x03 {
                println!("PASS - Antenna enabled successfully");
            } else {
                println!("WARNING - Antenna not enabled properly");
            }
        } else {
            println!("FAIL - Could not read TX Control register");
        }
    } else {
        println!("FAIL - Could not write to TX Control register");
    }
    
    // Step 4: Set RF gain
    println!("\n[4. RF Configuration]");
    if write_register(&mut spi, RF_CFG_REG, 0x70) {
        println!("RF gain set to maximum (0x70)");
        
        // Read back the register to confirm
        let (status, rf_cfg) = read_register(&mut spi, RF_CFG_REG);
        if status {
            println!("RF Config register: 0x{:02X}", rf_cfg);
            if (rf_cfg & 0x70) == 0x70 {
                println!("PASS - RF gain set successfully");
            } else {
                println!("WARNING - RF gain not set properly");
            }
        } else {
            println!("FAIL - Could not read RF Config register");
        }
    } else {
        println!("FAIL - Could not write to RF Config register");
    }
    
    // Step 5: Test FIFO buffer
    println!("\n[5. FIFO Test]");
    // First, clear the FIFO buffer
    if write_register(&mut spi, FIFO_LEVEL_REG, 0x80) {
        println!("FIFO buffer cleared");
        
        // Write a test byte to the FIFO
        if write_register(&mut spi, FIFO_DATA_REG, 0xA5) {
            println!("Test byte (0xA5) written to FIFO");
            
            // Read back the FIFO level
            let (status, fifo_level) = read_register(&mut spi, FIFO_LEVEL_REG);
            if status {
                println!("FIFO Level register: 0x{:02X}", fifo_level);
                if fifo_level > 0 {
                    println!("PASS - FIFO contains data");
                    
                    // Read the byte from FIFO
                    let (status, fifo_data) = read_register(&mut spi, FIFO_DATA_REG);
                    if status {
                        println!("FIFO Data read: 0x{:02X}", fifo_data);
                        if fifo_data == 0xA5 {
                            println!("PASS - FIFO data matches written value");
                        } else {
                            println!("WARNING - FIFO data does not match");
                        }
                    } else {
                        println!("FAIL - Could not read FIFO data");
                    }
                } else {
                    println!("FAIL - FIFO is empty");
                }
            } else {
                println!("FAIL - Could not read FIFO level");
            }
        } else {
            println!("FAIL - Could not write to FIFO");
        }
    } else {
        println!("FAIL - Could not clear FIFO buffer");
    }
    
    // Final summary
    println!("\n=== Final Summary ===");
    println!("The MFRC522 module appears to be:");
    
    if version == 0x91 || version == 0x92 {
        println!("✅ PROPERLY DETECTED AND WORKING");
        println!("\nRecommended configuration for your code:");
        println!("Bus: {}", if bus == Bus::Spi0 { "SPI0" } else { "SPI1" });
        println!("Chip Select: {}", match cs {
            SlaveSelect::Ss0 => "CS0",
            SlaveSelect::Ss1 => "CS1",
            SlaveSelect::Ss2 => "CS2",
            _ => "CS?",
        });
        println!("Speed: {} Hz", speed);
        println!("Mode: {:?}", mode);
    } else if version > 0 {
        println!("⚠️ DETECTED BUT WITH UNEXPECTED VERSION (0x{:02X})", version);
        println!("It may be a different revision or a compatible device");
    } else {
        println!("❌ NOT PROPERLY DETECTED");
        println!("Please double-check your wiring and connections");
    }
    
    Ok(())
}
