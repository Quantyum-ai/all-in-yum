//! # aiy-adapters
//!
//! Agent adapters for All-in-Yum multi-agent consensus pipeline.
//!
//! This crate provides trait definitions and implementations for
//! connecting to various AI agent backends (Claude, Codex, Gemini, etc.)

pub mod retry;
pub mod traits;

pub use traits::{
    AdapterError, AdapterErrorKind, AgentAdapter, AgentReview, Issue, RetryConfig, Severity, Verdict,
};
