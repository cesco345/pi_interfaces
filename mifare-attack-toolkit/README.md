# Mifare Attack Implementation Guide

This guide explains the core attack techniques implemented in the toolkit and how they relate to the Proxmark3 source code you were interested in.

## Mifare Classic Key Structure

Mifare Classic cards use Crypto1 encryption with 48-bit keys. Each sector has two keys:
- Key A - Used for most operations
- Key B - Often used for special operations or configuration

## Attack Techniques

### 1. Nested Attack

**Source files from Proxmark3:**
- `client/src/mifare/mfkey.c`
- `client/src/cmdhfmf.c` (nested attack function)

**How it works:**
1. You need at least one known key for any sector on the card
2. The attack exploits the fact that random number generators on these cards are sometimes predictable
3. It collects authentication nonces from the card
4. It uses these nonces to recover keys for other sectors

**Implementation details:**
- The `NestedAttack` struct in `crypto1.rs` handles this
- Key recovery is performed using the collected nonces
- The implementation requires precise timing to capture nonces

### 2. Darkside Attack

**Source files from Proxmark3:**
- `client/src/cmdhfmf.c` (darkside attack function)

**How it works:**
1. The attack exploits a vulnerability in the authentication process
2. It sends a special crafted authentication command
3. Some vulnerable cards will respond with information that leaks key bits
4. Multiple attempts can recover enough key bits to reconstruct the full key

**Implementation details:**
- The `DarksideAttack` struct in `crypto1.rs` implements this
- Success depends on card vulnerability (works on original Mifare Classic, not on most modern clones)
- Requires sending precisely timed and malformed authentication commands

### 3. Magic Card Operations

**Source files from Proxmark3:**
- `client/src/mifare/gen1a.c`
- `client/src/mifare/gen2.c`

**How it works:**
1. Magic cards have special commands that bypass normal security
2. Different generations (Gen1A, Gen2, CUID) use different commands
3. These commands allow writing to normally protected blocks, including block 0 (UID)

**Implementation details:**
- The `MagicCard` struct in `crypto1.rs` handles different magic card types
- Detection routines identify which backdoor commands to use
- UID writing functions implement the specific command sequences for each card type

### 4. Default Key Testing

**Source files from Proxmark3:**
- `client/src/cmdhfmf.c` (default keys array)

**How it works:**
1. Many cards are deployed with default keys
2. The toolkit tries common default keys (FFFFFFFFFFFF, A0A1A2A3A4A5, etc.)
3. Once a valid key is found, it can read or write to the respective sector

**Implementation details:**
- The `DEFAULT_KEYS` array in `mfrc522.rs` contains common default keys
- The code tries each key in sequence until authentication succeeds

## MFRC522 Interface Details

The toolkit uses the MFRC522 RFID reader module for communication. The key functions include:

1. **Authentication:** Communicates with the card using the Crypto1 protocol
2. **Reading/Writing:** Transfers data between the reader and card
3. **Direct commands:** For magic card operations and special commands
4. **Timing attacks:** For nested and darkside attacks

## Crypto1 Implementation

The Crypto1 cipher implementation is based on the Proxmark3 code but rewritten in Rust:

1. **Filter function:** Implements the nonlinear filter used in Crypto1
2. **LFSR clocking:** Manages the internal state of the cipher
3. **Key setup:** Initializes the cipher with a 48-bit key
4. **Encryption/Decryption:** Handles the actual data transformation

## Extending with Additional Attacks

To implement additional attacks from the Proxmark3 codebase:

### PRNG Attack
**Source:** `client/src/cmdhfmf.c`

This attack exploits weak pseudorandom number generators in some cards:
1. Collect multiple authentication nonces
2. Analyze patterns in the nonces
3. Predict future nonces and use them to recover keys

### Hardnested Attack
**Source:** `client/src/cmdhfmfhard.c`

This is the most complex attack:
1. Uses statistically-driven nested attack
2. Requires collecting many partial authentications
3. Uses a statistical model to narrow down key space
4. Can break cards that resist simpler attacks

## Performance Optimizations

For best performance running these attacks on the Raspberry Pi:

1. Use release builds with optimizations enabled
2. Consider using multithreading for compute-intensive operations
3. Implement the most critical parts (like bit manipulations) using bitwise operations
4. Use the `no_std` approach for the crypto code to minimize overhead

## Testing Your Implementation

You should test your implementation on different card types:

1. Original Mifare Classic cards (most vulnerable)
2. Chinese clones (vary in vulnerability)
3. Magic cards (Gen1A, Gen2, CUID)
4. More secure cards like Mifare Plus (to verify they resist attacks)

## Additional Resources

For further reading on the attacks implemented:

1. **Nested Attack:** "Dismantling MIFARE Classic" by Flavio Garcia et al.
2. **Darkside Attack:** "The Dark Side of MIFARE Classic" by Nijmegen Radboud University
3. **Magic Cards:** Documentation from proxmark.org forums
4. **Practical Attacks:** "Practical attacks on NFC and RFID Systems" presented at DEF CON
