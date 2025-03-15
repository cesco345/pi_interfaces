use anyhow::Result;
use env_logger::Builder;
use log::LevelFilter;

/// Initialize logging with customizable verbosity
pub fn init_logging(verbose: bool) -> Result<()> {
    let mut builder = Builder::new();
    
    if verbose {
        builder.filter_level(LevelFilter::Info);
        builder.filter_module("test_writer", LevelFilter::Info);
    } else {
        builder.filter_level(LevelFilter::Warn);
    }
    
    builder.init();
    
    Ok(())
}
