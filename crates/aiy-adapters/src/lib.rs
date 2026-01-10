//! # aiy-adapters
//!
//! Agent adapters for All-in-Yum multi-agent consensus pipeline.
//!
//! This crate provides trait definitions and implementations for
//! connecting to various AI agent backends (Claude, Codex, Gemini, etc.)

pub mod traits;

pub use traits::{AdapterError, AgentAdapter, AgentReview, Issue, Severity, Verdict};
