//! Manual integration tests for verification engine
//! These require real cargo/rustc and are marked #[ignore] for CI

use aiy_privacy::verification::*;
use tempfile::TempDir;

fn create_temp_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    // Create minimal Cargo.toml
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    // Create src directory
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
    dir
}

#[tokio::test]
#[ignore] // Manual integration test - requires real cargo installation
async fn test_engine_run_on_valid_project() {
    let temp = create_temp_project();
    let config = EngineConfig::new(temp.path()).without_repairs();
    let mut engine = VerificationEngine::new(config);

    let result = engine.run().await;
    // This should work on a minimal valid project
    assert!(result.is_ok());
}
