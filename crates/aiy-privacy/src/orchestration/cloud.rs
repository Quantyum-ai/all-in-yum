//! Cloud communicator for privacy mode
//!
//! Sends redacted context to cloud models and receives execution plans.
//! NEVER sends raw code, file paths, or identifiers to cloud.

use super::error::{OrchestrationError, OrchestrationResult};
use super::plan::PlanParser;
use super::types::ExecutionPlan;
use crate::enforcement::{PrivacyGuard, Redactor};
use crate::rag::QueryResult;

/// System prompt for cloud planner
#[cfg(feature = "http")]
const CLOUD_PLANNER_PROMPT: &str = r#"You are an AI assistant helping with code development in privacy mode.

IMPORTANT: You will ONLY receive redacted, abstract descriptions of code. You will NEVER see actual:
- File paths (replaced with FILE_001, FILE_002, etc.)
- Function names (replaced with FUNC_001, FUNC_002, etc.)
- Variable names (replaced with SYM_001, etc.)
- Type names (replaced with TYPE_001, etc.)
- Module names (replaced with MOD_001, etc.)
- Actual source code

Your task is to create a high-level execution plan that a local model will implement.

For each request, respond with a JSON execution plan:
{
    "description": "High-level description of what to do",
    "tasks": [
        {
            "task_type": "create_file|modify_file|delete_file|run_tests|verify|refactor|document",
            "description": "What to do (using opaque identifiers)",
            "dependencies": ["TASK_ID of tasks this depends on"],
            "priority": 1-10 (1 is highest),
            "effort": 1-10 (estimated complexity)
        }
    ]
}

Focus on the WHAT, not the HOW. The local model will handle implementation details."#;

/// Cloud communicator for sending redacted requests
pub struct CloudCommunicator {
    /// Privacy guard for validating outgoing content
    guard: PrivacyGuard,
    /// Agent for cloud communication (trait object for testing)
    #[cfg(feature = "http")]
    agent: Option<Box<dyn CloudAgent>>,
}

/// Trait for cloud agent operations
#[cfg(feature = "http")]
#[async_trait::async_trait]
pub trait CloudAgent: Send + Sync {
    /// Send a message and get response
    async fn send(&self, system_prompt: &str, user_message: &str) -> Result<String, String>;
}

impl Default for CloudCommunicator {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudCommunicator {
    /// Create a new cloud communicator
    pub fn new() -> Self {
        Self {
            guard: PrivacyGuard::new(),
            #[cfg(feature = "http")]
            agent: None,
        }
    }

    /// Set the cloud agent
    #[cfg(feature = "http")]
    pub fn with_agent(mut self, agent: Box<dyn CloudAgent>) -> Self {
        self.agent = Some(agent);
        self
    }

    /// Request an execution plan from cloud model
    ///
    /// Takes:
    /// - User request (must be redacted)
    /// - RAG context (already redacted)
    /// - Redactor for any additional redaction needed
    pub async fn request_plan(
        &self,
        redacted_request: &str,
        rag_context: Option<&QueryResult>,
        _redactor: &mut Redactor,
    ) -> OrchestrationResult<ExecutionPlan> {
        // Final safety check - ensure no violations
        if !self.guard.is_safe(redacted_request) {
            return Err(OrchestrationError::PrivacyViolation);
        }

        // Build user message
        let user_message = self.build_user_message(redacted_request, rag_context);

        // Final safety check on complete message
        if !self.guard.is_safe(&user_message) {
            return Err(OrchestrationError::PrivacyViolation);
        }

        #[cfg(feature = "http")]
        {
            if let Some(ref agent) = self.agent {
                let response = agent
                    .send(CLOUD_PLANNER_PROMPT, &user_message)
                    .await
                    .map_err(OrchestrationError::cloud)?;

                return PlanParser::parse_json(&response);
            }
        }

        // Without HTTP feature or agent, create a simple plan
        Ok(self.create_fallback_plan(redacted_request))
    }

    /// Build user message with context
    fn build_user_message(&self, request: &str, rag_context: Option<&QueryResult>) -> String {
        let mut message = String::new();

        // Add request
        message.push_str("USER REQUEST:\n");
        message.push_str(request);
        message.push('\n');

        // Add RAG context if available
        if let Some(context) = rag_context {
            if !context.is_empty() {
                message.push_str("\nCODEBASE CONTEXT (redacted):\n");
                for (i, chunk) in context.chunks.iter().enumerate() {
                    message.push_str(&format!(
                        "\n--- Chunk {} (similarity: {:.2}) ---\n",
                        i + 1,
                        chunk.similarity
                    ));
                    // Only include metadata, not actual content
                    message.push_str(&format!("Type: {:?}\n", chunk.chunk_type));
                    message.push_str(&format!("Tokens: {}\n", chunk.token_count));
                }
            }
        }

        message
    }

    /// Create a fallback plan when cloud is unavailable
    fn create_fallback_plan(&self, request: &str) -> ExecutionPlan {
        use super::types::TaskType;

        // Create a simple single-task plan
        PlanParser::from_tasks(
            "Execute user request locally",
            vec![(TaskType::Generic, format!("Implement: {}", request))],
        )
    }

    /// Validate content before sending
    pub fn validate(&self, content: &str) -> bool {
        self.guard.is_safe(content)
    }
}

/// Request builder for cloud communication
pub struct CloudRequestBuilder {
    request: String,
    context_files: Vec<String>,
    context_summaries: Vec<String>,
}

impl Default for CloudRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudRequestBuilder {
    /// Create a new request builder
    pub fn new() -> Self {
        Self {
            request: String::new(),
            context_files: Vec::new(),
            context_summaries: Vec::new(),
        }
    }

    /// Set the main request (must be redacted)
    pub fn with_request(mut self, request: impl Into<String>) -> Self {
        self.request = request.into();
        self
    }

    /// Add context file reference (must be opaque ID like FILE_001)
    pub fn add_context_file(mut self, file_ref: impl Into<String>) -> Self {
        self.context_files.push(file_ref.into());
        self
    }

    /// Add context summary (must be redacted)
    pub fn add_context_summary(mut self, summary: impl Into<String>) -> Self {
        self.context_summaries.push(summary.into());
        self
    }

    /// Build the request message
    pub fn build(&self) -> String {
        let mut message = self.request.clone();

        if !self.context_files.is_empty() {
            message.push_str("\n\nRelevant files: ");
            message.push_str(&self.context_files.join(", "));
        }

        if !self.context_summaries.is_empty() {
            message.push_str("\n\nContext:\n");
            for summary in &self.context_summaries {
                message.push_str("- ");
                message.push_str(summary);
                message.push('\n');
            }
        }

        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_communicator_validation() {
        let comm = CloudCommunicator::new();

        // Safe content should pass
        assert!(comm.validate("FILE_001 has 3 functions"));
        assert!(comm.validate("FUNC_001 needs refactoring"));

        // Unsafe content should fail
        assert!(!comm.validate("fn authenticate_user() {}"));
        assert!(!comm.validate("/home/user/project/src/main.rs"));
    }

    #[test]
    fn test_request_builder() {
        let request = CloudRequestBuilder::new()
            .with_request("Add logging to FUNC_001")
            .add_context_file("FILE_001")
            .add_context_file("FILE_002")
            .add_context_summary("FILE_001 contains FUNC_001 and FUNC_002")
            .build();

        assert!(request.contains("Add logging to FUNC_001"));
        assert!(request.contains("FILE_001"));
        assert!(request.contains("FILE_002"));
    }

    #[test]
    fn test_fallback_plan_creation() {
        let comm = CloudCommunicator::new();
        let plan = comm.create_fallback_plan("Add tests for FILE_001");

        assert_eq!(plan.task_count(), 1);
        assert!(plan.description.contains("locally"));
    }

    #[tokio::test]
    async fn test_request_plan_privacy_violation() {
        let comm = CloudCommunicator::new();
        let mut redactor = Redactor::new();

        // Should fail with actual code
        let result = comm
            .request_plan("fn main() {}", None, &mut redactor)
            .await;
        assert!(matches!(result, Err(OrchestrationError::PrivacyViolation)));
    }
}
