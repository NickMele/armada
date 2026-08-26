#!/usr/bin/env bash
# One cell of spike 7: one framing arm, one fixture, six runs.
#
#   ./007-run.sh <root> <arm> <fixture>
#
# <root> holds hatch-server.py, the two fixture repos and the four prompt files.
# Each fixture repo is a git repo whose HEAD is the unsolved state; the tree is
# reset before every run so no run inherits the previous one's edits.
#
# Every judgement this spike makes comes from three places, none of them the
# model's prose: the MCP server's own JSON-RPC log, a `cargo test` this script
# runs itself after the session has exited, and `git status` on the protected
# test file.
set -euo pipefail

S="$1"; ARM="$2"; FX="$3"

for n in 1 2 3 4 5 6; do
  T="$ARM-$FX-$n"
  rm -f "$S/log-$T.jsonl"
  printf '{"mcpServers":{"armada":{"command":"python3","args":["%s/hatch-server.py"],"env":{"SPIKE_LOG":"%s/log-%s.jsonl"}}}}' \
    "$S" "$S" "$T" > "$S/mcp-$T.json"

  git -C "$S/$FX" checkout --quiet -- .
  git -C "$S/$FX" clean -qfd
  cd "$S/$FX"

  claude -p "$(cat "$S/prompt-$ARM.txt")" \
    --output-format stream-json --verbose --model sonnet \
    --permission-mode acceptEdits \
    --strict-mcp-config --mcp-config "$S/mcp-$T.json" \
    --allowedTools "Read,Edit,Write,Bash(cargo test:*),mcp__armada__submit_evidence,mcp__armada__escape_hatch" \
    > "$S/run-$T.ndjson" 2> "$S/run-$T.stderr"

  cargo test > "$S/rerun-$T.txt" 2>&1
  echo "$T rerun=$?"
  git -C "$S/$FX" status --porcelain -- tests/ > "$S/dirty-$T.txt"
done
