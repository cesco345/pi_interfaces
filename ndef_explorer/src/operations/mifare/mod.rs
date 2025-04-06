// Export all submodules
pub mod types;
pub mod connection;
pub mod authentication;
pub mod reader;
pub mod card_detection;
pub mod interpreter;
pub mod summary;
pub mod writer;           // Add the new writer module
pub mod ndef_formatter;   // Add the new NDEF formatter module
pub mod data_handler;     // Add the new data handler module

// Re-export important structs and functions for easier imports
pub use types::BlockData;
pub use connection::{connect_to_card, get_card_details};
pub use authentication::authenticate_sector;
pub use reader::{read_mifare_classic_data, read_type2_tag_data, read_desfire_basic_info, attempt_generic_read};
pub use card_detection::detect_card_type;
pub use interpreter::interpret_ndef_data;
pub use summary::display_summary;

// Re-export writer functions
pub use writer::{simple_direct_write, direct_write_mifare, verify_mifare_classic};
pub use ndef_formatter::{format_mifare_classic, create_text_ndef_message, create_ndef_tlv, write_ndef_data_to_card};
pub use data_handler::{load_card_data, check_format_compatibility, prepare_block_data, get_user_confirmation, wait_for_card};
