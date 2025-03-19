// MFRC522 Commands
pub const PCD_IDLE: u8 = 0x00;
pub const PCD_AUTHENT: u8 = 0x0E;
pub const PCD_RECEIVE: u8 = 0x08;
pub const PCD_TRANSMIT: u8 = 0x04;
pub const PCD_TRANSCEIVE: u8 = 0x0C;
pub const PCD_RESETPHASE: u8 = 0x0F;
pub const PCD_CALCCRC: u8 = 0x03;

// MIFARE Commands
pub const PICC_REQIDL: u8 = 0x26;
pub const PICC_REQALL: u8 = 0x52;
pub const PICC_ANTICOLL: u8 = 0x93;
pub const PICC_SELECTTAG: u8 = 0x93;
pub const PICC_AUTHENT1A: u8 = 0x60;
pub const PICC_AUTHENT1B: u8 = 0x61;
pub const PICC_READ: u8 = 0x30;
pub const PICC_WRITE: u8 = 0xA0;
pub const PICC_DECREMENT: u8 = 0xC0;
pub const PICC_INCREMENT: u8 = 0xC1;
pub const PICC_RESTORE: u8 = 0xC2;
pub const PICC_TRANSFER: u8 = 0xB0;
pub const PICC_HALT: u8 = 0x50;

// Status codes
pub const MI_OK: u8 = 0;
pub const MI_NOTAGERR: u8 = 1;
pub const MI_ERR: u8 = 2;

// MFRC522 Registers
pub const COMMAND_REG: u8 = 0x01;
pub const COM_IEN_REG: u8 = 0x02;
pub const DIV_IEN_REG: u8 = 0x03;
pub const COM_IRQ_REG: u8 = 0x04;
pub const DIV_IRQ_REG: u8 = 0x05;
pub const ERROR_REG: u8 = 0x06;
pub const STATUS1_REG: u8 = 0x07;
pub const STATUS2_REG: u8 = 0x08;
pub const FIFO_DATA_REG: u8 = 0x09;
pub const FIFO_LEVEL_REG: u8 = 0x0A;
pub const WATER_LEVEL_REG: u8 = 0x0B;
pub const CONTROL_REG: u8 = 0x0C;
pub const BIT_FRAMING_REG: u8 = 0x0D;
pub const COLL_REG: u8 = 0x0E;

pub const MODE_REG: u8 = 0x11;
pub const TX_MODE_REG: u8 = 0x12;
pub const RX_MODE_REG: u8 = 0x13;
pub const TX_CONTROL_REG: u8 = 0x14;
pub const TX_AUTO_REG: u8 = 0x15;
pub const TX_SEL_REG: u8 = 0x16;
pub const RX_SEL_REG: u8 = 0x17;
pub const RX_THRESHOLD_REG: u8 = 0x18;
pub const DEMOD_REG: u8 = 0x19;
pub const MIFARE_REG: u8 = 0x1C;
pub const SERIAL_SPEED_REG: u8 = 0x1F;

pub const CRC_RESULT_REG_M: u8 = 0x21;
pub const CRC_RESULT_REG_L: u8 = 0x22;
pub const MOD_WIDTH_REG: u8 = 0x24;
pub const RF_CFG_REG: u8 = 0x26;
pub const GS_N_REG: u8 = 0x27;
pub const CW_GS_P_REG: u8 = 0x28;
pub const MOD_GS_P_REG: u8 = 0x29;
pub const T_MODE_REG: u8 = 0x2A;
pub const T_PRESCALER_REG: u8 = 0x2B;
pub const T_RELOAD_REG_H: u8 = 0x2C;
pub const T_RELOAD_REG_L: u8 = 0x2D;

pub const VERSION_REG: u8 = 0x37;

pub const MAX_LEN: usize = 16;
