//! `armada init` — set up **this machine**. Run once per box.
//!
//! Not to be confused with `armada manifest init`, which sets up a *workspace*.
//! This sets up *you, here* (`docs/commands/init.md`).
//!
//! # Four steps, and the third is the only question
//!
//! 1. **Preflight** — `git`, `claude`, a container runtime. Missing `claude` is
//!    fatal; a missing container runtime is a warning, because not every repo
//!    needs one.
//! 2. **Create `~/.armada/`** — `guild/`, `jobs/`, `workspaces/`, and
//!    `machine.yml`.
//! 3. **Ask the one question that matters:** *do you already have a guild?*
//!    Pull it from a remote, import a bundle, or build one now. Only the third
//!    reaches the five-question interview.
//! 4. **Write `machine.yml`** — which never syncs (`PLAN.md` §13.1).
//!
//! # Nothing here ever touches a real `~/.armada/`
//!
//! `armada_home` is a value the entrypoint captured and passed down
//! (`ARCHITECTURE.md` §1.4), which is what lets every test in this file point at
//! a `TempDir`. Destroying somebody's actual guild is unrecoverable, and the
//! defence against it is structural rather than a rule anyone has to remember.

use armada_core::ctx::Run;
use armada_core::envelope::{Asked, Envelope, Finding, GuildChoice, Health, MachineInitData};
use armada_core::error::{ArmadaError, ErrClass, Status};
use armada_guild::interview::QUESTIONS;
use armada_guild::layout::{DIRECTORIES, GUILD_DIRECTORIES};
use std::path::Path;

use crate::ask::Ask;
use crate::verbs::guild::{self, InitOptions, Where};
use crate::verbs::{preflight, Output};

/// The one question, and its three answers.
const QUESTION: &str = "Do you already have a guild?";

/// The three, in the order they are offered.
const ANSWERS: [&str; 3] = ["pull from a remote", "import a bundle", "build one now"];

/// Which answer `--guild`, `--bundle` and a bare run mean.
const FROM_REMOTE: usize = 1;
const FROM_BUNDLE: usize = 2;
const BUILD_ONE: usize = 3;

/// What `armada init` was asked for.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// `--guild <remote>`: skip the prompt and clone.
    pub guild: Option<String>,
    /// `--bundle <path>`: skip the prompt and unpack.
    pub bundle: Option<String>,
    /// `--defaults`: take every default answer.
    pub defaults: bool,
    /// `--force`: re-run against an existing `~/.armada/`.
    pub force: bool,
}

/// Set up this machine.
pub fn run(
    runner: &impl Run,
    place: &Where,
    ask: &mut dyn Ask,
    options: &Options,
) -> Result<Output, ArmadaError> {
    // **The refusal comes before the preflight**, so a second run against a
    // real `~/.armada/` cannot get as far as touching anything.
    let existed = place.armada_home.is_dir();
    if existed && !options.force {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: shown(&place.armada_home),
            message: format!("{} already exists", shown(&place.armada_home)),
            next_action: Some(
                "`armada doctor` reports what is missing; `armada init --force` re-runs"
                    .to_string(),
            ),
        });
    }

    let checks = preflight::run(runner, &place.cwd, false);
    let mut results = checks.results;

    // **The fatal check is reported before it is raised.** A caller who ran
    // `armada init` on a machine with no `claude` should see the whole
    // checklist, including what *is* there, rather than one line about the one
    // thing that is not.
    if let Some(fatal) = checks.fatal {
        return Ok(Output::MachineInit(Box::new(Envelope::failed(
            "init",
            None,
            fatal,
            MachineInitData {
                results,
                guild: None,
                imported: Vec::new(),
                asked: Vec::new(),
                questions: QUESTIONS.len(),
                skipped: 0,
                guild_path: shown(&place.armada_home.join("guild")),
            },
        ))));
    }

    results.push(create_layout(&place.armada_home, existed)?);

    // The one question. `--guild` and `--bundle` answer it from the command
    // line; `--defaults` takes the default, which is to build one now.
    let chosen = match (&options.guild, &options.bundle) {
        (Some(_), _) => FROM_REMOTE,
        (_, Some(_)) => FROM_BUNDLE,
        _ if options.defaults => BUILD_ONE,
        _ => ask.choose(QUESTION, &ANSWERS, BUILD_ONE),
    };

    let mut imported = Vec::new();
    let mut asked = Vec::new();
    let mut skipped = 0;

    match chosen {
        FROM_REMOTE => {
            let remote = options.guild.clone().ok_or_else(|| ArmadaError {
                class: ErrClass::BadInvocation,
                r#where: "--guild".to_string(),
                message: "pulling a guild needs the remote to pull it from".to_string(),
                next_action: Some("`armada init --guild <url>`".to_string()),
            })?;
            guild::clone(runner, place, &remote)?;
            imported.push(format!("cloned from {remote}"));
            imported.extend(guild::inventory_of(&place.guild()).facts());
        }
        FROM_BUNDLE => {
            let path = options.bundle.clone().ok_or_else(|| ArmadaError {
                class: ErrClass::BadInvocation,
                r#where: "--bundle".to_string(),
                message: "importing a guild needs the bundle to import".to_string(),
                next_action: Some("`armada init --bundle ./guild.tar.zst`".to_string()),
            })?;
            guild::import_bundle(runner, place, Path::new(&path), false, options.force)?;
            imported.push(format!("imported {path}"));
            imported.extend(guild::inventory_of(&place.guild()).facts());
        }
        _ => {
            // Import first, then the five questions — the order that makes the
            // interview five questions rather than thirty.
            let built = guild::init(
                runner,
                place,
                &mut Recording {
                    inner: ask,
                    asked: &mut asked,
                },
                &InitOptions {
                    force: options.force,
                    ..InitOptions::default()
                },
            )?;
            if let Output::GuildInit(envelope) = &built {
                imported = envelope.data.imported.clone();
                imported.insert(0, format!("imported from {}", place.claude_shown()));
                skipped = envelope.data.skipped;
            }
        }
    }

    Ok(Output::MachineInit(Box::new(Envelope::ok(
        "init",
        None,
        Status::Ready,
        MachineInitData {
            results,
            guild: Some(GuildChoice {
                question: QUESTION.to_string(),
                options: ANSWERS.iter().map(|a| (*a).to_string()).collect(),
                chosen,
            }),
            imported,
            asked,
            questions: QUESTIONS.len(),
            skipped,
            guild_path: shown(&place.armada_home.join("guild")),
        },
    ))))
}

/// An [`Ask`] that keeps a copy of every prompt it put.
///
/// **The transcript is the record, so it has to be recorded.** `--json` carries
/// the questions that were asked (`MachineInitData::asked`), and the only place
/// that knows what was put is the thing that put it.
struct Recording<'a> {
    inner: &'a mut dyn Ask,
    asked: &'a mut Vec<Asked>,
}

impl Ask for Recording<'_> {
    fn question(&mut self, asked: &Asked) -> Option<String> {
        self.asked.push(asked.clone());
        self.inner.question(asked)
    }

    fn choose(&mut self, question: &str, options: &[&str], default: usize) -> usize {
        self.inner.choose(question, options, default)
    }
}

/// Make `~/.armada/` and everything under it.
///
/// **`guild/` is created empty and that is not a guild** — `armada guild init`,
/// `pull` or `import` makes one. The distinction is what lets `--force` refuse
/// to overwrite a guild without refusing every first run
/// (`armada_guild::layout::Guild::exists`).
fn create_layout(armada_home: &Path, existed: bool) -> Result<Finding, ArmadaError> {
    for directory in DIRECTORIES {
        std::fs::create_dir_all(armada_home.join(directory))
            .map_err(|e| unwritable(armada_home, &e))?;
    }
    for directory in GUILD_DIRECTORIES {
        std::fs::create_dir_all(armada_home.join("guild").join(directory))
            .map_err(|e| unwritable(armada_home, &e))?;
    }
    Ok(Finding {
        check: shown(armada_home) + "/",
        // Re-running on a machine that already had it is `ok`, not `created`:
        // Armada made nothing, and saying it did would be the one thing a
        // checklist must never do.
        status: if existed { Health::Ok } else { Health::Created },
        detail: DIRECTORIES.join(", "),
        remedy: None,
    })
}

/// `~/.armada` however it is really spelled, as a person writes it.
pub fn shown(path: &Path) -> String {
    let text = path.display().to_string();
    match text.find("/.armada") {
        Some(at) => format!("~{}", &text[at..]),
        None => text,
    }
}

fn unwritable(path: &Path, error: &std::io::Error) -> ArmadaError {
    ArmadaError {
        class: ErrClass::Environment,
        r#where: path.display().to_string(),
        message: format!("cannot create {}: {error}", path.display()),
        next_action: Some("check the path is writable, then retry unchanged".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ask::Defaults;
    use armada_core::ctx::{RunOutput, RunRequest, SpawnError};
    use std::cell::RefCell;
    use std::path::PathBuf;

    /// A `git` and a `claude` that answer, and a `docker` that does not.
    struct Tools {
        calls: RefCell<Vec<Vec<String>>>,
    }

    impl Run for Tools {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            self.calls.borrow_mut().push(request.argv.clone());
            // **The fake makes the directory a real `git init` would.** Without
            // it, every assertion about what a guild *is* would pass against a
            // guild that was never created — which is the class of green test
            // that hides a whole verb doing nothing.
            let argv: Vec<&str> = request.argv.iter().map(String::as_str).collect();
            if matches!(argv.as_slice(), ["git", "init", ..] | ["git", "clone", ..]) {
                let repository = match argv.as_slice() {
                    ["git", "clone", _, name] => request.cwd.join(name),
                    _ => request.cwd.clone(),
                };
                std::fs::create_dir_all(repository.join(".git")).unwrap();
            }
            let banner = match request.argv.first().map(String::as_str) {
                Some("git") if request.argv.get(1).is_some_and(|a| a == "--version") => {
                    "git version 2.51.0\n"
                }
                Some("claude") => "2.0.14 (Claude Code)\n",
                _ => "",
            };
            Ok(RunOutput {
                code: Some(if request.argv[0] == "docker" { 1 } else { 0 }),
                signal: None,
                stdout: banner.to_string(),
                stderr: String::new(),
                timed_out: false,
            })
        }
    }

    fn tools() -> Tools {
        Tools {
            calls: RefCell::new(Vec::new()),
        }
    }

    /// **A scratch `$HOME`, always.** Destroying somebody's real guild is
    /// unrecoverable, so no test in this crate names a path it did not create.
    fn scratch() -> (tempfile::TempDir, Where) {
        let home = tempfile::tempdir().unwrap();
        let place = Where {
            armada_home: home.path().join(".armada"),
            cwd: home.path().to_path_buf(),
            claude_home: home.path().join(".claude"),
        };
        (home, place)
    }

    fn init(place: &Where, options: &Options) -> Result<Output, ArmadaError> {
        run(&tools(), place, &mut Defaults, options)
    }

    fn data(output: &Output) -> MachineInitData {
        match output {
            Output::MachineInit(envelope) => envelope.data.clone(),
            _ => panic!("not an init"),
        }
    }

    /// The done-when, on a machine that has never seen Armada.
    #[test]
    fn a_fresh_machine_gets_the_layout_a_guild_and_a_report() {
        let (_home, place) = scratch();
        let output = init(&place, &Options::default()).unwrap();
        let data = data(&output);

        for directory in ["guild", "jobs", "workspaces"] {
            assert!(
                place.armada_home.join(directory).is_dir(),
                "{directory} was not created"
            );
        }
        assert!(place.guild().exists(), "the guild is not a repository");
        assert!(place.guild().path("workflows/bug.yml").is_file());
        assert!(place.guild().path("subagents/helm.md").is_file());
        assert!(place.guild().path("skills/onboard-repo/SKILL.md").is_file());
        assert_eq!(output.exit_code(), 0);
        assert_eq!(data.guild.unwrap().chosen, BUILD_ONE);
    }

    /// **`--defaults` leaves a working guild, not an empty one**, and says how
    /// many questions it skipped so `doctor` and the reader agree.
    #[test]
    fn defaults_leaves_a_working_guild_and_reports_every_question_skipped() {
        let (_home, place) = scratch();
        let data = data(
            &init(
                &place,
                &Options {
                    defaults: true,
                    ..Options::default()
                },
            )
            .unwrap(),
        );
        assert_eq!(data.questions, 5);
        assert_eq!(data.skipped, 5);
        assert!(place.guild().path("voice.md").is_file());
    }

    /// **The one that would be unrecoverable.** A second run refuses rather
    /// than rebuilding over a guild somebody has been editing for a year.
    #[test]
    fn a_second_run_refuses_rather_than_replacing_a_guild() {
        let (_home, place) = scratch();
        init(&place, &Options::default()).unwrap();

        let error = init(&place, &Options::default()).unwrap_err();
        assert_eq!(error.class, ErrClass::BadInvocation);
        assert_eq!(error.class.exit_code(), 2);
        assert!(error.next_action.unwrap().contains("--force"));
    }

    /// Re-running with `--force` says `ok` rather than `created`: Armada made
    /// nothing this time, and a checklist that claims otherwise is a checklist
    /// nobody can trust.
    #[test]
    fn a_forced_rerun_does_not_claim_to_have_created_what_was_already_there() {
        let (_home, place) = scratch();
        init(&place, &Options::default()).unwrap();
        let data = data(
            &init(
                &place,
                &Options {
                    force: true,
                    ..Options::default()
                },
            )
            .unwrap(),
        );
        let layout = data.results.last().unwrap();
        assert_eq!(layout.status, Health::Ok);
    }

    /// **A missing `claude` is fatal, and the checklist is still printed.** A
    /// caller should see what *is* there rather than one line about what is not.
    #[test]
    fn a_machine_with_no_claude_reports_the_whole_checklist_and_then_fails() {
        struct NoClaude;
        impl Run for NoClaude {
            fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
                Ok(RunOutput {
                    code: Some(if request.argv[0] == "git" { 0 } else { 1 }),
                    signal: None,
                    stdout: "git version 2.51.0\n".to_string(),
                    stderr: String::new(),
                    timed_out: false,
                })
            }
        }
        let (_home, place) = scratch();
        let output = run(&NoClaude, &place, &mut Defaults, &Options::default()).unwrap();

        assert_eq!(output.exit_code(), 6, "a broken machine is `environment`");
        let data = data(&output);
        assert_eq!(data.results.len(), 3, "the whole checklist is reported");
        assert_eq!(data.results[0].status, Health::Found, "git is still found");
        assert!(
            !place.armada_home.exists(),
            "a fatal preflight created directories anyway"
        );
    }

    /// `--guild` and `--bundle` answer the question from the command line, so
    /// the second-machine path needs no terminal at all.
    #[test]
    fn a_remote_answers_the_question_without_asking_it() {
        let (_home, place) = scratch();
        // The clone is faked: what is asserted is that the question was
        // answered `1` and that git was asked to clone.
        let run_fake = tools();
        let output = run(
            &run_fake,
            &place,
            &mut Defaults,
            &Options {
                guild: Some("git@example.com:me/guild.git".to_string()),
                ..Options::default()
            },
        )
        .unwrap();
        assert_eq!(data(&output).guild.unwrap().chosen, FROM_REMOTE);
        assert!(
            run_fake
                .calls
                .borrow()
                .iter()
                .any(|argv| argv.contains(&"clone".to_string())),
            "nothing was cloned"
        );
    }

    /// A path is shown the way a person writes it, so no output ever carries a
    /// real home directory.
    #[test]
    fn the_armada_home_is_shown_the_way_a_person_writes_it() {
        assert_eq!(shown(&PathBuf::from("/Users/agent/.armada")), "~/.armada");
        assert_eq!(
            shown(&PathBuf::from("/scratch/elsewhere")),
            "/scratch/elsewhere"
        );
    }
}
