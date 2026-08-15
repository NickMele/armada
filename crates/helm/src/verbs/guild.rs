//! `armada guild <verb>` — the sequencing, and nothing that decides anything.
//!
//! Every rule this file appears to apply was decided in `armada-guild`:
//! whether a value is credential-shaped, what a fast-forward is, whether a
//! bundle validates. What lives here is the order the adapter calls go in
//! (`ARCHITECTURE.md` §1.3), and the one thing only Helm can do — wire Guild's
//! answers into the envelope `render.rs` draws.
//!
//! **Helm is the only crate that may name both Guild and Manifest** (§1.9), so
//! this is where `armada_home` — Manifest's — meets `Guild::at` — Guild's. That
//! is not a workaround for the sibling rule; it is the sibling rule working.

use armada_core::ctx::Run;
use armada_core::envelope::{
    Envelope, GuildBundleData, GuildInitData, GuildSyncData, Headline, Projection, Sync, SyncItem,
};
use armada_core::error::{ArmadaError, ErrClass, Status};
use armada_guild::interview::{self, Answers, Question, QUESTIONS};
use armada_guild::layout::Guild;
use armada_guild::{bundle, import, inventory, machine, memory, projector, remote, repo, starters};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::ask::Ask;
use crate::verbs::Output;

/// Everything a Guild verb needs from the machine, gathered at the entrypoint.
///
/// **`~/.armada` arrives as a value rather than being resolved here**, which is
/// `ARCHITECTURE.md` §1.4 — nothing below the entrypoint reads `$HOME` — and
/// also what lets the whole test suite point `armada init` at a `TempDir`
/// instead of at somebody's real guild.
pub struct Where {
    /// `~/.armada/`.
    pub armada_home: PathBuf,
    /// Where the working directory is, for the subprocess seam.
    pub cwd: PathBuf,
    /// Where an import reads from, when the caller did not say.
    pub claude_home: PathBuf,
}

/// `~`, for the one answer that may be typed with one in it.
///
/// **Derived from `armada_home` rather than read**, which is `ARCHITECTURE.md`
/// §1.4 — nothing below the entrypoint reads `$HOME` — and also what keeps every
/// test in this file pointing at a `TempDir`.
fn home_of(armada_home: &Path) -> PathBuf {
    armada_home
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| armada_home.to_path_buf())
}

impl Where {
    /// The guild under this `~/.armada/`.
    pub fn guild(&self) -> Guild {
        Guild::at(&self.armada_home)
    }

    /// Where an import reads from, as a person writes it — `~/.claude/`, on
    /// every machine. A golden fixture cannot hold a real home directory.
    pub fn claude_shown(&self) -> String {
        import::display(&self.claude_home)
    }
}

/// `armada guild init` — import, interview, and make it a repository.
///
/// The three steps of `docs/commands/guild/init.md`, in that order, and the
/// order is the point: **import runs first and does most of the work**, so the
/// interview is five questions rather than thirty.
pub fn init(
    run: &impl Run,
    place: &Where,
    ask: &mut dyn Ask,
    options: &InitOptions,
) -> Result<Output, ArmadaError> {
    let guild = place.guild();
    if guild.exists() && !options.force {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: guild.root().display().to_string(),
            message: "a guild is already here".to_string(),
            next_action: Some(
                "`armada guild init --force` replaces it; `armada guild edit` changes one file"
                    .to_string(),
            ),
        });
    }

    // 1. Adopt what is already on the machine.
    let from = options
        .from
        .clone()
        .unwrap_or_else(|| place.claude_home.clone());
    let imported = if options.no_import {
        import::run(Path::new("/nonexistent"), &guild).map_err(|e| unwritable(guild.root(), &e))?
    } else {
        import::run(&from, &guild).map_err(|e| unwritable(guild.root(), &e))?
    };

    // 2. Ask the five, from scratch. Every one of them has a default, and
    //    `--defaults` is an `Ask` that takes all of them. The guild is passed in
    //    so each question can show what pressing enter would keep — import has
    //    just written it, and a default nobody can see is not a default.
    let mut answers = interview(ask, Some(&guild));
    if let Some(remote) = &options.remote {
        answers.remote = Some(remote.clone());
    }
    // **A folder is made into a remote here, before anything is committed.**
    // Question 5 takes either, and a folder becomes a bare repository git can
    // push to — see `armada_guild::remote`. Done before the first commit so a
    // folder that cannot be written fails the verb rather than leaving a guild
    // whose remote points at nothing.
    if let Some(answer) = answers.remote.clone() {
        answers.remote = Some(remote::prepare(
            run,
            &remote::classify(&answer),
            &home_of(&place.armada_home),
        )?);
    }

    // Your answer wins over the import where the two overlap (PLAN.md §13.4).
    let mut wrote = Vec::new();
    for fragment in armada_guild::memory::FRAGMENTS {
        if let Some(answer) = answers.fragment(fragment) {
            std::fs::write(guild.path(fragment), format!("{}\n", answer.trim()))
                .map_err(|e| unwritable(&guild.path(fragment), &e))?;
        }
        wrote.push(fragment.to_string());
    }
    wrote
        .extend(starters::write(guild.root(), &answers).map_err(|e| unwritable(guild.root(), &e))?);

    // 3. The repository, and the remote if one was named.
    if !guild.exists() {
        repo::init(run, guild.root(), "guild: imported and interviewed")?;
    } else {
        repo::commit_all(run, guild.root(), "guild: re-initialised")?;
    }
    if let Some(remote) = &answers.remote {
        repo::set_remote(run, guild.root(), remote)?;
    }

    let withheld: Vec<String> = imported
        .withheld
        .iter()
        .map(|w| format!("{}:{}", w.source, w.key))
        .collect();
    // **The one place a pre-namespace `machine.yml` gets fixed, and it is
    // here because this is the only crate allowed to ask both modules.**
    // `machine.yml` carries one section per module, and a file written before
    // the sections existed has one module's keys loose at the top level — which
    // every other module's reader then has to tolerate. It is migrated on the
    // write that was going to happen anyway rather than on a read, so no verb
    // that merely looks at configuration rewrites it, and it is reported,
    // because a file that changed under somebody without being mentioned is
    // worse than one that did not change at all.
    let migrated = armada_manifest::machine::migrate(&place.armada_home)
        .map_err(|e| unwritable(&place.armada_home, &e))?;
    machine::record(&place.armada_home, &answers, &imported.withheld)
        .map_err(|e| unwritable(&place.armada_home, &e))?;

    // 4. **Put it where Claude Code will read it.** A `guild init` that stopped
    //    at step 3 left a guild nothing reads — the failure `PHASES.md` §8.4
    //    records, where a skill this verb has just installed answers `Unknown
    //    command` in the session Armada hands you to.
    let projected = projected(place);

    Ok(Output::GuildInit(Box::new(Envelope::ok(
        "guild init",
        None,
        Status::Ready,
        GuildInitData {
            guild_path: shown(guild.root()),
            imported: imported.inventory.facts(),
            withheld,
            wrote,
            migrated: migrated.map(|m| m.note()),
            remote: answers.remote.clone(),
            questions: QUESTIONS.len(),
            answered: answers.answered(),
            projected,
        },
    ))))
}

/// What `armada guild init` was asked for.
#[derive(Debug, Clone, Default)]
pub struct InitOptions {
    /// `--from <path>`.
    pub from: Option<PathBuf>,
    /// `--no-import`.
    pub no_import: bool,
    /// `--remote <url>`.
    pub remote: Option<String>,
    /// `--force`.
    pub force: bool,
}

/// Put the five questions and collect what comes back.
///
/// **Every answer is optional and nothing is re-asked.** An unparseable ceiling
/// takes the documented default rather than looping: a loop is a loop a piped
/// stdin cannot escape, and the default is a working outcome.
///
/// **Each question carries what pressing enter would keep.** Import has already
/// written the three fragments, so the standing value is read back off disk and
/// shown — a default you cannot see is not one you can accept with confidence,
/// which is what a real first run of this said about `(enter to keep what import
/// found)` printed over nothing.
pub fn interview(ask: &mut dyn Ask, guild: Option<&Guild>) -> Answers {
    let mut answers = Answers::default();
    for question in QUESTIONS {
        let asked = armada_core::envelope::Asked {
            number: question.number,
            of: QUESTIONS.len(),
            prompt: question.prompt.to_string(),
            purpose: question.purpose.to_string(),
            writes: question.writes.to_string(),
            keeps: question.keeps.to_string(),
            prose: question.shape == interview::Shape::Prose,
            standing: standing(question, guild),
        };
        let Some(given) = ask.question(&asked) else {
            continue;
        };
        record(&mut answers, question, &given);
    }
    answers
}

/// What the answer already is.
///
/// **Whole, and with its line breaks.** A prose question opens a text area
/// holding this, so it is the thing being edited rather than a preview of it —
/// and a fragment collapsed to one line would be a fragment whose paragraphs the
/// person had to put back by hand. The two short structured questions still show
/// theirs on a `now` line, because a triple of ceilings fits on one.
///
/// The fragments are read from the guild the import has just written; the
/// ceilings are a constant; the remote has no standing value because sync being
/// off is the absence of one, which the hint already says in words.
fn standing(question: Question, guild: Option<&Guild>) -> Option<String> {
    match question.number {
        4 => Some(interview::Ceilings::AUTONOMOUS.written()),
        5 => None,
        _ => {
            let body = std::fs::read_to_string(guild?.path(question.writes)).ok()?;
            // **Armada's own examples are not your answer.** A fragment import
            // found nothing for holds a template — a purpose line and four
            // examples marked as replaceable — and opening the box on those
            // would invite a reader to save them as his own words. The box opens
            // empty and the prompt says `nothing of yours yet`.
            if memory::state(&body) == Some(memory::Unedited::Example) {
                return None;
            }
            Some(content(&body, memory::Fragment::of(question.writes)))
                .filter(|text| !text.is_empty())
        }
    }
}

/// A fragment's content: **Armada's words dropped, yours kept, line breaks and
/// all**.
///
/// Comments go, the file's own heading goes, and so do the two lines Armada
/// writes into every fragment — who reads it, and what to write in it. What is
/// left is what the person wrote, with the blank lines between its paragraphs
/// still in place, because that is what goes into the box he then edits.
///
/// Every clause of that was earned by showing the wrong thing to a real reader.
/// A filter that dropped lines *beginning* with `<!--` left five lines of a
/// six-line comment behind, so all three questions offered the same standing
/// answer: a paragraph about how the file had been written. And the purpose
/// lines are prose rather than headings, so nothing structural distinguishes
/// them — they are matched against what `armada-guild` says it wrote, which is
/// the only reading that cannot drift from it.
fn content(body: &str, which: Option<memory::Fragment>) -> String {
    let armadas: Vec<&str> = which
        .map(|which| vec![which.read_by(), which.asks()])
        .unwrap_or_default();
    without_comments(body)
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with('#') && !armadas.contains(&trimmed)
        })
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Every `<!-- … -->` span removed, however many lines it covers.
fn without_comments(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(open) = rest.find("<!--") {
        out.push_str(&rest[..open]);
        rest = match rest[open..].find("-->") {
            Some(close) => &rest[open + close + 3..],
            // An unterminated comment swallows the rest of the file, which is
            // what a reader's markdown viewer does with it too.
            None => "",
        };
    }
    out.push_str(rest);
    out
}

fn record(answers: &mut Answers, question: Question, given: &str) {
    match question.number {
        1 => answers.voice = Some(given.to_string()),
        2 => answers.expectations = Some(given.to_string()),
        3 => answers.how_i_work = Some(given.to_string()),
        4 => answers.ceilings = interview::parse_ceilings(given).ok(),
        _ => answers.remote = Some(given.to_string()),
    }
}

/// `armada guild pull` — bring this machine's guild up to date.
///
/// **A fast-forward or nothing.** On a divergence this changes not one byte and
/// reports both counts and every file touched on both sides, which is the whole
/// of `guild/pull.md`'s exit-code contract.
pub fn pull(run: &impl Run, place: &Where) -> Result<Output, ArmadaError> {
    let guild = place.guild();
    let remote = require_remote(run, &guild)?;

    // **A folder remote is asked for before it is read.** iCloud Drive evicts
    // the contents of files it thinks are unused; a bare repository in that
    // state is one git reports as corrupt for a repository that is intact and
    // merely elsewhere (`armada_guild::remote`).
    if let remote::Destination::Folder(folder) = remote::classify(&remote) {
        remote::materialise(
            &remote::expand(&folder, &home_of(&place.armada_home)),
            std::time::Instant::now,
        )?;
    }

    let apart = match repo::fetch(run, guild.root()) {
        Ok(apart) => apart,
        // **A remote mid-sync is a conflict, not a crash.** A push writes
        // several files and a sync service replicates them in its own order, so
        // a machine reading in between sees refs pointing at objects that have
        // not arrived. It resolves itself; what the reader needs is to be told
        // to wait rather than to be shown a broken repository.
        Err(error) if remote::is_torn(&error.message) => return Ok(torn(&remote, &error)),
        Err(error) => return Err(error),
    };
    let incoming = repo::incoming(run, guild.root())?;
    let outgoing = repo::outgoing(run, guild.root())?;

    let diverged = apart.diverged();
    if !diverged && apart.behind > 0 {
        repo::fast_forward(run, guild.root())?;
    }

    let results = change_set(&incoming, &outgoing, diverged, Some(&guild));
    let conflicts = results
        .iter()
        .filter(|row| row.status == Sync::Conflict)
        .count();
    let applied = !diverged;

    // **Re-project what changed.** A pulled guild that has not been projected
    // is a guild that has not taken effect, and the gap between the two is a
    // confusing hour (`guild/pull.md`). Nothing was applied on a divergence, so
    // there is nothing new to project and the step is skipped rather than run
    // over a working tree the reader is about to resolve by hand.
    let projected = applied.then(|| projected(place)).flatten();
    let kept = projected.as_ref().map_or(0, |done| done.kept);

    let data = GuildSyncData {
        remote: Some(remote),
        ahead: apart.ahead,
        behind: apart.behind,
        results,
        applied,
        headline: (conflicts > 0 || kept > 0).then_some(Headline::NeedsAttention),
        projected,
    };

    Ok(Output::GuildSync(Box::new(if diverged {
        Envelope::failed(
            "guild pull",
            None,
            ArmadaError {
                class: ErrClass::ToolFailed,
                r#where: shown(guild.root()),
                message: format!("the guild has diverged: {}", apart.written()),
                next_action: Some(
                    "resolve it in ~/.armada/guild, then `armada guild push`".to_string(),
                ),
            },
            data,
        )
    } else {
        Envelope::ok("guild pull", None, Status::Ready, data)
    })))
}

/// A remote a sync service has half-replicated, reported as a conflict.
///
/// **Nothing was applied and nothing on this machine was touched.** The counts
/// are unknown — the fetch that would have measured them is the call that failed
/// — so the row says what happened rather than pretending to a number, and the
/// remedy is the only one there is.
fn torn(remote: &str, error: &ArmadaError) -> Output {
    Output::GuildSync(Box::new(Envelope::failed(
        "guild pull",
        None,
        ArmadaError {
            class: ErrClass::ToolFailed,
            r#where: remote.to_string(),
            message: format!("the remote is part-way through syncing: {}", error.message),
            next_action: Some(
                "wait for the sync to finish, then `armada guild pull` again".to_string(),
            ),
        },
        GuildSyncData {
            remote: Some(remote.to_string()),
            ahead: 0,
            behind: 0,
            results: vec![SyncItem {
                status: Sync::Conflict,
                item: "origin".to_string(),
                detail: "part-way through syncing, nothing read".to_string(),
            }],
            applied: false,
            headline: Some(Headline::NeedsAttention),
            projected: None,
        },
    )))
}

/// `armada guild project` — put the guild where Claude Code will read it, or
/// `--remove` to take back exactly what was put there.
///
/// Every rule is `armada_guild::project`'s. What is here is the two calls and
/// the envelope, which is `ARCHITECTURE.md` §1.3.
pub fn project(place: &Where, remove: bool) -> Result<Output, ArmadaError> {
    let guild = place.guild();
    if !remove && !guild.exists() {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: shown(guild.root()),
            message: "there is no guild here, so there is nothing to project".to_string(),
            next_action: Some("`armada init`, or `armada guild init`".to_string()),
        });
    }

    let projected = if remove {
        projector::remove(&place.claude_home, &place.armada_home)
    } else {
        projector::project(&guild, &place.claude_home, &place.armada_home)
    }
    .map_err(|e| unwritable(&place.claude_home, &e))?;

    let data = projection(place, &projected);
    let attention = data.kept > 0;
    Ok(Output::GuildProject(Box::new(Envelope::ok(
        if remove {
            "guild project --remove"
        } else {
            "guild project"
        },
        None,
        if attention {
            Status::Partial
        } else {
            Status::Ready
        },
        Projection {
            headline: attention.then_some(Headline::NeedsAttention),
            ..data
        },
    ))))
}

/// A projection, as the envelope carries it.
///
/// **The rows are grouped by area exactly as `guild pull`'s are**, and for the
/// same reason: a guild with forty skills would otherwise print forty rows on a
/// verb whose whole job is to be glanced at. The files left as yours are the
/// exception a reader has to act on, so their count is its own field rather
/// than something to be counted off the table.
fn projection(place: &Where, projected: &projector::Projected) -> Projection {
    Projection {
        at: place.claude_shown(),
        results: grouped(
            projected
                .steps
                .iter()
                .map(|step| (step.at.clone(), step.outcome)),
            "edited here; delete it to take the guild's",
        ),
        facts: armada_guild::project::facts(&projected.steps),
        kept: projected.yours().len(),
        headline: None,
    }
}

/// Put the guild on Claude Code's load path, on a verb whose main job is
/// something else.
///
/// **A failure here does not fail the verb.** `guild init` has just written a
/// guild and `guild pull` has just fast-forwarded one; losing that outcome
/// because `~/.claude/` is read-only would be reporting the wrong failure. It
/// comes back as `None`, and `armada doctor` is the check that says the guild
/// is not projected.
pub fn projected(place: &Where) -> Option<Projection> {
    let guild = place.guild();
    if !guild.exists() {
        return None;
    }
    let done = projector::project(&guild, &place.claude_home, &place.armada_home).ok()?;
    Some(projection(place, &done))
}

/// `armada guild push` — send local changes.
pub fn push(run: &impl Run, place: &Where, force: bool) -> Result<Output, ArmadaError> {
    let guild = place.guild();
    let remote = require_remote(run, &guild)?;
    // **Commit first**, so an edit made outside `armada guild edit` is not left
    // behind — the first half of the drift failure `PHASES.md` §11 names.
    repo::commit_all(run, guild.root(), "guild: local changes")?;
    let apart = repo::fetch(run, guild.root())?;

    if apart.diverged() && !force {
        let outgoing = repo::outgoing(run, guild.root())?;
        let incoming = repo::incoming(run, guild.root())?;
        let results = change_set(&incoming, &outgoing, true, Some(&guild));
        return Ok(Output::GuildSync(Box::new(Envelope::failed(
            "guild push",
            None,
            ArmadaError {
                class: ErrClass::ToolFailed,
                r#where: shown(guild.root()),
                message: format!("the guild has diverged: {}", apart.written()),
                next_action: Some("`armada guild pull` first".to_string()),
            },
            GuildSyncData {
                remote: Some(remote),
                ahead: apart.ahead,
                behind: apart.behind,
                results,
                applied: false,
                headline: Some(Headline::NeedsAttention),
                projected: None,
            },
        ))));
    }
    // **`--force` is refused unless the remote is strictly behind**, which is
    // to say unless it discards nothing. Checked here rather than trusted to
    // `--force-with-lease`, so the refusal names what would have been lost.
    if force && apart.behind > 0 {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: "--force".to_string(),
            message: format!(
                "--force would discard {} the remote has and this machine does not",
                crate::render::format::count(apart.behind, "commit")
            ),
            next_action: Some("`armada guild pull` first".to_string()),
        });
    }

    let outgoing = repo::outgoing(run, guild.root())?;
    repo::push(run, guild.root(), force)?;

    Ok(Output::GuildSync(Box::new(Envelope::ok(
        "guild push",
        None,
        Status::Ready,
        GuildSyncData {
            remote: Some(remote),
            ahead: apart.ahead,
            behind: 0,
            results: change_set(&outgoing, &[], false, None),
            applied: true,
            headline: None,
            // **A push does not project.** Nothing arrived; what is on this
            // machine's load path is already what this machine's guild says.
            projected: None,
        },
    ))))
}

/// `armada guild export`.
pub fn export(
    run: &impl Run,
    place: &Where,
    out: Option<&Path>,
    include_secrets: bool,
) -> Result<Output, ArmadaError> {
    let guild = place.guild();
    let out = out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| place.cwd.join(bundle::DEFAULT_OUT));
    let exported = bundle::export(run, &guild, &place.armada_home, &out, include_secrets)?;

    Ok(Output::GuildBundle(Box::new(Envelope::ok(
        "guild export",
        None,
        Status::Ready,
        GuildBundleData {
            path: shown(&exported.path),
            bytes: Some(exported.bytes),
            contents: exported.inventory.facts(),
            secrets: exported.secrets,
            skipped: Vec::new(),
            conflicts: Vec::new(),
        },
    ))))
}

/// `armada guild import`.
pub fn import_bundle(
    run: &impl Run,
    place: &Where,
    path: &Path,
    merge: bool,
    force: bool,
) -> Result<Output, ArmadaError> {
    let guild = place.guild();
    if guild.exists() && !merge && !force {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: shown(guild.root()),
            message: "a guild is already here".to_string(),
            next_action: Some(
                "`--merge` keeps yours where the two differ; `--force` replaces it".to_string(),
            ),
        });
    }
    let unpacked = bundle::import(run, path, &guild, &place.armada_home, merge)?;
    // A bundle carries no history, so the imported guild starts a fresh one.
    if guild.exists() {
        repo::commit_all(run, guild.root(), "guild: imported from a bundle")?;
    } else {
        repo::init(run, guild.root(), "guild: imported from a bundle")?;
    }

    let mut skipped = Vec::new();
    if unpacked.machine_skipped {
        skipped.push("machine.yml".to_string());
    }
    let conflicts = unpacked.conflicts.clone();
    let data = GuildBundleData {
        path: shown(path),
        bytes: None,
        contents: unpacked.inventory.facts(),
        secrets: false,
        skipped,
        conflicts,
    };
    Ok(Output::GuildBundle(Box::new(Envelope::ok(
        "guild import",
        None,
        if unpacked.conflicts.is_empty() {
            Status::Ready
        } else {
            Status::Partial
        },
        data,
    ))))
}

/// Clone a guild from a remote — `armada init --guild`, and the second-machine
/// path `PHASES.md` §8.4's done-when describes.
pub fn clone(run: &impl Run, place: &Where, remote: &str) -> Result<(), ArmadaError> {
    let guild = place.guild();
    if guild.exists() {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: shown(guild.root()),
            message: "a guild is already here".to_string(),
            next_action: Some("`armada guild pull` brings it up to date".to_string()),
        });
    }
    // **`git clone` makes the directory itself and refuses a non-empty one**,
    // and `armada init` has already created `guild/` with its four empty
    // subdirectories — so the scaffold is removed before the clone rather than
    // cloned into. Measured: without this, `armada init --guild <url>` — the
    // whole second-machine path — fails with *destination path 'guild' already
    // exists and is not an empty directory*.
    //
    // **Only a scaffold is removed.** If anything holds a file, this refuses:
    // `remove_dir_all` on a directory somebody has put content in is the one
    // unrecoverable mistake this verb could make.
    if let Some(file) = a_file_under(guild.root()) {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: shown(guild.root()),
            message: format!(
                "{} holds {file}, and cloning over it would lose it",
                shown(guild.root())
            ),
            next_action: Some(
                "move it aside, or `armada guild init` and then `armada guild pull`".to_string(),
            ),
        });
    }
    let _ = std::fs::remove_dir_all(guild.root());
    repo::clone(run, remote, guild.root())
}

/// The first regular file anywhere under a directory, guild-relative, or `None`
/// for a tree of empty directories.
fn a_file_under(root: &Path) -> Option<String> {
    fn walk(root: &Path, at: &Path) -> Option<String> {
        for entry in std::fs::read_dir(at).ok()?.filter_map(Result::ok) {
            let path = entry.path();
            let found = if path.is_dir() {
                walk(root, &path)
            } else {
                path.strip_prefix(root)
                    .ok()
                    .map(|relative| relative.display().to_string())
            };
            if found.is_some() {
                return found;
            }
        }
        None
    }
    walk(root, root)
}

/// The remote, or the refusal `guild/push.md` and `pull.md` both specify:
/// **exit `2`, and point at `export`.**
fn require_remote(run: &impl Run, guild: &Guild) -> Result<String, ArmadaError> {
    if !guild.exists() {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: shown(guild.root()),
            message: "there is no guild here".to_string(),
            next_action: Some("`armada init`, or `armada guild init`".to_string()),
        });
    }
    repo::remote(run, guild.root())?.ok_or_else(|| ArmadaError {
        class: ErrClass::BadInvocation,
        r#where: shown(guild.root()),
        message: "this guild has no remote, so there is nothing to sync with".to_string(),
        next_action: Some(
            "`armada guild export` writes a bundle instead; a remote is set by \
             `armada guild init --remote <url>`"
                .to_string(),
        ),
    })
}

/// Group the change set by the area of the guild it touches.
///
/// **One row per area, not per file**, which is the agreed layout
/// (`tests/golden/render/guild-pull.plain`): `skills`, `hooks`, `workflows` and
/// the three fragments by name. A guild with forty skills would otherwise print
/// forty rows on a verb whose whole job is to be glanced at.
///
/// A path touched on **both** sides is a `conflict`, and it is reported rather
/// than resolved. `diverged` is what decides whether the outgoing side counts:
/// on a fast-forward there is nothing local to conflict with.
fn change_set(
    incoming: &[repo::Touched],
    outgoing: &[repo::Touched],
    diverged: bool,
    guild: Option<&Guild>,
) -> Vec<SyncItem> {
    let local: Vec<&str> = outgoing.iter().map(|t| t.path.as_str()).collect();
    let mut rows = grouped(
        incoming.iter().map(|touched| {
            let status = if diverged && local.contains(&touched.path.as_str()) {
                Sync::Conflict
            } else {
                match touched.change {
                    repo::Change::Added => Sync::Added,
                    repo::Change::Changed => Sync::Changed,
                    repo::Change::Removed => Sync::Removed,
                }
            };
            (touched.path.clone(), status)
        }),
        "edited here and on origin",
    );

    // **What did not move is reported too.** A pull that lists three areas and
    // says nothing about the fourth leaves a reader wondering whether it was
    // checked — the same reasoning `clean` keeps its table for.
    if let Some(guild) = guild {
        let touched: Vec<&str> = rows.iter().map(|row| row.item.as_str()).collect();
        rows.extend(unchanged(guild, &touched));
    }
    rows
}

/// One row per area, from any set of paths that each had something happen to
/// them.
///
/// Shared by the change set and by projection, because both answer the same
/// question about a different tree, and the reader is scanning the same column
/// in both. `conflict` is the one sentence they do not share: a change set's
/// conflict was edited on two machines, and a projection's was edited by hand
/// here.
fn grouped(rows: impl IntoIterator<Item = (String, Sync)>, conflict: &str) -> Vec<SyncItem> {
    // Keyed on **the outcome first and the area second**, which is also the
    // order the rows come out in (`tests/golden/render/guild-pull.plain`):
    // everything that arrived, then everything that changed, then the conflicts,
    // then what did not move. A reader scanning the STATUS column is scanning
    // for one word, and sorting by area would scatter it.
    //
    // The pair is the key rather than the area alone, so one area can appear
    // once as `added` and once as `conflict` — which is what happens when a
    // skill arrives and another is edited on both sides.
    let mut by_area: BTreeMap<(usize, String), (Sync, Vec<String>)> = BTreeMap::new();
    for (path, status) in rows {
        let (area, name) = area_of(&path);
        by_area
            .entry((rank(status), area))
            .or_insert((status, Vec::new()))
            .1
            .push(name);
    }

    by_area
        .into_iter()
        .map(|((_, area), (status, mut names))| {
            names.sort();
            names.dedup();
            SyncItem {
                status,
                item: area.clone(),
                // **A conflict says what happened, not which files.** The names
                // are already in the ITEM column for a fragment, and for an
                // area the useful fact is that both sides moved.
                detail: if status == Sync::Conflict {
                    conflict.to_string()
                } else if names.len() == 1 && names[0] == *area {
                    // A file at the guild's root is its own area, so naming it
                    // twice — `changed  voice.md  voice.md` — says nothing the
                    // ITEM column has not already said.
                    String::new()
                } else {
                    names.join(", ")
                },
            }
        })
        .collect()
}

/// Where each outcome sits in the STATUS column's reading order.
///
/// Declared here rather than derived from the enum's discriminant, so a variant
/// added to `Sync` for some other reason cannot silently reorder a frozen
/// layout.
const fn rank(status: Sync) -> usize {
    match status {
        Sync::Added => 0,
        Sync::Changed => 1,
        Sync::Removed => 2,
        Sync::Conflict => 3,
        Sync::Unchanged => 4,
    }
}

/// One row per guild area the change set did not touch, with how many files it
/// holds.
fn unchanged(guild: &Guild, touched: &[&str]) -> Vec<SyncItem> {
    armada_guild::layout::GUILD_DIRECTORIES
        .iter()
        .filter(|area| !touched.contains(area))
        .filter_map(|area| {
            let count = std::fs::read_dir(guild.path(area))
                .map(|listing| listing.filter_map(Result::ok).count())
                .unwrap_or(0);
            // An area with nothing in it is not "unchanged", it is empty, and a
            // row saying `unchanged  hooks  0` is a row about nothing.
            (count > 0).then(|| SyncItem {
                status: Sync::Unchanged,
                item: (*area).to_string(),
                detail: count.to_string(),
            })
        })
        .collect()
}

/// The area a guild-relative path belongs to, and the name within it.
///
/// `skills/add-migration/SKILL.md` is `skills` and `add-migration`;
/// `voice.md` is its own area and its own name, because a fragment is a file a
/// person edits directly and naming it `.` would hide the one that changed.
fn area_of(path: &str) -> (String, String) {
    match path.split_once('/') {
        Some((area, rest)) => (
            area.to_string(),
            rest.split('/').next().unwrap_or(rest).to_string(),
        ),
        None => (path.to_string(), path.to_string()),
    }
}

/// A path as a person writes it. A golden fixture cannot hold a real home
/// directory, and nor should any output — it is the same string on every
/// machine and says nothing.
fn shown(path: &Path) -> String {
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
        message: format!("cannot write {}: {error}", path.display()),
        next_action: Some("check the path is writable, then retry unchanged".to_string()),
    }
}

/// What is in a guild, for `doctor` and for the summary lines.
pub fn inventory_of(guild: &Guild) -> inventory::Inventory {
    inventory::Inventory::of(guild.root())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touched(path: &str, change: repo::Change) -> repo::Touched {
        repo::Touched {
            path: path.to_string(),
            change,
        }
    }

    /// **One row per area, not per file.** A guild with forty skills would
    /// otherwise print forty rows on a verb whose whole job is to be glanced at.
    #[test]
    fn a_change_set_groups_by_the_area_of_the_guild_it_touches() {
        let rows = change_set(
            &[
                touched("skills/add-migration/SKILL.md", repo::Change::Added),
                touched("skills/triage-flake/SKILL.md", repo::Change::Added),
                touched("hooks/stop-notify.sh", repo::Change::Changed),
            ],
            &[],
            false,
            None,
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].status, Sync::Added);
        assert_eq!(rows[0].item, "skills");
        assert_eq!(rows[0].detail, "add-migration, triage-flake");
        assert_eq!(rows[1].item, "hooks");
        assert_eq!(rows[1].detail, "stop-notify.sh");
    }

    /// **A path touched on both sides is a conflict, and it is reported.** The
    /// row is named for the file rather than for its area, because a fragment
    /// is a file a person opens.
    #[test]
    fn a_path_touched_on_both_sides_is_a_conflict() {
        let rows = change_set(
            &[touched("voice.md", repo::Change::Changed)],
            &[touched("voice.md", repo::Change::Changed)],
            true,
            None,
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, Sync::Conflict);
        assert_eq!(rows[0].item, "voice.md");
        assert_eq!(rows[0].detail, "edited here and on origin");
    }

    /// On a fast-forward there is nothing local to conflict with, so the same
    /// two sides are not a conflict.
    #[test]
    fn the_same_path_is_not_a_conflict_when_the_histories_have_not_diverged() {
        let rows = change_set(
            &[touched("voice.md", repo::Change::Changed)],
            &[touched("voice.md", repo::Change::Changed)],
            false,
            None,
        );
        assert_eq!(rows[0].status, Sync::Changed);
    }

    /// **A torn remote is a conflict, and nothing on this machine moves.** A
    /// sync service replicates a push in its own order; a machine reading in
    /// between sees refs pointing at objects that have not arrived. The remedy
    /// is to wait, which is a remedy and not a crash.
    #[test]
    fn a_remote_part_way_through_syncing_is_reported_as_a_conflict() {
        let output = torn(
            "/scratch/Drive/guild",
            &ArmadaError {
                class: ErrClass::ToolFailed,
                r#where: "git fetch origin main".to_string(),
                message: "error: unable to read sha1 file".to_string(),
                next_action: None,
            },
        );
        let Output::GuildSync(envelope) = &output else {
            panic!("not a sync");
        };
        assert!(!envelope.data.applied, "a torn remote applied something");
        assert_eq!(envelope.data.results[0].status, Sync::Conflict);
        assert_eq!(envelope.data.headline, Some(Headline::NeedsAttention));
        let error = envelope.error.as_ref().expect("no error");
        assert!(error.next_action.as_deref().unwrap().contains("wait"));
    }

    /// **What goes in the box is what the file says, not who wrote it.**
    ///
    /// Run against what `armada-guild` actually writes rather than a hand-typed
    /// approximation of it, because every mistake this test exists for was a
    /// line of Armada's own boilerplate that the filter did not know about — a
    /// six-line comment matched only on its first line, and then the two purpose
    /// lines, which are prose and have nothing structural to match on at all.
    #[test]
    fn the_standing_value_is_the_content_and_not_what_armada_wrote_around_it() {
        let imported = &armada_guild::memory::split(
            "## Verbosity\n\n150 words maximum. Lead with the answer.\n",
        )[0]
        .1;
        assert_eq!(
            content(imported, Some(memory::Fragment::Voice)),
            "150 words maximum. Lead with the answer."
        );
    }

    /// **The paragraphs survive, because the box is what you then edit.**
    ///
    /// It used to be collapsed to one line, which was fine for a preview and is
    /// not fine for the thing being edited: a fragment whose blank lines had
    /// been eaten is one the person has to put back by hand before he can add to
    /// it.
    #[test]
    fn a_multi_paragraph_fragment_keeps_its_line_breaks() {
        let imported = &armada_guild::memory::split(
            "## Verbosity\n\nLead with the answer.\n\nTables for comparisons.\n",
        )[0]
        .1;
        assert_eq!(
            content(imported, Some(memory::Fragment::Voice)),
            "Lead with the answer.\n\nTables for comparisons."
        );
    }

    /// **A fragment import found nothing for opens an empty box**, even though
    /// the file is not empty: it holds Armada's examples, and pre-filling with
    /// those would invite a reader to press `ctrl-d` and save them as his own
    /// words.
    #[test]
    fn armadas_own_examples_are_not_offered_as_your_answer() {
        let home = tempfile::tempdir().unwrap();
        let guild = Guild::at(home.path());
        std::fs::create_dir_all(guild.root()).unwrap();
        for (name, body) in armada_guild::memory::split("") {
            std::fs::write(guild.path(name), body).unwrap();
        }
        for question in QUESTIONS.iter().take(3) {
            assert_eq!(
                standing(*question, Some(&guild)),
                None,
                "question {} offered Armada's examples as an answer",
                question.number
            );
        }
    }

    /// An unterminated comment takes the rest of the file with it, which is what
    /// a markdown viewer does too.
    #[test]
    fn an_unterminated_comment_swallows_what_follows_it() {
        assert_eq!(content("real\n<!-- open\nmore\n", None), "real");
    }

    /// `~` comes from the `~/.armada` the entrypoint passed down, never from a
    /// `$HOME` this file went and read.
    #[test]
    fn the_home_a_tilde_expands_against_is_the_one_armada_home_sits_in() {
        assert_eq!(
            home_of(Path::new("/scratch/agent/.armada")),
            PathBuf::from("/scratch/agent")
        );
    }

    #[test]
    fn a_path_is_split_into_the_area_and_the_name_within_it() {
        assert_eq!(
            area_of("skills/add-migration/SKILL.md"),
            ("skills".to_string(), "add-migration".to_string())
        );
        assert_eq!(
            area_of("voice.md"),
            ("voice.md".to_string(), "voice.md".to_string())
        );
    }

    /// A path is shown the way a person writes it, so no output and no fixture
    /// ever carries a real home directory.
    #[test]
    fn a_guild_path_is_shown_the_way_a_person_writes_it() {
        assert_eq!(
            shown(Path::new("/Users/agent/.armada/guild")),
            "~/.armada/guild"
        );
        assert_eq!(shown(Path::new("/tmp/elsewhere")), "/tmp/elsewhere");
    }

    /// The interview puts all five and records each answer against the file it
    /// belongs to.
    #[test]
    fn the_interview_puts_five_questions_and_files_each_answer() {
        let mut ask = crate::ask::Scripted {
            answers: vec![
                Some("brief".to_string()),
                None,
                None,
                Some("8, 200k, 30m".to_string()),
                Some("git@example.com:me/guild.git".to_string()),
            ],
            ..crate::ask::Scripted::default()
        };
        let answers = interview(&mut ask, None);

        assert_eq!(ask.asked.len(), 5);
        assert_eq!(ask.asked[0].of, 5);
        assert_eq!(answers.voice.as_deref(), Some("brief"));
        assert_eq!(answers.expectations, None);
        assert_eq!(answers.ceilings.unwrap().iterations, 8);
        assert_eq!(answers.kept(), 2);
    }

    /// **An unreadable ceiling takes the default rather than looping.** A loop
    /// is a loop a piped stdin cannot escape, and the default works.
    #[test]
    fn an_unreadable_ceiling_falls_back_to_the_default() {
        let mut ask = crate::ask::Scripted {
            answers: vec![None, None, None, Some("lots".to_string()), None],
            ..crate::ask::Scripted::default()
        };
        let answers = interview(&mut ask, None);
        assert_eq!(answers.ceilings, None);
        assert_eq!(answers.kept(), 5);
    }
}
