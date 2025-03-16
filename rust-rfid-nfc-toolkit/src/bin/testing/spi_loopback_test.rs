use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::{thread, time::Duration};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Let's try a lower clock speed and different mode
    const SPI_BUS: Bus = Bus::Spi0;       // spidev0
    const SPI_SS: SlaveSelect = SlaveSelect::Ss0; // spidev0.0
    const SPI_CLOCK: u32 = 100_000;       // 100 KHz (lower than before)
    
    println!("SPI Loopback test running. Press Ctrl+C to exit.");
    println!("Testing all SPI modes at 100 KHz...");
    
    // Test with different SPI modes
    let modes = [Mode::Mode0, Mode::Mode1, Mode::Mode2, Mode::Mode3];
    
    for mode in &modes {
        println!("\nTesting with SPI {:?}", mode);
        
        // Setup SPI with the current mode
        let mut spi = match Spi::new(SPI_BUS, SPI_SS, SPI_CLOCK, *mode) {
            Ok(s) => s,
            Err(e) => {
                println!("Error initializing SPI with {:?}: {}", mode, e);
                continue;
            }
        };
        
        // First, try a simple single-byte test
        println!("Single byte test:");
        for test_byte in [0x55, 0xAA] {
            let mut receive = [0u8; 1];
            let send = [test_byte];
            
            match spi.transfer(&mut receive, &send) {
                Ok(_) => println!("  TX: 0x{:02X} -> RX: 0x{:02X} {}", 
                                 send[0], receive[0],
                                 if send[0] == receive[0] { "✓" } else { "✗" }),
                Err(e) => println!("  Transfer error: {}", e)
            }
            
            thread::sleep(Duration::from_millis(100));
        }
        
        // Now try the regular byte pair test
        println!("Byte pair test:");
        for i in 0..5 {
            let v = i * 2;
            let send = [v, v + 1];
            let mut receive = [0u8; 2];
            
            match spi.transfer(&mut receive, &send) {
                Ok(_) => println!("  TX: {:?} -> RX: {:?} {}", 
                                 send, receive,
                                 if send == receive { "✓" } else { "✗" }),
                Err(e) => println!("  Transfer error: {}", e)
            }
            
            thread::sleep(Duration::from_millis(100));
        }
        
        println!("Would you like to continue with more tests using this mode? (Press Ctrl+C to exit)");
        thread::sleep(Duration::from_secs(2));
    }
    
    println!("\nAll mode tests completed.");
    println!("\nNow trying a cleanup transfer to reset SPI state...");
    
    // Try one more approach - initialize, do a cleanup transfer, then test
    let mut spi = Spi::new(SPI_BUS, SPI_SS, SPI_CLOCK, Mode::Mode0)?;
    
    // Do some "cleanup" transfers
    let mut dummy = [0u8; 2];
    let zeros = [0u8, 0u8];
    
    // Send a few zero bytes to clear any pending data
    for _ in 0..3 {
        spi.transfer(&mut dummy, &zeros)?;
        thread::sleep(Duration::from_millis(50));
    }
    
    println!("\nFinal test after cleanup:");
    // Now try with a known pattern
    let test_pattern = [0x5A, 0xA5];
    let mut receive = [0u8; 2];
    
    spi.transfer(&mut receive, &test_pattern)?;
    println!("  TX: 0x{:02X} 0x{:02X} -> RX: 0x{:02X} 0x{:02X} {}", 
             test_pattern[0], test_pattern[1], 
             receive[0], receive[1],
             if test_pattern == receive { "✓" } else { "✗" });
    
    println!("\nDebug information:");
    println!("SPI clock: {} Hz", spi.clock_speed()?);
    println!("SPI mode: {:?}", spi.mode());
    println!("SPI bits per word: 8 (fixed in RPPAL)");
    
    println!("\nIf tests are still failing, please verify your physical connections.");
    println!("Make sure MOSI (GPIO 10) is connected to MISO (GPIO 9) for SPI0.");
    
    Ok(())
}
