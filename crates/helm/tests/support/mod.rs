//! Scratch machines for the end-to-end suite.
//!
//! Every test gets its own `$HOME`, so `~/.armada/manifest.db` is a fresh
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
use std::sync::OnceLock;

/// The binary this workspace just built.
pub fn armada_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_armada"))
}

/// Why a command failed, in the form a reader can act on.
///
/// **`stderr` alone is not the answer, and assuming it was is what left six
/// tests failing in CI unread for a day.** Armada reports a refusal as an
/// envelope on *stdout* — `--json` or not — and writes nothing to `stderr`, so
/// an assertion message built from `stderr` renders as `export failed: ` with
/// the reason sitting in the buffer next to it. This prints both, labelled, and
/// the exit status, which is the third thing a reader wants and the one neither
/// stream carries.
pub fn why(output: &Output) -> String {
    format!(
        "exit {}\nstdout: {}\nstderr: {}",
        output
            .status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "signal".to_string()),
        String::from_utf8_lossy(&output.stdout).trim(),
        String::from_utf8_lossy(&output.stderr).trim(),
    )
}

/// A directory holding a stub `claude`, prepended to every scratch machine's
/// `PATH`.
///
/// **Armada refuses to `init` a machine with no `claude` on `PATH`** — a
/// preflight failure by design, since everything Fleet and Helm do runs through
/// it. So until this existed, the guild suite passed on a developer's machine
/// for the sole reason that the developer had Claude Code installed, and failed
/// on every CI runner, which does not. That is the `DOCKER_CONFIG` trap again:
/// a test whose outcome is decided by the machine's global state rather than by
/// anything the test set up (`docs/traps.md`).
///
/// **It is also the only way to honour "no test starts a real session".**
/// `armada doctor` does not stop at `claude --version`: it runs `claude --help`
/// and then [`armada_core::fleet::drone::probe_argv`], which *starts* a session
/// and exits at EOF. On a developer's machine that was the real binary. Here it
/// is nine lines of `sh` that talk to nothing.
///
/// The stub answers exactly the three probes Armada makes and nothing else:
///
/// | invocation | answer |
/// |---|---|
/// | `claude --version` | a fixed version banner |
/// | `claude --help` | every flag in `drone::FLAGS` and `helm::FLAGS` |
/// | `claude plugin validate <dir>` | exit 0, silent |
/// | anything else | exit 0, silent — never a turn, never a token |
fn stub_bin() -> &'static Path {
    static STUB: OnceLock<tempfile::TempDir> = OnceLock::new();
    let dir = STUB.get_or_init(|| {
        let dir = tempfile::tempdir().expect("a scratch directory");
        // Written from the constants rather than retyped, so a flag added to
        // either list is offered by the stub on the next run instead of turning
        // `doctor` red for a reason that has nothing to do with the test.
        let flags: Vec<&str> = armada_core::fleet::drone::FLAGS
            .iter()
            .chain(armada_core::helm::FLAGS.iter())
            .copied()
            .collect();
        write_script(
            dir.path(),
            "claude",
            &format!(
                "#!/bin/sh\n\
                 # A stub. It answers Armada's probes and talks to nothing.\n\
                 case \"$1\" in\n\
                 --version) echo '2.0.14 (Claude Code)'; exit 0 ;;\n\
                 --help) printf '%s\\n' {}; exit 0 ;;\n\
                 esac\n\
                 exit 0\n",
                flags
                    .iter()
                    .map(|flag| format!("'{flag}'"))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        );
        dir
    });
    dir.path()
}

/// The floor of the range scratch machines allocate from.
///
/// **High, and deliberately nowhere near the default.** The suite must not
/// reach for the ports a developer's own stack is using, and 5460 is exactly
/// where a developer's Armada puts its first workspace.
const SCRATCH_FLOOR: u16 = 20_000;

/// How many distinct bases the scratch range holds, at one block apart.
///
/// `20000 + 1200 * 10 = 32000`, which is under
/// [`armada_core::ports::PORT_CEILING`] with room for a block above every slot.
const SCRATCH_SLOTS: u32 = 1_200;

/// A base no other Armada on this machine is reaching for.
///
/// **The suite asserted on global machine state and called the failures
/// flakiness.** A golden snapshot says a probed port is `RESERVED`; anything
/// else on the machine holding 5460 makes it `CONFLICT`, and the thing most
/// likely to be holding it is another Armada — every one of them claimed the
/// same first block, because the base was a constant. Three agents hit it and
/// two wrote it off.
///
/// The pid spreads concurrent test binaries and the counter spreads the scratch
/// machines inside one, so neither two runs nor two `Machine`s share a floor.
/// This is a mitigation and not a proof: two processes whose pids land on the
/// same slot still collide, at roughly one in [`SCRATCH_SLOTS`]. What it removes
/// is the *systematic* collision, where sharing was the guaranteed outcome
/// rather than the unlucky one.
fn scratch_port_base() -> u16 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let slot = NEXT.fetch_add(1, Ordering::Relaxed);
    let spread = (std::process::id().wrapping_mul(7).wrapping_add(slot)) % SCRATCH_SLOTS;
    SCRATCH_FLOOR + (spread * 10) as u16
}

/// A scratch machine: one `$HOME`, one `manifest.db`, and repos under it.
pub struct Machine {
    /// The fake `$HOME`.
    pub home: tempfile::TempDir,
    /// Where scratch repositories live.
    pub root: tempfile::TempDir,
    /// The first port this machine hands out. See [`scratch_port_base`].
    pub port_base: u16,
}

impl Machine {
    pub fn new() -> Self {
        let machine = Machine {
            home: tempfile::tempdir().unwrap(),
            root: tempfile::tempdir().unwrap(),
            port_base: scratch_port_base(),
        };
        machine.seed_git_identity();
        machine
    }

    /// Give the scratch `$HOME` a git identity of its own.
    ///
    /// **Guild's verbs commit, and a commit needs an author.** With `$HOME`
    /// pointed at an empty directory there is no `.gitconfig` to read, so git
    /// falls back to guessing one from `getpwuid` and the hostname — and it
    /// *refuses the guess* unless the hostname canonicalises to something with a
    /// domain in it. A developer's laptop is `something.local` and the guess
    /// stands; a container's hostname is a hex id and it does not, so every
    /// guild verb fails with `Author identity unknown` and nothing about the
    /// test says why.
    ///
    /// So the identity is written rather than inherited. The suite is not
    /// testing git's fallback, and a test whose outcome depends on what the host
    /// is called is not a test.
    ///
    /// **Not `--global` and not the developer's.** It lands in the scratch
    /// `$HOME` this machine already owns and dies with it.
    fn seed_git_identity(&self) {
        std::fs::write(
            self.home.path().join(".gitconfig"),
            "[user]\n\
             \tname = Armada\n\
             \temail = armada@example.test\n\
             [init]\n\
             \tdefaultBranch = main\n\
             [commit]\n\
             \tgpgsign = false\n\
             [tag]\n\
             \tgpgsign = false\n",
        )
        .unwrap();
    }

    /// Put this machine's `port_base` in `machine.yml`, if nothing is there yet.
    ///
    /// **Written just before a command rather than at construction, and the
    /// reason is `armada init`.** That verb refuses a `~/.armada/` that already
    /// exists — the whole of its first-run contract — so a harness that created
    /// the directory up front would make every test of it assert against a
    /// machine that had been here before. So the seeding waits, and skips the
    /// one verb whose subject is the directory's absence.
    ///
    /// **Never overwrites.** Several tests write a `machine.yml` of their own to
    /// exercise `cpu_slots` or the pre-namespace layout, and clobbering one
    /// would be the harness quietly answering the question the test is asking.
    /// Those tests claim no port block, so the default base is correct for
    /// them.
    fn seed_machine_yml(&self, args: &[&str]) {
        if args.first() == Some(&"init") {
            return;
        }
        self.seed_port_base();
    }

    /// The same seeding, for a test that builds an `App` itself rather than
    /// running the binary — `golden.rs` does, and it is the suite that most
    /// needs its own floor.
    pub fn seed_port_base(&self) {
        let armada = self.home.path().join(".armada");
        let path = armada.join("machine.yml");
        if path.exists() {
            return;
        }
        std::fs::create_dir_all(&armada).unwrap();
        std::fs::write(
            &path,
            format!("manifest:\n  port_base: {}\n", self.port_base),
        )
        .unwrap();
    }

    /// A git repository with a committed `armada.yml` and the helper scripts the
    /// dispatch tests need.
    ///
    /// Committed rather than merely written, because `git worktree add` gives a
    /// worktree the tree at a commit — and the five-worktrees case turns on
    /// every sibling sharing **the same committed config**.
    pub fn repo(&self, name: &str, config: &str) -> PathBuf {
        let path = self.root.path().join(name);
        std::fs::create_dir_all(&path).unwrap();
        git(&path, &["init", "-q", "-b", "main"]);
        std::fs::write(path.join("armada.yml"), config).unwrap();
        write_script(&path, "exiter.sh", "#!/bin/sh\nexit \"$1\"\n");
        // **A child that prints its argv and interprets none of it.**
        //
        // The dispatch suite needs one, and `echo` is not it. GNU coreutils'
        // `/bin/echo` claims `--help` for itself when it is the only argument
        // and prints its own usage; BSD's on macOS prints `--help`. So a
        // fixture built on `echo` asserted that `armada manifest <name> --help`
        // reaches the child by asking a program that *eats* `--help` — green on
        // macOS, red on Linux, and the red one looked like Armada swallowing
        // the flag when Armada had passed it through correctly.
        //
        // `printf '%s\n' "$*"` has no options of its own to claim: the format
        // is a literal, and every argument is data.
        write_script(&path, "echoer.sh", "#!/bin/sh\nprintf '%s\\n' \"$*\"\n");
        write_script(
            &path,
            "enver.sh",
            "#!/bin/sh\necho \"declared=$DECLARED\"\necho \"workspace=$ARMADA_WORKSPACE\"\necho \"home=$HOME\"\n",
        );
        git(&path, &["add", "-A"]);
        git(
            &path,
            &[
                "-c",
                "user.email=Armada@example.test",
                "-c",
                "user.name=Armada",
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
        self.seed_machine_yml(args);
        let mut command = Command::new(armada_binary());
        command
            .args(args)
            .current_dir(cwd)
            .env("HOME", self.home.path())
            // Kept deliberately small: an inherited environment full of the
            // developer's variables makes a failure depend on whose machine it
            // ran on.
            .env_clear()
            .env("HOME", self.home.path())
            // The stub `claude` goes first, so what Armada finds is the same
            // program on a developer's machine and on a runner. See [`stub_bin`].
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    stub_bin().display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            );
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
        self.command(cwd, args).output().expect("Armada runs")
    }

    /// Run with extra variables layered on the deliberately small environment.
    ///
    /// The rendering suite needs it: `NO_COLOR` is the one input to the colour
    /// decision that is not a flag, and `env_clear` above means a test cannot
    /// set it by exporting it in the parent.
    pub fn run_with_env(&self, cwd: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
        let mut command = self.command(cwd, args);
        for (name, value) in env {
            command.env(name, value);
        }
        command.output().expect("Armada runs")
    }

    /// Start and leave running.
    pub fn spawn(&self, cwd: &Path, args: &[&str]) -> Child {
        self.command(cwd, args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Armada runs")
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
