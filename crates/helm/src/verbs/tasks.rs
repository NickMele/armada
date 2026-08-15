//! `armada task` — **a thing you mean to do, written down for nothing.**
//!
//! ## The complaint this exists to fix
//!
//! *"I really think we need the task and report feature. If I want to start
//! using this I need to be able to create tasks that can be kicked off as jobs
//! and report issues as I see them."* — and, on what a task is for: *"These
//! tasks don't need to be synced between machines … I don't wanna burn tokens on
//! it yet, and I don't want to lose track of it."*
//!
//! `docs/reserved/002-tasks.md` is the design. Working in a repository you
//! notice something and want it recorded without spending anything on it yet.
//! Today an agent writes it into a markdown file somewhere in the repository,
//! which is neither yours nor the repository's and goes stale in both
//! directions.
//!
//! ## Capture is free; execution is deliberate
//!
//! That is the whole of what makes a task different from a Job. A Job has a
//! worktree, a budget and a Drone from the moment it exists. **A task is a
//! sentence** — one file append, no model call, no classification, no
//! subprocess but the one that asks git where you are.
//!
//! | Verb | What it is |
//! |---|---|
//! | `armada task "<sentence>"` | write it down; nothing else happens |
//! | `armada tasks` | the list, navigable at a terminal |
//! | `armada tasks show <id>` | one task whole, and the prompt a Job would get |
//! | `armada tasks start <id>` | `armada fleet spawn`, with the task as its prompt |
//! | `armada tasks clear <id>` | done with it, or not doing it |
//!
//! **`task` files and `tasks` lists**, which is exactly how `report` and
//! `failures` already divide (`docs/reserved/014`). One positional sentence
//! cannot be confused with a sub-verb when the two live under different words.
//!
//! ## It is the failure store, and that is a deliberate divergence from `002`
//!
//! `002` proposed `~/.armada/tasks/<project>.yml`, keyed by project. What
//! shipped is [`armada_core::failure::Line::Written`] in
//! `~/.armada/failures.jsonl`, keyed by nothing, with the project as a column.
//!
//! **`002`'s own argument is what changed the answer.** It says a task and a
//! raised item are *"the same object from two directions"* and that they
//! *"should not be built as two lists"*; then, four paragraphs earlier, it puts
//! them in two files. Between it being written and it being built, `armada
//! report` shipped and settled the same question the other way — a report is a
//! column on the failure record, not a second store, because
//! `docs/reserved/001-raised-items-need-identity.md` says a thing needing
//! attention is useless until it has an id you can act on one at a time, and a
//! second store is a second id space, a second `show` and a second promotion
//! path.
//!
//! Every reason `002` gave for its path survives here and is satisfied better:
//!
//! | `002` wanted | Because | `~/.armada/failures.jsonl` |
//! |---|---|---|
//! | not in the repository | nothing to gitignore; no half-formed thought a teammate reads | it is under `~/.armada/` |
//! | not in `.armada/` | `clean` removes that directory; tasks would die with the workspace | it is machine state, and `clean` never touches it |
//! | one list per checkout, not per worktree | a Drone in a throwaway worktree must see what you wrote in `main` | one list per **machine**, so it does |
//! | never synced | *"what describes you syncs, what describes this machine does not"* (PLAN.md §13.1) | only `guild/` syncs, and this is not in it |
//!
//! ## The three questions `002` left open, answered
//!
//! **Does a task belong to a repository or to the machine? To the machine, with
//! the repository as a column.** He works across repositories and *"a task
//! written in one is often about another"* — a list you have to be standing in
//! the right directory to read is a list you stop reading, and the failure log
//! already proved the machine-wide shape works for rows that carry a directory.
//! What the row keeps is [`capture`]'s resolved workspace root, which is what
//! `armada tasks start` branches the Job from, so the task still knows which
//! repository it is about even though the list does not sort by it.
//!
//! **What happens to a task when the Job it became finishes? Nothing, until you
//! say so.** The promotion line puts the Job's handle on the row and the state
//! reads `FIXING`; it does not close on the Job ending, and that is a decision
//! rather than a gap. A Drone reaching the end of its workflow is not the same
//! claim as the thing being done — `docs/reserved/012` records exactly that
//! distinction, a Drone's *"done"* against a gate agreeing — and a task that
//! closed itself on the weaker of the two would quietly retire work nobody
//! checked. So the row keeps the handle, `armada fleet ls` answers whether it
//! landed, and `armada tasks clear` is the one keystroke that ends it.
//!
//! **Are tasks and failures listed together or separately? Separately, out of
//! one store.** See [`crate::verbs::failures::Lens`]: two lenses, one id space,
//! one promotion path. A flat list would mix a `bad_config` from Tuesday with
//! *"rename the port allocator"* and make both harder to find; two stores would
//! be the thing `001` forbids. `armada tasks show <id>` and `armada failures
//! show <id>` each take either id, because a person holding an id has already
//! said which row they mean.

use armada_core::ctx::{Clock, Run};
use armada_core::envelope::{Envelope, FailureData};
use armada_core::error::{ArmadaError, ErrClass, Status};
use armada_core::failure;

use crate::ask::Ask;
use crate::render::progress::Progress;
use crate::verbs::failures::{self, Lens};
use crate::verbs::Output;

pub use crate::verbs::fleet::Where;
pub use crate::verbs::guild::Look;

/// `armada task "<what to do>"` — write it down, and do nothing else.
///
/// **The sentence is required and there is no interactive fallback**, for
/// `armada report`'s reason: the default would be an empty row, which costs a
/// reader a click and tells them nothing, and a person who typed the verb has
/// the sentence in their head already.
///
/// **The one thing it looks up is where it is** — [`project`], one `git` call,
/// and [`workspace`], one config walk.
pub fn capture<R: Run, C: Clock>(
    runner: &R,
    now: &C,
    place: &Where,
    what: &str,
    argv: &[String],
) -> Result<Output, ArmadaError> {
    capture_text(
        now,
        place,
        what,
        argv,
        &project(runner, &place.cwd),
        workspace(runner, &place.cwd).as_deref(),
    )
}

/// Capture, with the repository already decided.
///
/// **Split from [`capture`] so that a caller who is not standing anywhere can
/// still write a task.** `armada untried` is the one: *"try `armada guild
/// edit`"* is not about the directory it was written in, and paying a `git`
/// call to record one would be a subprocess for a fact nobody will read. That
/// caller has no [`Run`] to spend on [`workspace`] either, so it passes
/// `None` — the same answer a real directory with no `armada.yml` gets.
pub fn capture_text<C: Clock>(
    now: &C,
    place: &Where,
    what: &str,
    argv: &[String],
    project: &std::path::Path,
    workspace: Option<&std::path::Path>,
) -> Result<Output, ArmadaError> {
    let what = what.trim();
    if what.is_empty() {
        return Err(empty());
    }

    let (id, line) = failure::written(
        what,
        &place.home,
        project,
        workspace,
        argv,
        &now.wall_rfc3339(),
        now.wall_ms(),
        &|text| crate::redact::scrub(text),
    );

    let path = armada_manifest::failures::path(&place.armada_home);
    // **Unlike recording a failure, this one is allowed to complain.** The
    // recorder is a side effect of a run that is already failing and must never
    // change an exit code; this is the whole of what the caller asked for, and
    // a task that silently did not land is worse than one that refused.
    if !armada_manifest::failures::append(&path, &line) {
        return Err(unwritable(&path));
    }

    // **Read back rather than constructed**, exactly as `armada report` does:
    // the entry this answers with is the one the fold produces from the line
    // just written, so `armada task` and `armada tasks show <id>` cannot
    // disagree about what was captured.
    let mut entries = armada_manifest::failures::read(&path)?;
    failure::age(&mut entries, now.wall_ms());
    let entry = entries
        .into_iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| unwritable(&path))?;
    let task = failure::task(&entry);

    Ok(Output::Failure(Box::new(Envelope::ok(
        "task",
        None,
        Status::Ok,
        FailureData {
            results: vec![entry],
            task,
        },
    ))))
}

/// **The checkout this task belongs to** — the repository, not the worktree and
/// not the subdirectory.
///
/// `docs/reserved/002-tasks.md` asks for tasks keyed by *project* rather than by
/// workspace, and gives the reason: *"a Drone in a throwaway worktree sees the
/// same tasks the checkout you wrote them in does."* That is PLAN.md §2.2's
/// project identity, and git already answers it — `--git-common-dir` is
/// `<checkout>/.git` from every linked worktree and every subdirectory alike,
/// so its parent is the one directory every view of a repository agrees on.
///
/// **Which also puts a task written by an agent where a person can act on it.**
/// A worktree under `.claude/worktrees/` is deleted when the work that made it
/// ends — [`armada_core::failure::scratch`] is why failures from one are not
/// recorded at all — and a task recording that path would name a directory that
/// is gone before anybody reads the row.
///
/// **One subprocess, and every failure of it is survivable.** Not a repository,
/// a worktree whose checkout was deleted, no `git` on `PATH`: each ends at the
/// working directory, which is a worse answer than the checkout and a much
/// better one than a refusal. Capture is the cheapest verb in Armada and it
/// does not get a precondition.
fn project(runner: &impl Run, cwd: &std::path::Path) -> std::path::PathBuf {
    armada_manifest::git::common_dir(runner, cwd)
        .filter(|dir| dir.file_name().is_some_and(|name| name == ".git"))
        .and_then(|dir| dir.parent().map(std::path::Path::to_path_buf))
        .filter(|root| root.is_dir())
        .or_else(|| armada_manifest::git::root(runner, cwd))
        .unwrap_or_else(|| cwd.to_path_buf())
}

/// **The Manifest workspace `cwd` is inside, if any** — a finer unit than
/// [`project`]'s repository. A monorepo is one git repository and may declare
/// several workspaces (`workspaces: […]` in the root `armada.yml`), each its
/// own `armada.yml`; a task written in `storefront/web` and one written in
/// `storefront/backend` share [`project`]'s answer and are told apart only by
/// this one.
///
/// **Reused rather than reimplemented.** `armada_manifest::discovery::resolve`
/// is the same walk `armada manifest status` resolves cwd against — no second
/// resolver, per the constraint that keeps the two from ever disagreeing about
/// what a workspace is.
///
/// **`None` is the honest answer for a directory with no `armada.yml`.** A
/// candidate that merely resolves its own dependencies is not a workspace
/// until something claims it with a config file — the same line the scanner
/// draws — so a missing config leaves this empty rather than guessing at the
/// nearest one that does not own `cwd`. Every failure of the walk (no git, no
/// config up to the root) is exactly the case `discovery::resolve` already
/// treats as "no workspace here", and none of them fails capture: writing a
/// task down cannot cost more than [`project`] already does.
fn workspace(runner: &impl Run, cwd: &std::path::Path) -> Option<std::path::PathBuf> {
    armada_manifest::discovery::resolve(runner, cwd)
        .ok()
        .map(|found| found.root)
}

/// `armada tasks` — the list, and at a terminal a way through it.
///
/// **A wrapper and nothing more.** Every line of the listing, the navigating
/// and the acting is [`crate::verbs::failures`]'s, because there is one store
/// and a second implementation of *"draw the rows, pick one, do something to
/// it"* would drift within a release. What this adds is [`Lens::Tasks`] and the
/// scope.
///
/// **Scoped to this repository by default, and `--all` is the one flag that
/// widens it** — the same word `armada manifest status`'s scope lens already
/// spends on *wider than my own narrow default* (`crates/core/src/scope.rs`),
/// reused rather than paired with a second flag nobody would remember to
/// combine with it. Without `--all` the listing is also every cleared task
/// gone, exactly as it always was — one flag, and it removes every filter this
/// verb has rather than one axis of them.
///
/// **A task written outside any repository is not lost.** It has no project to
/// be scoped out of — [`project`] falls back to the bare working directory —
/// so it shows by default only from that same directory, and `--all` always
/// reaches it, the same as every other task.
#[allow(clippy::too_many_arguments)]
pub fn ls<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    all: bool,
    ask: &mut dyn Ask,
    interactive: bool,
    look: Look,
    progress: &mut dyn Progress,
) -> Result<Output, ArmadaError> {
    let scope = (!all).then(|| scope(run, place));
    failures::ls(
        run,
        now,
        place,
        all,
        ask,
        interactive,
        look,
        progress,
        Lens::Tasks,
        scope.as_deref(),
    )
}

/// The value a task written from here, right now, would carry in its `cwd`
/// column — computed without writing anything, so the listing can filter on
/// exactly what a row already holds.
///
/// **Bit for bit the same transform [`capture`] applies**: [`project`], then
/// the same redaction and the same tilde. Computed any other way, a checkout
/// that captured a task under one spelling of its path could fail to find it
/// again under another.
fn scope<R: Run>(runner: &R, place: &Where) -> String {
    failure::tilde(
        &crate::redact::scrub(&project(runner, &place.cwd).display().to_string()),
        &place.home,
    )
}

/// `armada tasks show <id>` — one task, whole, and the prompt a Job would get.
pub fn show<C: Clock>(now: &C, place: &Where, id: &str) -> Result<Output, ArmadaError> {
    failures::show(now, place, id)
}

/// `armada tasks start <id>` — the Job, with the task as its prompt.
///
/// **`start`, not `fix`.** The mechanism is `failures fix`'s to the line, and
/// the word is not: you fix something broken and you start something that was
/// never begun, and offering `fix` on a task would read as Armada thinking the
/// thought was wrong (`docs/glossary.md` fixes the vocabulary and this is what
/// obeying it costs — one name, no second implementation).
#[allow(clippy::too_many_arguments)]
pub fn start<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    id: &str,
    dry_run: bool,
    workflow: Option<&str>,
    ask: Option<&mut dyn Ask>,
    progress: &mut dyn Progress,
) -> Result<Output, ArmadaError> {
    failures::fix(run, now, place, id, dry_run, workflow, ask, progress)
}

/// `armada tasks clear <id>`, or `armada tasks clear --all`.
pub fn clear<C: Clock>(
    now: &C,
    place: &Where,
    id: Option<&str>,
    all: bool,
) -> Result<Output, ArmadaError> {
    failures::clear(now, place, id, all, Lens::Tasks)
}

fn empty() -> ArmadaError {
    ArmadaError {
        class: ErrClass::BadInvocation,
        r#where: "task".to_string(),
        message: "`armada task` needs a sentence saying what to do".to_string(),
        next_action: Some("armada task \"rename the port allocator\"".to_string()),
    }
}

fn unwritable(path: &std::path::Path) -> ArmadaError {
    ArmadaError {
        class: ErrClass::Environment,
        r#where: path.display().to_string(),
        message: "the task could not be written".to_string(),
        next_action: Some("check ~/.armada/ is writable, then retry unchanged".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sentence that is nothing but whitespace is the same as none, and the
    /// refusal shows the line that would have worked rather than describing it.
    #[test]
    fn a_blank_task_is_refused_with_the_line_that_would_have_worked() {
        let error = empty();
        assert_eq!(error.class, ErrClass::BadInvocation);
        assert!(
            error.next_action.unwrap().starts_with("armada task \""),
            "the refusal shows the shape"
        );
    }
}
