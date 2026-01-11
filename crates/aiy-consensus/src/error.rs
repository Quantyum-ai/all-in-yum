//! Error types for the consensus engine.

use thiserror::Error;

/// Errors that can occur during consensus operations.
#[derive(Debug, Error)]
pub enum ConsensusError {
    /// No reviews were provided for consensus.
    #[error("No reviews provided for consensus")]
    NoReviews,

    /// Not enough reviews to reach consensus with the given strategy.
    #[error("Insufficient reviews: need at least {required}, got {provided}")]
    InsufficientReviews {
        /// The required number of reviews.
        required: usize,
        /// The number of reviews provided.
        provided: usize,
    },

    /// An adapter failed during review.
    #[error("Agent '{agent_id}' failed: {message}")]
    AgentFailure {
        /// The ID of the agent that failed.
        agent_id: String,
        /// The error message (sanitized to avoid leaking secrets).
        message: String,
    },

    /// Timeout waiting for agent response.
    #[error("Agent '{agent_id}' timed out after {timeout_ms}ms")]
    AgentTimeout {
        /// The ID of the agent that timed out.
        agent_id: String,
        /// The timeout duration in milliseconds.
        timeout_ms: u64,
    },

    /// Failed to aggregate reviews.
    #[error("Aggregation failed: {0}")]
    AggregationFailed(String),

    /// Invalid voting configuration.
    #[error("Invalid voting configuration: {0}")]
    InvalidConfiguration(String),

    /// Failed to serialize or deserialize data.
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl ConsensusError {
    /// Create a new agent failure error.
    pub fn agent_failure(agent_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::AgentFailure {
            agent_id: agent_id.into(),
            message: message.into(),
        }
    }

    /// Create a new agent timeout error.
    pub fn agent_timeout(agent_id: impl Into<String>, timeout_ms: u64) -> Self {
        Self::AgentTimeout {
            agent_id: agent_id.into(),
            timeout_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_reviews_error() {
        let err = ConsensusError::NoReviews;
        assert_eq!(err.to_string(), "No reviews provided for consensus");
    }

    #[test]
    fn test_insufficient_reviews_error() {
        let err = ConsensusError::InsufficientReviews {
            required: 3,
            provided: 1,
        };
        assert_eq!(
            err.to_string(),
            "Insufficient reviews: need at least 3, got 1"
        );
    }

    #[test]
    fn test_agent_failure_error() {
        let err = ConsensusError::agent_failure("grok", "Connection refused");
        assert_eq!(
            err.to_string(),
            "Agent 'grok' failed: Connection refused"
        );
    }

    #[test]
    fn test_agent_timeout_error() {
        let err = ConsensusError::agent_timeout("claude", 30000);
        assert_eq!(
            err.to_string(),
            "Agent 'claude' timed out after 30000ms"
        );
    }
}
