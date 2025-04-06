// NDEF Explorer - Operations module

// Export the operations submodules
pub mod ndef_operations;
pub mod ndef_operations_reader;
pub mod ndef_operations_writer;
pub mod ndef_operations_scanner;
pub mod mifare; // Add the new mifare module

// Re-export commonly used items for easier access
pub use self::ndef_operations::{
    select_ndef_application,
    read_capability_container,
    read_ndef_length
};

pub use self::ndef_operations_reader::{
    read_ndef_message
};

pub use self::ndef_operations_writer::{
    write_ndef_message
};

pub use self::ndef_operations_scanner::{
    scan_readable_memory
};

// Re-export common mifare operations
pub use self::mifare::{
    BlockData,
    connect_to_card,
    get_card_details,
    authenticate_sector,
    detect_card_type,
    display_summary
};
