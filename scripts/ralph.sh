#!/bin/sh
# ralph — autonomous task loop for Feiglet repository.
# Ported from ZoidMatter ralph.sh.
#
# Usage:
#   ./scripts/ralph.sh                              # multi-phase loop
#   ./scripts/ralph.sh 1.1.1                        # single task
#   ./scripts/ralph.sh --dry-run                    # preview next action
#   ./scripts/ralph.sh --minutes=30                 # time-bounded loop
#
# Requires: opencode CLI, cargo, git, clean working tree.

set -e

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
LOG="$REPO_ROOT/docs/ralph-log.md"

# Agent config
TASK_PLANNING_AGENT="${TASK_PLANNING_AGENT:-opencode-go/deepseek-v4-flash}"
MID_DEV_AGENT="${MID_DEV_AGENT:-opencode-go/deepseek-v4-pro}"
PRO_DEV_AGENT="${PRO_DEV_AGENT:-opencode-go/glm-5.1}"
TASK_REVIEW_AGENT="${TASK_REVIEW_AGENT:-opencode-go/deepseek-v4-pro}"
ARCHITECT_AGENT="${ARCHITECT_AGENT:-opencode-go/glm-5.1}"

BASE_BRANCH="$(git -C "$REPO_ROOT" rev-parse --abbrev-ref HEAD)"

BOLD=$(tput bold 2>/dev/null || true)
CYAN=$(tput setaf 6 2>/dev/null || true)
GREEN=$(tput setaf 2 2>/dev/null || true)
YELLOW=$(tput setaf 3 2>/dev/null || true)
RED=$(tput setaf 1 2>/dev/null || true)
RESET=$(tput sgr0 2>/dev/null || true)

log()  { printf '%s\n' "${CYAN}[ralph]${RESET} $*"; }
good() { printf '%s\n' "${GREEN}[ralph]${RESET} $*"; }
warn() { printf '%s\n' "${YELLOW}[ralph]${RESET} $*"; }
die()  { printf '%s\n' "${RED}[ralph]${RESET} $*" >&2; exit 1; }

SINGLE_TASK=""; DURATION_SECS=0; DRY_RUN=0
for _arg in "$@"; do
    case "$_arg" in
        --minutes=*) _mins="${_arg#--minutes=}"; DURATION_SECS=$((DURATION_SECS + _mins * 60)) ;;
        --hours=*)   _hrs="${_arg#--hours=}";   DURATION_SECS=$((DURATION_SECS + _hrs * 3600)) ;;
        --dry-run)   DRY_RUN=1 ;;
        -*)          die "Unknown flag: $_arg" ;;
        *)           [ -z "$SINGLE_TASK" ] || die "Only one task id allowed"; SINGLE_TASK="$_arg" ;;
    esac
done

START_TIME="$(date +%s)"; DEADLINE=0
[ "$DURATION_SECS" -gt 0 ] && DEADLINE=$((START_TIME + DURATION_SECS))

[ "$BASE_BRANCH" = "main" ] || case "$BASE_BRANCH" in release/*) ;; *) die "Start from main or release/X.Y" ;; esac

MINOR_VERSION=""
case "$BASE_BRANCH" in
    main) ;;
    release/*) MINOR_VERSION="${BASE_BRANCH#release/}" ;;
esac

all_todo_lines() {
    for _f in "$REPO_ROOT/docs"/todo-v*.md; do
        [ -f "$_f" ] && cat "$_f" || true
    done
}

next_task() {
    all_todo_lines | grep -m1 "^\- \[ \] \`${MINOR_VERSION}\.[0-9]\+\`" | sed "s/^- \[ \] \`\([^\`]*\)\`.*/\1/" || true
}

next_minor() {
    all_todo_lines | grep -m1 "^\- \[ \] \`[0-9]\+\.[0-9]\+\.[0-9]\+\`" | sed "s/^- \[ \] \`\([^\`]*\)\`.*/\1/" | sed 's/\.[0-9]*$//' || true
}

switch_to_phase() {
    _minor="$1"; _branch="release/${_minor}"
    if git show-ref --verify --quiet "refs/heads/${_branch}" 2>/dev/null; then
        log "Switching to ${_branch}"
        git checkout "$_branch" >/dev/null 2>&1
    else
        log "Creating ${_branch} from main"
        git checkout main >/dev/null 2>&1
        git checkout -b "$_branch"
    fi
    BASE_BRANCH="$_branch"; MINOR_VERSION="$_minor"
}

task_block() {
    all_todo_lines | awk -v tid="$1" '
        BEGIN { pat = "^- \\[.\\] `" tid "`" }
        $0 ~ pat      { found=1; print; next }
        found && /^- \[.\] `[0-9]/ { exit }
        found         { print }
    '
}

ralph_log() {
    printf '\n## %s\n\n%s\n' "$(date '+%Y-%m-%d %H:%M')" "$1" >> "$LOG"
}

invoke_agent() {
    _agent="$1"; shift; _prompt="$1"; shift
    _pf="$(mktemp)"; printf '%s' "$_prompt" > "$_pf"
    cat "$_pf" | opencode run --model "$_agent" "$@"
    rm -f "$_pf"
}

run_task() {
    TASK_ID="$1"; BRANCH="task-${TASK_ID}"
    log "Starting task ${BOLD}${TASK_ID}${RESET} on ${BRANCH}"

    git checkout "$BASE_BRANCH" >/dev/null 2>&1
    git checkout -b "$BRANCH" >/dev/null 2>&1 || die "Branch ${BRANCH} exists"

    TASK_BLOCK="$(task_block "$TASK_ID")"
    PLAN="$(invoke_agent "$TASK_PLANNING_AGENT" "Plan task ${TASK_ID} for Feiglet Rust port.
Task: ${TASK_BLOCK}
Write numbered implementation plan. No code.")"

    log "Implementing..."
    invoke_agent "$MID_DEV_AGENT" "Implement task ${TASK_ID} for Feiglet.
Tasks: ${TASK_BLOCK}
Plan: ${PLAN}
Write all files. Run: cargo fmt --check && cargo clippy -- -D warnings
Do NOT commit. Print IMPLEMENTATION_DONE when done."

    log "Reviewing..."
    DIFF="$(git diff HEAD 2>&1 | head -300)"
    invoke_agent "$TASK_REVIEW_AGENT" "Review task ${TASK_ID} diff.
${DIFF}
Fix any issues. Print REVIEW_DONE when done."

    git add -A && git commit -m "${TASK_ID}: ${TASK_BLOCK%%$'\n'*}"
    git checkout "$BASE_BRANCH" && git merge --no-ff "$BRANCH"
    git branch -d "$BRANCH"
    good "Task ${TASK_ID} done."
}

# Main
cd "$REPO_ROOT"
if git diff --quiet 2>/dev/null && git diff --cached --quiet 2>/dev/null; then
    : # clean
else
    die "Dirty working tree — commit or stash first"
fi

if [ -n "$SINGLE_TASK" ]; then
    if [ "$BASE_BRANCH" = "main" ]; then
        _minor="$(printf '%s' "$SINGLE_TASK" | sed 's/\.[0-9]*$//')"
        switch_to_phase "$_minor"
    fi
    run_task "$SINGLE_TASK"
    exit 0
fi

while true; do
    if [ "$BASE_BRANCH" = "main" ]; then
        _next="$(next_minor)"
        [ -z "$_next" ] && { good "All phases complete."; exit 0; }
        switch_to_phase "$_next"
    fi
    TASK_ID="$(next_task)"
    [ -z "$TASK_ID" ] && {
        good "Phase ${MINOR_VERSION} complete."
        git checkout main
        BASE_BRANCH="main"; MINOR_VERSION=""
        continue
    }
    run_task "$TASK_ID"
    sleep 1
done
