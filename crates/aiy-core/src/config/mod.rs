//! Configuration system for the All-in-Yum pipeline.
//!
//! This module provides configuration management for the multi-agent consensus pipeline.
//!
//! ## Security Note
//!
//! Configuration files must NEVER contain API keys, passwords, or other secrets.
//! All sensitive credentials are managed through the `security` module's credential system.

mod pipeline;

pub use pipeline::PipelineConfig;
