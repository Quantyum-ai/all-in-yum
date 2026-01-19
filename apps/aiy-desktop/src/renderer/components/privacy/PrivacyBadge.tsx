import React from 'react';
import { usePrivacyStore } from '../../stores/privacy-store';

/**
 * Privacy Mode Badge Component
 *
 * A persistent, always-visible indicator of the current privacy mode.
 * This badge appears in the header and provides at-a-glance confirmation
 * that the user's privacy preference is active.
 *
 * CRITICAL: Default mode is "always-local" - this must be clearly indicated.
 */
export function PrivacyBadge(): React.ReactElement {
  const { mode, isLoading } = usePrivacyStore();

  // Determine badge styles based on mode
  const isLocal = mode === 'always-local';
  const badgeColor = isLocal ? 'bg-privacy-local/20' : 'bg-privacy-hybrid/20';
  const dotColor = isLocal ? 'bg-privacy-local' : 'bg-privacy-hybrid';
  const textColor = isLocal ? 'text-privacy-local' : 'text-privacy-hybrid';

  // Icon for the badge
  const icon = isLocal ? (
    // Lock icon for local mode
    <svg
      className="h-4 w-4"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"
      />
    </svg>
  ) : (
    // Cloud icon for hybrid mode
    <svg
      className="h-4 w-4"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z"
      />
    </svg>
  );

  // Loading state
  if (isLoading) {
    return (
      <div
        className="flex items-center gap-2 rounded-full bg-aiy-surface px-3 py-1.5"
        role="status"
        aria-label="Loading privacy mode"
      >
        <span className="h-2 w-2 animate-pulse rounded-full bg-aiy-text-muted" />
        <span className="text-sm font-medium text-aiy-text-muted">Loading...</span>
      </div>
    );
  }

  return (
    <button
      type="button"
      className={`flex items-center gap-2 rounded-full ${badgeColor} px-3 py-1.5 transition-colors hover:opacity-80`}
      title={`Privacy Mode: ${isLocal ? 'Always Local' : 'Hybrid'}\nClick to view details`}
      aria-label={`Privacy mode: ${isLocal ? 'Always Local' : 'Hybrid'}. Click to view details.`}
    >
      {/* Status indicator dot */}
      <span className={`h-2 w-2 rounded-full ${dotColor}`} aria-hidden="true" />

      {/* Icon */}
      <span className={textColor} aria-hidden="true">
        {icon}
      </span>

      {/* Label */}
      <span className={`text-sm font-medium ${textColor}`}>
        {isLocal ? 'Always Local' : 'Hybrid'}
      </span>
    </button>
  );
}
