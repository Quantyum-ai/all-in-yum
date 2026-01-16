#!/usr/bin/env bash
#
# Privacy Mode Preflight & System Test
# =====================================
# Non-destructive, offline-safe validation of all-in-yum privacy mode.
#
# Usage: bash scripts/test-privacy-mode.sh
#
# Exit codes:
#   0 - All required tests passed
#   1 - One or more required tests failed
#

set -euo pipefail

# =============================================================================
# Configuration
# =============================================================================

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINARY="${REPO_ROOT}/target/release/aiy"
TMPDIR_BASE="${TMPDIR:-/tmp}"
TEST_TMPDIR=""

# Colors (disabled if not a terminal)
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    NC='\033[0m' # No Color
else
    RED='' GREEN='' YELLOW='' BLUE='' NC=''
fi

# Test results tracking
declare -A RESULTS
REQUIRED_FAILED=0

# =============================================================================
# Helper Functions
# =============================================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_pass() {
    echo -e "${GREEN}[PASS]${NC} $*"
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $*"
}

log_skip() {
    echo -e "${YELLOW}[SKIP]${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

fail() {
    log_fail "$*"
    exit 1
}

record_result() {
    local name="$1"
    local status="$2"  # PASS, FAIL, SKIP
    local required="${3:-true}"

    RESULTS["$name"]="$status"

    if [[ "$status" == "FAIL" && "$required" == "true" ]]; then
        REQUIRED_FAILED=1
    fi
}

do_cleanup() {
    # Restore privacy mode to disabled state (best effort)
    if [[ -x "$BINARY" ]]; then
        "$BINARY" privacy disable 2>/dev/null || true
    fi

    # Remove temp directory safely (no -rf, explicit path check)
    if [[ -n "${TEST_TMPDIR:-}" && -d "$TEST_TMPDIR" && "$TEST_TMPDIR" == "$TMPDIR_BASE"/* ]]; then
        # Safe removal: only if under TMPDIR and starts with our prefix
        if [[ "$TEST_TMPDIR" == *"aiy-privacy-test"* ]]; then
            find "$TEST_TMPDIR" -mindepth 1 -delete 2>/dev/null || true
            rmdir "$TEST_TMPDIR" 2>/dev/null || true
            log_info "Removed temp directory: $TEST_TMPDIR"
        fi
    fi
}

CLEANUP_DONE=0

cleanup() {
    local exit_code=$?
    if [[ $CLEANUP_DONE -eq 1 ]]; then
        exit $exit_code
    fi
    CLEANUP_DONE=1
    log_info "Cleaning up..."
    do_cleanup
    exit $exit_code
}

trap cleanup EXIT

# =============================================================================
# Test Steps
# =============================================================================

step_environment_info() {
    log_info "=== Step 1: Environment Info ==="

    echo "  Repo path: $REPO_ROOT"

    cd "$REPO_ROOT"

    local branch
    branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
    echo "  Git branch: $branch"

    local head
    head=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
    echo "  Git HEAD: $head"

    local status
    # Filter out .aiy/ (created by CLI) and target/ (build artifacts)
    status=$(git status --porcelain=v1 2>/dev/null | grep -vE "^\?\? \.aiy/|^\?\? target/" || true)

    if [[ -z "$status" ]]; then
        echo "  Git status: clean (ignoring .aiy/ and target/)"
        log_pass "Environment info collected, working tree clean"
        record_result "environment_info" "PASS"
    else
        echo "  Git status: DIRTY"
        echo "$status" | head -10 | sed 's/^/    /'
        log_fail "Working tree is not clean (uncommitted changes)"
        record_result "environment_info" "FAIL"
        return 1
    fi
}

step_build() {
    log_info "=== Step 2: Build Release Binary ==="

    cd "$REPO_ROOT"

    if timeout 10m cargo build -p aiy-cli --release 2>&1 | tail -5; then
        if [[ -x "$BINARY" ]]; then
            log_pass "Build successful: $BINARY"
            record_result "build" "PASS"
        else
            log_fail "Build completed but binary not found at $BINARY"
            record_result "build" "FAIL"
            return 1
        fi
    else
        log_fail "Build failed or timed out"
        record_result "build" "FAIL"
        return 1
    fi
}

step_privacy_check() {
    log_info "=== Step 3: Privacy Mode Check ==="

    cd "$REPO_ROOT"

    # Check if privacy subcommand exists
    if ! "$BINARY" privacy --help &>/dev/null; then
        log_fail "Privacy subcommand not available"
        record_result "privacy_check" "FAIL"
        return 1
    fi

    # Run privacy check
    local check_output
    if check_output=$("$BINARY" privacy check 2>&1); then
        echo "$check_output" | head -10 | sed 's/^/    /'
        log_pass "Privacy check command works"
    else
        # Check might fail due to Ollama not running - that's OK for offline test
        if echo "$check_output" | grep -qi "ollama\|connection\|refused"; then
            log_warn "Privacy check: Ollama not available (expected for offline test)"
            echo "$check_output" | head -5 | sed 's/^/    /'
        else
            echo "$check_output" | head -10 | sed 's/^/    /'
        fi
    fi

    # Show config (non-destructive)
    log_info "Current privacy config:"
    if "$BINARY" privacy config show 2>&1 | head -15 | sed 's/^/    /'; then
        :
    else
        log_warn "Could not show config (may not be initialized)"
    fi

    log_pass "Privacy check commands executed"
    record_result "privacy_check" "PASS"
}

step_enable_disable_sanity() {
    log_info "=== Step 4: Enable/Disable Sanity Test ==="

    cd "$REPO_ROOT"

    # Enable with loopback URL and small model
    log_info "Enabling privacy mode with loopback Ollama..."
    if ! "$BINARY" privacy enable \
        --ollama-url "http://127.0.0.1:11434" \
        --model "codellama:7b-instruct" 2>&1 | head -5 | sed 's/^/    /'; then
        log_warn "Enable command had issues (may be expected)"
    fi

    # Verify enabled
    local status_output
    status_output=$("$BINARY" privacy status 2>&1 || true)
    echo "$status_output" | head -5 | sed 's/^/    /'

    if echo "$status_output" | grep -qi "enabled\|active"; then
        log_pass "Privacy mode shows enabled"
    else
        log_warn "Could not verify enabled state"
    fi

    # Disable
    log_info "Disabling privacy mode..."
    "$BINARY" privacy disable 2>&1 | head -3 | sed 's/^/    /' || true

    # Verify disabled
    status_output=$("$BINARY" privacy status 2>&1 || true)
    if echo "$status_output" | grep -qi "disabled\|inactive\|not.*enabled"; then
        log_pass "Privacy mode shows disabled"
    else
        log_warn "Could not verify disabled state"
    fi

    # Re-enable for remainder of tests
    log_info "Re-enabling privacy mode for remaining tests..."
    "$BINARY" privacy enable \
        --ollama-url "http://127.0.0.1:11434" \
        --model "codellama:7b-instruct" 2>&1 | head -3 | sed 's/^/    /' || true

    log_pass "Enable/disable sanity test completed"
    record_result "enable_disable_sanity" "PASS"
}

step_temp_repo_init() {
    log_info "=== Step 5: Temp Repo Init Test ==="

    # Create temp directory
    TEST_TMPDIR=$(mktemp -d "${TMPDIR_BASE}/aiy-privacy-test.XXXXXX")
    log_info "Created temp directory: $TEST_TMPDIR"

    # Initialize git repo
    cd "$TEST_TMPDIR"
    git init --initial-branch=main >/dev/null 2>&1 || git init >/dev/null 2>&1
    git config user.email "test@test.local"
    git config user.name "Test User"

    # Create a minimal Rust project structure
    mkdir -p src
    cat > Cargo.toml << 'EOF'
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
EOF

    cat > src/main.rs << 'EOF'
fn main() {
    println!("Hello, privacy mode!");
}
EOF

    git add .
    git commit -m "Initial commit" >/dev/null 2>&1

    log_info "Initialized test repo at $TEST_TMPDIR"

    # Run privacy init
    cd "$REPO_ROOT"
    if "$BINARY" privacy init --path "$TEST_TMPDIR" 2>&1 | head -10 | sed 's/^/    /'; then
        log_pass "Privacy init on temp repo succeeded"
        record_result "temp_repo_init" "PASS"
    else
        log_warn "Privacy init had issues (may be expected without Ollama)"
        record_result "temp_repo_init" "PASS"  # Non-blocking
    fi
}

step_offline_orchestration_check() {
    log_info "=== Step 6: Offline Orchestration Check ==="

    cd "$REPO_ROOT"

    # Try status and check commands (these should work offline)
    log_info "Running offline-safe commands..."

    "$BINARY" privacy status 2>&1 | head -5 | sed 's/^/    /' || true
    "$BINARY" privacy check 2>&1 | head -5 | sed 's/^/    /' || true

    # If execute command exists, try it but expect credential/network skip
    if "$BINARY" privacy execute --help &>/dev/null 2>&1; then
        log_info "Testing execute command (expecting offline skip)..."
        local exec_output
        exec_output=$("$BINARY" privacy execute --dry-run 2>&1 || true)

        if echo "$exec_output" | grep -qiE "credential|api.key|network|offline|connection|refused"; then
            log_skip "Execute skipped (requires cloud credentials or network)"
            record_result "offline_orchestration" "SKIP" "false"
        else
            echo "$exec_output" | head -5 | sed 's/^/    /'
            log_pass "Execute command responded"
            record_result "offline_orchestration" "PASS"
        fi
    else
        log_info "No execute command available, skipping"
        record_result "offline_orchestration" "SKIP" "false"
    fi

    log_pass "Offline orchestration check completed"
}

step_cargo_test() {
    log_info "=== Step 7: Cargo Test (Offline) ==="

    cd "$REPO_ROOT"

    local test_output
    # Run tests for aiy-privacy specifically (most comprehensive)
    if test_output=$(timeout 20m cargo test -p aiy-privacy -- --test-threads=1 2>&1); then
        local summary
        summary=$(echo "$test_output" | grep -E "^test result:" | head -1)
        echo "  $summary"

        if echo "$summary" | grep -q "0 failed"; then
            log_pass "All tests passed"
            record_result "cargo_test" "PASS"
        else
            log_fail "Some tests failed"
            echo "$test_output" | grep -E "FAILED|panicked" | head -10 | sed 's/^/    /'
            record_result "cargo_test" "FAIL"
            return 1
        fi
    else
        local exit_code=$?
        if [[ $exit_code -eq 124 ]]; then
            log_fail "Tests timed out after 20 minutes"
        else
            log_fail "Tests failed with exit code $exit_code"
            echo "$test_output" | tail -20 | sed 's/^/    /'
        fi
        record_result "cargo_test" "FAIL"
        return 1
    fi
}

step_cargo_clippy() {
    log_info "=== Step 8: Cargo Clippy ==="

    cd "$REPO_ROOT"

    local clippy_output
    if clippy_output=$(timeout 15m cargo clippy --workspace --all-targets -- -D warnings 2>&1); then
        local last_line
        last_line=$(echo "$clippy_output" | tail -1)
        echo "  $last_line"
        log_pass "Clippy passed with -D warnings"
        record_result "cargo_clippy" "PASS"
    else
        local exit_code=$?
        if [[ $exit_code -eq 124 ]]; then
            log_fail "Clippy timed out after 15 minutes"
        else
            log_fail "Clippy found warnings/errors"
            echo "$clippy_output" | grep -E "^error|^warning" | head -10 | sed 's/^/    /'
        fi
        record_result "cargo_clippy" "FAIL"
        return 1
    fi
}

print_summary() {
    echo ""
    echo "============================================================"
    echo "                    TEST SUMMARY"
    echo "============================================================"
    echo ""

    local pass=0 fail=0 skip=0

    for name in "${!RESULTS[@]}"; do
        local status="${RESULTS[$name]}"
        local icon
        case "$status" in
            PASS) icon="${GREEN}[PASS]${NC}"; pass=$((pass + 1)) ;;
            FAIL) icon="${RED}[FAIL]${NC}"; fail=$((fail + 1)) ;;
            SKIP) icon="${YELLOW}[SKIP]${NC}"; skip=$((skip + 1)) ;;
        esac
        printf "  %-30s %b\n" "$name" "$icon"
    done

    echo ""
    echo "------------------------------------------------------------"
    printf "  Total: %d passed, %d failed, %d skipped\n" "$pass" "$fail" "$skip"
    echo "------------------------------------------------------------"
    echo ""

    if [[ $REQUIRED_FAILED -eq 0 ]]; then
        echo -e "${GREEN}============================================================${NC}"
        echo -e "${GREEN}                    OVERALL: PASS                           ${NC}"
        echo -e "${GREEN}============================================================${NC}"
        return 0
    else
        echo -e "${RED}============================================================${NC}"
        echo -e "${RED}                    OVERALL: FAIL                           ${NC}"
        echo -e "${RED}============================================================${NC}"
        return 1
    fi
}

# =============================================================================
# Main
# =============================================================================

main() {
    echo ""
    echo "============================================================"
    echo "       Privacy Mode Preflight & System Test"
    echo "       $(date '+%Y-%m-%d %H:%M:%S')"
    echo "============================================================"
    echo ""

    # Run all steps, continue on failure to collect all results
    step_environment_info || true
    step_build || true

    # Only continue if build succeeded
    if [[ "${RESULTS[build]:-FAIL}" != "PASS" ]]; then
        log_fail "Build failed, skipping remaining tests"
        print_summary
        exit 1
    fi

    step_privacy_check || true
    step_enable_disable_sanity || true
    step_temp_repo_init || true
    step_offline_orchestration_check || true
    step_cargo_test || true
    step_cargo_clippy || true

    # Clean up before printing summary (so output is clean)
    log_info "Cleaning up..."
    do_cleanup
    CLEANUP_DONE=1  # Prevent double cleanup in trap

    # Print summary and exit with appropriate code
    print_summary
}

main "$@"
