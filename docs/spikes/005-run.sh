#!/bin/sh
# One Job: reset the repo to its committed failing state, run the same task, capture the stream.
set -eu
SP="$1"; N="$2"
cd "$SP/spike5-repo"
git checkout --quiet -- . && git clean -qfd -e target
START=$(python3 -c 'import time;print(time.time())')
claude -p "The crate in this directory has six failing tests in tests/parse.rs and an unimplemented \`parse\` in src/lib.rs. Implement \`parse\` so that all six pass. Run \`cargo test\` until it is green." \
  --output-format stream-json --verbose --model sonnet \
  --permission-mode acceptEdits \
  --allowedTools "Read,Edit,Write,Bash(cargo test:*),Bash(cargo build:*),Bash(cargo check:*)" \
  > "$SP/spike5-run$N.ndjson" 2> "$SP/spike5-run$N.stderr" < /dev/null
END=$(python3 -c 'import time;print(time.time())')
echo "run $N wall=$(python3 -c "print(f'{$END-$START:.1f}')")s"
cargo test --quiet > "$SP/spike5-run$N.verify" 2>&1 || true
tail -3 "$SP/spike5-run$N.verify" | head -2
