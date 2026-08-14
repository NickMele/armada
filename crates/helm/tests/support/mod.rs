//! Scratch machines for the end-to-end suite.
//!
//! Every test gets its own `$HOME`, so `~/.char/char.db` is a fresh
//! machine-global store rather than the developer's. That is possible only
//! because the entrypoint reads `$HOME` once and passes it down — the same
//! property that makes `--project` and `--all` expressible at all.

// Each integration test binary compiles this module separately, so anything
// only one of them uses is "dead" in the others. The alternative is a helper
// per file, which is how two test suites start disagreeing about what a scratch
// machine is.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};

/// The binary this workspace just built.
pub fn char_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_char"))
}

/// A scratch machine: one `$HOME`, one `char.db`, and repos under it.
pub struct Machine {
    /// The fake `$HOME`.
    pub home: tempfile::TempDir,
    /// Where scratch repositories live.
    pub root: tempfile::TempDir,
}

impl Machine {
    pub fn new() -> Self {
        Machine {
            home: tempfile::tempdir().unwrap(),
            root: tempfile::tempdir().unwrap(),
        }
    }

    /// A git repository with a committed `char.yml` and the helper scripts the
    /// dispatch tests need.
    ///
    /// Committed rather than merely written, because `git worktree add` gives a
    /// worktree the tree at a commit — and the five-worktrees case turns on
    /// every sibling sharing **the same committed config**.
    pub fn repo(&self, name: &str, config: &str) -> PathBuf {
        let path = self.root.path().join(name);
        std::fs::create_dir_all(&path).unwrap();
        git(&path, &["init", "-q", "-b", "main"]);
        std::fs::write(path.join("char.yml"), config).unwrap();
        write_script(&path, "exiter.sh", "#!/bin/sh\nexit \"$1\"\n");
        write_script(
            &path,
            "enver.sh",
            "#!/bin/sh\necho \"declared=$DECLARED\"\necho \"workspace=$CHAR_WORKSPACE\"\necho \"home=$HOME\"\n",
        );
        git(&path, &["add", "-A"]);
        git(
            &path,
            &[
                "-c",
                "user.email=char@example.test",
                "-c",
                "user.name=char",
                "commit",
                "-q",
                "-m",
                "scratch",
            ],
        );
        std::fs::canonicalize(&path).unwrap()
    }

    /// A git worktree of `repo`, which is a **sibling workspace**: same
    /// committed config, its own id, its own block, its own lifecycle.
    pub fn worktree(&self, repo: &Path, name: &str) -> PathBuf {
        let path = self.root.path().join(name);
        git(
            repo,
            &["worktree", "add", "-q", path.to_str().unwrap(), "-b", name],
        );
        std::fs::canonicalize(&path).unwrap()
    }

    /// A directory that is not a workspace at all — where `clean --orphaned`
    /// is most needed.
    pub fn outside(&self) -> PathBuf {
        let path = self.root.path().join("elsewhere");
        std::fs::create_dir_all(&path).unwrap();
        std::fs::canonicalize(&path).unwrap()
    }

    fn command(&self, cwd: &Path, args: &[&str]) -> Command {
        let mut command = Command::new(char_binary());
        command
            .args(args)
            .current_dir(cwd)
            .env("HOME", self.home.path())
            // Kept deliberately small: an inherited environment full of the
            // developer's variables makes a failure depend on whose machine it
            // ran on.
            .env_clear()
            .env("HOME", self.home.path())
            .env("PATH", std::env::var("PATH").unwrap_or_default());
        // The one exception to the small environment, and it is not the
        // developer's: a coverage build writes its counters to the file named
        // by `LLVM_PROFILE_FILE`, and a binary that inherits no such variable
        // writes `default.profraw` into its working directory instead — a
        // scratch tempdir this suite deletes. Dropping it is why every line
        // reachable only through the real binary reads as never executed, so
        // the e2e tier silently stopped counting toward the coverage floor the
        // moment there was one. Absent outside a coverage run, so this is a
        // no-op for `cargo test`.
        if let Ok(profile) = std::env::var("LLVM_PROFILE_FILE") {
            command.env("LLVM_PROFILE_FILE", profile);
        }
        command
    }

    /// Run to completion.
    pub fn run(&self, cwd: &Path, args: &[&str]) -> Output {
        self.command(cwd, args).output().expect("char runs")
    }

    /// Start and leave running.
    pub fn spawn(&self, cwd: &Path, args: &[&str]) -> Child {
        self.command(cwd, args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("char runs")
    }
}

fn git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_script(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    std::fs::write(&path, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}
