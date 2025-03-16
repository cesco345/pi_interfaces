use clap::{App, Arg};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::{thread, time::Duration};
use std::error::Error;
use std::process;

// Add Debug derive to fix compilation error
#[derive(Debug)]
enum TestMethod {
    WriteBytes,
    Xfer,
}

// Convert byte array to hex string
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|b| format!("{:02X} ", b))
        .collect::<String>()
        .trim_end()
        .to_string()
}

// Dump SPI attributes
fn dump_attributes(spi: &Spi, msg: &str) {
    println!("\n{}", msg);
    println!("  clock_speed: {} Hz", spi.clock_speed().unwrap_or(0));
    println!("  mode: {:?}", spi.mode());
    // Note: RPPAL doesn't provide direct access to all attributes that spidev exposes
}

fn main() -> Result<(), Box<dyn Error>> {
    // Parse command line arguments
    let matches = App::new("SPI Explorer")
        .version("1.0")
        .about("Explore SPI functionality on Raspberry Pi")
        .arg(Arg::with_name("bus")
            .short('b')  // Fixed: Use char instead of &str
            .long("bus")
            .value_name("BUS")
            .help("SPI bus (0 for spidev0, 1 for spidev1 ...)")
            .takes_value(true)
            .default_value("0"))
        .arg(Arg::with_name("cs")
            .short('c')  // Fixed: Use char instead of &str
            .long("cs")
            .value_name("CS")
            .help("Chip select (0, 1 or 2)")
            .takes_value(true)
            .default_value("0"))
        .arg(Arg::with_name("speed")
            .short('s')  // Fixed: Use char instead of &str
            .long("speed")
            .value_name("SPEED")
            .help("Maximum speed (Hz)")
            .takes_value(true)
            .default_value("1000000"))
        .arg(Arg::with_name("mode")
            .short('m')  // Fixed: Use char instead of &str
            .long("mode")
            .value_name("MODE")
            .help("Mode (0, 1, 2 or 3)")
            .takes_value(true)
            .default_value("0"))
        .arg(Arg::with_name("test")
            .short('t')  // Fixed: Use char instead of &str
            .long("test")
            .value_name("TEST")
            .help("Test method: 'writebytes' or 'xfer'")
            .takes_value(true)
            .default_value("writebytes"))
        .arg(Arg::with_name("repeat")
            .short('r')  // Fixed: Use char instead of &str
            .long("repeat")
            .value_name("COUNT")
            .help("Number of repeat transmissions, 0 for endless repetitions")
            .takes_value(true)
            .default_value("0"))
        .arg(Arg::with_name("length")
            .short('L')  // Fixed: Use char instead of &str
            .long("length")
            .value_name("LENGTH")
            .help("Buffer length")
            .takes_value(true)
            .default_value("4"))
        .arg(Arg::with_name("first")
            .short('F')  // Fixed: Use char instead of &str
            .long("first")
            .value_name("FIRST")
            .help("First byte in buffer (0 to 255)")
            .takes_value(true)
            .default_value("254"))
        .arg(Arg::with_name("verbose")
            .short('v')  // Fixed: Use char instead of &str
            .long("verbose")
            .help("Display object attributes"))
        .get_matches();
    
    // Parse arguments
    let bus_num: u8 = matches.value_of("bus").unwrap().parse()?;
    let cs_num: u8 = matches.value_of("cs").unwrap().parse()?;
    let speed: u32 = matches.value_of("speed").unwrap().parse()?;
    let mode_num: u8 = matches.value_of("mode").unwrap().parse()?;
    let test_method = match matches.value_of("test").unwrap() {
        "writebytes" => TestMethod::WriteBytes,
        "xfer" => TestMethod::Xfer,
        _ => {
            eprintln!("Invalid test method");
            process::exit(1);
        }
    };
    let repeat: u32 = matches.value_of("repeat").unwrap().parse()?;
    let buffer_size: usize = matches.value_of("length").unwrap().parse()?;
    let first_byte: u8 = matches.value_of("first").unwrap().parse()?;
    let verbose = matches.is_present("verbose");
    
    // Convert parameters to RPPAL types
    let bus = match bus_num {
        0 => Bus::Spi0,
        1 => Bus::Spi1,
        _ => {
            eprintln!("Invalid SPI bus number");
            process::exit(1);
        }
    };
    
    let cs = match cs_num {
        0 => SlaveSelect::Ss0,
        1 => SlaveSelect::Ss1,
        2 => SlaveSelect::Ss2,
        _ => {
            eprintln!("Invalid chip select number");
            process::exit(1);
        }
    };
    
    let mode = match mode_num {
        0 => Mode::Mode0,
        1 => Mode::Mode1,
        2 => Mode::Mode2,
        3 => Mode::Mode3,
        _ => {
            eprintln!("Invalid SPI mode");
            process::exit(1);
        }
    };
    
    // Setup SPI
    let mut spi = Spi::new(bus, cs, speed, mode)?;
    
    if verbose {
        dump_attributes(&spi, "SPI attributes");
    }
    
    // Setup buffer
    let mut buffer: Vec<u8> = Vec::with_capacity(buffer_size);
    let mut current_byte = first_byte;
    
    for _ in 0..buffer_size {
        buffer.push(current_byte);
        current_byte = current_byte.wrapping_add(1);
    }
    
    let sent_data = bytes_to_hex(&buffer);
    
    println!("\nTest parameters");
    println!("  Buffer size: {} bytes", buffer_size);
    println!("  First byte in buffer: {}", first_byte);
    println!("  Testing method: {:?}", test_method);
    
    if repeat == 0 {
        println!("  Repeat transmission endlessly");
    } else {
        println!("  Repeat transmission {} times", repeat);
    }
    
    let mut loop_count = 0;
    
    loop {
        println!("\nTX: {}", sent_data);
        
        match test_method {
            TestMethod::WriteBytes => {
                spi.write(&buffer)?;
                println!("Write completed (no RX data when using write mode)");
            },
            TestMethod::Xfer => {
                let mut rx_buffer = vec![0u8; buffer.len()];
                spi.transfer(&mut rx_buffer, &buffer)?;
                let received_data = bytes_to_hex(&rx_buffer);
                println!("RX: {}", received_data);
                println!("Match: {}", if rx_buffer == buffer { "✓" } else { "✗" });
            }
        };
        
        if verbose && loop_count == 0 {
            dump_attributes(&spi, "Attributes after first transmission");
        }
        
        thread::sleep(Duration::from_millis(200));
        
        loop_count += 1;
        
        if repeat > 0 && loop_count >= repeat {
            break;
        }
    }
    
    Ok(())
}
