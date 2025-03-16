# Raspberry Pi SPI Examples in Rust

This project contains Rust implementations of the SPI (Serial Peripheral Interface) examples originally written in Python from the provided document. These examples demonstrate how to use SPI communication on a Raspberry Pi using Rust.

## Prerequisites

- Raspberry Pi with SPI enabled
- Rust and Cargo installed (https://www.rust-lang.org/tools/install)
- SPI loopback connections set up (for testing)

### Enabling SPI on Raspberry Pi

To enable SPI on your Raspberry Pi:

1. Run `sudo raspi-config`
2. Navigate to "Interface Options" → "SPI"
3. Select "Yes" to enable SPI
4. Reboot your Raspberry Pi

### Setting up SPI Loopback

For the loopback tests to work, you need to connect the MOSI (GPIO 10) and MISO (GPIO 9) pins together for SPI0, or MOSI (GPIO 20) and MISO (GPIO 19) pins for SPI1, as mentioned in the original document.

## Building the Project

To build all examples:

```bash
cargo build --release
```

## Running the Examples

### SPI Loopback Test

This program sends two bytes at a time and reads them back in a loop:

```bash
cargo run --bin spi_loopback_test
```

### SPI Loopback Speed Test

This program tests the maximum reliable SPI speed by incrementally doubling the frequency:

```bash
cargo run --bin spi_loopback_speed
```

### Simple SPI Write

This program writes a single byte (0x3A) to the SPI bus repeatedly:

```bash
cargo run --bin spi_simple_write
```

### SPI Explorer

This is a comprehensive tool for exploring SPI functionality, similar to the original `spi_explore.py` script:

```bash
cargo run --bin spi_explorer -- --help
```

Example usage:

```bash
# Basic usage with default parameters (bus 0, CS 0, speed 1MHz, mode 0)
cargo run --bin spi_explorer

# Use different chip select and mode
cargo run --bin spi_explorer -- -c 1 -m 2

# Transfer with verbose output
cargo run --bin spi_explorer -- -t xfer -v
```

## Key Differences from Python Implementation

1. **Library**: We use the `rppal` crate instead of the Python `spidev` module.
2. **Buffer Size**: The `rppal` crate handles buffer sizes differently than the Python `spidev` module.
3. **Attributes**: Some attributes like `lsbfirst`, `loop`, and `threewire` are not directly accessible in `rppal`.
4. **Error Handling**: Rust provides more explicit error handling.

## Notes on SPI Modes

- **Mode 0**: CPOL=0, CPHA=0 - Clock idle low, data sampled on rising edge
- **Mode 1**: CPOL=0, CPHA=1 - Clock idle low, data sampled on falling edge
- **Mode 2**: CPOL=1, CPHA=0 - Clock idle high, data sampled on falling edge
- **Mode 3**: CPOL=1, CPHA=1 - Clock idle high, data sampled on rising edge
