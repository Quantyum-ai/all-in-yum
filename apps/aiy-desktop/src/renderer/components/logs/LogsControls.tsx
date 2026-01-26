import React, { useCallback, useEffect, useState } from 'react';
import { Button } from '../common/Button';
import { useLogsStore, type LevelFilter } from '../../stores/logs-store';

export interface LogsControlsProps {
  /** Callback when cancel is clicked */
  onCancel?: () => void;
  /** Callback when clear is clicked */
  onClear?: () => void;
}

/**
 * Debounce hook for search input
 */
function useDebounce<T>(value: T, delay: number): T {
  const [debouncedValue, setDebouncedValue] = useState(value);

  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedValue(value);
    }, delay);

    return () => {
      clearTimeout(timer);
    };
  }, [value, delay]);

  return debouncedValue;
}

/**
 * Logs Controls Component
 *
 * Provides controls for the logs panel:
 * - Search input with debounce
 * - Level filter dropdown (all/stdout/stderr)
 * - Clear button
 * - Cancel button (for running jobs)
 */
export function LogsControls({ onCancel, onClear }: LogsControlsProps): React.ReactElement {
  const {
    searchFilter,
    setSearchFilter,
    levelFilter,
    setLevelFilter,
    isRunning,
    clear,
    lines,
  } = useLogsStore();

  // Local state for search input (with debounce)
  const [localSearch, setLocalSearch] = useState(searchFilter);
  const debouncedSearch = useDebounce(localSearch, 300);

  // Update store when debounced search changes
  useEffect(() => {
    setSearchFilter(debouncedSearch);
  }, [debouncedSearch, setSearchFilter]);

  // Handle search input change
  const handleSearchChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setLocalSearch(e.target.value);
  }, []);

  // Handle level filter change
  const handleLevelChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      setLevelFilter(e.target.value as LevelFilter);
    },
    [setLevelFilter]
  );

  // Handle clear button click
  const handleClear = useCallback(() => {
    clear();
    onClear?.();
  }, [clear, onClear]);

  // Handle cancel button click
  const handleCancel = useCallback(() => {
    onCancel?.();
  }, [onCancel]);

  // Clear search
  const handleClearSearch = useCallback(() => {
    setLocalSearch('');
    setSearchFilter('');
  }, [setSearchFilter]);

  return (
    <div className="flex items-center gap-3 border-b border-slate-700 bg-slate-800/50 px-4 py-2">
      {/* Search input */}
      <div className="relative flex-1 max-w-xs">
        <div className="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3">
          <svg
            className="h-4 w-4 text-slate-500"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>
        </div>
        <input
          type="text"
          value={localSearch}
          onChange={handleSearchChange}
          placeholder="Search logs..."
          className="w-full rounded-md border border-slate-600 bg-slate-900 py-1.5 pl-9 pr-8 text-sm text-slate-300 placeholder-slate-500 focus:border-aiy-primary focus:outline-none focus:ring-1 focus:ring-aiy-primary"
        />
        {localSearch && (
          <button
            type="button"
            onClick={handleClearSearch}
            className="absolute inset-y-0 right-0 flex items-center pr-2 text-slate-500 hover:text-slate-300"
            aria-label="Clear search"
          >
            <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        )}
      </div>

      {/* Level filter dropdown */}
      <select
        value={levelFilter}
        onChange={handleLevelChange}
        className="rounded-md border border-slate-600 bg-slate-900 px-3 py-1.5 text-sm text-slate-300 focus:border-aiy-primary focus:outline-none focus:ring-1 focus:ring-aiy-primary"
        aria-label="Filter by level"
      >
        <option value="all">All Levels</option>
        <option value="stdout">stdout</option>
        <option value="stderr">stderr</option>
      </select>

      {/* Line count indicator */}
      <span className="text-xs text-slate-500">
        {lines.length.toLocaleString()} lines
      </span>

      {/* Spacer */}
      <div className="flex-1" />

      {/* Clear button */}
      <Button
        variant="ghost"
        size="sm"
        onClick={handleClear}
        disabled={lines.length === 0}
        aria-label="Clear logs"
      >
        <svg className="mr-1.5 h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
          />
        </svg>
        Clear
      </Button>

      {/* Cancel button */}
      <Button
        variant="danger"
        size="sm"
        onClick={handleCancel}
        disabled={!isRunning}
        aria-label="Cancel running job"
      >
        <svg className="mr-1.5 h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
          />
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M9 10a1 1 0 011-1h4a1 1 0 011 1v4a1 1 0 01-1 1h-4a1 1 0 01-1-1v-4z"
          />
        </svg>
        Cancel
      </Button>
    </div>
  );
}
