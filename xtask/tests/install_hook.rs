//! End-to-end tests for `.githooks/reinstall-on-main.sh` (the post-merge and
//! post-checkout reinstall hook), driven through real `git merge`/`git
//! checkout` against a throwaway repo — never against this repo's own
//! `.git`, and never against a real toolchain.
//!
//! Every scenario points `core.hooksPath` at the *actual* `.githooks/`
//! this branch ships (found via `CARGO_MANIFEST_DIR`), so these tests
//! exercise the committed scripts verbatim rather than a copy that could
//! drift from them. Every scenario also reroutes `REINSTALL_HOOK_CARGO` at a
//! fake `cargo` written to a scratch directory: the fake never runs a real
//! build, a real install, or touches `~/.cargo/bin` — it only does what
//! `FAKE_CARGO_MODE` tells it to. No test spends a token or a build.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Absolute path to the real `.githooks/` this branch ships.
fn hooks_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .join(".githooks")
}

/// A disposable repo, unique per test and per process — a leftover directory
/// from a killed run must never be adopted by the next one (the same
/// discipline `xtask/src/privacy.rs`'s own tests use for their scratch
/// dirs).
struct Repo {
    dir: PathBuf,
}

impl Repo {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "armada-install-hook-{label}-{}",
            std::process::id()
        ));
        fs::remove_dir_all(&dir).ok();
        fs::create_dir_all(&dir).expect("scratch repo dir");
        let repo = Repo { dir };

        repo.git(&["init", "-q", "-b", "main"]);
        repo.git(&["config", "user.email", "hook-test@example.invalid"]);
        repo.git(&["config", "user.name", "Hook Test"]);
        // A throwaway repo has no signing key configured; this only ever
        // touches this scratch repo's local config, never the real one.
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo.git(&[
            "config",
            "core.hooksPath",
            hooks_dir().to_str().expect("hooks dir is utf8"),
        ]);

        repo.write("README.md", "init\n");
        repo.git(&["add", "-A"]);
        repo.git(&["commit", "-q", "-m", "init"]);
        repo
    }

    fn write(&self, rel: &str, body: &str) {
        let path = self.dir.join(rel);
        fs::create_dir_all(path.parent().expect("a parent")).expect("scratch subdir");
        fs::write(path, body).expect("scratch file");
    }

    /// Plain git commands used to set the scene — never routed through the
    /// hook, so `REINSTALL_HOOK_CARGO`/`REINSTALL_HOOK_TIMEOUT` are stripped
    /// in case the ambient environment happens to set them.
    fn git(&self, args: &[&str]) -> Output {
        Command::new("git")
            .arg("-C")
            .arg(&self.dir)
            .args(args)
            .env_remove("REINSTALL_HOOK_CARGO")
            .env_remove("REINSTALL_HOOK_TIMEOUT")
            .output()
            .unwrap_or_else(|e| panic!("git {args:?} failed to run: {e}"))
    }

    /// The commands that actually fire the hook — `merge`/`checkout` — run
    /// with the fake `cargo` wired in via `REINSTALL_HOOK_CARGO`. Returns the
    /// command's own exit status alongside its stderr (the hook writes there
    /// exclusively), since the whole point of rule 6 is that the git
    /// operation must succeed regardless of what the hook found.
    fn git_hooked(&self, args: &[&str], fake_cargo: &Path, extra_env: &[(&str, &str)]) -> (Output, String) {
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(&self.dir).args(args);
        cmd.env("REINSTALL_HOOK_CARGO", fake_cargo);
        for (k, v) in extra_env {
            cmd.env(k, v);
        }
        let out = cmd
            .output()
            .unwrap_or_else(|e| panic!("git {args:?} failed to run: {e}"));
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        (out, stderr)
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.dir).ok();
    }
}

/// A scratch directory to hold one test's fake `cargo`, cleaned up with the
/// repo itself is not enough — this lives outside the git repo so a `git
/// clean`/`reset` inside a test can never touch it.
struct Bin {
    dir: PathBuf,
}

impl Bin {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "armada-install-hook-bin-{label}-{}",
            std::process::id()
        ));
        fs::remove_dir_all(&dir).ok();
        fs::create_dir_all(&dir).expect("scratch bin dir");
        Bin { dir }
    }

    /// Writes a fake `cargo` that never runs a real build, a real install, or
    /// touches `~/.cargo/bin` — it only does what `FAKE_CARGO_MODE` says:
    /// `ok` (default) succeeds immediately, `fail` exits non-zero, `hang`
    /// sleeps past the hook's timeout so the lock-wait path can be exercised
    /// without a real contended build.
    fn fake_cargo(&self) -> PathBuf {
        let path = self.dir.join("cargo");
        fs::write(
            &path,
            "#!/bin/sh\n\
             # Fake cargo for install-hook tests only. Never a real build,\n\
             # never a real install, never touches ~/.cargo/bin.\n\
             case \"${FAKE_CARGO_MODE:-ok}\" in\n\
             \x20   fail)\n\
             \x20       echo \"fake cargo: simulated build failure\" >&2\n\
             \x20       exit 1\n\
             \x20       ;;\n\
             \x20   hang)\n\
             \x20       sleep \"${FAKE_CARGO_SLEEP:-5}\"\n\
             \x20       exit 0\n\
             \x20       ;;\n\
             \x20   *)\n\
             \x20       echo \"fake cargo: simulated install ok\"\n\
             \x20       exit 0\n\
             \x20       ;;\n\
             esac\n",
        )
        .expect("write fake cargo");
        let mut perms = fs::metadata(&path).expect("stat fake cargo").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).expect("chmod fake cargo");
        path
    }
}

impl Drop for Bin {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.dir).ok();
    }
}

fn touch_rust_file(repo: &Repo, branch: &str) {
    repo.git(&["checkout", "-q", "-b", branch]);
    repo.write("crates/helm/src/main.rs", "fn main() {}\n");
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "touch a rust file"]);
}

/// Case 1 of the three required by the task: a Rust change lands on `main`
/// (here via merge) and the hook reinstalls.
#[test]
fn rust_change_merged_into_main_reinstalls() {
    let repo = Repo::new("merge-rust");
    let bin = Bin::new("merge-rust");
    let cargo = bin.fake_cargo();

    touch_rust_file(&repo, "feature");
    repo.git(&["checkout", "-q", "main"]);

    let (out, stderr) = repo.git_hooked(
        &["merge", "--no-ff", "-m", "merge feature", "feature"],
        &cargo,
        &[],
    );

    assert!(out.status.success(), "the merge itself must succeed: {stderr}");
    assert!(
        stderr.contains("main moved and touched the binary's sources"),
        "expected the loud start line, got: {stderr}"
    );
    assert!(
        stderr.contains("reinstalled armada/arm"),
        "expected the success line, got: {stderr}"
    );
}

/// Case 2: a docs-only merge must not cost a rebuild — the hook exits
/// quietly, per rule 3.
#[test]
fn docs_only_merge_is_quiet() {
    let repo = Repo::new("merge-docs");
    let bin = Bin::new("merge-docs");
    let cargo = bin.fake_cargo();

    repo.git(&["checkout", "-q", "-b", "docs"]);
    repo.write("docs/NOTES.md", "nothing binary-affecting here\n");
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "docs only"]);
    repo.git(&["checkout", "-q", "main"]);

    let (out, stderr) = repo.git_hooked(&["merge", "--no-ff", "-m", "merge docs", "docs"], &cargo, &[]);

    assert!(out.status.success(), "the merge itself must succeed: {stderr}");
    assert!(
        stderr.trim().is_empty(),
        "a docs-only merge must produce no hook output, got: {stderr}"
    );
}

/// Case 3: the build fails. The hook must say so loudly, never claim
/// success, and never fail the merge it fired on (rules 1 and 6).
#[test]
fn failed_build_says_so_loudly_and_never_claims_success() {
    let repo = Repo::new("merge-fail");
    let bin = Bin::new("merge-fail");
    let cargo = bin.fake_cargo();

    touch_rust_file(&repo, "feature");
    repo.git(&["checkout", "-q", "main"]);

    let (out, stderr) = repo.git_hooked(
        &["merge", "--no-ff", "-m", "merge feature", "feature"],
        &cargo,
        &[("FAKE_CARGO_MODE", "fail")],
    );

    assert!(
        out.status.success(),
        "a failed install must never fail the git operation it fired on: {stderr}"
    );
    assert!(
        stderr.contains("REINSTALL FAILED"),
        "expected a loud failure banner, got: {stderr}"
    );
    assert!(
        stderr.contains("STALE"),
        "expected the hook to say the binary is now stale, got: {stderr}"
    );
    assert!(
        !stderr.contains("reinstalled armada/arm"),
        "a failed build must never look like a successful install, got: {stderr}"
    );
}

/// Rule 2: merging into a branch other than `main` must never reinstall,
/// even when the merge touches Rust sources.
#[test]
fn merge_into_non_main_branch_is_quiet() {
    let repo = Repo::new("merge-off-main");
    let bin = Bin::new("merge-off-main");
    let cargo = bin.fake_cargo();

    repo.git(&["checkout", "-q", "-b", "dev"]);
    touch_rust_file(&repo, "feature-off-dev");
    repo.git(&["checkout", "-q", "dev"]);

    let (out, stderr) = repo.git_hooked(
        &["merge", "--no-ff", "-m", "merge into dev", "feature-off-dev"],
        &cargo,
        &[],
    );

    assert!(out.status.success(), "the merge itself must succeed: {stderr}");
    assert!(
        stderr.trim().is_empty(),
        "a merge landing on a branch other than main must produce no hook output, got: {stderr}"
    );
}

/// The `post-checkout` half: landing on `main` with a Rust change in the
/// range reinstalls; checking *away* from `main` never does, even across the
/// same range.
#[test]
fn rust_change_via_checkout_onto_main_reinstalls() {
    let repo = Repo::new("checkout-rust");
    let bin = Bin::new("checkout-rust");
    let cargo = bin.fake_cargo();

    repo.git(&["checkout", "-q", "-b", "old"]);
    repo.git(&["checkout", "-q", "main"]);
    repo.write("crates/helm/src/main.rs", "fn main() {}\n");
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "touch rust on main"]);

    // Checking away from main must be silent regardless of the range.
    let (out_away, stderr_away) = repo.git_hooked(&["checkout", "-q", "old"], &cargo, &[]);
    assert!(out_away.status.success());
    assert!(
        stderr_away.trim().is_empty(),
        "checking out away from main must produce no hook output, got: {stderr_away}"
    );

    // Landing back on main, with a Rust change in the range, reinstalls.
    let (out_back, stderr_back) = repo.git_hooked(&["checkout", "-q", "main"], &cargo, &[]);
    assert!(out_back.status.success());
    assert!(
        stderr_back.contains("reinstalled armada/arm"),
        "expected the success line landing back on main, got: {stderr_back}"
    );
}

/// Rule 5: the hook does not fight the build lock. When `cargo` (here the
/// fake standing in for it) is still running past `REINSTALL_HOOK_TIMEOUT`,
/// the hook gives up, says so, and still exits cleanly.
#[test]
fn build_lock_timeout_gives_up_and_says_so() {
    let repo = Repo::new("timeout");
    let bin = Bin::new("timeout");
    let cargo = bin.fake_cargo();

    touch_rust_file(&repo, "feature");
    repo.git(&["checkout", "-q", "main"]);

    let (out, stderr) = repo.git_hooked(
        &["merge", "--no-ff", "-m", "merge feature", "feature"],
        &cargo,
        &[
            ("FAKE_CARGO_MODE", "hang"),
            ("FAKE_CARGO_SLEEP", "5"),
            ("REINSTALL_HOOK_TIMEOUT", "1"),
        ],
    );

    assert!(
        out.status.success(),
        "giving up on the lock must never fail the git operation: {stderr}"
    );
    assert!(
        stderr.contains("gave up after 1s"),
        "expected the bounded-wait message, got: {stderr}"
    );
    assert!(
        !stderr.contains("reinstalled armada/arm"),
        "a timed-out install must never claim success, got: {stderr}"
    );
}
