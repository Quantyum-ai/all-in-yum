//! Workflow management for privacy mode
//!
//! This module provides workflow state management that can be persisted.
//!
//! # Security
//!
//! The workflow state contains ONLY metadata (counts, timestamps, state names).
//! It NEVER contains:
//! - File paths (real or opaque)
//! - Code content
//! - Identifier mappings
//! - Task descriptions
//!
//! This ensures that persisted state cannot leak information about the codebase.

pub mod error;
pub mod state;

pub use error::{WorkflowError, WorkflowResult};
pub use state::{WorkflowState, WorkflowStateName};
