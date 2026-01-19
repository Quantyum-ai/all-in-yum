import React, { useCallback, useEffect, useRef } from 'react';
import { LogLine } from './LogLine';
import { LogsControls } from './LogsControls';
import { useLogsStore } from '../../stores/logs-store';
import type { OutputChunk } from '../../../shared/ipc-channels';

/**
 * Logs Panel Component
 *
 * Main component for displaying CLI output in real-time.
 *
 * Features:
 * - Real-time streaming of stdout/stderr
 * - Color-coded output by stream type
 * - Auto-scroll to bottom (pauses when user scrolls up)
 * - Search/filter functionality
 * - Clear and Cancel controls
 * - PASS/FAIL highlighting for privacy check output
 */
export function LogsPanel(): React.ReactElement {
  const {
    currentJobId,
    isRunning,
    searchFilter,
    autoScrollEnabled,
    setAutoScrollEnabled,
    addLine,
    setJobId,
    setIsRunning,
    getFilteredLines,
  } = useLogsStore();

  // Ref to the scroll container for auto-scroll
  const scrollContainerRef = useRef<HTMLDivElement>(null);

  // Ref to track if user is scrolling
  const userScrollingRef = useRef(false);

  // Get filtered lines for display
  const filteredLines = getFilteredLines();

  /**
   * Auto-scroll to bottom when new lines are added
   */
  const scrollToBottom = useCallback(() => {
    if (scrollContainerRef.current && autoScrollEnabled) {
      const container = scrollContainerRef.current;
      container.scrollTop = container.scrollHeight;
    }
  }, [autoScrollEnabled]);

  /**
   * Handle scroll events to detect user scrolling up
   */
  const handleScroll = useCallback(() => {
    if (scrollContainerRef.current) {
      const container = scrollContainerRef.current;
      const { scrollTop, scrollHeight, clientHeight } = container;

      // Check if scrolled near bottom (within 50px)
      const isNearBottom = scrollHeight - scrollTop - clientHeight < 50;

      // If user scrolled up, disable auto-scroll
      // If user scrolled back to bottom, re-enable auto-scroll
      if (!userScrollingRef.current) {
        setAutoScrollEnabled(isNearBottom);
      }
    }
  }, [setAutoScrollEnabled]);

  /**
   * Track when user starts scrolling (to prevent auto-scroll interference)
   */
  const handleMouseDown = useCallback(() => {
    userScrollingRef.current = true;
  }, []);

  const handleMouseUp = useCallback(() => {
    userScrollingRef.current = false;
    handleScroll();
  }, [handleScroll]);

  /**
   * Handle cancel button click
   */
  const handleCancel = useCallback(async () => {
    if (currentJobId) {
      try {
        await window.electronAPI.cli.cancel(currentJobId);
      } catch (error) {
        console.error('Failed to cancel job:', error);
      }
    }
  }, [currentJobId]);

  /**
   * Subscribe to CLI output events on mount
   */
  useEffect(() => {
    // Subscribe to output events
    const unsubscribeOutput = window.electronAPI.cli.onOutput(
      (jobId: string, chunk: OutputChunk) => {
        addLine(chunk, jobId);

        // Update current job ID if not set
        if (!currentJobId) {
          setJobId(jobId);
          setIsRunning(true);
        }
      }
    );

    // Subscribe to exit events
    const unsubscribeExit = window.electronAPI.cli.onExit(
      (jobId: string, _code: number) => {
        // Only update state if this was our current job
        if (jobId === currentJobId || !currentJobId) {
          setIsRunning(false);
        }
      }
    );

    // Cleanup on unmount
    return () => {
      unsubscribeOutput();
      unsubscribeExit();
    };
  }, [addLine, currentJobId, setJobId, setIsRunning]);

  /**
   * Auto-scroll when new lines are added
   */
  useEffect(() => {
    scrollToBottom();
  }, [filteredLines.length, scrollToBottom]);

  return (
    <div className="flex h-full flex-col overflow-hidden rounded-lg border border-slate-700 bg-slate-900">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-slate-700 bg-slate-800 px-4 py-2">
        <div className="flex items-center gap-2">
          <svg
            className="h-4 w-4 text-slate-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"
            />
          </svg>
          <h3 className="text-sm font-medium text-slate-300">Logs</h3>

          {/* Running indicator */}
          {isRunning && (
            <span className="flex items-center gap-1.5 rounded-full bg-green-900/30 px-2 py-0.5 text-xs text-green-400">
              <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-green-400" />
              Running
            </span>
          )}
        </div>

        {/* Auto-scroll indicator */}
        <button
          type="button"
          onClick={() => {
            setAutoScrollEnabled(!autoScrollEnabled);
            if (!autoScrollEnabled) {
              scrollToBottom();
            }
          }}
          className={`flex items-center gap-1 rounded px-2 py-1 text-xs transition-colors ${
            autoScrollEnabled
              ? 'bg-aiy-primary/20 text-aiy-primary'
              : 'bg-slate-700 text-slate-400 hover:text-slate-300'
          }`}
          title={autoScrollEnabled ? 'Auto-scroll enabled' : 'Auto-scroll disabled'}
        >
          <svg
            className="h-3.5 w-3.5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M19 14l-7 7m0 0l-7-7m7 7V3"
            />
          </svg>
          Auto-scroll
        </button>
      </div>

      {/* Controls bar */}
      <LogsControls onCancel={handleCancel} />

      {/* Log content area */}
      <div
        ref={scrollContainerRef}
        onScroll={handleScroll}
        onMouseDown={handleMouseDown}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
        className="flex-1 overflow-auto bg-slate-900 px-2 py-2"
      >
        {filteredLines.length === 0 ? (
          <div className="flex h-full items-center justify-center">
            <div className="text-center">
              <svg
                className="mx-auto h-12 w-12 text-slate-700"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={1.5}
                  d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"
                />
              </svg>
              <p className="mt-2 text-sm text-slate-500">
                {searchFilter
                  ? 'No logs match your search'
                  : 'No logs yet. Run a command to see output here.'}
              </p>
            </div>
          </div>
        ) : (
          <div className="space-y-0.5">
            {filteredLines.map((line, index) => (
              <LogLine
                key={line.id}
                line={line}
                lineNumber={index + 1}
                showTimestamp={true}
                highlightTerm={searchFilter}
              />
            ))}
          </div>
        )}
      </div>

      {/* Footer with scroll-to-bottom button when not at bottom */}
      {!autoScrollEnabled && filteredLines.length > 0 && (
        <div className="border-t border-slate-700 bg-slate-800/50 px-4 py-1">
          <button
            type="button"
            onClick={() => {
              setAutoScrollEnabled(true);
              scrollToBottom();
            }}
            className="flex w-full items-center justify-center gap-1.5 text-xs text-slate-400 hover:text-slate-300"
          >
            <svg
              className="h-3.5 w-3.5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M19 14l-7 7m0 0l-7-7m7 7V3"
              />
            </svg>
            Scroll to bottom
          </button>
        </div>
      )}
    </div>
  );
}
