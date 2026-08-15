//! `export` and `import` — one portable file, for a machine that will never
//! hold your git credentials.
//!
//! The escape hatch, and the thing you reach for when a remote is not worth
//! setting up (`guild/export.md`).
//!
//! # What a bundle is, and what it deliberately is not
//!
//! **Content only. The git history is not included.** A bundle is a snapshot,
//! and a machine restoring from one starts a fresh history — use a remote if you
//! want history to travel. That is why `import` runs `git init` and makes one
//! commit rather than trying to graft anything.
//!
//! **Everything under `~/.armada/` that is not `guild/` is excluded by
//! construction**, because it describes this machine (`PLAN.md` §13.1). Not by a
//! filter that could be got wrong — by the archive being rooted at `guild/` and
//! nothing else. `machine.yml` is the one exception and it takes an explicit
//! `--include-secrets`, which prints a warning when used, because the whole
//! point of that file is that it does not travel.
//!
//! # Validate before touching anything
//!
//! `import` unpacks to a scratch directory beside the guild, validates there,
//! and only then replaces. **A bundle that fails validation changes nothing** —
//! `guild/import.md` states it as an exit code (`3 bad_config`, and nothing
//! changed), which is only true if the failure happens before the swap.
//!
//! # `tar`, through the subprocess seam
//!
//! No archive crate. `tar` is on every machine Armada runs on, it is reached
//! through `ctx.run` like git and docker (`ARCHITECTURE.md` §1.1), and the
//! compression is chosen from the file extension so `--out guild.tar` and
//! `--out guild.tar.zst` both do what they look like they do.

use crate::inventory::Inventory;
use crate::layout::Guild;
use armada_core::ctx::{Run, RunRequest, StdioMode};
use armada_core::error::{ArmadaError, ErrClass};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Armada's own deadline on a `tar` call. Generous: a guild with a large skills
/// directory on a slow disk is still ordinary.
const DEADLINE: Duration = Duration::from_secs(300);

/// Where `export` writes when it is told nothing.
pub const DEFAULT_OUT: &str = "guild.tar.zst";

/// What an export produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exported {
    /// Where it was written.
    pub path: PathBuf,
    /// How big it is.
    pub bytes: u64,
    /// What went into it.
    pub inventory: Inventory,
    /// Whether `machine.yml` travelled.
    pub secrets: bool,
}

/// The compression a path's extension asks for.
///
/// **Read from the name rather than flagged**, so the file a person names is
/// the file they get. An unknown extension is an uncompressed tar, which is
/// always readable — guessing zstd for a `.bak` would produce a file nothing on
/// the machine can open.
fn compression(path: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?;
    if name.ends_with(".zst") || name.ends_with(".tzst") {
        Some("--zstd")
    } else if name.ends_with(".gz") || name.ends_with(".tgz") {
        Some("--gzip")
    } else if name.ends_with(".bz2") {
        Some("--bzip2")
    } else {
        None
    }
}

fn tar(run: &impl Run, cwd: &Path, args: &[String]) -> Result<(), ArmadaError> {
    let mut argv = vec!["tar".to_string()];
    argv.extend(args.iter().cloned());
    let request = RunRequest::new(argv.clone(), cwd.to_path_buf())
        .stdio(StdioMode::Capture)
        .timeout(DEADLINE);

    match run.call(&request) {
        Ok(output) if output.timed_out => Err(ArmadaError {
            class: ErrClass::Timeout,
            r#where: "tar".to_string(),
            message: format!("tar did not finish within {}s", DEADLINE.as_secs()),
            next_action: None,
        }),
        Ok(output) if output.ok() => Ok(()),
        Ok(output) => Err(ArmadaError {
            class: ErrClass::Environment,
            r#where: argv.join(" "),
            message: output
                .stderr
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("tar failed")
                .trim()
                .to_string(),
            next_action: Some(
                "check the path is writable, and that this tar supports the compression \
                 the file extension asks for"
                    .to_string(),
            ),
        }),
        Err(spawn) => Err(ArmadaError {
            class: ErrClass::Environment,
            r#where: "tar".to_string(),
            message: format!("cannot run tar: {}", spawn.message),
            next_action: Some("install tar, then retry unchanged".to_string()),
        }),
    }
}

/// Archive `~/.armada/guild/` to one file.
///
/// **`.git` is excluded**, which is what makes a bundle a snapshot rather than
/// a half-repository: an unpacked `.git` from another machine would carry that
/// machine's remote and its whole history into a guild `import` is about to
/// commit fresh.
pub fn export(
    run: &impl Run,
    guild: &Guild,
    armada_home: &Path,
    out: &Path,
    include_secrets: bool,
) -> Result<Exported, ArmadaError> {
    if !guild.root().is_dir() {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: guild.root().display().to_string(),
            message: "there is no guild here to export".to_string(),
            next_action: Some("`armada guild init`, or `armada init`".to_string()),
        });
    }

    let mut args = vec!["-c".to_string(), "-f".to_string(), absolute(out)];
    if let Some(flag) = compression(out) {
        args.push(flag.to_string());
    }
    args.push("--exclude=.git".to_string());
    args.push("guild".to_string());
    if include_secrets && armada_home.join("machine.yml").is_file() {
        args.push("machine.yml".to_string());
    }
    // Rooted at `~/.armada/`, so the archive's paths are `guild/…` and nothing
    // else under it can be reached by construction.
    tar(run, armada_home, &args)?;

    Ok(Exported {
        bytes: std::fs::metadata(out).map(|m| m.len()).unwrap_or(0),
        path: out.to_path_buf(),
        inventory: Inventory::of(guild.root()),
        secrets: include_secrets,
    })
}

/// What an import found in a bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unpacked {
    /// What is in it.
    pub inventory: Inventory,
    /// Whether the bundle carried a `machine.yml` that was **ignored** because
    /// this machine has its own.
    pub machine_skipped: bool,
    /// Paths `--merge` left alone because this machine's copy differs.
    pub conflicts: Vec<String>,
}

/// Unpack a bundle into `~/.armada/guild/`.
///
/// Order matters and is the whole of the exit-code contract: **unpack to
/// scratch, validate, and only then replace.**
pub fn import(
    run: &impl Run,
    bundle: &Path,
    guild: &Guild,
    armada_home: &Path,
    merge: bool,
) -> Result<Unpacked, ArmadaError> {
    if !bundle.is_file() {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: bundle.display().to_string(),
            message: format!("no bundle at {}", bundle.display()),
            next_action: None,
        });
    }

    let scratch = armada_home.join(".guild-import");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).map_err(|e| unwritable(&scratch, &e))?;

    let mut args = vec!["-x".to_string(), "-f".to_string(), absolute(bundle)];
    if let Some(flag) = compression(bundle) {
        args.push(flag.to_string());
    }
    if let Err(error) = tar(run, &scratch, &args) {
        let _ = std::fs::remove_dir_all(&scratch);
        return Err(error);
    }

    let unpacked = scratch.join("guild");
    let outcome = validate(&unpacked).and_then(|inventory| {
        let mut conflicts = Vec::new();
        if merge {
            conflicts =
                merge_into(&unpacked, guild.root()).map_err(|e| unwritable(guild.root(), &e))?;
        } else {
            replace(&unpacked, guild.root()).map_err(|e| unwritable(guild.root(), &e))?;
        }
        Ok(Unpacked {
            inventory,
            // **A bundle's `machine.yml` is ignored unless this machine has
            // none**, so importing your own export onto a different machine
            // cannot overwrite that machine's paths and capacity with another's.
            machine_skipped: scratch.join("machine.yml").is_file()
                && armada_home.join("machine.yml").is_file(),
            conflicts,
        })
    });

    let _ = std::fs::remove_dir_all(&scratch);
    outcome
}

/// Structure first, then every schema-backed file. **Nothing has been touched
/// when this runs**, which is what makes "a bundle that fails validation
/// changes nothing" true rather than aspirational.
fn validate(unpacked: &Path) -> Result<Inventory, ArmadaError> {
    if !unpacked.is_dir() {
        return Err(ArmadaError {
            class: ErrClass::BadConfig,
            r#where: "guild/".to_string(),
            message: "this bundle has no guild/ directory in it".to_string(),
            next_action: Some(
                "`armada guild export` writes one; an archive of some other directory is not a bundle"
                    .to_string(),
            ),
        });
    }

    let workflows = unpacked.join("workflows");
    if let Ok(listing) = std::fs::read_dir(&workflows) {
        for entry in listing.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "yml" && e != "yaml") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text).map_err(|e| ArmadaError {
                class: ErrClass::BadConfig,
                r#where: format!("workflows/{}", entry.file_name().to_string_lossy()),
                message: format!("this bundle's workflow does not parse: {e}"),
                next_action: Some(
                    "fix it on the machine that exported it, and export again".to_string(),
                ),
            })?;
        }
    }

    for name in ["plugins.yml", "mcp.yml", crate::permissions::FILE] {
        let path = unpacked.join(name);
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text).map_err(|e| ArmadaError {
            class: ErrClass::BadConfig,
            r#where: name.to_string(),
            message: format!("this bundle's {name} does not parse: {e}"),
            next_action: Some(
                "fix it on the machine that exported it, and export again".to_string(),
            ),
        })?;
    }

    Ok(Inventory::of(unpacked))
}

/// Replace the guild wholesale. The caller has already insisted with `--force`.
fn replace(from: &Path, to: &Path) -> std::io::Result<()> {
    let _ = std::fs::remove_dir_all(to);
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    copy_tree(from, to)
}

/// Merge, **reporting and skipping every conflict rather than overwriting one**
/// (`guild/import.md`). A file this machine does not have is copied; a file it
/// has and that differs is left exactly as it is and named in the report.
fn merge_into(from: &Path, to: &Path) -> std::io::Result<Vec<String>> {
    let mut conflicts = Vec::new();
    merge_walk(from, to, Path::new(""), &mut conflicts)?;
    conflicts.sort();
    Ok(conflicts)
}

fn merge_walk(
    from: &Path,
    to: &Path,
    relative: &Path,
    conflicts: &mut Vec<String>,
) -> std::io::Result<()> {
    let source = from.join(relative);
    let destination = to.join(relative);
    if source.is_dir() {
        std::fs::create_dir_all(&destination)?;
        for entry in std::fs::read_dir(&source)? {
            merge_walk(from, to, &relative.join(entry?.file_name()), conflicts)?;
        }
        return Ok(());
    }
    if !destination.exists() {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&source, &destination)?;
        return Ok(());
    }
    if std::fs::read(&source)? != std::fs::read(&destination)? {
        conflicts.push(relative.display().to_string());
    }
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

fn absolute(path: &Path) -> String {
    if path.is_absolute() {
        return path.display().to_string();
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(path).display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn unwritable(path: &Path, error: &std::io::Error) -> ArmadaError {
    ArmadaError {
        class: ErrClass::Environment,
        r#where: path.display().to_string(),
        message: format!("cannot write {}: {error}", path.display()),
        next_action: Some("check the path is writable, then retry unchanged".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::ctx::{RunOutput, SpawnError};
    use std::cell::RefCell;

    /// A fake that records argv and, for `-x`, lays down a guild so the rest of
    /// `import` has something to validate.
    #[derive(Default)]
    struct FakeTar {
        calls: RefCell<Vec<Vec<String>>>,
        lays_down: Option<String>,
    }

    impl Run for FakeTar {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            self.calls.borrow_mut().push(request.argv.clone());
            if request.argv.contains(&"-x".to_string()) {
                let guild = request.cwd.join("guild");
                std::fs::create_dir_all(guild.join("workflows")).unwrap();
                std::fs::write(guild.join("voice.md"), "brief\n").unwrap();
                std::fs::write(
                    guild.join("workflows/bug.yml"),
                    self.lays_down.as_deref().unwrap_or("name: bug\n"),
                )
                .unwrap();
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

    fn a_guild() -> (tempfile::TempDir, Guild) {
        let home = tempfile::tempdir().unwrap();
        let guild = Guild::at(home.path());
        std::fs::create_dir_all(guild.path("workflows")).unwrap();
        std::fs::create_dir_all(guild.path(".git")).unwrap();
        std::fs::write(guild.path("voice.md"), "brief\n").unwrap();
        std::fs::write(guild.path("workflows/bug.yml"), "name: bug\n").unwrap();
        (home, guild)
    }

    /// **Rooted at `~/.armada/`, archiving `guild` and nothing else.** That is
    /// what makes "everything that is not guild/ is excluded" a property of the
    /// argv rather than of a filter somebody has to keep correct.
    #[test]
    fn an_export_archives_the_guild_and_nothing_else_under_armada_home() {
        let (home, guild) = a_guild();
        std::fs::write(home.path().join("machine.yml"), "cpu_slots: 4\n").unwrap();
        std::fs::write(home.path().join("manifest.db"), "not a real db").unwrap();

        let run = FakeTar::default();
        let out = home.path().join("guild.tar.zst");
        export(&run, &guild, home.path(), &out, false).unwrap();

        let argv = run.calls.borrow()[0].clone();
        assert_eq!(argv.last().unwrap(), "guild");
        assert!(argv.contains(&"--exclude=.git".to_string()), "{argv:?}");
        assert!(argv.contains(&"--zstd".to_string()), "{argv:?}");
        assert!(
            !argv.iter().any(|a| a.contains("manifest.db")),
            "something that never syncs reached the bundle: {argv:?}"
        );
        assert!(
            !argv.iter().any(|a| a == "machine.yml"),
            "machine.yml travelled without --include-secrets: {argv:?}"
        );
    }

    /// `--include-secrets` is the one way `machine.yml` travels, and it has to
    /// be asked for by name.
    #[test]
    fn machine_yml_travels_only_when_it_is_asked_for() {
        let (home, guild) = a_guild();
        std::fs::write(home.path().join("machine.yml"), "cpu_slots: 4\n").unwrap();
        let run = FakeTar::default();
        let exported = export(
            &run,
            &guild,
            home.path(),
            &home.path().join("guild.tar.zst"),
            true,
        )
        .unwrap();
        assert!(exported.secrets);
        assert!(run.calls.borrow()[0].contains(&"machine.yml".to_string()));
    }

    /// The compression a person named is the compression they get, and an
    /// unknown extension is an uncompressed tar rather than a guess.
    #[test]
    fn the_compression_comes_from_the_name_the_person_wrote() {
        assert_eq!(compression(Path::new("g.tar.zst")), Some("--zstd"));
        assert_eq!(compression(Path::new("g.tgz")), Some("--gzip"));
        assert_eq!(compression(Path::new("g.tar")), None);
        assert_eq!(compression(Path::new("g.bak")), None);
    }

    /// Exporting from a machine with no guild is a bad invocation, not an empty
    /// archive somebody discovers is empty on the other machine.
    #[test]
    fn exporting_nothing_is_refused_rather_than_producing_an_empty_bundle() {
        let home = tempfile::tempdir().unwrap();
        let guild = Guild::at(home.path());
        let error = export(
            &FakeTar::default(),
            &guild,
            home.path(),
            &home.path().join("g.tar"),
            false,
        )
        .unwrap_err();
        assert_eq!(error.class, ErrClass::BadInvocation);
        assert_eq!(error.class.exit_code(), 2);
    }

    #[test]
    fn an_imported_bundle_lands_in_the_guild() {
        let home = tempfile::tempdir().unwrap();
        let guild = Guild::at(home.path());
        let bundle = home.path().join("g.tar");
        std::fs::write(&bundle, "pretend archive").unwrap();

        let unpacked = import(&FakeTar::default(), &bundle, &guild, home.path(), false).unwrap();
        assert_eq!(unpacked.inventory.workflows, 1);
        assert!(guild.path("voice.md").is_file());
        assert!(
            !home.path().join(".guild-import").exists(),
            "the scratch directory was left behind"
        );
    }

    /// **The exit-code contract, made true by ordering.** A bundle that fails
    /// validation changes nothing, which is only the case because validation
    /// runs against the scratch copy and before the swap.
    #[test]
    fn a_bundle_that_fails_validation_changes_nothing() {
        let (home, guild) = a_guild();
        std::fs::write(guild.path("voice.md"), "the original\n").unwrap();
        let bundle = home.path().join("g.tar");
        std::fs::write(&bundle, "pretend archive").unwrap();

        let run = FakeTar {
            lays_down: Some("name: [unclosed\n".to_string()),
            ..FakeTar::default()
        };
        let error = import(&run, &bundle, &guild, home.path(), false).unwrap_err();

        assert_eq!(error.class, ErrClass::BadConfig);
        assert_eq!(error.class.exit_code(), 3);
        assert_eq!(
            std::fs::read_to_string(guild.path("voice.md")).unwrap(),
            "the original\n",
            "the guild was modified by an import that failed validation"
        );
        assert!(!home.path().join(".guild-import").exists());
    }

    /// **`--merge` reports a conflict and skips it, never overwrites it.**
    #[test]
    fn merging_skips_a_file_this_machine_has_edited_and_names_it() {
        let (home, guild) = a_guild();
        std::fs::write(guild.path("voice.md"), "mine, edited here\n").unwrap();
        let bundle = home.path().join("g.tar");
        std::fs::write(&bundle, "pretend archive").unwrap();

        let unpacked = import(&FakeTar::default(), &bundle, &guild, home.path(), true).unwrap();

        assert_eq!(unpacked.conflicts, vec!["voice.md".to_string()]);
        assert_eq!(
            std::fs::read_to_string(guild.path("voice.md")).unwrap(),
            "mine, edited here\n",
            "a merge overwrote a file this machine had edited"
        );
    }

    /// A bundle's `machine.yml` is ignored when this machine has one, so
    /// importing your own export cannot overwrite this machine's capacity with
    /// another machine's.
    #[test]
    fn a_bundles_machine_file_is_ignored_when_this_machine_has_its_own() {
        let (home, guild) = a_guild();
        std::fs::write(home.path().join("machine.yml"), "cpu_slots: 4\n").unwrap();
        let bundle = home.path().join("g.tar");
        std::fs::write(&bundle, "pretend archive").unwrap();

        struct WithMachine(FakeTar);
        impl Run for WithMachine {
            fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
                let output = self.0.call(request)?;
                if request.argv.contains(&"-x".to_string()) {
                    std::fs::write(request.cwd.join("machine.yml"), "cpu_slots: 99\n").unwrap();
                }
                Ok(output)
            }
        }

        let unpacked = import(
            &WithMachine(FakeTar::default()),
            &bundle,
            &guild,
            home.path(),
            false,
        )
        .unwrap();
        assert!(unpacked.machine_skipped);
        assert_eq!(
            std::fs::read_to_string(home.path().join("machine.yml")).unwrap(),
            "cpu_slots: 4\n"
        );
    }

    /// A path that is not a file is refused before anything is unpacked.
    #[test]
    fn a_bundle_that_is_not_there_is_a_bad_invocation() {
        let home = tempfile::tempdir().unwrap();
        let error = import(
            &FakeTar::default(),
            Path::new("/nonexistent/g.tar"),
            &Guild::at(home.path()),
            home.path(),
            false,
        )
        .unwrap_err();
        assert_eq!(error.class, ErrClass::BadInvocation);
    }
}
