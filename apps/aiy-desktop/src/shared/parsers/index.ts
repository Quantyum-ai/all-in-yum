/**
 * CLI Output Parsers
 *
 * This module exports all parsers for aiy CLI command output.
 * Parsers are designed to be resilient to format drift and never throw errors.
 *
 * @module parsers
 */

// Re-export all types
export type {
  CheckMarker,
  CheckResult,
  PrivacyCheckResult,
  PrivacyStatusResult,
  WorkflowState,
  WorkflowStatusResult,
  KnownAgentId,
  AgentInfo,
  AgentsListResult,
  AgentStatusEntry,
  AgentsStatusResult,
  CredentialProvider,
  CredentialStatus,
  CredentialsStatusResult,
  ParserError,
  ParseResult,
} from './types';

// Privacy check parser
export {
  parsePrivacyCheck,
  extractFailures,
  extractWarnings,
  formatCheckResult,
  getCheckSummary,
} from './privacy-check';

// Privacy status parser
export {
  parsePrivacyStatus,
  isOllamaConfigured,
  getStatusSummary,
} from './privacy-status';

// Workflow status parser
export {
  parseWorkflowStatus,
  isTerminalState,
  isActiveState,
  getProgressPercent,
  formatDuration,
  getWorkflowSummary,
} from './workflow-status';

// Agents parser
export {
  parseAgentsList,
  parseAgentsStatus,
  getAgentsNeedingCredentials,
  getEnabledAgents,
  getAgentById,
} from './agents';

// Credentials parser
export {
  parseCredentialsStatus,
  getMissingProviders,
  getProviderDisplayName,
  isProviderConfigured,
  getCredentialsSummary,
} from './credentials';
