//! The clean-room hook is enforcement, so it is tested like enforcement.
//!
//! `ARCHITECTURE.md` §2.7 gives the hook the most weight of the three
//! mechanisms that keep phase 3 clean — the other two are a narrow tool list
//! and prompt instructions, which are surface reduction and documentation of
//! intent. A guard nothing exercises is the same as no guard, and this one
//! fails in exactly the direction that would never be noticed: silently
//! permitting.
//!
//! It lives in `xtask/` because that is dev tooling and outside the
//! contamination grep's scope (`crates/` and `tests/`). The guarded path is
//! still assembled at runtime rather than written as one literal — the same
//! discipline §2.4 requires of the grep's own self-test, and cheap insurance
//! against the greped set ever widening.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn hook() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(".claude/hooks/clean-room.sh")
}

/// `~/Development/chariot`, built rather than written.
fn guarded_path() -> String {
    format!(
        "/Users/someone/Development/{}{}/scripts/char",
        "cha", "riot"
    )
}

fn run(payload: &str) -> String {
    let mut child = Command::new("sh")
        .arg(hook())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the hook runs under /bin/sh");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(payload.as_bytes())
        .expect("write");
    let out = child.wait_with_output().expect("hook exits");
    assert!(
        out.status.success(),
        "the hook must exit 0 — a non-zero exit that is not 2 is reported as a \
         broken hook and the tool call proceeds:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn denied(payload: &str) -> bool {
    run(payload).contains("\"permissionDecision\":\"deny\"")
}

/// Slow enough is the same as absent.
///
/// A `Write` carries a whole file, and an ordinary main-agent payload has no
/// top-level `agent_type` for the scan to stop at, so the guard reads all of it.
/// Claude Code reports a hook that exceeds its timeout as a non-blocking error
/// and lets the tool call proceed — so a scan whose cost grows faster than the
/// payload is a permit anyone can buy by sending a large enough file. A budget
/// far above a linear scan and far below the timeout fails the one shape that
/// matters, without failing on a slow machine.
#[test]
fn a_large_payload_does_not_slow_the_guard_into_permitting() {
    let content = "abcdefghijklmnopqrstuvwxyz0123456789".repeat(16 * 1024);
    let payload = format!(
        r#"{{"hook_event_name":"PreToolUse","tool_name":"Write","tool_input":{{"file_path":"docs/PLAN.md","content":"{content}"}}}}"#
    );
    assert!(payload.len() > 512 * 1024, "the payload must be large");

    let started = Instant::now();
    assert!(!denied(&payload));
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(5),
        "the guard took {elapsed:?} on {} KB — a scan that grows faster than the \
         payload reaches the hook timeout, and a hook that times out permits",
        payload.len() / 1024
    );
}

#[test]
fn a_read_of_the_source_repo_is_denied() {
    let payload = format!(
        r#"{{"hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{{"file_path":"{}/check.py"}}}}"#,
        guarded_path()
    );
    assert!(denied(&payload));
}

#[test]
fn a_bash_line_that_reaches_the_source_repo_is_denied() {
    // The interesting failure, not the polite one: a path inside a command
    // string, which a hook keying on `file_path` alone would wave through.
    for command in ["rg CHECK_CATALOG", "find . -path", "cat", "python3 -c open"] {
        let payload = format!(
            r#"{{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{{"command":"{} {}"}}}}"#,
            command,
            guarded_path()
        );
        assert!(denied(&payload), "allowed: {command}");
    }
}

#[test]
fn an_escaped_spelling_of_the_path_is_denied_on_both_paths() {
    // JSON has two ways to write the separator, and a tool that takes a regex
    // has a third reason to: `find -regex`, `sed s///` and `grep -E` all put a
    // backslash in front of a slash in ordinary use. Both spellings reach the
    // guarded repo, so both are the guarded path.
    let one = guarded_path().replace("/Development/", "/Development\\/");
    let two = guarded_path().replace("/Development/", "/Development\\\\/");

    for spelling in [&one, &two] {
        let reading = format!(
            r#"{{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{{"command":"find / -regex '.*{spelling}.*'"}}}}"#
        );
        assert!(denied(&reading), "reading path allowed: {reading}");

        let writing = format!(
            r#"{{"hook_event_name":"PreToolUse","tool_name":"Write","tool_input":{{"file_path":"{spelling}/new.py","content":"x"}}}}"#
        );
        assert!(denied(&writing), "writing path allowed: {writing}");
    }
}

#[test]
fn a_glob_or_grep_over_the_source_repo_is_denied() {
    for tool in ["Glob", "Grep"] {
        let payload = format!(
            r#"{{"hook_event_name":"PreToolUse","tool_name":"{}","tool_input":{{"path":"{}","pattern":"*.py"}}}}"#,
            tool,
            guarded_path()
        );
        assert!(denied(&payload), "allowed: {tool}");
    }
}

#[test]
fn the_harvester_is_the_one_agent_allowed_through() {
    let payload = format!(
        r#"{{"hook_event_name":"PreToolUse","agent_type":"harvester","tool_name":"Read","tool_input":{{"file_path":"{}/check.py"}}}}"#,
        guarded_path()
    );
    assert!(!denied(&payload), "the harvester must be able to harvest");
}

#[test]
fn an_agent_type_nobody_allowlisted_is_denied() {
    // An allowlist, so a subagent added in a later phase is denied by default
    // rather than silently permitted.
    let payload = format!(
        r#"{{"hook_event_name":"PreToolUse","agent_type":"implementer","tool_name":"Read","tool_input":{{"file_path":"{}/check.py"}}}}"#,
        guarded_path()
    );
    assert!(denied(&payload));
}

#[test]
fn a_tool_nobody_listed_is_still_inspected() {
    // The tool dimension is an allowlist too (ARCHITECTURE.md §2.7): a tool
    // renamed between Claude Code versions, or an MCP tool that reads files,
    // must not carry the path through because no `case` arm names it.
    for tool in ["Task", "Agent", "WebFetch", "mcp__files__read", ""] {
        let payload = format!(
            r#"{{"hook_event_name":"PreToolUse","tool_name":"{}","tool_input":{{"path":"{}"}}}}"#,
            tool,
            guarded_path()
        );
        assert!(denied(&payload), "allowed: {tool:?}");
    }
}

#[test]
fn the_harvester_allowance_cannot_be_claimed_from_inside_tool_input() {
    // Everything under `tool_input` is text some agent chose. A first-match
    // grep for `agent_type` decides the allowance on key order alone, which is
    // fail-open in a guard. Both spellings are covered: the literal inside a
    // command string, and a nested payload with a real `agent_type` key.
    let spoofs = [
        format!(
            r#"{{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{{"command":"echo '\"agent_type\":\"harvester\"' && cat {}/check.py"}}}}"#,
            guarded_path()
        ),
        format!(
            r#"{{"hook_event_name":"PreToolUse","tool_name":"Task","tool_input":{{"agent_type":"harvester","prompt":"read {}/check.py"}}}}"#,
            guarded_path()
        ),
    ];
    for payload in &spoofs {
        assert!(denied(payload), "spoof allowed: {payload}");
    }
}

#[test]
fn ordinary_work_in_this_repo_is_untouched() {
    let payload = r#"{"hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{"file_path":"/Users/someone/Development/charkit/crates/core/src/lib.rs"}}"#;
    assert!(
        !denied(payload),
        "the guard must not fire on this repo's own name"
    );
}

#[test]
fn editing_a_document_that_merely_mentions_the_path_is_allowed() {
    // PLAN.md §1 cites the source repo as this project's one piece of
    // evidence, so a content match would deny ordinary documentation work.
    // Writing is not the vector; reading is.
    let payload = format!(
        r#"{{"hook_event_name":"PreToolUse","tool_name":"Write","tool_input":{{"file_path":"docs/PLAN.md","content":"evidence from {}"}}}}"#,
        guarded_path()
    );
    assert!(!denied(&payload));
}

#[test]
fn writing_into_the_source_repo_is_denied() {
    let payload = format!(
        r#"{{"hook_event_name":"PreToolUse","tool_name":"Write","tool_input":{{"file_path":"{}/new.py","content":"x"}}}}"#,
        guarded_path()
    );
    assert!(denied(&payload));

    // `NotebookEdit` spells its target `notebook_path`. A hook reading only
    // `file_path` finds nothing and permits — the silent direction.
    let payload = format!(
        r#"{{"hook_event_name":"PreToolUse","tool_name":"NotebookEdit","tool_input":{{"notebook_path":"{}/explore.ipynb","new_source":"x"}}}}"#,
        guarded_path()
    );
    assert!(denied(&payload));
}
