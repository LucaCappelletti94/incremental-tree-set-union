#!/usr/bin/env bash
set -euo pipefail

SESSION="fuzz-itsu"
DIR="$(cd "$(dirname "$0")" && pwd)"

# Kill existing session if any
tmux kill-session -t "$SESSION" 2>/dev/null || true

# Build all targets first
echo "Building fuzz targets..."
cd "$DIR"
cargo fuzz build
# new inputs land in fuzz/corpus, the committed seeds stay read-only
mkdir -p fuzz/corpus/fuzz_static fuzz/corpus/fuzz_incremental fuzz/corpus/fuzz_both

# Create tmux session with first fuzzer
tmux new-session -d -s "$SESSION" -n fuzz \
  "cd '$DIR' && cargo fuzz run fuzz_static fuzz/corpus/fuzz_static fuzz/seeds/fuzz_static -- -jobs=0; read"

# Split horizontally for second fuzzer
tmux split-window -t "$SESSION" -h \
  "cd '$DIR' && cargo fuzz run fuzz_incremental fuzz/corpus/fuzz_incremental fuzz/seeds/fuzz_incremental -- -jobs=0; read"

# Split the left pane vertically for third fuzzer
tmux split-window -t "$SESSION".0 -v \
  "cd '$DIR' && cargo fuzz run fuzz_both fuzz/corpus/fuzz_both fuzz/seeds/fuzz_both -- -jobs=0; read"

# Even out the layout
tmux select-layout -t "$SESSION" tiled

# Attach
tmux attach-session -t "$SESSION"
