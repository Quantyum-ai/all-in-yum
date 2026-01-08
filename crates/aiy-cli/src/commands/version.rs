//! Version command implementation

/// Run the version command
pub fn run() -> anyhow::Result<()> {
    println!("aiy {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
