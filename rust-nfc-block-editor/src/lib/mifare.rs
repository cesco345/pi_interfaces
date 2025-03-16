// Re-export modules
pub mod access;
pub mod operations;
pub mod admin;
pub mod dump;
pub mod block_editor;


// Re-export common items for convenience
pub use access::AccessBits;
pub use operations::{read_card_uid, wait_for_card_removal, read_sector_data, 
                    write_block_data, write_block_raw, DEFAULT_KEYS};
pub use admin::{modify_sector_access, change_sector_keys, format_card};
pub use dump::{dump_card, dump_sector};
pub use block_editor::{read_block, write_block, create_sector_trailer, 
                     format_text_block, interactive_edit};
