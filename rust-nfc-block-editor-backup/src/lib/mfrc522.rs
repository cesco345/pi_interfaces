// Re-export modules
pub mod constants;
pub mod register;
pub mod init;
pub mod communication;
pub mod operations;
pub mod block;

// Re-export common items
pub use constants::*;
pub use register::{read_register, write_register, set_bit_mask, clear_bit_mask};
pub use init::{mfrc522_init, antenna_on, antenna_off};
pub use communication::{mfrc522_to_card, calculate_crc};
pub use operations::{mfrc522_request, mfrc522_anticoll, mfrc522_select_tag, 
                     mfrc522_auth, mfrc522_stop_crypto1};
pub use block::{mfrc522_read, mfrc522_write};
