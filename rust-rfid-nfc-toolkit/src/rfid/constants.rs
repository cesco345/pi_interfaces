// Constants for MFRC522 RFID/NFC module
// =======================================

// Hardware pins
pub const SPI_BUS: u8 = 0;
pub const SPI_DEVICE: u8 = 0;
pub const RESET_PIN: u8 = 25; // GPIO pin for reset (BCM numbering)

// MFRC522 Commands
pub const COMMAND_IDLE: u8 = 0x00;
pub const COMMAND_CALC_CRC: u8 = 0x03;
pub const COMMAND_TRANSMIT: u8 = 0x04;
pub const COMMAND_RECEIVE: u8 = 0x08;
pub const COMMAND_TRANSCEIVE: u8 = 0x0C;
pub const COMMAND_MF_AUTHENT: u8 = 0x0E;
pub const COMMAND_SOFT_RESET: u8 = 0x0F;

// MFRC522 Registers
pub const REG_COMMAND: u8 = 0x01;
pub const REG_COM_I_EN: u8 = 0x02;
pub const REG_DIV_I_EN: u8 = 0x03;
pub const REG_COM_IRQ: u8 = 0x04;
pub const REG_DIV_IRQ: u8 = 0x05;
pub const REG_ERROR: u8 = 0x06;
pub const REG_STATUS1: u8 = 0x07;
pub const REG_STATUS2: u8 = 0x08;
pub const REG_FIFO_DATA: u8 = 0x09;
pub const REG_FIFO_LEVEL: u8 = 0x0A;
pub const REG_CONTROL: u8 = 0x0C;
pub const REG_BIT_FRAMING: u8 = 0x0D;
pub const REG_MODE: u8 = 0x11;
pub const REG_TX_CONTROL: u8 = 0x14;
pub const REG_TX_AUTO: u8 = 0x15;
pub const REG_CRC_RESULT_H: u8 = 0x21;
pub const REG_CRC_RESULT_L: u8 = 0x22;
pub const REG_VERSION: u8 = 0x37;

// PICC Commands (ISO 14443A)
pub const PICC_REQIDL: u8 = 0x26;      // Request in idle mode
pub const PICC_REQALL: u8 = 0x52;      // Request all cards
pub const PICC_ANTICOLL: u8 = 0x93;    // Anticollision
pub const PICC_SELECTTAG: u8 = 0x93;   // Select tag
pub const PICC_AUTHENT1A: u8 = 0x60;   // Authentication with key A
pub const PICC_AUTHENT1B: u8 = 0x61;   // Authentication with key B
pub const PICC_READ: u8 = 0x30;        // Read block
pub const PICC_WRITE: u8 = 0xA0;       // Write block
pub const PICC_HALT: u8 = 0x50;        // Halt command

// Timeouts and operation parameters
pub const CARD_DETECTION_TIMEOUT_SECS: u64 = 8;
pub const KEY_TESTING_TIMEOUT_SECS: u64 = 15;
pub const SPI_FREQUENCY_HZ: u32 = 5_000; // 5 kHz for better compatibility
