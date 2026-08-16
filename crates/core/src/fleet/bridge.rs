//! The Bridge's pure half: what one frame holds, what `--filter` accepts, where
//! the cursor is, and what a keypress means (`commands/helm/bridge.md`).
//!
//! **The Bridge holds no state, and this is the whole of what it does hold.** A
//! cursor position and a filter expression — both of which are questions about
//! what you are *looking at*, never about what the fleet *is*. Closing the
//! screen loses a cursor position, which is the definition of losing nothing.
//!
//! **It is a renderer over Fleet and there is no second source.** A frame is
//! built from the [`FleetLsData`] `armada fleet ls` already produces
//! (`commands/fleet/ls.md`), so a frame and a listing cannot disagree about what
//! a Job is doing. Nothing here reads a transcript, asks the process table or
//! probes a Drone: watching must not change what is watched (PLAN.md §15.2).
//!
//! **The keys are decided here and pressed somewhere else.** [`Key`] is the
//! Bridge's own small vocabulary rather than a terminal library's, which is what
//! lets every binding below be a unit test instead of somebody pressing a
//! button. `crates/helm/src/bridge/` maps a `crossterm` event onto it and does
//! nothing else with it — the same split `ask/select.rs` uses, one level lower
//! so that the decisions land in the pure crate.

use crate::envelope::{FleetLsData, JobRow};
use crate::error::{ArmadaError, ErrClass};
use crate::fleet::JobState;

/// Which part of a Job an expression is asking about.
///
/// **A named field or anywhere, and nothing in between.** A grammar with
/// booleans in it is a query language, and the question the Bridge is filtering
/// is always "show me fewer rows" — which one field, or one word, answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    /// A bare word: the name, the workflow, the state or the task.
    Anywhere,
    /// `job=`.
    Job,
    /// `workflow=`.
    Workflow,
    /// `state=`.
    State,
    /// `task=`.
    Task,
    /// `needs=you`.
    Needs,
}

/// The keys `--filter` knows, and the one place they are spelled.
const FIELDS: [(&str, Field); 5] = [
    ("job", Field::Job),
    ("workflow", Field::Workflow),
    ("state", Field::State),
    ("task", Field::Task),
    ("needs", Field::Needs),
];

/// A parsed `--filter`.
///
/// **Case-insensitive and substring, both deliberately.** `state=running`
/// matching `RUNNING` is what somebody typing at a live screen will do, and a
/// filter that demanded the screaming form would be a filter nobody uses. The
/// exception is `needs=you`, which is a boolean and is matched exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    field: Field,
    needle: String,
    /// What the caller typed, so the screen can show the filter it is under.
    source: String,
}

impl Filter {
    /// The expression as it was typed.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Whether this row survives.
    pub fn matches(&self, row: &JobRow) -> bool {
        let has = |text: &str| text.to_ascii_lowercase().contains(&self.needle);
        match self.field {
            Field::Job => has(&row.name),
            Field::Workflow => has(&row.workflow),
            Field::State => has(row.state.word()),
            Field::Task => has(&row.task),
            // The only boolean, and the only one that is not a substring.
            Field::Needs => row.needs_attention,
            Field::Anywhere => {
                has(&row.name) || has(&row.workflow) || has(row.state.word()) || has(&row.task)
            }
        }
    }
}

/// Read an expression, or say why it is not one.
///
/// **`bad_invocation`, which is exit 2** (`commands/helm/bridge.md`). The two
/// ways to get it are a key nothing knows and a state word that is not a state:
/// both are typos that would otherwise silently show an empty screen, and an
/// empty screen is indistinguishable from an idle fleet.
///
/// An empty expression is **not** an error — it is no filter at all, which is
/// what pressing `/` and then `enter` on an empty box means.
pub fn parse_filter(expr: &str) -> Result<Option<Filter>, ArmadaError> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Ok(None);
    }

    let (field, value) = match expr.split_once('=') {
        None => (Field::Anywhere, expr),
        Some((key, value)) => {
            let key = key.trim().to_ascii_lowercase();
            let field = FIELDS
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, field)| *field)
                .ok_or_else(|| unknown_key(&key))?;
            (field, value.trim())
        }
    };

    if value.is_empty() {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: expr.to_string(),
            message: format!("`{expr}` has a key and nothing to match against it"),
            next_action: Some(format!("write a value: `--filter {expr}running`")),
        });
    }

    // **A state that is not a state is refused rather than matched against
    // nothing.** `state=runing` is the typo this catches, and its symptom
    // without the check is a screen that looks like an empty fleet.
    if field == Field::State
        && !JobState::ALL
            .iter()
            .any(|state| state.word() == value.to_ascii_uppercase())
    {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: expr.to_string(),
            message: format!("`{value}` is not a Job state"),
            next_action: Some(format!(
                "one of: {}",
                JobState::ALL
                    .iter()
                    .map(|state| state.word())
                    .collect::<Vec<_>>()
                    .join(" ")
            )),
        });
    }

    Ok(Some(Filter {
        field,
        needle: value.to_ascii_lowercase(),
        source: expr.to_string(),
    }))
}

fn unknown_key(key: &str) -> ArmadaError {
    ArmadaError {
        class: ErrClass::BadInvocation,
        r#where: key.to_string(),
        message: format!("`{key}=` is not something a Job has"),
        next_action: Some(format!(
            "one of: {}",
            FIELDS
                .iter()
                .map(|(name, _)| format!("{name}="))
                .collect::<Vec<_>>()
                .join(" ")
        )),
    }
}

/// One redraw's worth of fleet, filtered and counted.
///
/// **Counted over what is shown, not over what exists.** A filtered screen
/// saying `5 jobs` above three rows is a screen that is lying about the two
/// questions it is there to answer; `hidden` is how the rest is accounted for.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// The rows to draw, in the order `ls` produced them.
    pub rows: Vec<JobRow>,
    /// How many of the shown rows are running.
    pub running: usize,
    /// How many of the shown rows are waiting on you.
    pub needs_you: usize,
    /// What the shown rows have cost between them.
    pub spent_usd: f64,
    /// How many rows the filter removed.
    pub hidden: usize,
    /// The filter this frame was built under, as it was typed.
    pub filter: Option<String>,
    /// The rate-limit window the fleet is working inside.
    ///
    /// **Not filtered with the rows.** Every other number on this struct counts
    /// what is on the screen; this one is a fact about the account, and a filter
    /// is a question about which Jobs to look at rather than about how much of a
    /// five-hour window is gone.
    pub window: Option<crate::envelope::Window>,
}

/// Build one frame.
///
/// **Nothing is estimated and nothing is derived** (PHASES.md §9.1 F2): every
/// number here is a sum or a count of fields Claude Code already emitted, which
/// is also why there is no progress column to build.
pub fn frame(data: &FleetLsData, filter: Option<&Filter>) -> Frame {
    let rows: Vec<JobRow> = data
        .results
        .iter()
        .filter(|row| filter.is_none_or(|filter| filter.matches(row)))
        .cloned()
        .collect();
    Frame {
        running: rows
            .iter()
            .filter(|row| row.state == JobState::Running)
            .count(),
        needs_you: rows.iter().filter(|row| row.needs_attention).count(),
        spent_usd: rows.iter().map(|row| row.cost_usd).sum(),
        hidden: data.results.len() - rows.len(),
        filter: filter.map(|filter| filter.source.clone()),
        window: data.window,
        rows,
    }
}

/// Which row the cursor is on.
///
/// **Zero-based, wrapping, and clamped against a list that changed underneath
/// it.** The fleet is re-read every interval, so the row count moves while the
/// cursor sits still — a Job that finished between two frames must not leave the
/// cursor pointing past the end.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Cursor {
    at: usize,
}

impl Cursor {
    /// Where it is.
    pub const fn at(self) -> usize {
        self.at
    }

    /// Whether this is the row under the cursor.
    pub const fn is_on(self, index: usize) -> bool {
        self.at == index
    }

    /// Down one, wrapping. Nothing at all when there are no rows.
    pub fn next(&mut self, rows: usize) {
        if rows == 0 {
            self.at = 0;
            return;
        }
        self.at = (self.at + 1) % rows;
    }

    /// Up one, wrapping.
    pub fn previous(&mut self, rows: usize) {
        if rows == 0 {
            self.at = 0;
            return;
        }
        self.at = (self.at + rows - 1) % rows;
    }

    /// Bring it back inside a list that shrank.
    pub fn clamp(&mut self, rows: usize) {
        self.at = self.at.min(rows.saturating_sub(1));
    }

    /// The row it is on, if there is one.
    pub fn selected(self, rows: &[JobRow]) -> Option<&JobRow> {
        rows.get(self.at)
    }
}

/// The Bridge's key vocabulary.
///
/// **Its own, rather than a terminal library's.** Core imports nothing of a
/// backend's, and the practical gain is that every binding below is asserted by
/// a test rather than by somebody pressing a button at a terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// `↑`.
    Up,
    /// `↓`.
    Down,
    /// `↵`.
    Enter,
    /// `esc`.
    Esc,
    /// `backspace`.
    Backspace,
    /// Any printable character.
    Char(char),
    /// `ctrl-c`.
    Interrupt,
    /// `ctrl-d` — **the text area's submit, and only a text area's**.
    ///
    /// **The same chord the interview uses, because a second convention would
    /// be worse than either** (`docs/commands/render.md`). Every prose answer in
    /// Armada is finished with `ctrl-d`: `enter` has to stay a newline in a box
    /// that holds paragraphs, so something else has to mean *done*, and this is
    /// the one that already does everywhere else.
    ///
    /// Outside [`Mode::Composing`] it means nothing and is ignored, which is
    /// what it meant before this key existed.
    Save,
}

/// Which Job a key acted on.
///
/// **The name and the uuid together, because they answer different questions.**
/// The name is what a notice says back to the person who pressed the key; the
/// uuid is what the verb is given. A name is a *handle* rather than a key —
/// [`crate::fleet::job::Job`] records are kept after a Job ends and the name
/// stays on them, so a finished `rate-limit` and a running one are both
/// ordinary and both on disk. Dispatching on the name alone is how `x` on the
/// running row reached the one that ended last week, reported success, and left
/// the Job on the screen exactly as it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The handle, for saying which Job this was about.
    pub job: String,
    /// The session id, for saying which Job to a verb.
    pub uuid: String,
}

impl Target {
    /// The Job under the cursor, as something a verb can be given.
    pub fn of(row: &JobRow) -> Target {
        Target {
            job: row.name.clone(),
            uuid: row.uuid.clone(),
        }
    }

    /// The uuid as a handle a person can retype: the first eight characters.
    ///
    /// **Because a name is not enough to tell two rows apart.** Two Jobs may
    /// share one, and on a screen that is about to delete things the row has to
    /// carry the thing that is actually unique.
    pub fn short(&self) -> &str {
        match self.uuid.char_indices().nth(8) {
            Some((at, _)) => &self.uuid[..at],
            None => &self.uuid,
        }
    }
}

/// One row of the reap preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReapRow {
    /// Which Job.
    pub target: Target,
    /// What it is really doing, observed rather than recorded.
    pub state: JobState,
    /// Whether this row is ticked.
    pub selected: bool,
    /// What it is holding, already worded — the port block, the worktree.
    ///
    /// **Rendered by the shell that read it and carried as text**, because the
    /// preview's whole job is to put the cost of *not* reaping beside the cost
    /// of reaping, and what a Job holds is a question about the machine rather
    /// than about the screen.
    pub holding: String,
}

/// The reap preview: every candidate, and which of them are ticked.
///
/// **Mandatory, and the feature rather than a confirmation bolted on.** A bulk
/// delete that only listed names would ask a person to approve a decision on
/// less information than the machine already has.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reap {
    /// The candidates, in the order the plan produced them.
    pub rows: Vec<ReapRow>,
    /// Which row the cursor is on **inside the preview**, which is a different
    /// question from which Job the fleet table is on.
    pub cursor: Cursor,
}

impl Reap {
    /// Every ticked row, as targets.
    pub fn selected(&self) -> Vec<Target> {
        self.rows
            .iter()
            .filter(|row| row.selected)
            .map(|row| row.target.clone())
            .collect()
    }
}

/// What the screen is doing between keypresses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Mode {
    /// The ordinary state: a table and a cursor.
    #[default]
    Watching,
    /// `/` is open and this is what has been typed into it.
    Filtering(String),
    /// `n` was pressed and this is the task being written for the new Job.
    ///
    /// **A mode rather than a [`Departure`], for the reason [`Mode::Detail`] is
    /// one.** `n` used to end the screen and ask for the task in the shell it
    /// had just given back, and what that produced was *"it took me to a screen
    /// that felt like it took me out of the bridge"* — the fleet gone, a bare
    /// question, and no way back that anybody could see. The box is drawn
    /// **under the table rather than over it**, which is where it differs from
    /// the detail pane and the reap preview: those two answer a question *about*
    /// a row, so they take the whole screen; this one is a thing you do while
    /// watching, and watching is what the rows are for.
    ///
    /// **The keys are the interview's** — `enter` for a newline, `ctrl-d` to
    /// start it, `esc` to throw it away — and they are named on the screen, so
    /// nobody has to guess `ctrl-d` the way the first reader did.
    Composing(String),
    /// `d` was pressed and this Job's detail view is open, covering the table.
    ///
    /// **A mode rather than a [`Departure`], because the Bridge is live.** Every
    /// other key that shows you something leaves the screen and runs a verb in
    /// the terminal it gives back; this one answers *why does it need me*, which
    /// is the question you are already at the Bridge to ask, and leaving to read
    /// the answer would mean coming back to a screen that had moved on. What is
    /// drawn is `armada fleet show <job>`'s own payload — the same verb, on the
    /// same read, so the Bridge still adds no capability
    /// (`commands/helm/bridge.md`).
    Detail(String),
    /// `x` was pressed on this Job and it is waiting for a `y`.
    ///
    /// **Abort is the one key that asks twice.** Every other key on the Bridge
    /// is reversible or reaches a verb that asks for itself; `x` ends a Job,
    /// deletes its worktree and drops its branch, and a single keypress that
    /// does that on whatever row the cursor happened to be on is the mistake
    /// worth one extra character to avoid.
    Confirming(Target),
    /// `r` was pressed and this is what a reap would take.
    ///
    /// **`esc` leaves everything untouched**, which is what makes the preview
    /// safe to open out of curiosity — and being safe to open is what makes it
    /// get read.
    Reaping(Reap),
    /// `?` was pressed: every binding, including the ones the key line could
    /// not carry.
    ///
    /// **The honest answer to a line that is one line.** Nine key/word pairs is
    /// eighty-two columns against a budget of seventy-eight, so a key line
    /// either wraps — which changes the frame's height, and the tests forbid
    /// that — or stops listing everything. This is what "stops listing
    /// everything" is allowed to cost: one keystroke, and the rest are there.
    ///
    /// **Any key closes it**, because it is a thing you glanced at rather than
    /// a mode you work in.
    Keys,
}

/// Leaving the screen, and what to do on the way out.
///
/// **Every one of these is a verb reachable from a shell** — that is what keeps
/// the Bridge a rendering choice rather than an architectural one
/// (`commands/helm/bridge.md`). The screen does not perform any of them; it
/// stops, and says which one.
///
/// **These are the four that genuinely cannot happen on the screen**: quitting
/// is the deliberate exit, boarding replaces this process with `claude`, and
/// `n` and `a` need a text box the alternate screen cannot hold. Everything
/// else a key does is an [`Action`] and happens without giving the screen back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Departure {
    /// `q` — nothing further.
    Quit,
    /// `↵` — `armada fleet board <job>`.
    Board(Target),
    /// `ctrl-d` in the compose box — `armada fleet spawn "<task>"`.
    ///
    /// **The task is already in hand, which is the whole of the change.** The
    /// Bridge used to leave on `n` and ask for the task in a shell it had just
    /// handed back, so the one keystroke that starts work began by taking the
    /// fleet off the screen. Now it is typed over the table
    /// ([`Mode::Composing`]) and the screen is given back only for the spawn
    /// itself — which needs the terminal for its progress table and for the
    /// workflow question a low-confidence guess puts (PLAN.md §3.1.1) — and
    /// taken again the moment that is over.
    Spawn(String),
    /// `a` — `armada fleet answer <job>`.
    Answer(Target),
}

/// Something a key does **without giving the screen back**.
///
/// **This is the class fix, and it matters more than any one key.** Every one
/// of these used to end the Bridge, so a `kill` that could not remove a
/// worktree took away the view of the other four Jobs in order to say something
/// about one — and what it said landed in a shell the reader was no longer
/// looking at. An action that fails is a line under the table.
///
/// Each still names a verb reachable from a shell, which is the rule that keeps
/// the Bridge a rendering choice: what changed is *where the answer is
/// printed*, not what runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// `x`, confirmed — `armada fleet kill <job>`.
    Abort(Target),
    /// `p` on a Job that is running — `armada fleet pause <job>`.
    Pause(Target),
    /// `p` on a Job that is paused — `armada fleet resume <job>`.
    Resume(Target),
    /// `r` — read `armada fleet reap --dry-run` and open the preview over it.
    Preview,
    /// The preview, confirmed — `armada fleet reap` over exactly these.
    Reap(Vec<Target>),
    /// `c` — `armada helm`, which is not built and says so here rather than by
    /// ending the screen to report it.
    Chat,
    /// `↵` on a machine with a [`Workspace`] — open the Job's worktree beside
    /// the screen, and **keep drawing** (`020` §9).
    ///
    /// **An [`Action`] and not a [`Departure`], which is the whole decision.**
    /// [`Departure::Board`] exists because `--exec` *replaces this process*: from
    /// the `exec` on, the tty and the exit code belong to `claude`, so there is
    /// nothing left to draw with. Handing a path to a multiplexer starts
    /// something else and returns, so the fleet never leaves the screen — which
    /// is what `020` §9 asked for in the words *"the menu and the panes stay
    /// up"*.
    Open(Target),
}

/// What carrying out an [`Action`] produced.
///
/// **A line and, at most, a mode.** The shell performs the action and hands
/// back what to say; the screen decides nothing about it, which keeps the whole
/// of the Bridge's key handling a unit test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Done {
    /// One line under the table, cleared by the next keypress.
    pub notice: String,
    /// A mode to switch into — the reap preview, and nothing else so far.
    pub mode: Option<Mode>,
}

impl Done {
    /// Said, and nothing else changed.
    pub fn said(notice: impl Into<String>) -> Done {
        Done {
            notice: notice.into(),
            mode: None,
        }
    }
}

/// What one keypress did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pressed {
    /// Redraw and carry on.
    Stay,
    /// Carry this out, put the answer under the table, and carry on drawing.
    Act(Action),
    /// Stop drawing, and do this off the screen.
    Leave(Departure),
}

/// Everything the Bridge remembers between two frames.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Screen {
    /// Which row.
    pub cursor: Cursor,
    /// The filter in force, if any.
    pub filter: Option<Filter>,
    /// What the screen is doing.
    pub mode: Mode,
    /// One line under the table, cleared by the next keypress.
    ///
    /// **The Bridge answers a key it cannot carry out rather than swallowing
    /// it.** A `p` that did nothing at all is indistinguishable from a dead
    /// keyboard.
    pub notice: Option<String>,
    /// Where a Job can be opened on this machine
    /// (`docs/reserved/020-the-tui-decided.md` §9).
    ///
    /// **A fact the shell measures once and hands in**, because the core owns no
    /// `PATH`. It is set when the Bridge starts and not re-probed per keypress:
    /// a multiplexer does not appear halfway through a session, and a probe on
    /// the hot path would put a subprocess behind `↵`.
    pub workspace: Workspace,
}

/// What can open a Job's worktree beside the screen
/// (`docs/reserved/020-the-tui-decided.md` §9).
///
/// **Armada does not own a terminal, and this does not change that**
/// (PHASES.md §9.1 F1). `↵` used to end the Bridge and hand back a path and a
/// resume command to paste; where something on this machine can open a
/// directory, the same keypress asks *it* to, and the fleet stays on the screen.
/// Nothing is attached to and no pty is taken.
///
/// **Why not a chat pane instead.** `020` §9 weighed one and refused it: the
/// transport is not the problem — Armada already drives Claude Code as a two-way
/// `stream-json` channel, because that is how every Drone runs — but rendering a
/// live conversation is a terminal chat client, with streaming, tool calls,
/// scrollback, resize and interrupt. Claude Code is already an excellent one,
/// and rebuilding it to avoid one screen handoff is owning every rough edge to
/// save a keypress.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Workspace {
    /// Nothing found. `↵` prints the path and the resume command, which is what
    /// `armada fleet board` has always done — **the fallback is the old
    /// behaviour exactly**, so a machine without a multiplexer loses nothing.
    #[default]
    None,
    /// `cmux` is on `PATH`. `cmux <path>` opens a directory in a new workspace,
    /// which is the whole of the handoff.
    Cmux,
}

/// The program `020` §9 hands a Job to.
pub const CMUX: &str = "cmux";

/// What is run to find out whether [`Workspace::Cmux`] is available.
///
/// **`--help`, because it is free and it proves the thing that matters.** A bare
/// `which` proves a file exists; this proves the *form Armada is about to use*
/// is one the program offers — the rule `armada doctor` already follows for the
/// Drone, held against `claude --help` on every run. `cmux --help` starts
/// nothing, so probing is not a side effect.
pub fn cmux_probe_argv() -> Vec<String> {
    vec![CMUX.to_string(), "--help".to_string()]
}

/// The argv that opens a worktree beside the screen.
///
/// **A path and nothing else**, which is the whole of the handoff: `cmux <path>`
/// opens a directory in a new workspace and launches cmux if it is not already
/// up. Armada does not attach, does not take a pty, and does not tell the
/// workspace what to run — a Job's conversation is an ordinary resumable Claude
/// Code session in a git worktree, and opening the worktree is the whole of what
/// Armada knows that cmux does not (`commands/fleet/board.md`).
pub fn cmux_open_argv(worktree: &str) -> Vec<String> {
    vec![CMUX.to_string(), worktree.to_string()]
}

/// Whether that help text offers the form [`cmux_open_argv`] builds.
///
/// **It fails closed, and that is the point of checking rather than assuming.**
/// A `cmux` whose CLI has moved on answers `false`, `↵` goes back to printing
/// the path and the resume command, and the reader loses a convenience rather
/// than pressing a key that silently does nothing — which is the failure
/// `Workspace::None` is the honest name for.
pub fn cmux_offers_open(help: &str) -> bool {
    help.contains("cmux <path>")
}

/// Apply one keypress. **The whole of the Bridge's key handling, and it touches
/// nothing**: `rows` is the frame that is currently drawn and everything else is
/// plain data, so every binding is a unit test.
pub fn press(screen: &mut Screen, rows: &[JobRow], key: Key) -> Pressed {
    // A notice describes the last keypress, so it goes as soon as there is
    // another one.
    screen.notice = None;

    match std::mem::take(&mut screen.mode) {
        Mode::Filtering(typed) => filtering(screen, typed, key),
        Mode::Composing(typed) => composing(screen, typed, key),
        Mode::Confirming(target) => confirming(screen, target, key),
        Mode::Detail(job) => detail(screen, job, rows, key),
        Mode::Reaping(reap) => reaping(screen, reap, key),
        // **Any key closes it.** It is a thing you glanced at rather than a
        // mode you work in, and a page that took a second deliberate keystroke
        // to leave would be one nobody opens twice.
        Mode::Keys => Pressed::Stay,
        Mode::Watching => watching(screen, rows, key),
    }
}

fn watching(screen: &mut Screen, rows: &[JobRow], key: Key) -> Pressed {
    let selected = screen.cursor.selected(rows).map(Target::of);
    match key {
        Key::Up | Key::Char('k') => screen.cursor.previous(rows.len()),
        Key::Down | Key::Char('j') => screen.cursor.next(rows.len()),

        // **`esc` clears the filter before it quits.** A filtered screen is the
        // one place `esc` has an obvious smaller meaning, and a person who
        // narrowed to one Job and wants everything back should not have to
        // leave to get it.
        Key::Esc if screen.filter.is_some() => {
            screen.filter = None;
            screen.notice = Some("filter cleared".to_string());
        }
        Key::Esc | Key::Interrupt | Key::Char('q') => return Pressed::Leave(Departure::Quit),

        Key::Char('/') => {
            let existing = screen
                .filter
                .as_ref()
                .map(|filter| filter.source.clone())
                .unwrap_or_default();
            screen.mode = Mode::Filtering(existing);
        }

        // **One key, two destinations, decided by what the machine has**
        // (`020` §9). Where something can open a directory the Job is opened
        // beside the screen and the fleet stays up; where nothing can, this is
        // the `armada fleet board` handoff it has always been. The key is the
        // same either way, because *step aboard this Job* is the same intent —
        // only the thing that carries it out differs.
        Key::Enter => match (selected, screen.workspace) {
            (Some(target), Workspace::Cmux) => return Pressed::Act(Action::Open(target)),
            (Some(target), Workspace::None) => return Pressed::Leave(Departure::Board(target)),
            (None, _) => screen.notice = Some(nothing_selected("board")),
        },
        // **`n` opens the box here rather than ending the screen.** The task is
        // the one thing the frame cannot know, and asking for it used to cost
        // the whole view of the fleet.
        Key::Char('n') => screen.mode = Mode::Composing(String::new()),
        // **`c` answers on the screen rather than by ending it.** `armada helm`
        // is not built, and tearing the frame down to say so took away the view
        // of a whole fleet in order to report one unimplemented key.
        Key::Char('c') => return Pressed::Act(Action::Chat),

        Key::Char('a') => match screen.cursor.selected(rows) {
            // **Only a Job with something open can be answered.** `armada fleet
            // answer` refuses one that has nothing waiting, and finding that out
            // by leaving the screen and coming back is a worse way to be told.
            Some(row) if row.needs_attention => {
                return Pressed::Leave(Departure::Answer(Target::of(row)))
            }
            Some(row) => {
                screen.notice = Some(format!("`{}` has nothing open to answer", row.name));
            }
            None => screen.notice = Some(nothing_selected("answer")),
        },

        // **`d` for the detail view**, the key `k9s` and `htop` already spend on
        // describing the row under the cursor — and the Bridge is modelled on
        // both (PLAN.md §15.1). It sits **second on the key line**, behind
        // `enter` only: it is the one key that answers `NEEDS YOU`, which is the
        // one column that is ever a call to action. It was unnamed for as long
        // as the line could only wrap or lie; now the line drops by priority and
        // says `? keys` when it has, so naming `d` costs the two pairs at the
        // end of the order rather than the frame's height.
        Key::Char('d') => match selected {
            Some(target) => screen.mode = Mode::Detail(target.job),
            None => screen.notice = Some(nothing_selected("show")),
        },

        Key::Char('x') => match selected {
            Some(target) => {
                screen.notice = Some(format!("abort `{}`? press y", target.job));
                screen.mode = Mode::Confirming(target);
            }
            None => screen.notice = Some(nothing_selected("abort")),
        },

        // **`p` reads the row it is on.** One key, two verbs, decided by what
        // the selected Job is doing — which is the same thing the key line says
        // it will do, because both ask [`pause_key`].
        Key::Char('p') => match screen.cursor.selected(rows) {
            Some(row) => {
                let target = Target::of(row);
                return Pressed::Act(match row.state {
                    JobState::Paused => Action::Resume(target),
                    _ => Action::Pause(target),
                });
            }
            None => screen.notice = Some(nothing_selected("pause")),
        },

        // **`r` opens the preview and reaps nothing.** Reading what would be
        // taken is the whole feature, so the key that starts it must be safe to
        // press out of curiosity.
        Key::Char('r') => return Pressed::Act(Action::Preview),

        // **`?` is what a one-line key line costs.** The line drops its lowest
        // -priority pairs rather than wrapping, because wrapping would change
        // the frame's height; this is where the ones it dropped are.
        Key::Char('?') => screen.mode = Mode::Keys,

        // `ctrl-d` belongs to the box and there is no box open.
        Key::Save | Key::Backspace | Key::Char(_) => {}
    }
    Pressed::Stay
}

/// The compose box is open. **The interview's keys, in the Bridge's own
/// screen.**
///
/// `enter` is a newline because a task is prose and a box that submitted on
/// `enter` could not hold a second paragraph; `ctrl-d` is therefore what means
/// *done*, which is what it means in every other text area Armada draws
/// (`ask/editor.rs`); `esc` throws the draft away.
fn composing(screen: &mut Screen, mut typed: String, key: Key) -> Pressed {
    match key {
        Key::Char(c) => typed.push(c),
        Key::Enter => typed.push('\n'),
        Key::Backspace => {
            typed.pop();
        }

        // **An empty box is a keypress that meant something else, not a Job
        // with no task.** Saying so keeps the box open rather than starting a
        // Drone on nothing or closing on a `ctrl-d` that looked ignored.
        Key::Save => {
            if typed.trim().is_empty() {
                screen.notice = Some("nothing typed — esc closes the box".to_string());
                screen.mode = Mode::Composing(typed);
                return Pressed::Stay;
            }
            return Pressed::Leave(Departure::Spawn(typed.trim().to_string()));
        }

        // **`esc` closes it and says the draft went**, because a box that
        // vanished silently is one a reader assumes started something.
        Key::Esc | Key::Interrupt => {
            screen.notice = Some("no Job was started".to_string());
            return Pressed::Stay;
        }

        // The cursor belongs to the table underneath, and the table is still on
        // the screen — but moving it while typing would put the caret in two
        // places, so the arrows do nothing here.
        Key::Up | Key::Down => {}
    }
    screen.mode = Mode::Composing(typed);
    Pressed::Stay
}

/// The preview's own keys. **Nothing here touches the fleet** — it moves a
/// cursor over rows the shell already read, ticks them, and either confirms or
/// throws the whole list away.
fn reaping(screen: &mut Screen, mut reap: Reap, key: Key) -> Pressed {
    let rows = reap.rows.len();
    match key {
        Key::Up | Key::Char('k') => reap.cursor.previous(rows),
        Key::Down | Key::Char('j') => reap.cursor.next(rows),

        // Space toggles, because it is the one key a list of checkboxes has
        // meant everywhere for thirty years.
        Key::Char(' ') => {
            if let Some(row) = reap.rows.get_mut(reap.cursor.at()) {
                row.selected = !row.selected;
            }
        }

        Key::Enter => {
            let taken = reap.selected();
            if taken.is_empty() {
                // **Confirming an empty selection is not a reap of nothing, it
                // is a keypress that meant something else.** Saying so keeps
                // the preview open, with the rows still ticked as they were.
                screen.notice = Some("nothing is ticked — space ticks a row".to_string());
                screen.mode = Mode::Reaping(reap);
                return Pressed::Stay;
            }
            return Pressed::Act(Action::Reap(taken));
        }

        // `ctrl-d` belongs to the compose box, which cannot be open from here.
        Key::Save => {}

        // **`esc` leaves everything untouched**, which is what makes the
        // preview safe to open — and being safe to open is what makes it read.
        Key::Esc | Key::Interrupt => {
            screen.notice = Some("nothing was reaped".to_string());
            return Pressed::Stay;
        }

        Key::Backspace | Key::Char(_) => {}
    }
    screen.mode = Mode::Reaping(reap);
    Pressed::Stay
}

fn filtering(screen: &mut Screen, mut typed: String, key: Key) -> Pressed {
    match key {
        Key::Char(c) => {
            typed.push(c);
            screen.mode = Mode::Filtering(typed);
        }
        Key::Backspace => {
            typed.pop();
            screen.mode = Mode::Filtering(typed);
        }
        // **Cancelling puts back what was there**, which is why the box opens
        // holding the filter in force rather than empty.
        Key::Esc | Key::Interrupt => {}
        Key::Enter => match parse_filter(&typed) {
            Ok(filter) => {
                screen.filter = filter;
                // The rows are about to change under it.
                screen.cursor = Cursor::default();
            }
            // **A bad expression keeps the box open.** Closing it would throw
            // away what was typed and leave the reader to retype it in order to
            // find the typo.
            Err(error) => {
                screen.notice = Some(error.message);
                screen.mode = Mode::Filtering(typed);
            }
        },
        // `ctrl-d` belongs to the compose box; the filter is one line and
        // `enter` already commits it.
        Key::Save | Key::Up | Key::Down => screen.mode = Mode::Filtering(typed),
    }
    Pressed::Stay
}

/// The detail view is open. **It is a reader, so almost every key closes it.**
///
/// A pane you have to remember the exit key for is a pane people stop opening.
/// `esc`, `d` again and `q` all go back, and only `ctrl-c` leaves the Bridge —
/// `q` closing the whole screen from in here would quit an application when the
/// reader meant to close a page of it.
fn detail(screen: &mut Screen, job: String, rows: &[JobRow], key: Key) -> Pressed {
    let _ = job;
    match key {
        Key::Interrupt => Pressed::Leave(Departure::Quit),
        // **Moving the cursor moves the pane with it**, so a fleet is read one
        // Job at a time rather than closed and reopened between each.
        Key::Up | Key::Char('k') | Key::Down | Key::Char('j') => {
            match key {
                Key::Up | Key::Char('k') => screen.cursor.previous(rows.len()),
                _ => screen.cursor.next(rows.len()),
            }
            // A fleet that emptied under an open pane has nothing left to
            // describe, so the screen goes back to saying so.
            screen.mode = match screen.cursor.selected(rows) {
                Some(row) => Mode::Detail(row.name.clone()),
                None => Mode::Watching,
            };
            Pressed::Stay
        }
        _ => Pressed::Stay,
    }
}

fn confirming(screen: &mut Screen, target: Target, key: Key) -> Pressed {
    match key {
        Key::Char('y') | Key::Char('Y') => Pressed::Act(Action::Abort(target)),
        // **Anything else is a no.** A confirmation that only one key can
        // decline is a confirmation that gets answered by accident.
        _ => {
            screen.notice = Some(format!("`{}` was not aborted", target.job));
            Pressed::Stay
        }
    }
}

fn nothing_selected(verb: &str) -> String {
    format!("no Job to {verb}")
}

/// What `p` does to the Job under the cursor, as the word the key line prints.
///
/// **One function, asked by the binding and by the renderer**, which is the
/// whole of the fix: a key line that always said `pause` over a `PAUSED` Job
/// advertised the one thing that key would not do, and left the reader with no
/// way at all to start it again. Two lists would have drifted the first time a
/// state was added.
pub const fn pause_key(selected: Option<JobState>) -> &'static str {
    match selected {
        Some(JobState::Paused) => "resume",
        // Everything else — including nothing selected — is `pause`, because
        // that is what the key does to a Job that is not held.
        _ => "pause",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fleet::job::Remaining;

    fn row(name: &str, state: JobState, needs: bool) -> JobRow {
        JobRow {
            uuid: format!("{name}-uuid"),
            name: name.to_string(),
            workflow: "feature".to_string(),
            state,
            detail: "implement".to_string(),
            step: "implement".to_string(),
            on_step_s: Some(720),
            task: format!("do the {name} thing"),
            runtime_s: 840,
            cost_usd: 2.10,
            tokens: 1_000,
            turns: 3,
            budget_remaining: Remaining {
                iterations: 9,
                cost_usd: 1.0,
                wall_clock_ms: 1,
            },
            needs_attention: needs,
            acting: None,
            acting_for_s: None,
        }
    }

    fn fleet() -> FleetLsData {
        let results = vec![
            row("rate-limit", JobState::Running, false),
            row("xlsx-report", JobState::Stalled, false),
            row("release-merge", JobState::Blocked, true),
        ];
        FleetLsData {
            needs_you: 1,
            spent_usd: 6.30,
            results,
            window: None,
        }
    }

    fn watching_at(rows: usize) -> (Screen, Vec<JobRow>) {
        let all = fleet().results;
        (Screen::default(), all[..rows].to_vec())
    }

    // ------------------------------------------------------------- the filter

    /// **A bare word looks everywhere**, because a person at a live screen types
    /// the thing they remember rather than the field it is in.
    #[test]
    fn a_bare_word_matches_the_name_the_workflow_the_state_or_the_task() {
        let data = fleet();
        for (word, expected) in [
            ("rate", vec!["rate-limit"]),
            (
                "feature",
                vec!["rate-limit", "xlsx-report", "release-merge"],
            ),
            ("BLOCKED", vec!["release-merge"]),
            // The task is one of the four fields, so a phrase out of it
            // matches — and a phrase that is in none of them matches nothing.
            ("the xlsx-report thing", vec!["xlsx-report"]),
            ("nothing is called this", vec![]),
        ] {
            let filter = parse_filter(word).expect("parses");
            let frame = frame(&data, filter.as_ref());
            let names: Vec<&str> = frame.rows.iter().map(|row| row.name.as_str()).collect();
            assert_eq!(names, expected, "--filter {word}");
        }
    }

    /// Case is not part of the question. `state=running` is what somebody types.
    #[test]
    fn a_filter_ignores_case_on_both_sides() {
        let data = fleet();
        for expr in ["state=running", "state=RUNNING", "STATE=Running"] {
            let filter = parse_filter(expr).expect("parses");
            let frame = frame(&data, filter.as_ref());
            assert_eq!(frame.rows.len(), 1, "{expr}");
            assert_eq!(frame.rows[0].name, "rate-limit");
        }
    }

    /// `needs=you` is the one boolean, and it is the column that is ever a call
    /// to action.
    #[test]
    fn needs_you_selects_the_jobs_waiting_on_a_person() {
        let filter = parse_filter("needs=you").expect("parses");
        let frame = frame(&fleet(), filter.as_ref());
        assert_eq!(frame.rows.len(), 1);
        assert!(frame.rows[0].needs_attention);
    }

    /// **A typo is refused rather than shown as an empty fleet**, which is the
    /// one failure mode a filter has: an empty screen and an idle fleet look
    /// exactly alike.
    #[test]
    fn an_unparseable_filter_is_a_bad_invocation() {
        for expr in ["colour=blue", "state=runing", "state=", "job="] {
            let error = parse_filter(expr).expect_err(expr);
            assert_eq!(error.class, ErrClass::BadInvocation, "{expr}");
            assert!(
                error.next_action.is_some(),
                "{expr} does not say what to do"
            );
        }
    }

    /// An empty expression is no filter, not a bad one — `/` then `enter`.
    #[test]
    fn an_empty_expression_is_no_filter_at_all() {
        assert_eq!(parse_filter("").expect("parses"), None);
        assert_eq!(parse_filter("   ").expect("parses"), None);
    }

    // -------------------------------------------------------------- the frame

    /// **Counted over what is shown.** A filtered screen reporting the whole
    /// fleet's totals is lying about the two questions it exists to answer.
    #[test]
    fn a_frame_counts_the_rows_it_draws_and_says_how_many_it_hid() {
        let data = fleet();
        let whole = frame(&data, None);
        assert_eq!(whole.rows.len(), 3);
        assert_eq!(whole.running, 1);
        assert_eq!(whole.needs_you, 1);
        assert_eq!(whole.hidden, 0);
        assert!(whole.filter.is_none());

        let filter = parse_filter("needs=you").expect("parses");
        let narrow = frame(&data, filter.as_ref());
        assert_eq!(narrow.rows.len(), 1);
        assert_eq!(narrow.running, 0);
        assert_eq!(narrow.needs_you, 1);
        assert_eq!(narrow.hidden, 2);
        assert!((narrow.spent_usd - 2.10).abs() < 1e-9);
        assert_eq!(narrow.filter.as_deref(), Some("needs=you"));
    }

    /// **Reading a frame changes nothing about the fleet.** The Bridge is a
    /// renderer, so building one twice from the same listing gives the same
    /// answer and the listing is untouched.
    #[test]
    fn building_a_frame_leaves_the_listing_alone() {
        let data = fleet();
        let before = data.clone();
        let once = frame(&data, None);
        let twice = frame(&data, None);
        assert_eq!(once, twice);
        assert_eq!(data, before);
    }

    // ------------------------------------------------------------- the cursor

    /// Both spellings of every direction, and a list of three is a ring.
    #[test]
    fn arrows_and_jk_both_move_and_both_wrap() {
        for (down, up) in [(Key::Down, Key::Up), (Key::Char('j'), Key::Char('k'))] {
            let (mut screen, rows) = watching_at(3);
            press(&mut screen, &rows, down);
            assert_eq!(screen.cursor.at(), 1);
            press(&mut screen, &rows, up);
            assert_eq!(screen.cursor.at(), 0);
            press(&mut screen, &rows, up);
            assert_eq!(screen.cursor.at(), 2, "{up:?} did not wrap");
            press(&mut screen, &rows, down);
            assert_eq!(screen.cursor.at(), 0, "{down:?} did not wrap");
        }
    }

    /// **The fleet changes under the cursor.** A Job that finished between two
    /// frames must not leave it pointing past the end.
    #[test]
    fn a_cursor_past_the_end_is_brought_back_inside() {
        let mut cursor = Cursor::default();
        cursor.next(5);
        cursor.next(5);
        assert_eq!(cursor.at(), 2);
        cursor.clamp(1);
        assert_eq!(cursor.at(), 0);
        cursor.clamp(0);
        assert_eq!(cursor.at(), 0);
    }

    /// An empty fleet has no cursor to move and no row to act on.
    #[test]
    fn an_empty_fleet_moves_nowhere_and_acts_on_nothing() {
        let mut screen = Screen::default();
        assert_eq!(press(&mut screen, &[], Key::Down), Pressed::Stay);
        assert_eq!(screen.cursor.at(), 0);
        assert_eq!(press(&mut screen, &[], Key::Enter), Pressed::Stay);
        assert_eq!(screen.notice.as_deref(), Some("no Job to board"));
    }

    // ---------------------------------------------------------------- the keys

    fn target(name: &str) -> Target {
        Target {
            job: name.to_string(),
            uuid: format!("{name}-uuid"),
        }
    }

    /// **Every key that leaves names a verb reachable from a shell**, which is
    /// what keeps the Bridge a rendering choice rather than an architectural
    /// one.
    ///
    /// **And `↵` boards.** It has to be asserted by name, because the failure it
    /// is written against was a keypress arriving at the wrong verb entirely:
    /// `enter` answering with "`armada helm` is not built yet" — an error whose
    /// own next-action line names the command that should have run.
    #[test]
    fn each_key_leaves_for_the_verb_the_page_gives_it() {
        let (_, rows) = watching_at(3);
        for (key, expected) in [
            (Key::Enter, Departure::Board(target("rate-limit"))),
            (Key::Char('q'), Departure::Quit),
            (Key::Interrupt, Departure::Quit),
            (Key::Esc, Departure::Quit),
        ] {
            let mut screen = Screen::default();
            assert_eq!(
                press(&mut screen, &rows, key),
                Pressed::Leave(expected),
                "{key:?}"
            );
        }
    }

    /// `↵` opens the Job **beside** the screen where something can open it, and
    /// hands the screen back where nothing can
    /// (`docs/reserved/020-the-tui-decided.md` §9).
    ///
    /// **Both halves, because the fallback is the feature.** The point of
    /// detecting a workspace is that a machine without one loses nothing: `↵` is
    /// the `armada fleet board` handoff it has always been. A test that only
    /// asserted the cmux side would pass over a build where the fallback had
    /// become a keypress that did nothing.
    #[test]
    fn enter_opens_a_workspace_where_there_is_one_and_boards_where_there_is_not() {
        let (_, rows) = watching_at(3);

        let mut with = Screen {
            workspace: Workspace::Cmux,
            ..Screen::default()
        };
        assert_eq!(
            press(&mut with, &rows, Key::Enter),
            Pressed::Act(Action::Open(target("rate-limit"))),
            "an action, not a departure — `020` §9 keeps the panes up"
        );

        let mut without = Screen::default();
        assert_eq!(
            without.workspace,
            Workspace::None,
            "nothing found is the default, so a fresh Screen keeps the old behaviour"
        );
        assert_eq!(
            press(&mut without, &rows, Key::Enter),
            Pressed::Leave(Departure::Board(target("rate-limit"))),
            "the fallback stopped being `armada fleet board`"
        );
    }

    /// An empty fleet answers `↵` the same way whichever workspace is there:
    /// there is no row to open.
    #[test]
    fn enter_over_nothing_says_so_rather_than_opening_a_workspace() {
        for workspace in [Workspace::None, Workspace::Cmux] {
            let mut screen = Screen {
                workspace,
                ..Screen::default()
            };
            assert_eq!(press(&mut screen, &[], Key::Enter), Pressed::Stay);
            assert!(screen.notice.is_some(), "{workspace:?} swallowed the key");
        }
    }

    /// The argv Armada hands cmux, and the help text that proves the form is
    /// offered.
    ///
    /// **The help text below is a measurement, not a guess** — it is what
    /// `cmux --help` printed, and `docs/traps.md` records where it came from.
    /// Holding the argv against the program's own help is the rule `armada
    /// doctor` follows for the Drone; the difference here is that no test may
    /// require cmux to be installed, so the measurement is carried rather than
    /// re-taken.
    #[test]
    fn the_cmux_argv_is_a_bare_path_and_the_probe_asks_its_help() {
        assert_eq!(cmux_probe_argv(), vec!["cmux", "--help"]);
        assert_eq!(
            cmux_open_argv("~/.armada/workspaces/api/rate-limit"),
            vec!["cmux", "~/.armada/workspaces/api/rate-limit"],
            "a path and nothing else — Armada does not tell the workspace what to run"
        );

        let measured = "\
Usage:
  cmux <path>                Open a directory in a new workspace (launches cmux if needed)
  cmux [global-options] <command> [options]
";
        assert!(cmux_offers_open(measured));
        // **Fails closed.** A cmux whose CLI has moved on sends `↵` back to
        // printing the path, which is a lost convenience rather than a key that
        // silently does nothing.
        assert!(!cmux_offers_open("Usage:\n  cmux open <path>\n"));
        assert!(!cmux_offers_open(""));
    }

    /// **`n` opens the box instead of ending the screen**, which is the second
    /// fix: asking for a task used to cost the whole view of the fleet, and what
    /// the first reader reported was a screen that *"felt like it took me out of
    /// the bridge"*.
    #[test]
    fn n_opens_the_compose_box_and_leaves_the_fleet_on_the_screen() {
        let (mut screen, rows) = watching_at(3);
        assert_eq!(press(&mut screen, &rows, Key::Char('n')), Pressed::Stay);
        assert_eq!(screen.mode, Mode::Composing(String::new()));
    }

    /// **The box takes the interview's keys, and `ctrl-d` is the one that
    /// starts it.** `enter` has to stay a newline in a box that holds a
    /// paragraph, so something else means *done* — and it is the chord every
    /// other text area in Armada already uses.
    #[test]
    fn the_compose_box_types_newlines_on_enter_and_spawns_on_ctrl_d() {
        let (mut screen, rows) = watching_at(3);
        press(&mut screen, &rows, Key::Char('n'));
        for key in [
            Key::Char('f'),
            Key::Char('i'),
            Key::Char('x'),
            Key::Char('!'),
            Key::Backspace,
            Key::Enter,
            Key::Char('i'),
            Key::Char('t'),
        ] {
            assert_eq!(press(&mut screen, &rows, key), Pressed::Stay, "{key:?}");
        }
        assert_eq!(screen.mode, Mode::Composing("fix\nit".to_string()));
        assert_eq!(
            press(&mut screen, &rows, Key::Save),
            Pressed::Leave(Departure::Spawn("fix\nit".to_string()))
        );
    }

    /// **An empty box is a keypress that meant something else.** Starting a
    /// Drone on nothing is worse than saying so, and closing on a `ctrl-d` that
    /// looked ignored is worse than both.
    #[test]
    fn ctrl_d_on_an_empty_box_says_so_and_keeps_it_open() {
        let (mut screen, rows) = watching_at(3);
        press(&mut screen, &rows, Key::Char('n'));
        press(&mut screen, &rows, Key::Char(' '));
        assert_eq!(press(&mut screen, &rows, Key::Save), Pressed::Stay);
        assert_eq!(screen.mode, Mode::Composing(" ".to_string()));
        assert_eq!(
            screen.notice.as_deref(),
            Some("nothing typed — esc closes the box")
        );
    }

    /// **`esc` throws the draft away and says it did**, because a box that
    /// vanished silently is one a reader assumes started something.
    #[test]
    fn esc_closes_the_compose_box_without_starting_anything() {
        let (mut screen, rows) = watching_at(3);
        press(&mut screen, &rows, Key::Char('n'));
        press(&mut screen, &rows, Key::Char('x'));
        assert_eq!(press(&mut screen, &rows, Key::Esc), Pressed::Stay);
        assert_eq!(screen.mode, Mode::Watching);
        assert_eq!(screen.notice.as_deref(), Some("no Job was started"));
    }

    /// **`ctrl-d` means nothing where there is no box.** It is the text area's
    /// key, carried through the terminal layer for the one mode that answers it.
    #[test]
    fn ctrl_d_is_ignored_by_every_mode_but_the_box() {
        let (mut screen, rows) = watching_at(3);
        assert_eq!(press(&mut screen, &rows, Key::Save), Pressed::Stay);
        assert_eq!(screen.mode, Mode::Watching);
        assert_eq!(screen.notice, None);

        screen.mode = Mode::Filtering("state=run".to_string());
        assert_eq!(press(&mut screen, &rows, Key::Save), Pressed::Stay);
        assert_eq!(screen.mode, Mode::Filtering("state=run".to_string()));
    }

    /// **A key carries the uuid as well as the name.** A name is a handle rather
    /// than a key — a finished `rate-limit` and a running one are both on disk —
    /// so a keypress dispatched on the name alone can reach a Job the cursor was
    /// never on.
    #[test]
    fn every_key_that_acts_names_the_job_by_uuid_and_not_only_by_name() {
        let (mut screen, rows) = watching_at(3);
        let Pressed::Leave(Departure::Board(boarded)) = press(&mut screen, &rows, Key::Enter)
        else {
            panic!("enter did not board");
        };
        assert_eq!(boarded.uuid, "rate-limit-uuid");
        assert_eq!(boarded.job, "rate-limit");
    }

    /// **`c` answers on the screen.** `armada helm` is not built; ending the
    /// frame to say so took away the view of a whole fleet to report one key.
    #[test]
    fn chat_is_answered_without_ending_the_screen() {
        let (mut screen, rows) = watching_at(3);
        assert_eq!(
            press(&mut screen, &rows, Key::Char('c')),
            Pressed::Act(Action::Chat)
        );
    }

    /// **`a` only leaves for a Job with something open.** `armada fleet answer`
    /// refuses one that has nothing waiting, and learning that by leaving the
    /// screen and coming back is a worse way to be told.
    #[test]
    fn answer_leaves_only_for_a_job_that_is_waiting_on_you() {
        let (mut screen, rows) = watching_at(3);
        assert_eq!(press(&mut screen, &rows, Key::Char('a')), Pressed::Stay);
        assert_eq!(
            screen.notice.as_deref(),
            Some("`rate-limit` has nothing open to answer")
        );

        screen.cursor.next(3);
        screen.cursor.next(3);
        assert_eq!(
            press(&mut screen, &rows, Key::Char('a')),
            Pressed::Leave(Departure::Answer(target("release-merge")))
        );
    }

    /// **Abort asks twice, and anything but `y` is a no.** One keypress that
    /// ends a Job, deletes its worktree and drops its branch — on whatever row
    /// the cursor happened to be on — is the mistake worth one character.
    ///
    /// **And it acts rather than leaving.** An abort that ended the screen
    /// reported its failure to a shell the reader was no longer looking at.
    #[test]
    fn abort_takes_two_keys_and_declines_on_anything_but_y() {
        let (mut screen, rows) = watching_at(3);
        assert_eq!(press(&mut screen, &rows, Key::Char('x')), Pressed::Stay);
        assert_eq!(screen.mode, Mode::Confirming(target("rate-limit")));
        assert_eq!(
            press(&mut screen, &rows, Key::Char('y')),
            Pressed::Act(Action::Abort(target("rate-limit")))
        );

        for declined in [Key::Char('n'), Key::Esc, Key::Enter, Key::Char('x')] {
            let (mut screen, rows) = watching_at(3);
            press(&mut screen, &rows, Key::Char('x'));
            assert_eq!(
                press(&mut screen, &rows, declined),
                Pressed::Stay,
                "{declined:?}"
            );
            assert_eq!(screen.mode, Mode::Watching);
            assert_eq!(
                screen.notice.as_deref(),
                Some("`rate-limit` was not aborted")
            );
        }
    }

    /// **`p` reads the row it is on.** A key line that always said `pause` over
    /// a `PAUSED` Job advertised the one thing that key would not do, and left
    /// the reader with no way at all to start it again.
    #[test]
    fn pause_reads_the_selected_rows_state_and_the_key_line_says_the_same_word() {
        let running = row("rate-limit", JobState::Running, false);
        let held = row("xlsx-report", JobState::Paused, false);
        let rows = vec![running, held];

        let mut screen = Screen::default();
        assert_eq!(
            press(&mut screen, &rows, Key::Char('p')),
            Pressed::Act(Action::Pause(target("rate-limit")))
        );
        assert_eq!(pause_key(Some(JobState::Running)), "pause");

        screen.cursor.next(2);
        assert_eq!(
            press(&mut screen, &rows, Key::Char('p')),
            Pressed::Act(Action::Resume(target("xlsx-report")))
        );
        assert_eq!(pause_key(Some(JobState::Paused)), "resume");
    }

    /// An empty fleet has no row for `p` to read, and says so rather than
    /// pausing something.
    #[test]
    fn pause_on_an_empty_fleet_says_there_is_no_job() {
        let mut screen = Screen::default();
        assert_eq!(press(&mut screen, &[], Key::Char('p')), Pressed::Stay);
        assert_eq!(screen.notice.as_deref(), Some("no Job to pause"));
        assert_eq!(pause_key(None), "pause");
    }

    // --------------------------------------------------------------- the reap

    fn reap_row(name: &str, state: JobState, selected: bool) -> ReapRow {
        ReapRow {
            target: target(name),
            state,
            selected,
            holding: "5470-5479 · worktree".to_string(),
        }
    }

    fn previewing() -> Screen {
        Screen {
            mode: Mode::Reaping(Reap {
                rows: vec![
                    reap_row("rate-limit", JobState::Done, true),
                    reap_row("xlsx-report", JobState::Paused, false),
                    reap_row("release-merge", JobState::Aborted, true),
                ],
                cursor: Cursor::default(),
            }),
            ..Screen::default()
        }
    }

    /// **`r` opens the preview and reaps nothing.** Reading what would be taken
    /// is the whole feature, so the key that starts it must be safe to press out
    /// of curiosity.
    #[test]
    fn r_opens_the_preview_rather_than_reaping() {
        let (mut screen, rows) = watching_at(3);
        assert_eq!(
            press(&mut screen, &rows, Key::Char('r')),
            Pressed::Act(Action::Preview)
        );
    }

    /// Rows toggle, and `enter` confirms exactly what is ticked.
    #[test]
    fn the_preview_toggles_rows_and_confirms_only_what_is_ticked() {
        let mut screen = previewing();
        // Tick the second row, which is the `PAUSED` one nothing takes by
        // default.
        press(&mut screen, &[], Key::Down);
        press(&mut screen, &[], Key::Char(' '));
        // And untick the first, which is ticked by default.
        press(&mut screen, &[], Key::Up);
        press(&mut screen, &[], Key::Char(' '));

        assert_eq!(
            press(&mut screen, &[], Key::Enter),
            Pressed::Act(Action::Reap(vec![
                target("xlsx-report"),
                target("release-merge"),
            ]))
        );
    }

    /// **`esc` leaves everything untouched**, which is what makes the preview
    /// safe to open — and being safe to open is what makes it read.
    #[test]
    fn esc_cancels_the_preview_and_reaps_nothing() {
        let mut screen = previewing();
        assert_eq!(press(&mut screen, &[], Key::Esc), Pressed::Stay);
        assert_eq!(screen.mode, Mode::Watching);
        assert_eq!(screen.notice.as_deref(), Some("nothing was reaped"));
    }

    /// Confirming an empty selection is a keypress that meant something else,
    /// not a reap of nothing — so the preview stays open and says so.
    #[test]
    fn confirming_with_nothing_ticked_keeps_the_preview_open() {
        let mut screen = Screen {
            mode: Mode::Reaping(Reap {
                rows: vec![reap_row("xlsx-report", JobState::Paused, false)],
                cursor: Cursor::default(),
            }),
            ..Screen::default()
        };
        assert_eq!(press(&mut screen, &[], Key::Enter), Pressed::Stay);
        assert!(matches!(screen.mode, Mode::Reaping(_)), "{:?}", screen.mode);
        assert_eq!(
            screen.notice.as_deref(),
            Some("nothing is ticked — space ticks a row")
        );
    }

    /// **A state you might still act on is not garbage.** `PAUSED` means
    /// needs-you and `STALLED` is a Job somebody may want to start again, so
    /// both are listed and neither is taken; a `RUNNING` Job is not offered at
    /// all, because the observation only produces that word while its process
    /// group is provably still Armada's.
    #[test]
    fn a_reap_takes_the_finished_offers_the_arguable_and_never_the_running() {
        use crate::fleet::Reaping;
        for (state, expected) in [
            (JobState::Done, Reaping::Taken),
            (JobState::Aborted, Reaping::Taken),
            (JobState::Stalled, Reaping::Offered),
            (JobState::Blocked, Reaping::Offered),
            (JobState::Paused, Reaping::Offered),
            (JobState::Running, Reaping::Never),
            (JobState::Queued, Reaping::Never),
        ] {
            assert_eq!(state.reaping(), expected, "{}", state.word());
        }
        assert!(JobState::Paused.reaping().is_offered());
        assert!(!JobState::Paused.reaping().is_default());
        assert!(!JobState::Running.reaping().is_offered());
    }

    /// `/` opens holding what is in force, takes characters, and commits on
    /// enter.
    #[test]
    fn the_filter_box_types_and_commits() {
        let (mut screen, rows) = watching_at(3);
        press(&mut screen, &rows, Key::Char('/'));
        assert_eq!(screen.mode, Mode::Filtering(String::new()));
        for c in "xlsx".chars() {
            press(&mut screen, &rows, Key::Char(c));
        }
        assert_eq!(screen.mode, Mode::Filtering("xlsx".to_string()));
        press(&mut screen, &rows, Key::Backspace);
        assert_eq!(screen.mode, Mode::Filtering("xls".to_string()));
        press(&mut screen, &rows, Key::Enter);
        assert_eq!(screen.mode, Mode::Watching);
        assert_eq!(screen.filter.as_ref().map(Filter::source), Some("xls"));

        // It reopens holding what is in force, so narrowing is an edit.
        press(&mut screen, &rows, Key::Char('/'));
        assert_eq!(screen.mode, Mode::Filtering("xls".to_string()));
    }

    /// **A bad expression keeps the box open.** Closing it would throw away what
    /// was typed and leave the reader to retype it to find the typo.
    #[test]
    fn a_refused_expression_stays_in_the_box_with_the_reason_under_it() {
        let (mut screen, rows) = watching_at(3);
        press(&mut screen, &rows, Key::Char('/'));
        for c in "state=runing".chars() {
            press(&mut screen, &rows, Key::Char(c));
        }
        press(&mut screen, &rows, Key::Enter);
        assert_eq!(screen.mode, Mode::Filtering("state=runing".to_string()));
        assert!(screen.filter.is_none());
        assert_eq!(
            screen.notice.as_deref(),
            Some("`runing` is not a Job state")
        );
    }

    /// `esc` in the box puts back what was there; `esc` on a filtered screen
    /// clears it; `esc` on an unfiltered one leaves.
    #[test]
    fn esc_backs_out_one_step_at_a_time() {
        let (mut screen, rows) = watching_at(3);
        press(&mut screen, &rows, Key::Char('/'));
        press(&mut screen, &rows, Key::Char('x'));
        press(&mut screen, &rows, Key::Esc);
        assert_eq!(screen.mode, Mode::Watching);
        assert!(screen.filter.is_none(), "esc committed the box");

        press(&mut screen, &rows, Key::Char('/'));
        press(&mut screen, &rows, Key::Char('x'));
        press(&mut screen, &rows, Key::Enter);
        assert!(screen.filter.is_some());
        assert_eq!(press(&mut screen, &rows, Key::Esc), Pressed::Stay);
        assert!(screen.filter.is_none(), "esc did not clear the filter");
        assert_eq!(
            press(&mut screen, &rows, Key::Esc),
            Pressed::Leave(Departure::Quit)
        );
    }

    /// A notice describes the last keypress, so the next one clears it.
    #[test]
    fn a_notice_lasts_exactly_one_keypress() {
        let (mut screen, rows) = watching_at(3);
        press(&mut screen, &rows, Key::Char('a'));
        assert!(screen.notice.is_some());
        press(&mut screen, &rows, Key::Down);
        assert!(screen.notice.is_none());
    }

    /// Committing a filter puts the cursor back at the top, because the rows
    /// under it are a different set.
    #[test]
    fn committing_a_filter_returns_the_cursor_to_the_first_row() {
        let (mut screen, rows) = watching_at(3);
        press(&mut screen, &rows, Key::Down);
        assert_eq!(screen.cursor.at(), 1);
        press(&mut screen, &rows, Key::Char('/'));
        press(&mut screen, &rows, Key::Char('x'));
        press(&mut screen, &rows, Key::Enter);
        assert_eq!(screen.cursor.at(), 0);
    }

    // ------------------------------------------------------------- the detail

    /// **`d` opens the row under the cursor and stays on the screen.** The
    /// question it answers is one you are already at the Bridge to ask, so
    /// answering it must not cost the screen you asked it from.
    #[test]
    fn d_opens_the_selected_jobs_detail_without_leaving() {
        let (mut screen, rows) = watching_at(3);
        press(&mut screen, &rows, Key::Down);
        press(&mut screen, &rows, Key::Down);
        assert_eq!(
            press(&mut screen, &rows, Key::Char('d')),
            Pressed::Stay,
            "the Bridge left"
        );
        assert_eq!(screen.mode, Mode::Detail("release-merge".to_string()));
    }

    /// An empty fleet has no row to describe, and says so rather than opening a
    /// pane about nothing.
    #[test]
    fn d_on_an_empty_fleet_says_there_is_nothing_to_show() {
        let mut screen = Screen::default();
        press(&mut screen, &[], Key::Char('d'));
        assert_eq!(screen.mode, Mode::Watching);
        assert_eq!(screen.notice.as_deref(), Some("no Job to show"));
    }

    /// **Every ordinary key closes it, and `q` closes the pane rather than the
    /// Bridge.** A reader who opened a page of an application and pressed `q`
    /// meant to close the page.
    #[test]
    fn any_ordinary_key_closes_the_detail_and_q_does_not_quit() {
        for key in [Key::Esc, Key::Char('d'), Key::Char('q'), Key::Enter] {
            let (mut screen, rows) = watching_at(3);
            press(&mut screen, &rows, Key::Char('d'));
            assert_eq!(
                press(&mut screen, &rows, key),
                Pressed::Stay,
                "{key:?} left the Bridge"
            );
            assert_eq!(screen.mode, Mode::Watching, "{key:?} did not close it");
        }
    }

    /// `ctrl-c` still leaves from anywhere. A pane that trapped it would be the
    /// one place in Armada where the interrupt did not work.
    #[test]
    fn ctrl_c_leaves_from_inside_the_detail() {
        let (mut screen, rows) = watching_at(3);
        press(&mut screen, &rows, Key::Char('d'));
        assert_eq!(
            press(&mut screen, &rows, Key::Interrupt),
            Pressed::Leave(Departure::Quit)
        );
    }

    /// **The pane follows the cursor**, so a fleet is read one Job at a time
    /// rather than closed and reopened between each.
    #[test]
    fn the_arrows_move_the_detail_to_the_next_job() {
        let (mut screen, rows) = watching_at(3);
        press(&mut screen, &rows, Key::Char('d'));
        press(&mut screen, &rows, Key::Down);
        assert_eq!(screen.cursor.at(), 1);
        assert_eq!(screen.mode, Mode::Detail("xlsx-report".to_string()));

        press(&mut screen, &rows, Key::Char('k'));
        assert_eq!(screen.cursor.at(), 0);
        assert_eq!(screen.mode, Mode::Detail("rate-limit".to_string()));
    }
}
