# DESFire Card Setup Tool

This project contains tools for working with DESFire cards, keeping the code modular and maintainable.

## Structure

- `src/desfire_common.rs` - Reusable library with common DESFire operations
- `src/bin/setup_card.rs` - Card setup application that creates applications and files
- `src/lib.rs` - Library exports for reuse in other projects

## Setup Instructions

1. Ensure your project structure is set up correctly:

```
desfire_tools/
├── src/
│   ├── desfire_common.rs
│   ├── lib.rs
│   └── bin/
│       └── setup_card.rs
├── Cargo.toml
└── README.md
```

2. Update your Cargo.toml to include the required dependencies:

```toml
[package]
name = "desfire_tools"
version = "0.1.0"
edition = "2021"

[dependencies]
pcsc = "2.6"
openssl = { version = "0.10", features = ["vendored"] }
```

## Using the Tools

### Setting Up a Card

1. Run the card setup tool:

```
cargo run --bin setup_card
```

2. The tool will:
   - Connect to your card reader
   - Authenticate with the master key (default: all zeros)
   - Give you options to:
     - Create a new application
     - List existing applications
     - Exit

3. When creating a new application:
   - You'll be prompted for an Application ID (6 hex digits)
   - Choose the number of keys for the application
   - A standard file will be created in the application
   - Sample data will be written to the file and read back

### Extending the Library

To create your own DESFire tools:

1. Import the common functions:

```rust
use desfire_tools::desfire_common::{
    connect_to_card, authenticate_des, send_apdu, HexSlice
};
```

2. Use these building blocks to implement your specific functionality

## Common DESFire Commands

- `0x90 0x0A` - Authenticate with DES
- `0x90 0x1A` - Authenticate with 3DES
- `0x90 0xAA` - Authenticate with AES
- `0x90 0x6A` - Get application IDs
- `0x90 0x5A` - Select application
- `0x90 0xCA` - Create application
- `0x90 0xDA` - Delete application
- `0x90 0xCD` - Create standard file
- `0x90 0x3D` - Write data
- `0x90 0xBD` - Read data

## Notes

- Default master key is all zeros: `00 00 00 00 00 00 00 00`
- Fresh cards have no applications
- Applications must be created before files can be created
- File operations must be performed after selecting an application

How to get back to a clean state:

First, try to reset the session with an Abort command:

90 EF 00 00 00

Then try selecting the master application (AID 000000):

90 5A 00 00 03 00 00 00 00

If that works, then try the authentication challenge request again:

90 1A 00 00 01 00 00
If you're still getting errors, you might want to try:

Getting the card version information:

90 60 00 00 00

Getting application IDs (to see what applications exist on the card):

90 6A 00 00 00
This will help us understand what state the card is in and make proper adjustments to the commands.

+++ additional troubleshooting
Try getting the card version again (should work regardless of authentication):

90 60 00 00 00

Try selecting the PICC (card) level:

90 5A 00 00 00

Or try selecting the master application again:

90 5A 00 00 03 00 00 00 00

Then try the proper authentication command (note it's 1A, not 0A):

90 1A 00 00 01 00 00
