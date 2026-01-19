import React, { memo } from 'react';
import type { LogLine as LogLineType } from '../../stores/logs-store';

export interface LogLineProps {
  /** The log line data */
  line: LogLineType;
  /** Optional line number to display */
  lineNumber?: number;
  /** Whether to show timestamps */
  showTimestamp?: boolean;
  /** Search term to highlight */
  highlightTerm?: string;
}

/**
 * Format timestamp for display
 * Extracts HH:MM:SS.mmm from ISO timestamp
 */
function formatTimestamp(isoTimestamp: string): string {
  try {
    const date = new Date(isoTimestamp);
    const hours = date.getHours().toString().padStart(2, '0');
    const minutes = date.getMinutes().toString().padStart(2, '0');
    const seconds = date.getSeconds().toString().padStart(2, '0');
    const ms = date.getMilliseconds().toString().padStart(3, '0');
    return `${hours}:${minutes}:${seconds}.${ms}`;
  } catch {
    return '';
  }
}

/**
 * Check if text contains [PASS] or [FAIL] markers for privacy check output
 */
function getStatusMarker(text: string): 'pass' | 'fail' | null {
  if (text.includes('[PASS]') || text.includes('PASS') || text.includes('passed')) {
    return 'pass';
  }
  if (text.includes('[FAIL]') || text.includes('FAIL') || text.includes('failed')) {
    return 'fail';
  }
  return null;
}

/**
 * Highlight search term in text
 */
function highlightText(text: string, term: string): React.ReactNode {
  if (!term) {
    return text;
  }

  const termLower = term.toLowerCase();
  const parts: React.ReactNode[] = [];
  let lastIndex = 0;
  let matchIndex = text.toLowerCase().indexOf(termLower);

  while (matchIndex !== -1) {
    // Add text before match
    if (matchIndex > lastIndex) {
      parts.push(text.slice(lastIndex, matchIndex));
    }
    // Add highlighted match
    parts.push(
      <span key={matchIndex} className="bg-yellow-500/30 text-yellow-200">
        {text.slice(matchIndex, matchIndex + term.length)}
      </span>
    );
    lastIndex = matchIndex + term.length;
    matchIndex = text.toLowerCase().indexOf(termLower, lastIndex);
  }

  // Add remaining text
  if (lastIndex < text.length) {
    parts.push(text.slice(lastIndex));
  }

  return parts.length > 0 ? <>{parts}</> : text;
}

/**
 * Single Log Line Component
 *
 * Displays a single log line with:
 * - Timestamp (optional)
 * - Line number (optional)
 * - Color-coded by stream type (stdout/stderr)
 * - PASS/FAIL highlighting for privacy check output
 * - Search term highlighting
 */
function LogLineComponent({
  line,
  lineNumber,
  showTimestamp = true,
  highlightTerm,
}: LogLineProps): React.ReactElement {
  const statusMarker = getStatusMarker(line.text);

  // Determine text color based on stream type and status markers
  const getTextColorClass = (): string => {
    if (statusMarker === 'pass') {
      return 'text-green-400';
    }
    if (statusMarker === 'fail') {
      return 'text-red-400';
    }
    if (line.stream === 'stderr') {
      return 'text-red-400';
    }
    return 'text-slate-300';
  };

  // Determine background color for status markers
  const getBackgroundClass = (): string => {
    if (statusMarker === 'pass') {
      return 'bg-green-900/20';
    }
    if (statusMarker === 'fail') {
      return 'bg-red-900/20';
    }
    return '';
  };

  return (
    <div
      className={`flex items-start font-mono text-sm leading-relaxed hover:bg-slate-800/50 ${getBackgroundClass()}`}
    >
      {/* Line number */}
      {lineNumber !== undefined && (
        <span className="mr-3 min-w-[3rem] select-none text-right text-slate-600">
          {lineNumber}
        </span>
      )}

      {/* Timestamp */}
      {showTimestamp && (
        <span className="mr-3 min-w-[7rem] select-none text-xs text-slate-500">
          {formatTimestamp(line.timestamp)}
        </span>
      )}

      {/* Stream indicator */}
      <span
        className={`mr-2 min-w-[4rem] select-none text-xs ${
          line.stream === 'stderr' ? 'text-red-500' : 'text-slate-600'
        }`}
      >
        [{line.stream}]
      </span>

      {/* Log text content */}
      <span className={`flex-1 whitespace-pre-wrap break-all ${getTextColorClass()}`}>
        {highlightTerm ? highlightText(line.text, highlightTerm) : line.text}
      </span>
    </div>
  );
}

/**
 * Memoized LogLine component for performance
 * Only re-renders when line data or props change
 */
export const LogLine = memo(LogLineComponent, (prevProps, nextProps) => {
  return (
    prevProps.line.id === nextProps.line.id &&
    prevProps.lineNumber === nextProps.lineNumber &&
    prevProps.showTimestamp === nextProps.showTimestamp &&
    prevProps.highlightTerm === nextProps.highlightTerm
  );
});

LogLine.displayName = 'LogLine';
