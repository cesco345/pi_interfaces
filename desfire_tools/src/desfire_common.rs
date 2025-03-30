// This module re-exports functionality from other modules
// to maintain backward compatibility with existing code

// Re-export from card module
pub use crate::card::{
    connect_to_card,
    send_apdu,
    verify_desfire_card
};

// Re-export from crypto module
pub use crate::crypto::{
    authenticate_des,
    des_encrypt,
    des_decrypt,
    DEFAULT_MASTER_KEY
};

// Re-export from util module
pub use crate::util::{
    HexSlice,
    prompt_card_action
};

// Re-export from error module
pub use crate::error::print_desfire_error;

// Re-export from application module
pub use crate::application::{
    create_application,
    select_application,
    list_applications,
    delete_application
};

// Re-export from file_operations module
pub use crate::file_operations::{
    create_standard_file,
    write_data,
    read_data,
    create_value_file,
    create_record_file
};
