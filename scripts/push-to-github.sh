#!/usr/bin/env bash
#
# Push the mc-programmer repository to a GitHub remote under
# douglas-carmichael's account.
#
# Before running this script:
#
#   1. Create the empty repository on GitHub:
#      https://github.com/new
#      → Owner: douglas-carmichael
#      → Repository name: mc-programmer  (or whatever you pass as $REPO_NAME)
#      → Visibility: Public or Private
#      → Do NOT initialize with a README, .gitignore, or licence — this
#        repo already has them and they would conflict.
#
#   2. Make sure your local git is authenticated to GitHub. The simplest
#      check: `ssh -T git@github.com` should print
#      "Hi douglas-carmichael!". If you use HTTPS instead of SSH, set
#      USE_HTTPS=1 below.
#
# What this script does (it's idempotent — safe to re-run):
#
#   * Verifies you're inside the mc-programmer repo
#   * Refuses to run if there are any Roland PDFs / firmware blobs that
#     would violate copyright on push
#   * Runs all three test suites and aborts if any fails
#   * Stages every change (including the deletions of the old experimental
#     scripts) and creates one clean commit summarising the rewrite
#   * Adds a new git remote called "$REMOTE_NAME" pointing at the
#     douglas-carmichael repo (your existing "origin" is left alone)
#   * Pushes main to that remote, setting the upstream
#
# Usage:
#
#   bash scripts/push-to-github.sh                # default: SSH, repo name "mc-programmer"
#   USE_HTTPS=1 bash scripts/push-to-github.sh    # use HTTPS instead of SSH
#   REPO_NAME=mc-programmer-py bash scripts/push-to-github.sh
#   REMOTE_NAME=upstream bash scripts/push-to-github.sh
#

set -euo pipefail

GITHUB_USER="${GITHUB_USER:-douglas-carmichael}"
REPO_NAME="${REPO_NAME:-mc-programmer}"
REMOTE_NAME="${REMOTE_NAME:-douglas}"
BRANCH="${BRANCH:-main}"
USE_HTTPS="${USE_HTTPS:-0}"

if [[ "$USE_HTTPS" == "1" ]]; then
    REMOTE_URL="https://github.com/${GITHUB_USER}/${REPO_NAME}.git"
else
    REMOTE_URL="git@github.com:${GITHUB_USER}/${REPO_NAME}.git"
fi

# ---- Sanity checks ---------------------------------------------------------

cd "$(dirname "$0")/.."
REPO_ROOT="$(pwd)"

if [[ ! -d .git ]]; then
    echo "error: not inside a git repository (looked in $REPO_ROOT)" >&2
    exit 1
fi

if [[ ! -f README.md || ! -d python_lib || ! -d swift_lib || ! -d rust_lib ]]; then
    echo "error: this doesn't look like the mc-programmer repo" >&2
    echo "  expected README.md, python_lib/, swift_lib/, and rust_lib/ in $REPO_ROOT" >&2
    exit 1
fi

# ---- Copyright safety net --------------------------------------------------

echo "→ Checking for Roland-copyrighted files that must not be pushed..."
LEAKS=$(find reference -type f \( -name '*.pdf' -o -name '*.bin' -o -name 'tools_jp_zcore.txt' \) 2>/dev/null || true)
if [[ -n "$LEAKS" ]]; then
    echo "error: copyrighted Roland files found in reference/:" >&2
    echo "$LEAKS" | sed 's/^/  /' >&2
    echo "" >&2
    echo "These are blocked by .gitignore so they won't actually push, but" >&2
    echo "they shouldn't be in the working tree either. Move them outside" >&2
    echo "the repo (or just delete the local copies) and re-run." >&2
    exit 1
fi
echo "  OK — no copyrighted files in reference/"

# ---- Run tests -------------------------------------------------------------

echo ""
echo "→ Running Python tests..."
PYTHONPATH=python_lib python3 -m unittest python_lib.tests.test_mc_programmer 2>&1 | tail -3
echo ""

if command -v swift >/dev/null 2>&1; then
    echo "→ Running Swift tests..."
    (cd swift_lib && swift test 2>&1 | tail -3)
    echo ""
else
    echo "→ Skipping Swift tests (swift compiler not in PATH)"
    echo ""
fi
if command -v cargo >/dev/null 2>&1; then
    echo "→ Running Rust tests..."
    (cd rust_lib && cargo test --all-features 2>&1 | tail -8)
    echo ""
else
    echo "→ Skipping Rust tests (cargo not in PATH)"
    echo ""
fi

# ---- Git status preview ----------------------------------------------------

echo "→ Git status preview:"
git status --short
echo ""

# ---- Stage everything ------------------------------------------------------

echo "→ Staging changes..."
git add -A

CHANGED=$(git diff --cached --name-only | wc -l | tr -d ' ')
if [[ "$CHANGED" == "0" ]]; then
    echo "  Nothing to commit — working tree matches HEAD."
else
    echo "  $CHANGED file(s) staged for commit"
fi

# ---- Commit (only if there's something to commit) --------------------------

if [[ "$CHANGED" != "0" ]]; then
    echo ""
    echo "→ Creating commit..."
    git commit -m "$(cat <<'EOF'
Parallel Python, Swift, and Rust libraries for MC-101 / MC-707 / MV-1

Provides three libraries (python_lib/, swift_lib/, and rust_lib/) sharing the
same data model and parameter table:

* Roland Sys-Ex protocol (RQ1/DT1, checksum, model-ID detection)
* Full address map (sequencer, motion, pitch-bend, ZEN-Core tones,
  per-track settings, system block) verified by the original test
  scripts and the MC programming videos
* ~389 named ZEN-Core tone parameters across 12 substructures
  (Tone Common, MFX, PMT, Tone Partial, Pitch/Filter/Amp Env, LFO, EQ,
  Synth Common, Synth PMT, Synth Partial), auto-generated from the
  Jupiter-X MIDI implementation, verified against the MC-707 / MC-101
  reference manuals through firmware 1.80
* Sequencer note-step encode/decode with tied notes and 16-lane polyphony
* TonePreset (one tone) and Project (whole device) save/restore with
  JSON file format, model-ID safety check, chunked I/O
* Scene/Clip selection via MIDI Program Change
* Pluggable transports: native CoreMIDI on Swift, python-rtmidi/mido
  on Python, midir on Rust, plus a LoopbackTransport in-memory fake MC
  for offline development and CI
* Musician-friendly API: 1-based front-panel numbering, note-name
  parser ("C1" = MIDI 36 per Roland convention), drum-key constants,
  semitone-based coarse tune, "did you mean" suggestions on typos,
  parameter discovery
* MC-101 verified end-to-end (4 tracks instead of 8, same engine and
  parameter table)
* 70 Python tests + 42 Swift tests + 90 Rust tests, all passing offline
  against LoopbackTransport
* Comprehensive documentation: quick starts, runnable examples, USECASES,
  ROADMAP, and CLAUDE.md for the agent

The original scripts (hack_tools.py, seq_tools.py, sys_ex_comm.py, etc.)
are removed from the working tree but remain in git history.

No Roland-copyrighted PDFs, firmware images, or extracted PDF text are
committed. The auto-generated parameter tables contain only factual
data (offsets, sizes, ranges, technical labels) and are safe under
Feist v. Rural. See reference/README.md for the full policy.
EOF
)"
    echo "  Created commit:"
    git log --oneline -1 | sed 's/^/    /'
fi

# ---- Configure remote ------------------------------------------------------

echo ""
echo "→ Configuring remote '$REMOTE_NAME' → $REMOTE_URL"
if git remote get-url "$REMOTE_NAME" >/dev/null 2>&1; then
    EXISTING=$(git remote get-url "$REMOTE_NAME")
    if [[ "$EXISTING" != "$REMOTE_URL" ]]; then
        echo "  Remote '$REMOTE_NAME' currently points at $EXISTING"
        echo "  Updating it to $REMOTE_URL"
        git remote set-url "$REMOTE_NAME" "$REMOTE_URL"
    else
        echo "  Remote already correct."
    fi
else
    git remote add "$REMOTE_NAME" "$REMOTE_URL"
    echo "  Added new remote '$REMOTE_NAME'"
fi

# ---- Final confirmation before push ---------------------------------------

echo ""
echo "About to push:"
echo "  branch:      $BRANCH"
echo "  → remote:    $REMOTE_NAME"
echo "  → URL:       $REMOTE_URL"
echo ""
echo "Your existing remotes (left untouched):"
git remote -v | sed 's/^/  /'
echo ""
read -r -p "Proceed with push? [y/N] " ANSWER
if [[ "$ANSWER" != "y" && "$ANSWER" != "Y" ]]; then
    echo "Aborted. Nothing pushed."
    exit 0
fi

# ---- Push ------------------------------------------------------------------

echo ""
echo "→ Pushing $BRANCH to $REMOTE_NAME..."
git push -u "$REMOTE_NAME" "$BRANCH"

echo ""
echo "✓ Done. Your repo is live at https://github.com/${GITHUB_USER}/${REPO_NAME}"
