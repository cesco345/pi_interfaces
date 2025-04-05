// NDEF Explorer - Main library file

// Export main modules
pub mod util;
pub mod commands;
pub mod interpreter;
pub mod operations;
pub mod card_handling;
// Re-export commonly used items
pub use util::ndef_util::{hex_string, protocol_to_string, interpret_status_code};
pub use commands::ndef_commands::send_apdu;
pub use operations::ndef_operations::{
    select_ndef_application,
    read_capability_container,
    read_ndef_length
};
pub use operations::ndef_operations_reader::read_ndef_message;
pub use operations::ndef_operations_writer::write_ndef_message;
pub use operations::ndef_operations_scanner::scan_readable_memory;
