//! What a long run says while it is still going.
//!
//! **Progress goes to stderr. Always.** A redraw on stdout means `armada
//! manifest check | jq` receives frames of animation, and the one consumer the
//! envelope exists for is the one that breaks (PLAN.md §3.1.1). Anything that
//! redraws, animates or reports intermediate state is stderr's; stdout carries
//! the result and nothing else.
//!
//! **And only when stderr is a terminal.** The audience for a redraw is a
//! person watching one; a captured stderr is a log file, and a log file full of
//! half-erased frames is worse than a silent one. That is why the decision is
//! made from `stderr_is_tty` rather than from the flag that decides stdout's
//! colour — `armada manifest check | jq` is a piped stdout *and* a terminal
//! stderr, and it wants both.
//!
//! **It leaves nothing behind.** [`Progress::finish`] gives back the lines it
//! drew on, so the run's real output — the envelope on stdout, a failure on
//! stderr — arrives on a clean stream.
//!
//! This file is the contract. [`super::live`] is the only thing that implements
//! it for a person, and the run that decided its shape is described there.

use armada_core::error::Status;

use super::palette::Role;

/// Which table a run is drawing.
///
/// **This is the one thing the live table and the final table must agree on,**
/// so it is a value they are both handed rather than a noun each of them spells.
/// `super::columns_for` is the only function that turns it into columns, and
/// both renders call it — which is what makes "same columns" structural instead
/// of two call sites that currently happen to match. The header drifted the
/// first time a second verb wanted a live table: the run said `CHECK` and the
/// answer said `STEP`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// `armada manifest check` — `STATUS · CHECK · DETAIL · TIME`.
    Check,
    /// `armada fleet spawn` — `STATUS · STEP · DETAIL · TIME`.
    Spawn,
}

/// One row the run is about to attempt, and the budget it has.
///
/// **The deadline travels with the name.** A row that shows only how long a
/// check has been running cannot be read: seven minutes is alarming against a
/// one-minute budget and unremarkable against fifteen, and the reader has no way
/// to tell which without opening `machine.yml`. The reported run was the second
/// case and looked like the first.
#[derive(Debug, Clone, Copy)]
pub struct Planned<'a> {
    /// `<component>:<check>` for a check, the step's name for a spawn.
    pub id: &'a str,
    /// This row's own deadline, in milliseconds — a check's `timeout:`, or the
    /// machine's `check_timeout` when it declares none.
    ///
    /// **`None` for a step nobody bounded**, which is every step of a spawn: a
    /// worktree and a `git` clone have no configured ceiling, and a row that
    /// printed `timeout 0s` would be reporting a deadline that does not exist.
    /// The budget is shown when there is one and the cell is empty when there is
    /// not, which is the same rule the final table's columns follow.
    pub timeout_ms: Option<u64>,
}

/// The word a finished row shows, and what colours it.
///
/// **Two variants because two vocabularies are already in the payload.**
/// `check` answers in envelope [`Status`]es and `spawn` answers in words that
/// are not one — `CLASSIFIED`, `CREATED`, `CLAIMED`, `STARTED`
/// (`commands/render.md`). Widening this rather than making the live table say
/// `PASS` where the final one says `CREATED` is the point: a reader who watched
/// a table redraw and is then handed different words for the same four rows has
/// to work out that they *are* the same four rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// A terminal state the envelope also spells.
    Status(Status),
    /// A word that is not a [`Status`], with the role that colours it. Upper
    /// cased by the renderer, never by the caller — the same chokepoint every
    /// other non-`Status` status cell goes through.
    Word(&'static str, Role),
}

/// The four steps `armada fleet spawn` reports, live and finally.
///
/// **One definition, because both tables name the same rows.** The live table is
/// driven by events as the spawn happens and the final one is rendered from the
/// envelope afterwards, so the two genuinely read from different sources — this
/// enum is what stops them reading different words out of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnStep {
    /// One cheap model call turns the task text into a workflow name.
    Classify,
    /// `git worktree add`, on a new branch.
    Worktree,
    /// `armada manifest init` claims the block, or finds there is none to claim.
    Ports,
    /// The Drone, started detached.
    Drone,
}

impl SpawnStep {
    /// Every step, in the order both tables list them.
    pub const ALL: [SpawnStep; 4] = [
        SpawnStep::Classify,
        SpawnStep::Worktree,
        SpawnStep::Ports,
        SpawnStep::Drone,
    ];

    /// What the `STEP` column calls this row.
    ///
    /// **Not the verb's own name for the step.** The column names the *thing*
    /// that was acted on, which is what a reader is scanning for: `workflow`,
    /// not `classify`.
    pub const fn id(self) -> &'static str {
        match self {
            SpawnStep::Classify => "workflow",
            SpawnStep::Worktree => "worktree",
            SpawnStep::Ports => "ports",
            SpawnStep::Drone => "drone",
        }
    }

    /// What the `STATUS` column says once this step is done.
    pub const fn done(self) -> &'static str {
        match self {
            SpawnStep::Classify => "classified",
            SpawnStep::Worktree => "created",
            SpawnStep::Ports => "claimed",
            SpawnStep::Drone => "started",
        }
    }
}

/// A run reporting on itself as it goes.
///
/// **Every method has a do-nothing default**, so the silent case is
/// `impl Progress for Silent {}` and adding a hook later cannot break it. The
/// alternative — a `None` checked at four call sites — is four places to forget.
pub trait Progress {
    /// Which table this is, the rows it will report on **by name**, and the
    /// monotonic reading it starts from.
    ///
    /// **The ids rather than a count**, because the live table has a row per
    /// check from the first frame. A run that could only say `0/5` is the
    /// report that ended the thing this replaced.
    fn begin(&mut self, _shape: Shape, _rows: &[Planned<'_>], _now_mono: u64) {}
    /// A row's work was started.
    fn started(&mut self, _id: &str) {}
    /// A row reached a verdict, with the one line explaining it if there is
    /// one — the same text the final table's `DETAIL` column will carry.
    fn finished(&mut self, _id: &str, _verdict: Verdict, _detail: Option<&str>) {}
    /// A turn of the scheduler's loop went by, at this monotonic reading.
    /// Redraw.
    ///
    /// **The clock is handed in rather than read.** It is injected
    /// (`ARCHITECTURE.md` §1.1), and a renderer that read the real one would be
    /// the one part of a run a test could not hold still.
    fn tick(&mut self, _now_mono: u64) {}
    /// The run is over. Leave the stream as it was found.
    fn finish(&mut self) {}
}

/// Reports nothing at all: a pipe, a `--json` run, and every test that is not
/// about progress.
pub struct Silent;

impl Progress for Silent {}

#[cfg(test)]
mod tests {
    use super::*;

    /// The silent reporter is the whole of the non-terminal path, and it writes
    /// nothing anywhere — there is no stream for it to get wrong.
    ///
    /// **Which is what the do-nothing defaults buy.** A pipe, a `--json` run
    /// and every test that is not about progress all take this path, so a hook
    /// added to the trait later cannot break any of them.
    #[test]
    fn the_silent_reporter_has_nowhere_to_write() {
        let mut silent = Silent;
        silent.begin(
            Shape::Check,
            &[Planned {
                id: "api:lint",
                timeout_ms: Some(900_000),
            }],
            0,
        );
        silent.started("api:lint");
        silent.finished("api:lint", Verdict::Status(Status::Pass), Some("exited 0"));
        silent.tick(20);
        silent.finish();
    }

    /// **The step's name and its finished word are different strings**, and
    /// both tables need both. Inverted — asserting `id()` equals `done()` —
    /// this fails on every variant, which is the point: a reader scanning the
    /// `STEP` column is looking for the thing acted on, and the `STATUS` column
    /// says what happened to it.
    #[test]
    fn a_spawn_step_names_the_thing_and_says_what_happened_to_it() {
        let named: Vec<&str> = SpawnStep::ALL.iter().map(|s| s.id()).collect();
        assert_eq!(named, ["workflow", "worktree", "ports", "drone"]);
        let done: Vec<&str> = SpawnStep::ALL.iter().map(|s| s.done()).collect();
        assert_eq!(done, ["classified", "created", "claimed", "started"]);
        for step in SpawnStep::ALL {
            assert_ne!(step.id(), step.done(), "{step:?}");
        }
    }
}
