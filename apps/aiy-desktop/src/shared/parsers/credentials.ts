/**
 * Credentials Output Parser
 *
 * Parses output from `aiy credentials status` command.
 *
 * Expected output format (from Rust CLI):
 * ```
 * Credential Status:
 * -----------------
 * Backend: EncryptedFile { path: "/home/user/.config/all-in-yum/credentials.enc" }
 * Stored providers: 2
 *   - xai
 *   - anthropic
 * ```
 */

import type { CredentialStatus, CredentialsStatusResult, ParseResult } from './types';

/**
 * All known credential providers with display names
 */
const KNOWN_PROVIDERS: Record<string, string> = {
  xai: 'xAI (Grok)',
  anthropic: 'Anthropic (Claude)',
  google: 'Google (Gemini)',
  openai: 'OpenAI (Codex)',
};

/**
 * Parse credentials status output.
 *
 * @param output - Raw output from `aiy credentials status`
 * @returns Parsed result with credential status information
 *
 * @example
 * ```typescript
 * const result = parseCredentialsStatus(cliOutput);
 * if (result.success) {
 *   console.log(`Backend: ${result.data.backend}`);
 *   console.log(`Configured: ${result.data.providers.map(p => p.provider).join(', ')}`);
 * }
 * ```
 */
export function parseCredentialsStatus(output: string): ParseResult<CredentialsStatusResult> {
  // Handle empty input
  if (!output || output.trim().length === 0) {
    return {
      success: false,
      error: {
        type: 'empty_input',
        message: 'Credentials status output is empty',
      },
      rawOutput: output || '',
    };
  }

  const lines = output.split('\n');
  let backend = 'Unknown';
  let storedCount = 0;
  const configuredProviders: string[] = [];

  for (const line of lines) {
    const trimmed = line.trim();

    // Parse backend line
    const backendMatch = trimmed.match(/Backend:\s*(.+)/);
    if (backendMatch) {
      backend = backendMatch[1].trim();
      continue;
    }

    // Parse stored providers count
    const countMatch = trimmed.match(/Stored providers:\s*(\d+)/);
    if (countMatch) {
      storedCount = parseInt(countMatch[1], 10);
      continue;
    }

    // Parse provider list items
    const providerMatch = trimmed.match(/^-\s*(\w+)/);
    if (providerMatch) {
      configuredProviders.push(providerMatch[1].toLowerCase());
    }
  }

  // Build providers list from what we found
  const providers: CredentialStatus[] = configuredProviders.map((provider) => ({
    provider,
    configured: true,
  }));

  // Build all providers list with status
  const allProviders: CredentialStatus[] = Object.keys(KNOWN_PROVIDERS).map((provider) => ({
    provider,
    configured: configuredProviders.includes(provider),
  }));

  return {
    success: true,
    data: {
      backend,
      storedCount,
      providers,
      allProviders,
      rawOutput: output,
    },
  };
}

/**
 * Get missing (unconfigured) providers
 */
export function getMissingProviders(result: CredentialsStatusResult): CredentialStatus[] {
  return result.allProviders.filter((p) => !p.configured);
}

/**
 * Get display name for a provider
 */
export function getProviderDisplayName(provider: string): string {
  return KNOWN_PROVIDERS[provider.toLowerCase()] || provider;
}

/**
 * Check if a specific provider is configured
 */
export function isProviderConfigured(result: CredentialsStatusResult, provider: string): boolean {
  return result.providers.some((p) => p.provider.toLowerCase() === provider.toLowerCase());
}

/**
 * Get credential configuration summary
 */
export function getCredentialsSummary(result: CredentialsStatusResult): string {
  const configured = result.providers.length;
  const total = result.allProviders.length;

  if (configured === 0) {
    return 'No credentials configured';
  }

  if (configured === total) {
    return 'All credentials configured';
  }

  return `${configured}/${total} credentials configured`;
}
