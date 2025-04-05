# NFC Development Toolkit

This project combines a React Native mobile application (DESFire Emulator) with a Raspberry Pi-based command-line toolkit (NDEF Explorer) for working with various NFC card technologies. The toolkit provides tools for reading, writing, cloning, and emulating different NFC card types.

## Table of Contents

- [Overview](#overview)
- [Setup and Installation](#setup-and-installation)
  - [Android App Setup](#android-app-setup)
  - [Raspberry Pi Setup](#raspberry-pi-setup)
- [Working with Different Card Types](#working-with-different-card-types)
  - [NTAG213/215/216](#ntag213215216)
  - [MIFARE Classic](#mifare-classic)
  - [MIFARE DESFire](#mifare-desfire)
  - [MIFARE Ultralight](#mifare-ultralight)
- [Using Mobile Apps as Helpers](#using-mobile-apps-as-helpers)
  - [NXP Tag Info](#nxp-tag-info)
  - [NXP Tag Writer](#nxp-tag-writer)
- [Troubleshooting](#troubleshooting)
- [Additional Resources](#additional-resources)

## Overview

This toolkit provides a comprehensive set of tools for working with NFC technologies:

- **Android App**: React Native application for emulating DESFire cards and interacting with NFC tags
- **Raspberry Pi Tools**: Rust-based command-line tools for working with different NFC card types

## Setup and Installation

### Android App Setup

#### Prerequisites

- Node.js (v14+)
- Java Development Kit (JDK 11+)
- Android Studio
- Android SDK
- React Native CLI

#### Installation Steps

1. **Clone the repository**:
   ```bash
   git clone https://your-repository-url/desfiremulator.git
   cd desfiremulator
   ```

2. **Install JavaScript dependencies**:
   ```bash
   npm install
   ```

3. **Install native dependencies**:
   ```bash
   npm run preserve-native-files  # This runs the preserve-native-files.sh script
   ```

4. **Setting up Android environment variables**:
   Add the following to your `~/.bashrc` or `~/.zshrc`:
   ```bash
   export ANDROID_HOME=$HOME/Android/Sdk
   export PATH=$PATH:$ANDROID_HOME/emulator
   export PATH=$PATH:$ANDROID_HOME/tools
   export PATH=$PATH:$ANDROID_HOME/tools/bin
   export PATH=$PATH:$ANDROID_HOME/platform-tools
   ```

5. **Prebuild the project**:
   ```bash
   npx prebuild
   ```

6. **Build and run the Android app**:
   ```bash
   cd android
   ./gradlew assembleDebug
   cd ../
   npx react-native run-android
   ```

### Raspberry Pi Setup

#### Prerequisites

- Raspberry Pi (3B+ or newer recommended)
- Raspbian OS (Buster or newer)
- ACR122U NFC Reader or compatible
- Rust (latest stable version)

#### Installation Steps

1. **Install required packages**:
   ```bash
   sudo apt update
   sudo apt install -y build-essential git curl pkg-config libssl-dev libudev-dev libusb-1.0-0-dev
   sudo apt install -y pcscd pcsc-tools libpcsclite-dev
   ```

2. **Install Rust**:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env
   ```

3. **Clone the NDEF Explorer repository**:
   ```bash
   git clone https://your-repository-url/pi_afr.git
   cd pi_afr/ndef_explorer
   ```

4. **Build the project**:
   ```bash
   cargo build --release
   ```

5. **Start the PC/SC daemon**:
   ```bash
   sudo systemctl enable pcscd
   sudo systemctl start pcscd
   ```

6. **Test the NFC reader**:
   ```bash
   # List connected readers
   pcsc_scan -r
   ```

## Working with Different Card Types

### NTAG213/215/216

NTAG2xx series are simple, NFC Forum Type 2 compliant tags with different memory capacities (NTAG213: 144 bytes, NTAG215: 504 bytes, NTAG216: 888 bytes).

#### Reading an NTAG

```bash
# Using the read_ntag tool
cargo run --bin read_ntag
```

#### Writing to an NTAG

1. **Create a JSON file for the tag data**:
   ```bash
   # Create manually or use the clone_ntag tool on an existing tag
   cargo run --bin clone_ntag data.txt
   ```

2. **Write the data to a new tag**:
   ```bash
   cargo run --bin ntag_writer ntag213_clone.json
   ```

3. **For formatting issues, add the --force flag**:
   ```bash
   cargo run --bin ntag_writer ntag213_clone.json --force
   ```

### MIFARE Classic

MIFARE Classic uses a proprietary protocol with sector-based memory and key-based authentication.

#### Reading a MIFARE Classic

```bash
cargo run --bin mifare_reader
```

#### Writing to a MIFARE Classic

```bash
# Clone a MIFARE Classic card
cargo run --bin clone_mifare tag_dump.txt

# Write data to the card
cargo run --bin mifare_writer mifare_clone.json
```

#### NDEF Formatting a MIFARE Classic

```bash
cargo run --bin mifare_ndef_formatter
```

### MIFARE DESFire

DESFire is a more advanced, secure NFC card with a file system structure.

#### DESFire Operations

When working with DESFire, the card_writer tool will detect it and use the appropriate protocol:

```bash
cargo run --bin card_writer desfire_data.json
```

Follow the on-screen instructions for:
1. Creating NDEF applications
2. Selecting applications
3. Creating Capability Container (CC) files
4. Creating NDEF data files
5. Writing data

### MIFARE Ultralight

MIFARE Ultralight is a low-cost, NFC Forum Type 2 compliant tag similar to NTAG but with different memory organization.

```bash
# Ultralight tags can often be written using the card_writer
cargo run --bin card_writer ultralight_data.json
```

## Using Mobile Apps as Helpers

### NXP Tag Info

NXP Tag Info is invaluable for identifying tag types and examining their contents:

1. **Install from Google Play Store**
2. **Tag Identification**:
   - Place an unknown tag on your phone
   - The app will identify the exact type, manufacturer, and specifications
   - View the memory contents, including available space and usage

3. **Memory Map Analysis**:
   - Examine the memory blocks/pages
   - See what data is stored where
   - Check for lock bits and other protection features

4. **Use Tag Info Before Programming**:
   - Verify the tag type to ensure you use the correct writing tool
   - Check for any existing data or locked sectors
   - Note the total memory size for your JSON file preparation

### NXP Tag Writer

NXP Tag Writer helps with formatting and writing standardized NDEF content:

1. **Install from Google Play Store**
2. **Erase and Format Tags**:
   - Use "Erase tags" feature
   - Choose between:
     - "Erase to factory default": Complete reset
     - "Erase & format as NDEF": Clear and prepare for NDEF data

3. **Custom Format Settings**:
   - **DESFire vs Ultralight question**: Choose based on your tag type
   - **Byte size question**: Enter appropriate size based on tag type:
     - NTAG213: ~128 bytes
     - NTAG215: ~480 bytes
     - NTAG216: ~880 bytes
     - MIFARE Classic 1K: ~704 bytes
     - DESFire: 256-1024 bytes (as needed)

4. **Use Tag Writer When**:
   - You need to reset a tag to a clean state
   - You want to ensure NDEF compatibility for general use
   - You're having trouble with direct writing using the Raspberry Pi tools

## Troubleshooting

### Common Issues

1. **Card Not Detected**:
   - Ensure the pcscd service is running: `sudo systemctl status pcscd`
   - Try disconnecting and reconnecting the reader
   - Make sure the card is properly placed on the reader

2. **Permission Issues**:
   ```bash
   # Add your user to the proper groups
   sudo usermod -a -G plugdev $USER
   # Create udev rules for the reader
   echo 'SUBSYSTEM=="usb", ATTRS{idVendor}=="072f", ATTRS{idProduct}=="2200", GROUP="plugdev", MODE="0660"' | sudo tee /etc/udev/rules.d/99-acr122u.rules
   sudo udevadm control --reload-rules
   sudo udevadm trigger
   ```

3. **Write Failures**:
   - Check if the card is write-protected
   - For MIFARE Classic, ensure you have the correct authentication keys
   - For protected sectors, use the --force flag when applicable

4. **Card Type Detection Issues**:
   - Use NXP Tag Info to confirm the exact card type
   - Try different card positioning on the reader
   - Some counterfeit cards may not correctly identify themselves

## Additional Resources

- [MIFARE Type Identification Procedure](https://www.nxp.com/docs/en/application-note/AN10833.pdf)
- [NFC Forum Type 2 Tag Operation Specification](https://nfc-forum.org/build/specifications/)
- [NDEF Message Formatting Guide](https://developer.android.com/reference/android/nfc/NdefMessage)
- [DESFire EV1 Application Programming Guide](https://www.nxp.com/docs/en/application-note/AN10787.pdf)

---

This project combines mobile and Raspberry Pi technologies to provide a comprehensive toolkit for NFC development, allowing for reading, writing, and emulation of various NFC card types.

## Extra Notes

# NFC Card Tools

A collection of Rust tools for working with various NFC card types.

## Overview

This toolkit provides command-line utilities for reading, writing, and formatting different types of NFC cards, including MIFARE Classic, DESFire, and other NDEF-compatible cards.

## Card Type Operations

### MIFARE Classic Cards

#### Reading
```bash
cargo run --bin mifare_reader
```
This reads MIFARE Classic cards, showing sector data and interpreting NDEF messages if present.

#### Writing
```bash
cargo run --bin mifare_writer data1.json
```
Writes data from the specified JSON file to a MIFARE Classic card. It automatically tries different authentication keys and writes to the appropriate sectors.

#### NDEF Formatting
```bash
cargo run --bin mifare_ndef_formatter data1.json
```
Formats a MIFARE Classic card for NDEF compatibility, configuring MAD sectors and writing proper NDEF structures.

### DESFire Cards

#### Writing
```bash
cargo run --bin card_writer data1.json
```
This detects the card type and if it's a DESFire, it will show DESFire setup instructions and optionally attempt automatic setup.

### Generic NDEF Operations

#### Reading
```bash
cargo run --bin focused_ndef_reader
```
An interactive NDEF explorer that handles multiple card types and provides a menu for various operations:
- Select NDEF Application
- Read Capability Container
- Read NDEF Message Length
- Read NDEF Message
- Write Sample NDEF Message
- Scan Memory
- Send Raw Commands
- Import Card Data for Writing

#### Writing to Type 2 Tags (NTAG, MIFARE Ultralight)
The `card_writer` can handle Type 2 tags by detecting them and using the appropriate writing method:
```bash
cargo run --bin card_writer data1.json
```

#### Writing to Type 4 Tags (like DESFire)
Also handled by `card_writer` with dedicated operations for Type 4 tags:
```bash
cargo run --bin card_writer data1.json
```

### Other Useful Commands

#### Sending Raw Commands
```bash
cargo run --bin raw_command
```
Allows you to send custom APDU commands to any card type for advanced operations or troubleshooting.

## Step-by-Step Process for Each Card Type

### MIFARE Classic
1. **Read the card first to identify it:**
   ```bash
   cargo run --bin mifare_reader
   ```

2. **Format for NDEF if necessary:**
   ```bash
   cargo run --bin mifare_ndef_formatter data1.json
   ```

3. **Write data:**
   ```bash
   cargo run --bin mifare_writer data1.json
   ```

4. **Verify by reading again:**
   ```bash
   cargo run --bin mifare_reader
   ```

### DESFire Cards
1. **Read the card first:**
   ```bash
   cargo run --bin focused_ndef_reader
   ```
   (Then select option 1 to identify the card)

2. **Write data:**
   ```bash
   cargo run --bin card_writer data1.json
   ```
   - Note: This will show DESFire setup instructions
   - Choose "y" when asked about attempting automatic setup

3. **Verify by reading again:**
   ```bash
   cargo run --bin focused_ndef_reader
   ```
   (Then select option 4 to read NDEF message)

### MIFARE Ultralight/NTAG (Type 2 Tags)
1. **Read the card:**
   ```bash
   cargo run --bin focused_ndef_reader
   ```

2. **Write data:**
   ```bash
   cargo run --bin card_writer data1.json
   ```
   (It will detect Type 2 tags and use the appropriate method)

3. **Verify:**
   ```bash
   cargo run --bin focused_ndef_reader
   ```

## JSON Data Format

The JSON files used for writing should contain data in the CardExport format:

```json
{
  "name": "Card 07:09:00:00:03:00:00:02",
  "applicationId": 1,
  "fileId": 1,
  "fileData": "Hello World",
  "exportDate": "2025-04-04T18:55:52.938Z"
}
```

## Troubleshooting

If you encounter issues with card reading or writing:

1. Try using the `raw_command` tool to send direct APDU commands
2. Check that you're using the correct card type for the intended operation
3. For MIFARE Classic cards, authentication can sometimes fail if non-standard keys are used
4. DESFire cards require special formatting before NDEF data can be written

## Mobile App Integration

For the React Native mobile app integration, ensure the emulation function properly formats the NDEF message:

1. Use the updated useEmulation hook that includes proper NDEF formatting
2. Test with basic text records first before trying more complex formats
3. Ensure text data is properly converted to hex format when needed

## Hardware Requirements

- PC/SC compliant NFC reader (ACR122U recommended)
- Supported card types:
  - MIFARE Classic (1K/4K)
  - MIFARE Ultralight/NTAG (Type 2)
  - DESFire (Type 4)
  - Other ISO14443A compatible cards
