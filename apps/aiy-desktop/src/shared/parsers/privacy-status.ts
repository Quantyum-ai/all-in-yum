/**
 * Privacy Status Output Parser
 *
 * Parses output from `aiy privacy status` command.
 *
 * Expected output format (from Rust CLI):
 * ```
 * Privacy Mode Status
 * ===================
 *
 * Status: [ENABLED]
 *
 * Local Executor:
 *   Ollama URL: http://127.0.0.1:11434
 *   Model: qwen2.5-coder:14b
 *   Context size: 32768 tokens
 *   Timeout: 120000ms
 *
 * RAG Settings:
 *   Token budget: 4096 tokens
 *   Top-k results: 10
 *   Min similarity: 0.70
 *
 * Verification Limits:
 *   Max fmt repairs: 3
 *   Max clippy repairs: 3
 *   Max test repairs: 3
 *   Max global repairs: 10
 *
 * Exclude Patterns:
 *   - target/**
 *   - node_modules/**
 * ```
 */

import type { PrivacyStatusResult, ParseResult } from './types';

/**
 * Parse privacy status output.
 *
 * @param output - Raw output from `aiy privacy status`
 * @returns Parsed result with configuration details
 */
export function parsePrivacyStatus(output: string): ParseResult<PrivacyStatusResult> {
  // Handle empty input
  if (!output || output.trim().length === 0) {
    return {
      success: false,
      error: {
        type: 'empty_input',
        message: 'Privacy status output is empty',
      },
      rawOutput: output || '',
    };
  }

  const lines = output.split('\n');
  const result: PrivacyStatusResult = {
    enabled: false,
    rawOutput: output,
  };

  let currentSection = '';

  for (const line of lines) {
    const trimmed = line.trim();

    // Check for enabled/disabled status
    if (trimmed.includes('Status:')) {
      result.enabled = trimmed.includes('[ENABLED]');
      continue;
    }

    // Also check for "DISABLED" without brackets or other variants
    if (trimmed.toLowerCase().includes('disabled')) {
      result.enabled = false;
    }

    // Track current section
    if (trimmed === 'Local Executor:') {
      currentSection = 'executor';
      result.localExecutor = {
        ollamaUrl: '',
        model: '',
      };
      continue;
    }
    if (trimmed === 'RAG Settings:') {
      currentSection = 'rag';
      result.ragSettings = {
        tokenBudget: 0,
        topK: 0,
        minSimilarity: 0,
      };
      continue;
    }
    if (trimmed === 'Verification Limits:') {
      currentSection = 'verification';
      result.verificationLimits = {
        maxFmtRepairs: 0,
        maxClippyRepairs: 0,
        maxTestRepairs: 0,
        maxGlobalRepairs: 0,
      };
      continue;
    }
    if (trimmed === 'Exclude Patterns:') {
      currentSection = 'exclude';
      result.excludePatterns = [];
      continue;
    }

    // Parse section content
    if (currentSection === 'executor' && result.localExecutor) {
      const urlMatch = trimmed.match(/Ollama URL:\s*(.+)/);
      if (urlMatch) result.localExecutor.ollamaUrl = urlMatch[1].trim();

      const modelMatch = trimmed.match(/Model:\s*(.+)/);
      if (modelMatch) result.localExecutor.model = modelMatch[1].trim();

      const contextMatch = trimmed.match(/Context size:\s*(\d+)/);
      if (contextMatch) result.localExecutor.contextSize = parseInt(contextMatch[1], 10);

      const timeoutMatch = trimmed.match(/Timeout:\s*(\d+)/);
      if (timeoutMatch) result.localExecutor.timeout = parseInt(timeoutMatch[1], 10);
    }

    if (currentSection === 'rag' && result.ragSettings) {
      const budgetMatch = trimmed.match(/Token budget:\s*(\d+)/);
      if (budgetMatch) result.ragSettings.tokenBudget = parseInt(budgetMatch[1], 10);

      const topKMatch = trimmed.match(/Top-k results:\s*(\d+)/);
      if (topKMatch) result.ragSettings.topK = parseInt(topKMatch[1], 10);

      const simMatch = trimmed.match(/Min similarity:\s*([\d.]+)/);
      if (simMatch) result.ragSettings.minSimilarity = parseFloat(simMatch[1]);
    }

    if (currentSection === 'verification' && result.verificationLimits) {
      const fmtMatch = trimmed.match(/Max fmt repairs:\s*(\d+)/);
      if (fmtMatch) result.verificationLimits.maxFmtRepairs = parseInt(fmtMatch[1], 10);

      const clippyMatch = trimmed.match(/Max clippy repairs:\s*(\d+)/);
      if (clippyMatch) result.verificationLimits.maxClippyRepairs = parseInt(clippyMatch[1], 10);

      const testMatch = trimmed.match(/Max test repairs:\s*(\d+)/);
      if (testMatch) result.verificationLimits.maxTestRepairs = parseInt(testMatch[1], 10);

      const globalMatch = trimmed.match(/Max global repairs:\s*(\d+)/);
      if (globalMatch) result.verificationLimits.maxGlobalRepairs = parseInt(globalMatch[1], 10);
    }

    if (currentSection === 'exclude' && result.excludePatterns) {
      const patternMatch = trimmed.match(/^-\s*(.+)/);
      if (patternMatch) {
        result.excludePatterns.push(patternMatch[1].trim());
      }
    }
  }

  return {
    success: true,
    data: result,
  };
}

/**
 * Check if Ollama is configured
 */
export function isOllamaConfigured(result: PrivacyStatusResult): boolean {
  return !!(
    result.localExecutor &&
    result.localExecutor.ollamaUrl &&
    result.localExecutor.model
  );
}

/**
 * Get a summary string for the privacy status
 */
export function getStatusSummary(result: PrivacyStatusResult): string {
  if (!result.enabled) {
    return 'Privacy mode is disabled';
  }

  const parts = ['Privacy mode enabled'];

  if (result.localExecutor?.model) {
    parts.push(`Model: ${result.localExecutor.model}`);
  }

  if (result.localExecutor?.ollamaUrl) {
    parts.push(`Ollama: ${result.localExecutor.ollamaUrl}`);
  }

  return parts.join(' | ');
}
