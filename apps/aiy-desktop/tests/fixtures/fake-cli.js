#!/usr/bin/env node
/**
 * Fake aiy CLI for testing
 *
 * This script simulates the aiy CLI behavior for integration testing
 * the CLI service without requiring the real Rust binary.
 *
 * Usage:
 *   node fake-cli.js version
 *   node fake-cli.js privacy check
 *   node fake-cli.js privacy status
 *   node fake-cli.js --hang          # For testing cancel functionality
 *   node fake-cli.js --error         # For testing error handling
 *   node fake-cli.js --slow          # For testing timeout handling
 *   node fake-cli.js --stream        # For testing streaming output
 *   node fake-cli.js --stderr        # For testing stderr output
 */

const fs = require('fs');

function stdout(line) {
  fs.writeSync(1, line + '\n');
}

function stderr(line) {
  fs.writeSync(2, line + '\n');
}

const command = process.argv[2];
const args = process.argv.slice(3);

// Handle special test flags
if (command === '--hang') {
  // Hang indefinitely for testing cancel
  stdout('[Fake CLI] Hanging indefinitely (use cancel to stop)');
  setInterval(() => {
    // Keep process alive
  }, 1000);
  return;
}

if (command === '--error') {
  stderr('[Fake CLI] Simulated error occurred');
  process.exit(1);
}

if (command === '--slow') {
  stdout('[Fake CLI] Starting slow operation...');
  setTimeout(() => {
    stdout('[Fake CLI] Still working...');
  }, 2000);
  setTimeout(() => {
    stdout('[Fake CLI] Slow operation completed');
    process.exit(0);
  }, 35000); // 35 seconds - longer than default timeout
  return;
}

if (command === '--stream') {
  // Stream output over time for testing streaming functionality
  let count = 0;
  const interval = setInterval(() => {
    count++;
    stdout(`[Fake CLI] Stream output line ${count}`);
    if (count >= 5) {
      clearInterval(interval);
      stdout('[Fake CLI] Stream completed');
      process.exit(0);
    }
  }, 200);
  return;
}

if (command === '--stderr') {
  stdout('[Fake CLI] stdout message');
  stderr('[Fake CLI] stderr message');
  process.exit(0);
}

// Handle version command
if (command === 'version' || command === '--version') {
  stdout('aiy 0.1.0');
  process.exit(0);
}

// Handle privacy subcommands
if (command === 'privacy') {
  const subcommand = args[0];

  if (subcommand === 'check') {
    stdout('[PASS] Privacy mode enabled');
    stdout('[PASS] Ollama reachable');
    stdout('[PASS] Local model available');
    process.exit(0);
  }

  if (subcommand === 'status') {
    stdout('Privacy Mode: always-local');
    stdout('Ollama URL: http://localhost:11434');
    stdout('Model: qwen2.5-coder:7b');
    stdout('Status: Ready');
    process.exit(0);
  }

  if (subcommand === 'enable') {
    stdout('[OK] Privacy mode enabled');
    process.exit(0);
  }

  if (subcommand === 'disable') {
    stdout('[OK] Privacy mode disabled');
    process.exit(0);
  }

  if (subcommand === 'execute') {
    stdout('[Fake CLI] Starting execution...');
    stdout('[Fake CLI] Processing request...');
    setTimeout(() => {
      stdout('[Fake CLI] Execution completed');
      process.exit(0);
    }, 500);
    return;
  }

  stdout(`[Fake CLI] Unknown privacy subcommand: ${subcommand}`);
  process.exit(1);
}

// Handle agents subcommands
if (command === 'agents') {
  const subcommand = args[0];

  if (subcommand === 'list') {
    stdout('Available agents:');
    stdout('  - coder (enabled)');
    stdout('  - reviewer (disabled)');
    stdout('  - tester (enabled)');
    process.exit(0);
  }

  if (subcommand === 'status') {
    stdout('Agent Status:');
    stdout('  coder: running');
    stdout('  tester: idle');
    process.exit(0);
  }

  stdout(`[Fake CLI] Unknown agents subcommand: ${subcommand}`);
  process.exit(1);
}

// Handle credentials subcommands
if (command === 'credentials') {
  const subcommand = args[0];

  if (subcommand === 'status') {
    stdout('Credentials Status:');
    stdout('  anthropic: [CONFIGURED]');
    stdout('  openai: [NOT SET]');
    process.exit(0);
  }

  stdout(`[Fake CLI] Unknown credentials subcommand: ${subcommand}`);
  process.exit(1);
}

// Handle config subcommands
if (command === 'config') {
  const subcommand = args[0];

  if (subcommand === 'show') {
    stdout(
      JSON.stringify(
        {
          privacy_mode: 'always-local',
          ollama_url: 'http://localhost:11434',
          model: 'qwen2.5-coder:7b',
        },
        null,
        2
      )
    );
    process.exit(0);
  }

  if (subcommand === 'path') {
    stdout('/home/user/.config/aiy/config.toml');
    process.exit(0);
  }

  stdout(`[Fake CLI] Unknown config subcommand: ${subcommand}`);
  process.exit(1);
}

// Handle help
if (command === 'help' || command === '--help' || !command) {
  stdout('aiy 0.1.0 (fake CLI for testing)');
  stdout('');
  stdout('USAGE:');
  stdout('    aiy <COMMAND>');
  stdout('');
  stdout('COMMANDS:');
  stdout('    version      Print version info');
  stdout('    privacy      Privacy mode commands');
  stdout('    agents       Agent management');
  stdout('    credentials  Credential management');
  stdout('    config       Configuration');
  stdout('    help         Print this help');
  process.exit(0);
}

// Default: echo the command back
stdout(`[Fake CLI] Command: ${command} ${args.join(' ')}`);
setTimeout(() => {
  stdout('[Fake CLI] Completed');
  process.exit(0);
}, 100);
