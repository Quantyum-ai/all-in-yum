//! Plan parsing from cloud response
//!
//! Parses execution plans from cloud model responses.

use super::error::OrchestrationError;
use super::types::{ExecutionPlan, PlanTask, TaskType};
use serde::Deserialize;
use uuid::Uuid;

/// Raw plan response from cloud
#[derive(Debug, Deserialize)]
struct RawPlanResponse {
    #[serde(default)]
    description: String,
    #[serde(default)]
    tasks: Vec<RawTask>,
}

/// Raw task from cloud response
#[derive(Debug, Deserialize)]
struct RawTask {
    #[serde(default)]
    task_type: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    priority: Option<u8>,
    #[serde(default)]
    effort: Option<u8>,
}

/// Plan parser for cloud responses
pub struct PlanParser;

impl PlanParser {
    /// Parse an execution plan from JSON response
    pub fn parse_json(response: &str) -> Result<ExecutionPlan, OrchestrationError> {
        // Try to extract JSON from response (might be wrapped in markdown)
        let json_str = Self::extract_json(response)?;

        // Parse the JSON
        let raw: RawPlanResponse = serde_json::from_str(&json_str)
            .map_err(|e| OrchestrationError::plan_parsing(format!("Invalid JSON: {}", e)))?;

        Self::convert_raw_plan(raw)
    }

    /// Parse a plan from structured data
    pub fn from_tasks(
        description: impl Into<String>,
        tasks: Vec<(TaskType, String)>,
    ) -> ExecutionPlan {
        let mut plan = ExecutionPlan::new(
            format!("PLAN_{}", &Uuid::new_v4().to_string()[..8]),
            description,
        );

        for (idx, (task_type, desc)) in tasks.into_iter().enumerate() {
            let task_id = format!("TASK_{:03}", idx + 1);
            plan.add_task(PlanTask {
                id: task_id,
                task_type,
                description: desc,
                dependencies: Vec::new(),
                priority: (idx + 1) as u8,
                estimated_effort: 1,
            });
        }

        plan
    }

    /// Extract JSON from response (handles markdown code blocks)
    fn extract_json(response: &str) -> Result<String, OrchestrationError> {
        let trimmed = response.trim();

        // If it starts with {, assume it's raw JSON
        if trimmed.starts_with('{') {
            return Ok(trimmed.to_string());
        }

        // Try to extract from markdown code block
        if let Some(start) = trimmed.find("```json") {
            let after_marker = &trimmed[start + 7..];
            if let Some(end) = after_marker.find("```") {
                return Ok(after_marker[..end].trim().to_string());
            }
        }

        // Try plain code block
        if let Some(start) = trimmed.find("```") {
            let after_marker = &trimmed[start + 3..];
            if let Some(end) = after_marker.find("```") {
                let content = after_marker[..end].trim();
                // Skip language identifier if present
                if let Some(newline) = content.find('\n') {
                    let possible_json = content[newline..].trim();
                    if possible_json.starts_with('{') {
                        return Ok(possible_json.to_string());
                    }
                }
                if content.starts_with('{') {
                    return Ok(content.to_string());
                }
            }
        }

        // Last resort: find first { and last }
        if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
            if start < end {
                return Ok(trimmed[start..=end].to_string());
            }
        }

        Err(OrchestrationError::plan_parsing(
            "Could not extract JSON from response",
        ))
    }

    /// Convert raw plan to ExecutionPlan
    fn convert_raw_plan(raw: RawPlanResponse) -> Result<ExecutionPlan, OrchestrationError> {
        let plan_id = format!("PLAN_{}", &Uuid::new_v4().to_string()[..8]);
        let mut plan = ExecutionPlan::new(plan_id, raw.description);

        for (idx, raw_task) in raw.tasks.into_iter().enumerate() {
            let task_type = Self::parse_task_type(&raw_task.task_type);
            let task_id = format!("TASK_{:03}", idx + 1);

            plan.add_task(PlanTask {
                id: task_id,
                task_type,
                description: raw_task.description,
                dependencies: raw_task.dependencies,
                priority: raw_task.priority.unwrap_or((idx + 1) as u8),
                estimated_effort: raw_task.effort.unwrap_or(1),
            });
        }

        Ok(plan)
    }

    /// Parse task type from string
    fn parse_task_type(s: &str) -> TaskType {
        match s.to_lowercase().as_str() {
            "create_file" | "create" | "new" => TaskType::CreateFile,
            "modify_file" | "modify" | "update" | "edit" => TaskType::ModifyFile,
            "delete_file" | "delete" | "remove" => TaskType::DeleteFile,
            "run_tests" | "test" | "tests" => TaskType::RunTests,
            "verify" | "verification" | "check" => TaskType::Verify,
            "refactor" => TaskType::Refactor,
            "document" | "docs" | "documentation" => TaskType::Document,
            _ => TaskType::Generic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_json() {
        let json = r#"{
            "description": "Implement feature",
            "tasks": [
                {"task_type": "create_file", "description": "Create FILE_001"},
                {"task_type": "modify", "description": "Update FILE_002", "dependencies": ["TASK_001"]}
            ]
        }"#;

        let plan = PlanParser::parse_json(json).unwrap();
        assert_eq!(plan.task_count(), 2);
        assert_eq!(plan.tasks[0].task_type, TaskType::CreateFile);
        assert_eq!(plan.tasks[1].task_type, TaskType::ModifyFile);
    }

    #[test]
    fn test_parse_json_in_markdown() {
        let response = r#"Here's the plan:

```json
{
    "description": "Test",
    "tasks": [
        {"task_type": "test", "description": "Run tests"}
    ]
}
```

Done!"#;

        let plan = PlanParser::parse_json(response).unwrap();
        assert_eq!(plan.task_count(), 1);
        assert_eq!(plan.tasks[0].task_type, TaskType::RunTests);
    }

    #[test]
    fn test_from_tasks() {
        let tasks = vec![
            (TaskType::CreateFile, "Create new module".to_string()),
            (TaskType::ModifyFile, "Update imports".to_string()),
            (TaskType::RunTests, "Run tests".to_string()),
        ];

        let plan = PlanParser::from_tasks("Implement feature", tasks);
        assert_eq!(plan.task_count(), 3);
        assert!(plan.plan_id.starts_with("PLAN_"));
        assert_eq!(plan.tasks[0].id, "TASK_001");
        assert_eq!(plan.tasks[1].id, "TASK_002");
    }

    #[test]
    fn test_parse_task_type() {
        assert_eq!(PlanParser::parse_task_type("create_file"), TaskType::CreateFile);
        assert_eq!(PlanParser::parse_task_type("MODIFY"), TaskType::ModifyFile);
        assert_eq!(PlanParser::parse_task_type("delete"), TaskType::DeleteFile);
        assert_eq!(PlanParser::parse_task_type("test"), TaskType::RunTests);
        assert_eq!(PlanParser::parse_task_type("unknown"), TaskType::Generic);
    }

    #[test]
    fn test_extract_json_raw() {
        let result = PlanParser::extract_json(r#"{"key": "value"}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_json_not_found() {
        let result = PlanParser::extract_json("No JSON here");
        assert!(result.is_err());
    }
}
