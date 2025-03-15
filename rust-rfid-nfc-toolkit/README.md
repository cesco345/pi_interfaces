# SPI Testing Commands

The following commands can be used to test SPI communication with the MFRC522 RFID reader.

## Basic SPI Mode Tests

Test different SPI modes to determine which works best with your hardware:

```bash
# Mode 0 (CPOL=0, CPHA=0) - Default for most devices
cargo run --bin spidev_test_c -- -v -c -s 100000

# Mode 1 (CPOL=0, CPHA=1)
cargo run --bin spidev_test_c -- -v -c -H -s 100000

# Mode 2 (CPOL=1, CPHA=0)
cargo run --bin spidev_test_c -- -v -c -O -s 100000

# Mode 3 (CPOL=1, CPHA=1)
cargo run --bin spidev_test_c -- -v -c -H -O -s 100000
```

## SPI Speed Tests

Test different clock speeds to find the optimal balance between reliability and performance:

```bash
# Very slow (10 kHz) - Most reliable, good for troubleshooting
cargo run --bin spidev_test_c -- -v -c -s 10000

# Slow (100 kHz) - Good reliability with most devices
cargo run --bin spidev_test_c -- -v -c -s 100000

# Medium (500 kHz) - Default speed
cargo run --bin spidev_test_c -- -v -c -s 500000

# Fast (1 MHz)
cargo run --bin spidev_test_c -- -v -c -s 1000000

# Very fast (2 MHz) - May be too fast for some devices
cargo run --bin spidev_test_c -- -v -c -s 2000000
```

## MFRC522-Specific Command Tests

Test specific MFRC522 register commands:

```bash
# Read version register (0x37) - Should return the chip version
# Format: [Address byte] [dummy byte]
# 0x37 << 1 | 0x80 = 0x6F (Read command for register 0x37)
cargo run --bin spidev_test_c -- -v -c -s 100000 -p "6F00"

# Read command register (0x01)
# 0x01 << 1 | 0x80 = 0x83
cargo run --bin spidev_test_c -- -v -c -s 100000 -p "0300"

# Read status register (0x07)
# 0x07 << 1 | 0x80 = 0x8F
cargo run --bin spidev_test_c -- -v -c -s 100000 -p "0F00"

# Read FIFO data register (0x09)
# 0x09 << 1 | 0x80 = 0x93
cargo run --bin spidev_test_c -- -v -c -s 100000 -p "1300"
```

## Custom Command Testing

You can also try arbitrary hex commands:

```bash
# Sample command (replace with your custom command)
cargo run --bin spidev_test_c -- -v -c -s 100000 -p "FF00FF00"
```

## Using Delays

If you're having communication issues, adding a delay might help:

```bash
# Add a 10 microsecond delay between bytes
cargo run --bin spidev_test_c -- -v -c -s 100000 -d 10 -p "6F00"
```

## Interpretation Guide

When interpreting results:

1. The first byte received is usually not meaningful (received during command transmission)
2. Standard MFRC522 version values are:
   - 0x91 = Version 1.0
   - 0x92 = Version 2.0
   - 0x88 = Clone chip (FM17522)
3. Look for consistent response patterns at different speeds/modes
4. Non-zero, non-0xFF responses usually indicate successful communication

A typical response pattern might show zeros for the first few bytes, followed by mirroring of your transmitted data. This usually indicates the SPI communication is working properly.
