//! # aiy-adapters
//!
//! Agent adapters for All-in-Yum multi-agent consensus pipeline.
//!
//! This crate provides trait definitions and implementations for
//! connecting to various AI agent backends (Claude, Codex, Gemini, etc.)

#![warn(missing_docs)]

pub mod retry;
/// Core trait definitions and shared types for agent adapters.
pub mod traits;

pub use traits::{
    AdapterError, AdapterErrorKind, AgentAdapter, AgentReview, Issue, RetryConfig, Severity,
    Verdict,
};
