use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::{thread, time::Duration};
use std::error::Error;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn Error>> {
    println!("Enhanced SPI Loopback Test");
    println!("==========================");
    println!("This test requires a direct connection between MOSI and MISO pins");
    println!("For Raspberry Pi SPI0: Connect GPIO 10 (MOSI) to GPIO 9 (MISO)");
    println!("For Raspberry Pi SPI1: Connect GPIO 20 (MOSI) to GPIO 19 (MISO)");
    println!("\nIMPORTANT: Disconnect any SPI devices (like MFRC522) before testing!");
    
    // Ask which SPI bus to use
    println!("\nWhich SPI bus would you like to test?");
    println!("0: SPI0 (default)");
    println!("1: SPI1");
    print!("Selection [0]: ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let bus = match input.trim() {
        "1" => Bus::Spi1,
        _ => Bus::Spi0,
    };
    
    // Ask which chip select to use
    println!("\nWhich chip select would you like to use?");
    println!("0: CS0 (default)");
    println!("1: CS1");
    println!("2: CS2");
    print!("Selection [0]: ");
    io::stdout().flush()?;
    
    input.clear();
    io::stdin().read_line(&mut input)?;
    let cs = match input.trim() {
        "1" => SlaveSelect::Ss1,
        "2" => SlaveSelect::Ss2,
        _ => SlaveSelect::Ss0,
    };
    
    // Ask for speed
    println!("\nWhat clock speed would you like to use (Hz)?");
    println!("Recommended: 100000 for reliable testing");
    print!("Speed [100000]: ");
    io::stdout().flush()?;
    
    input.clear();
    io::stdin().read_line(&mut input)?;
    let speed = match input.trim().parse::<u32>() {
        Ok(s) if s > 0 => s,
        _ => 100_000, // Default to 100 KHz
    };
    
    // Testing all modes
    let modes = [Mode::Mode0, Mode::Mode1, Mode::Mode2, Mode::Mode3];
    
    // Configure the test patterns for better diagnostics
    let test_patterns = [
        // Simple patterns
        vec![0x55],                // 01010101
        vec![0xAA],                // 10101010
        vec![0xFF],                // 11111111
        vec![0x00],                // 00000000
        
        // Sequences
        vec![0x00, 0xFF],          // Min-max
        vec![0x55, 0xAA],          // Alternating patterns
        vec![0x12, 0x34, 0x56],    // Increasing sequence
        
        // Longer patterns to check for shifts
        vec![0x01, 0x02, 0x03, 0x04, 0x05], 
        vec![0x55, 0xAA, 0x55, 0xAA, 0x55],
    ];
    
    // Print test configuration
    println!("\nTest Configuration:");
    println!("------------------");
    println!("SPI Bus:      {}", if bus == Bus::Spi0 { "SPI0" } else { "SPI1" });
    println!("Chip Select:  CS{}", if cs == SlaveSelect::Ss0 { 0 } else if cs == SlaveSelect::Ss1 { 1 } else { 2 });
    println!("Clock Speed:  {} Hz", speed);
    println!("Modes:        All modes will be tested");
    
    let mut total_passes = 0;
    let mut total_tests = 0;
    
    println!("\nReady to begin test? (Make sure loopback connection is in place)");
    println!("Press Enter to start or Ctrl+C to quit");
    input.clear();
    io::stdin().read_line(&mut input)?;
    
    println!("\nBeginning SPI loopback test...");
    
    // Test each mode
    for mode in modes {
        println!("\n============================================");
        println!("Testing with SPI {:?}", mode);
        println!("============================================");
        
        // Initialize SPI with current settings
        let mut spi = match Spi::new(bus, cs, speed, mode) {
            Ok(spi) => spi,
            Err(e) => {
                println!("Error initializing SPI with {:?}: {}", mode, e);
                continue;
            }
        };
        
        let mut mode_passes = 0;
        let mut mode_tests = 0;
        
        // Run tests with each pattern
        for pattern in &test_patterns {
            let pattern_len = pattern.len();
            let mut rx_buffer = vec![0u8; pattern_len];
            
            print!("Testing pattern {:?} -> ", pattern);
            io::stdout().flush()?;
            
            match spi.transfer(&mut rx_buffer, pattern) {
                Ok(_) => {
                    let matches = rx_buffer == *pattern;
                    println!("Received {:?} {}", 
                            rx_buffer, 
                            if matches { "✓" } else { "✗" });
                    
                    if matches {
                        mode_passes += 1;
                    }
                    mode_tests += 1;
                },
                Err(e) => println!("Transfer error: {}", e)
            }
            
            // Add small delay between tests
            thread::sleep(Duration::from_millis(50));
        }
        
        // Print results for this mode
        println!("\nResults for {:?}: {}/{} patterns passed", 
                mode, mode_passes, mode_tests);
        
        total_passes += mode_passes;
        total_tests += mode_tests;
    }
    
    // Print overall results
    println!("\n============================================");
    println!("Final Test Results");
    println!("============================================");
    println!("Total Passed: {}/{} tests", total_passes, total_tests);
    
    if total_passes == 0 {
        println!("\nNo tests passed! Likely causes:");
        println!("1. MOSI and MISO pins are not connected");
        println!("2. An SPI device (like MFRC522) is still connected and interfering");
        println!("3. Hardware issue with the SPI interface");
    } else if total_passes < total_tests {
        println!("\nSome tests passed. This might indicate:");
        println!("1. Intermittent connection issues");
        println!("2. Specific mode incompatibilities");
        println!("3. Speed-related issues");
    } else {
        println!("\nAll tests passed! Your SPI loopback is working correctly.");
    }
    
    println!("\nTest complete. Press Enter to exit.");
    input.clear();
    io::stdin().read_line(&mut input)?;
    
    Ok(())
}
