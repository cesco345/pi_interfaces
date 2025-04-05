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
- [COVID-19 Test Result Writer](#covid-19-test-result-writer)
- [Transferring Files from Android to Raspberry Pi](#transferring-files-from-android-to-raspberry-pi)
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

There are two main methods for writing to NTAG213/215/216 cards:

1. **Cloning from memory dump**:
   ```bash
   # Create a clone file from a tag memory dump (e.g., from NXP TagInfo)
   cargo run --bin clone_ntag taginfo.txt
   
   # Write the cloned data to a new tag
   cargo run --bin ntag_writer ntag213_clone.json
   ```

2. **Direct writing from JSON data file** (Recommended):
   ```bash
   # Write data directly from a JSON file exported from the mobile app
   cargo run --bin ntag_writer data.txt --force
   ```

The second method is particularly useful when transferring data from your mobile app emulator. The `--force` flag ensures compatibility when importing from different format types (like DESFire NDEF to NTAG).

#### JSON Format for NTAG Writing

The JSON file should contain data in this format:
```json
{
  "id": "card_1743832619319",
  "name": "Card 00:04:00:00:09:00:01:02:00:02:01:00:09:00",
  "applicationId": 1,
  "fileId": 1,
  "fileData": "Hello beautiful ❤️",
  "format": "desfire_ndef",
  "exportDate": "2025-04-05T05:57:36.852Z"
}
```

The `ntag_writer` with `--force` flag will intelligently convert this format to work with NTAG213 cards, handling text data including emojis.

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

## COVID-19 Test Result Writer

This specialized tool allows you to encode COVID-19 test results onto NTAG213 tags for use with the COVID-19 Test Reader in the mobile application.

### Features

- Creates minimal-size JSON data that fits within NTAG213's 144-byte capacity
- Stores essential test information (result, timestamps, lot number)
- Tracks both test result validity and physical test kit expiration
- Formats data in NDEF Text Record format for compatibility with NFC-enabled devices

### Data Format

The COVID test writer uses a compact data format with the following fields:
- `res`: Test result as a single character ("p" for positive, "n" for negative, "i" for invalid)
- `ts`: Unix timestamp of when the test was performed
- `exp`: Unix timestamp of when the test result expires
- `lot`: Lot number of the physical test
- `mfg`: Unix timestamp of the test kit's manufacturing date
- `shf`: Shelf life of the test kit in months

### Usage

```bash
# Run the COVID test writer
cargo run --bin covid_test_writer
```

The tool will:
1. Generate a test result with the current timestamp
2. Show if the data will fit on an NTAG213 tag
3. Display human-readable information about the test
4. Prompt to write the data to a tag

### Customizing Test Data

You can modify the `create_compact_test_result` function to:
- Change the test result ("p", "n", or "i")
- Set different manufacturing dates
- Adjust shelf life in months
- Modify test validity period (default is 72 hours)
- Change the lot number

### Reading COVID Test Tags

To read a COVID test tag you've created:
1. Open the DESFire Emulator app on your Android device
2. Tap the "COVID-19 Test Reader" button
3. Place your phone near the tag

The app will display:
- Test result (Positive/Negative/Invalid)
- Test date and expiration
- Manufacturing date and kit expiration date
- Lot number
- Validity status of both the test result and the physical kit

### Compatibility Notes

- The COVID test data is optimized to fit within the 144-byte limit of NTAG213 tags
- For larger data sets (more fields), consider using NTAG215 (504 bytes) or NTAG216 (888 bytes)
- The mobile app can automatically parse and expand the minimal data format

## Transferring Files from Android to Raspberry Pi

To transfer export files from your Android device to your Raspberry Pi for tag writing, you can use Termux with SCP. This is useful when you've created card data on your mobile emulator app and want to write it to physical tags.

### Setup Termux for File Transfer

1. **Install Termux on your Android device** from the Google Play Store or F-Droid

2. **Install required packages in Termux**:
   ```bash
   pkg update
   pkg install openssh
   ```

3. **Configure password-less SSH (optional but recommended)**:
   ```bash
   # Generate SSH key
   ssh-keygen -t rsa
   
   # Copy your key to the Raspberry Pi
   # (You'll need to enter your Pi's password)
   ssh-copy-id pi@raspberry_pi_ip_address
   ```

### Transfer Files from Android to Raspberry Pi

1. **Export your card data from the emulator app** (this creates a data.txt file in your app's storage)

2. **In Termux, navigate to the file location**:
   ```bash
   # You might need to grant Termux storage permission first
   termux-setup-storage
   
   # Navigate to your app's files
   cd ~/storage/shared/Android/data/com.stemapks.desfiremulator/files
   # or wherever your app stores exported files
   ```

3. **Transfer the file to your Raspberry Pi**:
   ```bash
   scp data.txt pi@raspberry_pi_ip_address:~/rust/pi_afr/ndef_explorer/
   ```

4. **On your Raspberry Pi, write the data to a tag**:
   ```bash
   cd ~/rust/pi_afr/ndef_explorer
   cargo run --bin ntag_writer data.txt --force
   ```

### Alternative File Transfer Methods

If SCP is not available, you can use other methods:

- **Android File Transfer apps** like Solid Explorer or FX File Explorer with SFTP plugin
- **HTTP transfer** using a simple HTTP server:
  ```bash
  # On Raspberry Pi
  python3 -m http.server 8000
  
  # Then upload from your Android browser
  # http://raspberry_pi_ip_address:8000
  ```
- **USB drive transfer**: Copy to a USB drive from your Android device, then plug into Raspberry Pi
- **Email or cloud storage** (Google Drive, Dropbox, etc.)

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
- With NTAG213 writing, make sure to use the --force flag when importing from different format types
   - For COVID test writer, ensure the card is properly placed and wait for the beep before pressing Enter

4. **Card Type Detection Issues**:
   - Use NXP Tag Info to confirm the exact card type
   - Try different card positioning on the reader
   - Some counterfeit cards may not correctly identify themselves

5. **Emoji and Special Character Issues**:
   - When writing text with emojis, make sure to use the improved ntag_writer with --force flag
   - If emojis are corrupted, check that your JSON file uses UTF-8 encoding

6. **File Transfer Problems**:
   - Verify network connectivity between your Android device and Raspberry Pi
   - Check that SSH or SCP services are running on the Raspberry Pi
   - Try alternative transfer methods if SCP fails

7. **COVID Test Writer Timing Issues**:
   - If you encounter "NoSmartcard" errors, make sure to place the card firmly on the reader
   - Try adding a small delay (300-500ms) between card placement and initialization
   - For more reliable detection, press the card firmly against the reader when the LED is solid

## COVID Test Integration with Mobile App

### Setting Up the Test Reader

1. Add the COVID-19 Test Reader button to your React Native app:
   ```jsx
   {/* COVID-19 Test Reader Button - Added below the main reading button */}
   <TouchableOpacity
     style={[
       commonStyles.button,
       styles.covidButton,
       isEmulating ? commonStyles.buttonDisabled : null,
     ]}
     onPress={() => router.push('/test-result')}
     disabled={isEmulating}
   >
     <Text style={commonStyles.buttonText}>
       COVID-19 Test Reader
     </Text>
   </TouchableOpacity>
   ```

2. Create a TestResultScreen.js component in your app that can:
   - Parse the minimalist data format from the tags
   - Expand it into a full display format
   - Show clear visual indicators for test validity

3. Connect your mobile app to the NTAG213 tags by:
   - Using the useTagReader hook to detect NFC tags
   - Extracting the JSON data from the NDEF Text Record
   - Processing both the test result and the physical kit expiration dates

### Tag Data Structure and Size Considerations

When working with NTAG213 tags for COVID test data:

1. Keep your JSON payload minimal:
   - Use short field names (e.g., "res" instead of "result")
   - Use single characters for result codes when possible
   - Use Unix timestamps to save space over full date strings

2. Understand size limitations:
   - NTAG213: 144 bytes total (including NDEF overhead)
   - NDEF overhead: ~10-16 bytes
   - Keep your actual JSON data under 130 bytes for reliable writing

3. Make size vs. feature tradeoffs:
   - If you need more data fields, consider NTAG215 tags
   - Focus on essential information that users need immediately
   - Move less critical data to the mobile app's expanded display

## Additional Resources

- [MIFARE Type Identification Procedure](https://www.nxp.com/docs/en/application-note/AN10833.pdf)
- [NFC Forum Type 2 Tag Operation Specification](https://nfc-forum.org/build/specifications/)
- [NDEF Message Formatting Guide](https://developer.android.com/reference/android/nfc/NdefMessage)
- [DESFire EV1 Application Programming Guide](https://www.nxp.com/docs/en/application-note/AN10787.pdf)
- [Termux Wiki](https://wiki.termux.com/wiki/Main_Page) - For Android command-line usage
- [NDEF Message Format Documentation](https://learn.adafruit.com/adafruit-pn532-rfid-nfc/ndef)
- [NTAG213/215/216 Product Data Sheet](https://www.nxp.com/docs/en/data-sheet/NTAG213_215_216.pdf)

---

This project combines mobile and Raspberry Pi technologies to provide a comprehensive toolkit for NFC development, allowing for reading, writing, and emulation of various NFC card types.

## Extra Notes

### NFC Card Tools

A collection of Rust tools for working with various NFC card types.

#### Overview

This toolkit provides command-line utilities for reading, writing, and formatting different types of NFC cards, including MIFARE Classic, DESFire, and other NDEF-compatible cards.

#### Card Type Operations

##### MIFARE Classic Cards

###### Reading
```bash
cargo run --bin mifare_reader
```
This reads MIFARE Classic cards, showing sector data and interpreting NDEF messages if present.

###### Writing
```bash
cargo run --bin mifare_writer data1.json
```
Writes data from the specified JSON file to a MIFARE Classic card. It automatically tries different authentication keys and writes to the appropriate sectors.

###### NDEF Formatting
```bash
cargo run --bin mifare_ndef_formatter data1.json
```
Formats a MIFARE Classic card for NDEF compatibility, configuring MAD sectors and writing proper NDEF structures.

##### DESFire Cards

###### Writing
```bash
cargo run --bin card_writer data1.json
```
This detects the card type and if it's a DESFire, it will show DESFire setup instructions and optionally attempt automatic setup.

##### Generic NDEF Operations

###### Reading
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

###### Writing to Type 2 Tags (NTAG, MIFARE Ultralight)
The `card_writer` can handle Type 2 tags by detecting them and using the appropriate writing method:
```bash
cargo run --bin card_writer data1.json
```

###### Writing to Type 4 Tags (like DESFire)
Also handled by `card_writer` with dedicated operations for Type 4 tags:
```bash
cargo run --bin card_writer data1.json
```

#### Other Useful Commands

##### Sending Raw Commands
```bash
cargo run --bin raw_command
```
Allows you to send custom APDU commands to any card type for advanced operations or troubleshooting.

##### COVID Test Writing
```bash
cargo run --bin covid_test_writer
```
Specialized tool for writing COVID-19 test results to NTAG213 tags with minimal data format.

#### Step-by-Step Process for Each Card Type

##### MIFARE Classic
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

##### DESFire Cards
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

##### MIFARE Ultralight/NTAG (Type 2 Tags)
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

##### COVID Test Tags
1. **Generate and write test data:**
   ```bash
   cargo run --bin covid_test_writer
   ```

2. **Read with the mobile app:**
   - Open the DESFire Emulator app
   - Use the COVID-19 Test Reader
   - Hold the tag against your phone

#### JSON Data Format

The JSON files used for writing should contain data in the CardExport format:

```json
{
  "id": "card_1743832619319",
  "name": "Card 07:09:00:00:03:00:00:02",
  "applicationId": 1,
  "fileId": 1,
  "fileData": "Hello World",
  "format": "ntag_213",
  "exportDate": "2025-04-04T18:55:52.938Z"
}
```

For cross-format compatibility (e.g., writing DESFire NDEF data to NTAG213), use the `--force` flag with the appropriate writer tool.

For COVID test data, the writer uses a specialized compact format:
```json
{"res":"p","ts":1743879763,"exp":1744138963,"lot":"25-04-123","mfg":1736899200,"shf":6}
```
