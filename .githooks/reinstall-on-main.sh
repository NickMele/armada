#!/bin/sh
# .githooks/reinstall-on-main.sh
#
# Shared logic for the post-merge and post-checkout hooks: when `main` moves
# and the change could affect the built binary, rebuild and reinstall it with
# `cargo install --path crates/helm --force` (crates/helm produces the
# `armada` and `arm` binaries). Enable both hooks with:
#
#   git config core.hooksPath .githooks
#
# Design notes:
#
#   - Synchronous, not backgrounded. `cargo install` can take a couple of
#     minutes, but a background install that fails can fail unseen, and an
#     install failure the user doesn't know about is worse than a terminal
#     that blocks for a minute: he'd keep testing a stale binary believing
#     it's the merge he just pulled. If the wait is unwelcome, the escape
#     hatch is to not opt in (`core.hooksPath` is per-clone, not automatic),
#     not a background mode that can fail quietly.
#   - A failed build must never look like success. `cargo install` only
#     replaces the installed binary after a successful build, so the old
#     binary staying in place is correct — but the hook prints a loud banner
#     on any non-zero exit so that's obvious rather than assumed.
#   - Bounded wait on the build lock, not a fight with it. Several agents can
#     be running `cargo build`/`cargo test` against the same target dir at
#     once; cargo will just block on the lock. We give it
#     $REINSTALL_HOOK_TIMEOUT seconds (default 180) and, if it's still
#     running after that, kill it, say so, and exit — we never hang a git
#     command indefinitely.
#   - Every exit path is 0. post-merge/post-checkout can't fail the git
#     operation they're attached to anyway, but stray non-zero exits and
#     unstructured output still confuse tooling that watches hook exit
#     status, so this hook is always well-behaved on that front.

set -eu

HOOK_NAME="${1:-post-merge}"

REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null) || exit 0
cd "$REPO_ROOT" || exit 0

# --- Only main ---------------------------------------------------------
branch=$(git symbolic-ref --short -q HEAD || true)
if [ "$branch" != "main" ]; then
    exit 0
fi

# --- Work out the commit range this hook is reacting to ----------------
case "$HOOK_NAME" in
    post-checkout)
        prev_head="${2:-}"
        new_head="${3:-}"
        branch_flag="${4:-1}"
        # git passes 0 here for a file-level checkout (e.g. `git checkout --
        # some/file`), not a branch switch — nothing to react to.
        [ "$branch_flag" = "1" ] || exit 0
        [ -n "$prev_head" ] && [ -n "$new_head" ] || exit 0
        [ "$prev_head" != "$new_head" ] || exit 0
        range="$prev_head..$new_head"
        ;;
    post-merge)
        # ORIG_HEAD is set by the merge that just ran.
        prev_head=$(git rev-parse -q --verify ORIG_HEAD 2>/dev/null || true)
        new_head=$(git rev-parse -q --verify HEAD 2>/dev/null || true)
        if [ -z "$prev_head" ] || [ -z "$new_head" ] || [ "$prev_head" = "$new_head" ]; then
            exit 0
        fi
        range="$prev_head..$new_head"
        ;;
    *)
        exit 0
        ;;
esac

# --- Skip quietly unless something that affects the binary changed -----
# Rust sources anywhere in the tree, plus the two files that pin
# dependencies/features (Cargo.toml, Cargo.lock, at any depth). A docs-only
# merge shouldn't cost a rebuild.
changed=$(git diff --name-only "$range" -- '*.rs' 'Cargo.toml' 'Cargo.lock' '**/Cargo.toml' '**/Cargo.lock' 2>/dev/null || true)
if [ -z "$changed" ]; then
    exit 0
fi

label="post-${HOOK_NAME#post-}"
echo "$label: main moved and touched the binary's sources — reinstalling..." >&2

# --- Reinstall: synchronous, bounded ------------------------------------
TIMEOUT="${REINSTALL_HOOK_TIMEOUT:-180}"
LOGFILE=$(mktemp "${TMPDIR:-/tmp}/armada-reinstall.XXXXXX")
CARGO_BIN="${REINSTALL_HOOK_CARGO:-cargo}"

"$CARGO_BIN" install --path crates/helm --force >"$LOGFILE" 2>&1 &
cargo_pid=$!

elapsed=0
while kill -0 "$cargo_pid" 2>/dev/null; do
    if [ "$elapsed" -ge "$TIMEOUT" ]; then
        kill "$cargo_pid" 2>/dev/null || true
        wait "$cargo_pid" 2>/dev/null || true
        echo "############################################################" >&2
        echo "$label: gave up after ${TIMEOUT}s waiting for 'cargo install'" >&2
        echo "(likely the target-dir build lock is held by another cargo" >&2
        echo "run). The binary was NOT reinstalled and may be stale. Once" >&2
        echo "other cargo processes finish, run by hand:" >&2
        echo "  cargo install --path crates/helm --force" >&2
        echo "############################################################" >&2
        rm -f "$LOGFILE"
        exit 0
    fi
    sleep 1
    elapsed=$((elapsed + 1))
done

status=0
wait "$cargo_pid" 2>/dev/null || status=$?

if [ "$status" -ne 0 ]; then
    echo "############################################################" >&2
    echo "$label: REINSTALL FAILED (cargo install exit $status)." >&2
    echo "The installed binary is STALE: it is still the version from" >&2
    echo "before this merge, NOT what you just pulled into main." >&2
    echo "Build log: $LOGFILE" >&2
    echo "Fix the build, then: cargo install --path crates/helm --force" >&2
    echo "############################################################" >&2
    exit 0
fi

cat "$LOGFILE" >&2
echo "$label: reinstalled armada/arm from crates/helm." >&2
rm -f "$LOGFILE"
exit 0
