#!/usr/bin/env bash
# Reproduce a Drone: exact env from crates/fleet/src/drone.rs environment(),
# exact flags from crates/adapters/src/harness.rs render(), cwd a worktree,
# detached with setsid. One adversarial prompt per invocation, on argv like
# spikes 6/7, transcript to stdout parsed for tool_use and permission denials.
SP="/private/tmp/claude-501/-Users-user-Development-armada/9e12a4c8-adcf-40d1-b5e9-a454c1aea9f7/scratchpad"
PROMPT="$1"; TAG="$2"
CLAUDE=/Users/user/.local/bin/claude
DRONE_PATH="/Users/user/.cargo/bin:/Users/user/.local/bin:/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin"
# The drone's exact allowlist for the tests step (see live ps 88187), minus the
# 3 mcp tools since we run --strict-mcp-config with an empty config.
ALLOW="mcp__armada__submit_evidence,mcp__armada__declare_scope,mcp__armada__run_checks,Read,Glob,Grep,Edit,Write,Bash(cargo fmt --all:*),Bash(cargo xtask verify-foundations:*)"
cd "$SP/drone-wt"
/usr/bin/env -i \
  PATH="$DRONE_PATH" HOME="/Users/user" LANG="en_US.UTF-8" TERM="dumb" USER="user" \
  "$CLAUDE" -p "$PROMPT" \
    --output-format stream-json --verbose --model sonnet \
    --permission-mode dontAsk \
    --strict-mcp-config --mcp-config "$SP/empty-mcp.json" \
    --allowedTools "$ALLOW" \
    > "$SP/out-$TAG.ndjson" 2> "$SP/out-$TAG.stderr"
echo "exit=$? tag=$TAG"
