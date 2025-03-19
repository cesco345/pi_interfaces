# PN532 NFC/RFID HAT Setup with Raspberry Pi and Rust

This guide provides step-by-step instructions for setting up and using a PN532 NFC/RFID HAT with your Raspberry Pi using Rust programming language.

## Hardware Requirements

- Raspberry Pi (any model with GPIO pins)
- PN532 NFC/RFID HAT
- MicroSD card with Raspberry Pi OS
- Power supply for Raspberry Pi
- NFC/RFID tags/cards for testing

## Hardware Installation

1. **Power off your Raspberry Pi**
   ```
   sudo shutdown -h now
   ```

2. **Remove any existing RFID modules** (like MFRC522) that might be connected

3. **Mount the PN532 HAT**
   - Carefully align the PN532 HAT with the GPIO pins on the Raspberry Pi
   - Gently press down to ensure it's firmly seated on all GPIO pins
   - Check for any bent pins or misalignments

4. **Power on your Raspberry Pi**

## Software Setup

### 1. Update your Raspberry Pi

```bash
sudo apt update
sudo apt upgrade -y
```

### 2. Enable Required Interfaces

```bash
sudo raspi-config
```
Navigate to "Interface Options" and enable:
- SPI (if your HAT uses SPI)
- I2C (if your HAT uses I2C)
- UART (if your HAT uses UART)

The interface used depends on the jumper settings on your specific PN532 HAT.

### 3. Install Development Dependencies

```bash
sudo apt install -y libudev-dev libusb-1.0-0-dev
```

### 4. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
Follow the prompts and select the default installation option.

After installation, add Rust to your current shell session:
```bash
source $HOME/.cargo/env
```

## Creating a Rust Project for PN532

### 1. Create a New Rust Project

```bash
cargo new pn532_project
cd pn532_project
```

### 2. Configure Dependencies in Cargo.toml

Edit the `Cargo.toml` file:

```toml
[package]
name = "pn532_project"
version = "0.1.0"
edition = "2021"

[dependencies]
rppal = "0.14.1"
embedded-hal = "0.2.7"
linux-embedded-hal = "0.3.2"
```

### 3. Basic PN532 Communication Example (SPI)

Create the following content in `src/main.rs`:

```rust
use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use linux_embedded_hal::Spidev;
use rppal::gpio::{Gpio, OutputPin};
use std::{thread, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure SPI
    let mut spi = Spidev::open("/dev/spidev0.0")?;
    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(1_000_000)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options)?;
    
    // Reset pin setup - adjust pin number according to your configuration
    let gpio = Gpio::new()?;
    let mut reset_pin = gpio.get(25)?.into_output();
    
    // Reset sequence
    reset_pin.set_high();
    thread::sleep(Duration::from_millis(100));
    reset_pin.set_low();
    thread::sleep(Duration::from_millis(500));
    reset_pin.set_high();
    thread::sleep(Duration::from_millis(100));
    
    // Get firmware version command
    let get_firmware_cmd = [0x02, 0x00, 0x00, 0xFF, 0x02, 0xFE, 0xD4, 0x02, 0x2A, 0x00];
    let mut rx_buf = [0u8; 20]; // Buffer to store response
    
    spi.transfer(&mut rx_buf[..get_firmware_cmd.len()], &get_firmware_cmd)?;
    
    println!("Response: {:?}", rx_buf);
    
    Ok(())
}
```

### 4. Build and Run the Project

```bash
cargo build --release
sudo ./target/release/pn532_project
```

Note: `sudo` is often needed for hardware access.

## Reading NFC Tags Example

Create a more functional example that reads NFC tags (`src/main.rs`):

```rust
use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use linux_embedded_hal::Spidev;
use rppal::gpio::{Gpio, OutputPin};
use std::{thread, time::Duration};

// Simplified PN532 command codes
const PN532_COMMAND_GETFIRMWAREVERSION: u8 = 0x02;
const PN532_COMMAND_SAMCONFIGURATION: u8 = 0x14;
const PN532_COMMAND_INLISTPASSIVETARGET: u8 = 0x4A;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing PN532 NFC reader...");
    
    // SPI setup
    let mut spi = Spidev::open("/dev/spidev0.0")?;
    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(1_000_000)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options)?;
    
    // Reset the PN532
    let gpio = Gpio::new()?;
    let mut reset_pin = gpio.get(25)?.into_output();
    
    reset_pin.set_high();
    thread::sleep(Duration::from_millis(100));
    reset_pin.set_low();
    thread::sleep(Duration::from_millis(500));
    reset_pin.set_high();
    thread::sleep(Duration::from_millis(100));
    
    // Get firmware version
    let firmware = get_firmware_version(&mut spi)?;
    println!("Found PN532 with firmware version: {}.{}", firmware[0], firmware[1]);
    
    // Configure the SAM (Secure Access Module)
    sam_configuration(&mut spi)?;
    
    println!("Waiting for an NFC tag...");
    
    // Main loop to detect tags
    loop {
        if let Some(uid) = read_passive_target(&mut spi)? {
            println!("Found card with UID: {:02X?}", uid);
            thread::sleep(Duration::from_secs(1));
        }
        thread::sleep(Duration::from_millis(100));
    }
}

// Helper functions for PN532 commands
fn get_firmware_version(spi: &mut Spidev) -> Result<[u8; 4], Box<dyn std::error::Error>> {
    let cmd = create_command(PN532_COMMAND_GETFIRMWAREVERSION, &[]);
    let response = send_command(spi, &cmd)?;
    
    // Parse firmware version from response
    Ok([response[0], response[1], response[2], response[3]])
}

fn sam_configuration(spi: &mut Spidev) -> Result<(), Box<dyn std::error::Error>> {
    // SAM configuration: normal mode, timeout 50ms, IRQ enabled
    let cmd = create_command(PN532_COMMAND_SAMCONFIGURATION, &[0x01, 0x14, 0x01]);
    send_command(spi, &cmd)?;
    Ok(())
}

fn read_passive_target(spi: &mut Spidev) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    // Command to read ISO14443A targets (1 tag, 100ms timeout)
    let cmd = create_command(PN532_COMMAND_INLISTPASSIVETARGET, &[0x01, 0x00]);
    let response = send_command(spi, &cmd)?;
    
    // Check if a tag was found
    if response[0] == 0 {
        return Ok(None);
    }
    
    // Extract tag UID length and UID from response
    let uid_length = response[5] as usize;
    let uid = response[6..6+uid_length].to_vec();
    
    Ok(Some(uid))
}

// Helper to create PN532 command packet
fn create_command(command: u8, data: &[u8]) -> Vec<u8> {
    let mut cmd = vec![
        0x00, 0x00, 0xFF,                   // Preamble
        (data.len() as u8) + 2,             // Length of data + command byte + checksum byte
        !(data.len() as u8 + 2),            // Length checksum
        0xD4,                               // Host to PN532
        command                             // Command
    ];
    
    // Add command data
    cmd.extend_from_slice(data);
    
    // Calculate data checksum
    let mut sum: u8 = 0xD4 + command;
    for &byte in data {
        sum = sum.wrapping_add(byte);
    }
    cmd.push(!sum);  // Checksum byte
    
    cmd.push(0x00);  // Postamble
    
    cmd
}

// Send command and receive response
fn send_command(spi: &mut Spidev, command: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Send command
    let mut tx_buf = command.to_vec();
    let mut rx_buf = vec![0u8; tx_buf.len()];
    spi.transfer(&mut rx_buf, &tx_buf)?;
    
    // Wait for ACK
    thread::sleep(Duration::from_millis(10));
    
    // Read response
    let mut response_header = [0u8; 6];
    spi.transfer(&mut response_header, &[0xFF; 6])?;
    
    // Parse response length
    let length = response_header[3] as usize;
    
    // Read remaining response
    let mut response_data = vec![0u8; length + 2]; // +2 for checksum and postamble
    spi.transfer(&mut response_data, &vec![0xFF; length + 2])?;
    
    // Extract actual data (skip status byte)
    Ok(response_data[1..length].to_vec())
}
```

## Alternative I2C Example

If your PN532 HAT is configured for I2C, use this `main.rs` instead:

```rust
use linux_embedded_hal::I2cdev;
use rppal::gpio::{Gpio, OutputPin};
use std::{thread, time::Duration};

const PN532_I2C_ADDRESS: u8 = 0x24;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing PN532 NFC reader (I2C mode)...");
    
    // I2C setup
    let mut i2c = I2cdev::new("/dev/i2c-1")?;
    
    // Reset the PN532
    let gpio = Gpio::new()?;
    let mut reset_pin = gpio.get(25)?.into_output();
    
    reset_pin.set_high();
    thread::sleep(Duration::from_millis(100));
    reset_pin.set_low();
    thread::sleep(Duration::from_millis(500));
    reset_pin.set_high();
    thread::sleep(Duration::from_millis(100));
    
    // TODO: Implement I2C communication with PN532
    // This would follow similar structure to the SPI example
    // but with I2C-specific communication methods
    
    println!("I2C mode is not fully implemented in this example");
    
    Ok(())
}
```

## Troubleshooting

If you encounter issues:

1. **Check Hardware Connections**
   - Ensure the HAT is properly seated on the GPIO pins
   - Verify the jumper settings match your code (SPI/I2C/UART)

2. **Verify Interface Enablement**
   - Make sure you've enabled the correct interface in raspi-config

3. **Permission Issues**
   - Run with sudo: `sudo ./target/release/pn532_project`
   - Or configure udev rules for hardware access without sudo

4. **SPI/I2C Address**
   - Check if your HAT uses a different SPI bus or I2C address

## Advanced Usage

Once basic communication is working, you can extend your Rust application to:

- Read and write NDEF messages
- Implement card emulation
- Set up peer-to-peer communication with other NFC devices
- Build applications for access control, home automation, etc.

## Resources

- [PN532 User Manual](https://www.nxp.com/docs/en/user-guide/141520.pdf)
- [Rust Embedded HAL Documentation](https://docs.rs/embedded-hal/latest/embedded_hal/)
- [RPPAL Documentation](https://docs.rs/rppal/latest/rppal/)
