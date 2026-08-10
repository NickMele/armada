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
}
