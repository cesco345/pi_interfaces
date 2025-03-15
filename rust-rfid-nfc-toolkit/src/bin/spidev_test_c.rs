use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use clap::{Arg, App};
use hex;

fn main() -> io::Result<()> {
    let matches = App::new("SPI Testing Utility")
        .version("1.0")
        .author("Rust Version")
        .about("Tests SPI communication")
        .arg(Arg::with_name("device")
            .short('D')
            .long("device")
            .value_name("DEVICE")
            .help("Device to use (0.0, 0.1, etc.)")
            .takes_value(true)
            .default_value("0.0"))
        .arg(Arg::with_name("speed")
            .short('s')
            .long("speed")
            .value_name("HZ")
            .help("Max speed (Hz)")
            .takes_value(true)
            .default_value("500000"))
        .arg(Arg::with_name("delay")
            .short('d')
            .long("delay")
            .value_name("USEC")
            .help("Delay (usec)")
            .takes_value(true)
            .default_value("0"))
        .arg(Arg::with_name("bpw")
            .short('b')
            .long("bpw")
            .value_name("BITS")
            .help("Bits per word")
            .takes_value(true)
            .default_value("8"))
        .arg(Arg::with_name("loop")
            .short('l')
            .long("loop")
            .help("Loopback mode"))
        .arg(Arg::with_name("cpha")
            .short('H')
            .long("cpha")
            .help("Clock phase"))
        .arg(Arg::with_name("cpol")
            .short('O')
            .long("cpol")
            .help("Clock polarity"))
        .arg(Arg::with_name("input")
            .short('i')
            .long("input")
            .value_name("FILE")
            .help("Input data from a file")
            .takes_value(true))
        .arg(Arg::with_name("output")
            .short('o')
            .long("output")
            .value_name("FILE")
            .help("Output data to a file")
            .takes_value(true))
        .arg(Arg::with_name("verbose")
            .short('v')
            .long("verbose")
            .help("Verbose (show TX buffer)"))
        .arg(Arg::with_name("c-format")
            .short('c')
            .long("c-format")
            .help("Use C-style output format"))
        .arg(Arg::with_name("data")
            .short('p')
            .help("Send data (hex format e.g. \"DEADBEEF\")")
            .takes_value(true))
        .get_matches();

    // Parse arguments
    let device_str = matches.value_of("device").unwrap();
    let device_parts: Vec<&str> = device_str.split('.').collect();
    if device_parts.len() != 2 {
        eprintln!("Invalid device format. Expected 'bus.device' (e.g. '0.0')");
        process::exit(1);
    }
    
    let bus = match device_parts[0].parse::<u8>() {
        Ok(0) => Bus::Spi0,
        Ok(1) => Bus::Spi1,
        _ => {
            eprintln!("Invalid SPI bus. Must be 0 or 1.");
            process::exit(1);
        }
    };
    
    let slave = match device_parts[1].parse::<u8>() {
        Ok(0) => SlaveSelect::Ss0,
        Ok(1) => SlaveSelect::Ss1,
        _ => {
            eprintln!("Invalid SPI slave select. Must be 0 or 1.");
            process::exit(1);
        }
    };
    
    let speed = matches.value_of("speed").unwrap().parse::<u32>().unwrap_or(500000);
    let bits = matches.value_of("bpw").unwrap().parse::<u8>().unwrap_or(8);
    
    // Determine SPI mode
    let spi_mode = if matches.is_present("cpol") && matches.is_present("cpha") {
        Mode::Mode3
    } else if matches.is_present("cpol") {
        Mode::Mode2
    } else if matches.is_present("cpha") {
        Mode::Mode1
    } else {
        Mode::Mode0
    };
    
    let verbose = matches.is_present("verbose");
    let c_format = matches.is_present("c-format");
    
    println!("SPI Configuration:");
    println!("  Bus:      SPI{}", if bus == Bus::Spi0 { 0 } else { 1 });
    println!("  Device:   {}", if slave == SlaveSelect::Ss0 { 0 } else { 1 });
    println!("  Mode:     {:?}", spi_mode);
    println!("  Speed:    {} Hz ({} KHz)", speed, speed / 1000);
    println!("  Bits/word: {}", bits);
    if c_format {
        println!("spi mode: 0x{}", match spi_mode {
            Mode::Mode0 => "0",
            Mode::Mode1 => "1",
            Mode::Mode2 => "2",
            Mode::Mode3 => "3",
        });
        println!("bits per word: {}", bits);
        println!("max speed: {} Hz ({} KHz)", speed, speed / 1000);
    }
    
    // Initialize SPI
    let spi = match Spi::new(bus, slave, speed, spi_mode) {
        Ok(spi) => spi,
        Err(err) => {
            eprintln!("Failed to initialize SPI: {}", err);
            process::exit(1);
        }
    };
    
    // Default data if no input specified
    let default_tx: [u8; 32] = [
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0x40, 0x00, 0x00, 0x00, 0x00, 0x95,
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xF0, 0x0D,
    ];
    
    // Determine data to send
    let tx_data: Vec<u8> = if let Some(data_hex) = matches.value_of("data") {
        match hex::decode(data_hex.replace(" ", "")) {
            Ok(data) => data,
            Err(err) => {
                eprintln!("Invalid hex data: {}", err);
                process::exit(1);
            }
        }
    } else if let Some(input_file) = matches.value_of("input") {
        match read_file(input_file) {
            Ok(data) => data,
            Err(err) => {
                eprintln!("Failed to read input file: {}", err);
                process::exit(1);
            }
        }
    } else {
        default_tx.to_vec()
    };
    
    // Prepare receive buffer
    let mut rx_data = vec![0u8; tx_data.len()];
    
    // Transfer
    if verbose && !c_format {
        println!("TX Buffer:");
        hex_dump_rust(&tx_data);
    } else if verbose && c_format {
        print!("TX | ");
        hex_dump_c(&tx_data);
    }
    
    match spi.transfer(&mut rx_data, &tx_data) {
        Ok(_) => {
            if !c_format {
                println!("Transfer completed successfully.");
                println!("RX Buffer:");
                hex_dump_rust(&rx_data);
            } else {
                print!("RX | ");
                hex_dump_c(&rx_data);
            }
            
            // Save to output file if requested
            if let Some(output_file) = matches.value_of("output") {
                match write_file(output_file, &rx_data) {
                    Ok(_) => println!("Data written to {}", output_file),
                    Err(err) => eprintln!("Failed to write output file: {}", err),
                }
            }
        },
        Err(err) => {
            eprintln!("SPI transfer failed: {}", err);
            process::exit(1);
        }
    }
    
    Ok(())
}

// Rust-style formatting with address and clean structure
fn hex_dump_rust(data: &[u8]) {
    const LINE_SIZE: usize = 16;
    let mut i = 0;
    
    while i < data.len() {
        print!("{:04x} | ", i);
        
        // Print hex values
        for j in 0..LINE_SIZE {
            if i + j < data.len() {
                print!("{:02x} ", data[i + j]);
            } else {
                print!("   ");
            }
        }
        
        // Print ASCII representation
        print!("| ");
        for j in 0..LINE_SIZE {
            if i + j < data.len() {
                let c = data[i + j];
                if c >= 32 && c <= 126 {
                    print!("{}", c as char);
                } else {
                    print!(".");
                }
            }
        }
        println!();
        
        i += LINE_SIZE;
    }
}

// C-style formatting that matches the original spidev_test.c output
fn hex_dump_c(data: &[u8]) {
    // Print hex values first
    for byte in data {
        print!("{:02X} ", byte);
    }
    
    // Print separator
    print!("| ");
    
    // Print ASCII representation
    for byte in data {
        if *byte >= 32 && *byte <= 126 {
            print!("{}", *byte as char);
        } else {
            print!(".");
        }
    }
    println!();
}

fn read_file<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    Ok(data)
}

fn write_file<P: AsRef<Path>>(path: P, data: &[u8]) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(data)?;
    Ok(())
}
