//! Where a guild syncs to — **a git URL or a folder** (`PLAN.md` §13.5).
//!
//! The interview used to ask for *a private git remote*, and the honest answer
//! for most people is that they do not have one and do not want to create one to
//! finish setting a tool up. They do have a folder that is already on every
//! machine they own: iCloud Drive, Dropbox, a NAS mount, a drive they plug in.
//!
//! **Git supports a filesystem remote natively**, so a folder is not a lesser
//! mode. `git init --bare` in it and every one of `fetch`, `push`,
//! `merge --ff-only` and the divergence counts works exactly as it does against
//! a server: real merges, real history, and conflicts that surface as conflicts.
//! Nothing else in `repo` needs to know which kind it got.
//!
//! # The two things a sync folder does that a server does not
//!
//! Both are consequences of a *file syncing service* sitting between the two
//! machines, and both are handled here rather than hidden.
//!
//! **Eviction.** iCloud Drive removes the contents of files it thinks you are
//! not using and leaves a `.name.icloud` placeholder in their place. A bare
//! repository whose pack files have been evicted is a repository `git fetch`
//! cannot read — so [`materialise`] asks for them back before anything reads the
//! remote, and waits, rather than letting git report a corrupt repository for
//! something that is merely not downloaded yet.
//!
//! **A torn repository.** A push writes several files, and the sync service
//! replicates them in its own order and its own time. A machine that reads the
//! remote in between sees refs pointing at objects that have not arrived. That
//! is not corruption and it is not the reader's fault: it resolves itself.
//! [`is_torn`] recognises git's words for it so `guild pull` can report a
//! conflict — *wait, and pull again* — instead of a broken repository.

use armada_core::ctx::{Run, RunRequest, StdioMode};
use armada_core::error::{ArmadaError, ErrClass};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How long [`materialise`] waits for an evicted file to come back.
///
/// **Bounded, because the alternative is a verb that hangs on a train.** A guild
/// is small — a few hundred kilobytes of pack — so a download that has not
/// happened in this long is not a download that is happening.
const DOWNLOAD_DEADLINE: Duration = Duration::from_secs(30);

/// How often the wait re-checks. Short: the common case is already local and
/// costs one `stat`.
const POLL: Duration = Duration::from_millis(200);

/// The suffix macOS gives a file whose contents have been evicted.
const PLACEHOLDER: &str = ".icloud";

/// What was typed at question 5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// Something git can already clone from.
    Url(String),
    /// A directory on this machine — a sync folder, a mount, a stick.
    Folder(PathBuf),
}

/// The URL schemes git speaks. A string opening with one of these is a URL and
/// nothing else needs deciding.
const SCHEMES: [&str; 6] = [
    "ssh://",
    "git://",
    "https://",
    "http://",
    "file://",
    "git+ssh://",
];

/// Read an answer as one or the other.
///
/// **Three rules, in order, and no filesystem access.** Whether the folder
/// exists is not part of deciding what was meant — a path that is not there yet
/// is still a path, and asking the disk would make the same answer mean
/// different things on two machines.
///
/// 1. A known scheme is a URL.
/// 2. `user@host:path`, git's scp-like form, is a URL. Recognised by an `@`
///    followed by a `:` with no `/` between them, which is what distinguishes it
///    from `/Volumes/My Drive:backup/…`.
/// 3. Everything else is a folder.
pub fn classify(answer: &str) -> Destination {
    let answer = answer.trim();
    if SCHEMES.iter().any(|scheme| answer.starts_with(scheme)) {
        return Destination::Url(answer.to_string());
    }
    if let Some((before, after)) = answer.split_once('@') {
        if !before.is_empty() && !before.contains('/') {
            if let Some((host, _)) = after.split_once(':') {
                if !host.is_empty() && !host.contains('/') {
                    return Destination::Url(answer.to_string());
                }
            }
        }
    }
    Destination::Folder(PathBuf::from(answer))
}

/// `~/Library/…` as an absolute path.
///
/// `home` is passed in rather than read, because `ARCHITECTURE.md` §1.4 keeps
/// every read of the ambient world at the entrypoint — the same reason Guild is
/// told where `~/.armada` is rather than working it out.
pub fn expand(path: &Path, home: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        None if text == "~" => home.to_path_buf(),
        None => path.to_path_buf(),
    }
}

/// Turn a destination into the remote URL `repo` will be given.
///
/// A URL passes through. A folder is created if it is not there, made a **bare**
/// repository if it is not one already, and handed back as an absolute path.
///
/// **Bare, and that is the whole of why this cannot be left to the person.** A
/// non-bare repository refuses a push to its checked-out branch, which is the
/// error every "I pointed git at a Dropbox folder" story ends with. `git init
/// --bare` is one flag and knowing to type it is the entire difficulty.
///
/// **An existing repository is adopted rather than re-initialised.** The second
/// machine names the same folder, and `git init --bare` over a repository that
/// already has your history in it would be the one unrecoverable outcome here.
pub fn prepare(
    run: &impl Run,
    destination: &Destination,
    home: &Path,
) -> Result<String, ArmadaError> {
    let folder = match destination {
        Destination::Url(url) => return Ok(url.clone()),
        Destination::Folder(path) => expand(path, home),
    };

    std::fs::create_dir_all(&folder).map_err(|error| ArmadaError {
        class: ErrClass::Environment,
        r#where: folder.display().to_string(),
        message: format!("cannot create {}: {error}", folder.display()),
        next_action: Some("check the path is writable, then retry unchanged".to_string()),
    })?;

    if !is_repository(&folder) {
        let argv = vec![
            "git".to_string(),
            "init".to_string(),
            "--bare".to_string(),
            "--initial-branch".to_string(),
            crate::repo::BRANCH.to_string(),
            folder.display().to_string(),
        ];
        let request = RunRequest::new(argv, folder.clone()).stdio(StdioMode::Capture);
        let output = run.call(&request).map_err(|spawn| ArmadaError {
            class: ErrClass::Environment,
            r#where: "git".to_string(),
            message: format!("cannot run git: {}", spawn.message),
            next_action: Some("install git, then retry unchanged".to_string()),
        })?;
        if !output.ok() {
            return Err(ArmadaError {
                class: ErrClass::ToolFailed,
                r#where: folder.display().to_string(),
                message: first_line(&output.stderr)
                    .unwrap_or_else(|| "git init --bare failed".to_string()),
                next_action: Some("choose another folder, or give a git URL".to_string()),
            });
        }
    }

    Ok(folder.display().to_string())
}

/// Whether a directory already holds a bare repository.
///
/// **Checked on disk rather than with `git rev-parse`**, because the question is
/// asked before the folder is known to be anything and a subprocess that answers
/// "no" by failing is a subprocess whose failure has to be told apart from a git
/// that is not installed.
fn is_repository(folder: &Path) -> bool {
    folder.join("HEAD").is_file() && folder.join("objects").is_dir()
}

/// Ask a sync service for every file it has evicted, and wait.
///
/// **Called before anything reads a filesystem remote.** iCloud Drive replaces
/// an unused file's contents with a `.name.icloud` placeholder; a bare
/// repository in that state is one `git fetch` reports as corrupt, for a
/// repository that is perfectly intact and merely elsewhere.
///
/// Reading the real path is what asks for it back — the same thing opening the
/// file in Finder does — so this walks the tree, touches every evicted file, and
/// waits for the placeholders to go. It returns `Ok` when the tree is whole and
/// a `timeout` when it is not, which is the honest report: nothing is wrong with
/// the repository, and it is not readable yet.
pub fn materialise(folder: &Path, now: impl Fn() -> std::time::Instant) -> Result<(), ArmadaError> {
    let started = now();
    loop {
        let evicted = placeholders(folder);
        if evicted.is_empty() {
            return Ok(());
        }
        for placeholder in &evicted {
            // Opening the file the placeholder stands for is the request. The
            // read is discarded; it is the `open` that matters.
            if let Some(real) = wanted(placeholder) {
                let _ = std::fs::File::open(&real);
            }
        }
        if now().duration_since(started) >= DOWNLOAD_DEADLINE {
            return Err(ArmadaError {
                class: ErrClass::Timeout,
                r#where: folder.display().to_string(),
                message: format!(
                    "{} of the guild remote {} still not downloaded after {}s",
                    evicted.len(),
                    if evicted.len() == 1 {
                        "file is"
                    } else {
                        "files are"
                    },
                    DOWNLOAD_DEADLINE.as_secs()
                ),
                next_action: Some(
                    "open the folder in Finder to force the download, then retry unchanged"
                        .to_string(),
                ),
            });
        }
        std::thread::sleep(POLL);
    }
}

/// Every evicted file under a directory.
fn placeholders(folder: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(listing) = std::fs::read_dir(folder) else {
        return found;
    };
    for entry in listing.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            found.extend(placeholders(&path));
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with('.') && name.ends_with(PLACEHOLDER))
        {
            found.push(path);
        }
    }
    found
}

/// The real path a `.name.icloud` placeholder stands in for.
pub fn wanted(placeholder: &Path) -> Option<PathBuf> {
    let name = placeholder.file_name()?.to_str()?;
    let real = name.strip_prefix('.')?.strip_suffix(PLACEHOLDER)?;
    Some(placeholder.with_file_name(real))
}

/// Whether git's complaint is a remote that is **mid-sync rather than broken**.
///
/// A push writes several files and a sync service replicates them in its own
/// order. A machine reading the remote in between sees refs pointing at objects
/// that have not arrived yet — which git reports in the same words it uses for a
/// repository somebody has damaged, because from where it is standing the two
/// are identical.
///
/// **Matched on git's own words, and that is a knowing trade** — the same one
/// `repo::empty_remote` already makes and for the same reason: there is no exit
/// code for it. Getting it wrong reports a torn remote as a plain failure, which
/// is loud rather than silent, and the remedy printed is *wait and pull again*
/// either way.
pub fn is_torn(stderr: &str) -> bool {
    let lowered = stderr.to_ascii_lowercase();
    [
        "unable to read",
        "object not found",
        "did not send all necessary objects",
        "corrupt loose object",
        "packfile",
        "unable to find",
        "no such file or directory",
    ]
    .iter()
    .any(|phrase| lowered.contains(phrase))
}

fn first_line(text: &str) -> Option<String> {
    text.lines()
        .find(|line| !line.trim().is_empty())
        .map(str::trim)
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::ctx::{RunOutput, SpawnError};
    use std::cell::RefCell;

    struct Git {
        calls: RefCell<Vec<Vec<String>>>,
    }

    impl Run for Git {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            self.calls.borrow_mut().push(request.argv.clone());
            // A real `git init --bare` makes the two things `is_repository`
            // looks for, and the fake makes them too — without it every
            // assertion about adopting an existing repository would pass
            // against a folder that was never initialised.
            if request.argv.get(1).is_some_and(|a| a == "init") {
                if let Some(path) = request.argv.last() {
                    std::fs::create_dir_all(Path::new(path).join("objects")).unwrap();
                    std::fs::write(Path::new(path).join("HEAD"), "ref: refs/heads/main\n").unwrap();
                }
            }
            Ok(RunOutput {
                code: Some(0),
                signal: None,
                stdout: String::new(),
                stderr: String::new(),
                timed_out: false,
            })
        }
    }

    fn git() -> Git {
        Git {
            calls: RefCell::new(Vec::new()),
        }
    }

    /// **Both forms of git URL are recognised**, including the scp-like one that
    /// has a colon in it and is the form every hosting provider hands you.
    #[test]
    fn a_git_url_is_read_as_a_url_in_both_of_its_forms() {
        for url in [
            "git@github.com:me/guild.git",
            "ssh://git@example.com/me/guild.git",
            "https://example.com/me/guild.git",
            "file:///Volumes/backup/guild.git",
        ] {
            assert_eq!(classify(url), Destination::Url(url.to_string()), "{url}");
        }
    }

    /// **A folder is a folder, including one whose name has an `@` in it.** The
    /// scp-like form needs a host between the `@` and the `:`, and a path has a
    /// `/` in the way.
    #[test]
    fn a_path_is_read_as_a_folder() {
        for path in [
            "~/Library/Mobile Documents/com~apple~CloudDocs/guild",
            "/Volumes/nas/guild",
            "./guild",
            "guild",
            "/Users/me/backup@2026/guild",
        ] {
            assert_eq!(
                classify(path),
                Destination::Folder(PathBuf::from(path)),
                "{path}"
            );
        }
    }

    /// `~` is expanded against the home the entrypoint captured, never against
    /// one this crate went and read.
    #[test]
    fn a_tilde_is_expanded_against_the_home_it_was_given() {
        let home = Path::new("/scratch/home");
        assert_eq!(
            expand(Path::new("~/Drive/guild"), home),
            PathBuf::from("/scratch/home/Drive/guild")
        );
        assert_eq!(expand(Path::new("~"), home), PathBuf::from("/scratch/home"));
        assert_eq!(
            expand(Path::new("/absolute"), home),
            PathBuf::from("/absolute")
        );
    }

    /// A URL is handed back untouched and git is not run at all.
    #[test]
    fn preparing_a_url_runs_nothing() {
        let run = git();
        let url = prepare(
            &run,
            &Destination::Url("git@example.com:me/guild.git".to_string()),
            Path::new("/scratch/home"),
        )
        .unwrap();
        assert_eq!(url, "git@example.com:me/guild.git");
        assert!(run.calls.borrow().is_empty());
    }

    /// **A folder becomes a bare repository, on the named branch.** Bare,
    /// because a non-bare one refuses the push; named, for the same reason
    /// `repo::BRANCH` is named rather than inherited.
    #[test]
    fn preparing_a_folder_makes_it_a_bare_repository_on_the_named_branch() {
        let home = tempfile::tempdir().unwrap();
        let run = git();
        let folder = home.path().join("Drive/guild");
        let remote = prepare(&run, &Destination::Folder(folder.clone()), home.path()).unwrap();

        assert_eq!(remote, folder.display().to_string());
        assert!(folder.is_dir(), "the folder was not created");
        let argv = run.calls.borrow()[0].clone();
        assert!(argv.contains(&"--bare".to_string()), "{argv:?}");
        assert!(argv.contains(&"main".to_string()), "{argv:?}");
    }

    /// **The second machine names the same folder.** Re-initialising over a
    /// repository that already holds your history is the one unrecoverable
    /// outcome here, so an existing one is adopted.
    #[test]
    fn a_folder_that_is_already_a_repository_is_adopted_rather_than_reinitialised() {
        let home = tempfile::tempdir().unwrap();
        let folder = home.path().join("Drive/guild");
        std::fs::create_dir_all(folder.join("objects")).unwrap();
        std::fs::write(folder.join("HEAD"), "ref: refs/heads/main\n").unwrap();

        let run = git();
        prepare(&run, &Destination::Folder(folder), home.path()).unwrap();
        assert!(
            run.calls.borrow().is_empty(),
            "git init ran over an existing repository: {:?}",
            run.calls.borrow()
        );
    }

    /// A tree with nothing evicted is whole immediately and costs one walk.
    #[test]
    fn a_folder_with_nothing_evicted_is_ready_at_once() {
        let folder = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(folder.path().join("objects/pack")).unwrap();
        std::fs::write(folder.path().join("objects/pack/a.pack"), b"x").unwrap();
        assert!(materialise(folder.path(), std::time::Instant::now).is_ok());
    }

    /// **An evicted file is asked for and waited on, and the wait is bounded.**
    /// A guild is a few hundred kilobytes; a download that has not happened in
    /// thirty seconds is not happening, and a verb that hangs is worse than one
    /// that says why it cannot read.
    #[test]
    fn an_evicted_file_that_never_arrives_is_a_timeout_and_not_a_corrupt_repository() {
        let folder = tempfile::tempdir().unwrap();
        let pack = folder.path().join("objects/pack");
        std::fs::create_dir_all(&pack).unwrap();
        std::fs::write(pack.join(".a.pack.icloud"), b"").unwrap();

        // A clock that is already past the deadline on its second reading, so
        // the bound is asserted without a test that takes thirty seconds.
        let start = std::time::Instant::now();
        let past = start + DOWNLOAD_DEADLINE + Duration::from_secs(1);
        let readings = std::cell::Cell::new(0);
        let error = materialise(folder.path(), || {
            readings.set(readings.get() + 1);
            if readings.get() == 1 {
                start
            } else {
                past
            }
        })
        .unwrap_err();

        assert_eq!(error.class, ErrClass::Timeout);
        assert!(error.message.contains("not downloaded"), "{error:?}");
        assert!(error.next_action.is_some());
    }

    /// The real path behind a placeholder, which is what has to be opened to
    /// ask for the file back.
    #[test]
    fn a_placeholder_names_the_file_it_stands_in_for() {
        assert_eq!(
            wanted(Path::new("/drive/guild/objects/pack/.a.pack.icloud")),
            Some(PathBuf::from("/drive/guild/objects/pack/a.pack"))
        );
        assert_eq!(wanted(Path::new("/drive/guild/HEAD")), None);
    }

    /// **A remote mid-sync is told apart from a remote somebody damaged**, so
    /// `guild pull` can say *wait and pull again* rather than reporting a broken
    /// repository for something that fixes itself.
    #[test]
    fn a_partly_replicated_remote_is_recognised_as_torn() {
        for stderr in [
            "error: unable to read sha1 file of refs/heads/main",
            "fatal: bad object 3f2a1c; did not send all necessary objects",
            "error: object not found: 9a1f",
            "fatal: packfile .git/objects/pack/pack-9a.pack cannot be accessed",
        ] {
            assert!(is_torn(stderr), "{stderr}");
        }
        assert!(!is_torn("fatal: Could not read from remote repository."));
        assert!(!is_torn("fatal: couldn't find remote ref main"));
    }
}
