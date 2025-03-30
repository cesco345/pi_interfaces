# DESFire Card Tools

A comprehensive toolkit for working with MIFARE DESFire cards, designed with modularity and maintainability in mind.

## Project Structure

```
desfire_tools/
├── src/
│   ├── desfire_common.rs   # Core library with common DESFire operations
│   ├── lib.rs              # Library exports for reuse in other projects
│   └── bin/
│       └── setup_card.rs   # Card setup application for creating applications and files
├── Cargo.toml
└── README.md
```

## Setup Instructions

1. **Configure Dependencies**

   Update your `Cargo.toml` to include the required dependencies:

   ```toml
   [package]
   name = "desfire_tools"
   version = "0.1.0"
   edition = "2021"

   [dependencies]
   pcsc = "2.6"
   openssl = { version = "0.10", features = ["vendored"] }
   ```

2. **Build the Project**

   ```bash
   cargo build
   ```

## Using the Tools

### Card Setup Tool

The setup tool allows you to initialize and configure DESFire cards:

```bash
cargo run --bin setup_card
```

#### Features:

- Connects to your card reader automatically
- Authenticates with the master key (default: all zeros)
- Provides options to:
  - Create new applications
  - List existing applications
  - Format card
  - Create and manage files within applications

#### Creating a New Application:

1. Select "Create application" from the menu
2. Enter an Application ID (6 hex digits)
3. Choose the number of keys for the application
4. Specify file creation options
5. Sample data will be written to verify functionality

### Extending the Library

To create your own custom DESFire tools:

```rust
use desfire_tools::desfire_common::{
    connect_to_card, authenticate_des, send_apdu, HexSlice
};

// Your implementation here
```

## Common DESFire Commands

| Command | Description |
|---------|-------------|
| `90 0A` | Authenticate with DES |
| `90 1A` | Authenticate with 3DES |
| `90 AA` | Authenticate with AES |
| `90 6A` | Get application IDs |
| `90 5A` | Select application |
| `90 CA` | Create application |
| `90 DA` | Delete application |
| `90 CD` | Create standard file |
| `90 3D` | Write data |
| `90 BD` | Read data |

## Troubleshooting

### Resetting Card State

If your card is in an unknown state, try the following sequence:

1. Reset the session:
   ```
   90 EF 00 00 00
   ```

2. Select the master application:
   ```
   90 5A 00 00 03 00 00 00 00
   ```

3. Request authentication:
   ```
   90 1A 00 00 01 00 00
   ```

### Diagnostic Commands

To understand the current state of your card:

- Get card version (works without authentication):
  ```
  90 60 00 00 00
  ```

- Get application IDs (requires authentication):
  ```
  90 6A 00 00 00
  ```

- Select PICC level:
  ```
  90 5A 00 00 00
  ```

## Cryptographic Operations

### Command Reference

**Encrypt a file with AES-256:**
```bash
openssl aes-256-cbc -a -salt -pbkdf2 -in secrets.txt -out secrets.txt.enc
```

**Decrypt an encrypted file:**
```bash
openssl aes-256-cbc -d -a -pbkdf2 -in secrets.txt.enc -out secrets.txt.new
```

### Parameters Explained

| Parameter | Description |
|-----------|-------------|
| `aes-256-cbc` | AES encryption with 256-bit key in CBC mode |
| `-a` | Output in base64 encoding (ASCII armor) |
| `-salt` | Add random salt when deriving encryption key |
| `-pbkdf2` | Use PBKDF2 for key derivation from password |
| `-in <file>` | Input file path |
| `-out <file>` | Output file path |
| `-d` | Decrypt mode (for decryption only) |

## Notes

- Default master key is all zeros: `00 00 00 00 00 00 00 00`
- Fresh cards have no applications
- Applications must be created before files
- File operations require selecting an application first
- Authentication is required for most operations

## DESFire Authentication Process

The authentication process follows these steps:

1. Select application
2. Request authentication challenge
3. Decrypt challenge with appropriate key
4. Process challenge according to protocol
5. Send response to card
6. Verify card's response

For detailed examples of authentication, see the included authentication script.
