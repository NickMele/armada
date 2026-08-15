//! The human renderer.
//!
//! **The renderer is the only thing that differs between human and agent
//! output** (`ARCHITECTURE.md` §1.6). Both read the same envelope; this one
//! flattens it into lines, and `--json` emits it whole.
//!
//! Nothing here decides anything. If a rule is being applied — which state is a
//! failure, which resources were skipped and why — it was decided upstream and
//! this file is reading a field.
//!
//! # The agreed layout
//!
//! Every table below is `STATUS · NAME · DETAIL · TIME`, in that order, drawn by
//! the one renderer in [`table`]. The shape is agreed in
//! `docs/reference-output/command-output.html` and frozen by the fixtures in
//! `tests/golden/render/`; **the fixture is the specification and this file
//! follows it**, so a change here that a fixture does not agree with is a
//! failure rather than an improvement.
//!
//! **Status is first and always a word.** Not a tick, not a cross, not a
//! coloured bullet: a symbol that only appears at a terminal means the two human
//! audiences have different shapes, and one shape is the entire point
//! (PLAN.md §3.1.1). Colour agrees with the word and never replaces it.
//!
//! **A status column has one spelling, and it is SCREAMING.** `PASS`, `FAILED`,
//! `REAPED`, `CLAIMED`, `OWNS`. That is PLAN.md §3.1's rule stated once and
//! applied everywhere — one spelling, in the payload and in the human render —
//! and it now holds for the four enums that were serialising lowercase
//! (`Health`, `Sync`, `Disposition`, the inbox's `kind`) as well as for the ones
//! that already screamed.
//!
//! **This replaced a rule where the case carried a meaning**, and it is worth
//! recording what was given up. `lowercase` used to mark a render-only word —
//! something the envelope states structurally rather than as a status — so a
//! reader could tell at a glance which words they could have grepped out of
//! `--json`. It was a real distinction, and it cost more than it bought: the
//! column it was drawn in has one meaning, a reader scanning it reads
//! `ABORTED` and `reaped` as two kinds of thing when they are one, and the
//! question the case answered is one almost nobody was asking. The question
//! everyone *is* asking — did this go well — is answered by the word and the
//! colour, which is where it always belonged.

pub mod banner;
pub mod format;
pub mod help;
pub mod live;
pub mod palette;
pub mod progress;
pub mod style;
pub mod table;
pub mod term;

use armada_core::envelope::{
    TickData,
    AnswerData, AskData, BoardData, BridgeData, CheckData, CheckDryRun, CleanData, CleanDryRun,
    CommandsData, ComponentsData, DispatchData, Disposition, DoctorData, Envelope, FailureData,
    FailuresData, Finding, FleetLsData, GuildBundleData, GuildChangeData, GuildInitData,
    GuildItemData, GuildListData, GuildSyncData, Headline, HelmData, InboxData, InitData,
    InitDryRun, KillData, MachineInitData, McpData, PauseData, ProbeData, Projection, ReapPlanData,
    ReportData, ResultRow, ResumeData, ScanData, ServicesData, ShowData, SkillsData, SpawnData,
    StatusData, Unreclaimed, UpDryRun, VerdictData, VerifyData, Wiring,
};
use armada_core::error::{ArmadaError, Status};
use armada_core::failure::{Entry as FailureEntry, State as FailureState};
use armada_core::fleet::JobState;
use armada_core::id::WorkspaceId;
use armada_core::ports::{PortBlock, PortState};
use armada_core::reap::ReapPlan;
use armada_core::scan::{Handover, TellWhy};

use crate::ask::Choice;
use crate::verbs::Output;
use palette::Role;
use style::Style;
use table::{Cell, Column, Span, Table};
use term::Terminal;

/// Render for a terminal.
///
/// **`style` is the only thing that differs between the two human audiences**
/// (PLAN.md §3.1.1). A person at a terminal and an agent reading stdout get the
/// same columns, the same order and the same words; one of them also gets colour
/// and typographic dashes. Every line below is written once, for both.
pub fn human(output: &Output, style: Style, terminal: Terminal) -> String {
    let width = terminal.usable_width();
    match output {
        Output::Init(envelope) => init(envelope, style, width),
        Output::InitDryRun(envelope) => init_dry(envelope, style, width),
        Output::Up(envelope) => services(envelope, style, width, "service"),
        Output::UpDryRun(envelope) => up_dry(envelope, style, width),
        Output::Down(envelope) => services(envelope, style, width, "service"),
        Output::Clean(envelope) => clean(envelope, style, width),
        Output::CleanDryRun(envelope) => clean_dry(envelope, style, width),
        Output::Status(envelope) => status(envelope, style, width),
        Output::Check(envelope) => check(envelope, style, width),
        Output::CheckDryRun(envelope) => check_dry(envelope, style, width),
        Output::Dispatch(envelope) => dispatch(envelope, style),
        Output::Scan(envelope) => scan(envelope, style, width),
        Output::Verify(envelope) => verify(envelope, style, width),
        Output::Skills(envelope) => skills(envelope, style, width),
        Output::Components(envelope) => components(envelope, style, width),
        Output::Commands(envelope) => commands(envelope, style, width),
        Output::MachineInit(envelope) => machine_init(envelope, style, width),
        Output::Doctor(envelope) => doctor(envelope, style, width),
        Output::GuildSync(envelope) => guild_sync(envelope, style, width),
        Output::GuildInit(envelope) => guild_init(envelope, style, width),
        Output::GuildBundle(envelope) => guild_bundle(envelope, style, width),
        Output::GuildProject(envelope) => guild_project(envelope, style, width),
        Output::GuildList(envelope) => guild_list(envelope, style, width),
        Output::GuildItem(envelope) => guild_item(envelope, style, width),
        Output::GuildChange(envelope) => guild_change(envelope, style, width),
        Output::Failures(envelope) => failures(envelope, style, width),
        Output::Failure(envelope) => failure(envelope, style, width),
        Output::Spawn(envelope) => spawn(envelope, style, width),
        Output::FleetLs(envelope) => fleet_ls(envelope, style, width),
        Output::Bridge(envelope) => bridge(envelope, style, width),
        Output::Helm(envelope) => helm(envelope, style, width),
        Output::Show(envelope) => show(envelope, style, width),
        Output::Board(envelope) => board(envelope, style, width),
        Output::Kill(envelope) => kill(envelope, style, width),
        Output::Inbox(envelope) => inbox(envelope, style, width),
        Output::Answer(envelope) => answer(envelope, style, width),
        Output::Pause(envelope) => pause(envelope, style, width),
        Output::Resume(envelope) => resume(envelope, style, width),
        Output::ReapPlan(envelope) => reap_plan(envelope, style, width),
        Output::Mcp(envelope) => served(envelope, style),
        Output::Probe(envelope) => probe(envelope, style, width),
        Output::Report(envelope) => reported(envelope, style, width),
        Output::Ask(envelope) => asked(envelope, style, width),
        Output::Verdict(envelope) => stepped(envelope, style, width),
        Output::Tick(envelope) => ticked(envelope, style, width),
    }
}

// -------------------------------------------------------- M3: the toolbelt
//
// **Four of these five draw for a reader who will almost never see them.**
// `fleet.probe`, `.report`, `.ask_human` and `.verdict` are MCP tools with no
// CLI verb (`commands/helm/mcp.md`), and their callers read the envelope. They
// are drawn anyway because [`Output`] is one type with one renderer: a variant
// answering `unreachable!()` here would be a panic waiting for the day somebody
// adds the verb, and one answering `""` would print nothing and read as a hang.

/// `armada mcp serve`, once the client hangs up.
///
/// **It names the belt it served**, which is the one fact a person running this
/// by hand is trying to establish: whether this process would have been allowed
/// to spawn. It is decided by the environment, so there is nowhere else to read
/// it off.
fn served(envelope: &Envelope<McpData>, style: Style) -> String {
    summary(
        style,
        envelope.status,
        &[
            format!("{} toolbelt", envelope.data.toolbelt),
            format::count(envelope.data.tools.len(), "tool"),
            envelope.data.transport.clone(),
        ],
    )
}

/// `fleet.probe` — one Job's transcript, summarised.
///
/// **The summary gets the width and the facts get one line.** What was asked is
/// "how is it going", and a four-column table above three sentences of prose
/// buries the sentences — which are the answer.
fn probe(envelope: &Envelope<ProbeData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut out = job_summary(
        style,
        data.state,
        &[
            data.job.clone(),
            data.step.clone(),
            format::count(data.events, "event"),
        ],
    );
    out.push('\n');
    for line in wrap_prose(&data.summary, width.saturating_sub(2)) {
        out.push_str(&format!("  {line}\n"));
    }
    out
}

/// `fleet.report` — a Drone's own progress note, appended.
fn reported(envelope: &Envelope<ReportData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let table = Table::new(columns("job", "detail", false))
        .indent(2)
        .row(vec![
            token("noted", Role::BeaconGreen),
            Cell::painted(data.job.clone(), Role::NavalBlue),
            detail_cell(style, Some(&data.step)),
        ]);
    let mut out = table.render(style, width);
    out.push('\n');
    out.push_str(&summary(
        style,
        envelope.status,
        &[format::count(data.notes, "note")],
    ));
    out
}

/// `fleet.ask_human` — the entry raised, and the answer if one arrived.
fn asked(envelope: &Envelope<AskData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let table = Table::new(columns("job", "detail", false))
        .indent(2)
        .row(vec![
            token(
                match data.answered {
                    Some(_) => "answered",
                    None => "open",
                },
                match data.answered {
                    Some(_) => Role::BeaconGreen,
                    None => Role::FlareOrange,
                },
            ),
            Cell::painted(data.job.clone(), Role::NavalBlue),
            detail_cell(style, Some(&data.question)),
        ]);

    let mut out = table.render(style, width);
    out.push('\n');
    out.push_str(&summary(
        style,
        envelope.status,
        &[
            data.entry.clone(),
            // **An unanswered question is a state and not a failure**, so the
            // line names what closes it rather than apologising: the entry
            // outlives the wait and is still in the inbox.
            match &data.answered {
                Some(said) => said.clone(),
                None => "armada fleet answer <job> \"…\"".to_string(),
            },
        ],
    ));
    out
}

/// `fleet.verdict` — how a step ended, and what it rests on.
///
/// **The evidence is a table because it is the load-bearing half.** A `PASS`
/// with nothing under it is refused by the verb (PLAN.md §14.3), so a reader
/// looking at a `PASS` is looking to see *what* passed.
fn stepped(envelope: &Envelope<VerdictData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut table = Table::new(columns("scope", "detail", false)).indent(2);
    for piece in &data.evidence {
        table = table.row(vec![
            token(
                &piece.kind,
                match piece.exit {
                    0 => Role::BeaconGreen,
                    _ => Role::DistressRed,
                },
            ),
            Cell::painted(piece.scope.clone(), Role::NavalBlue),
            detail_cell(style, Some(&format!("exit {}", piece.exit))),
        ]);
    }

    let mut out = table.render(style, width);
    if table.is_empty() {
        out.push_str("  no evidence\n");
    }
    out.push('\n');
    out.push_str(&job_summary(
        style,
        data.state,
        &[
            data.job.clone(),
            format!("{} {}", data.step, data.verdict.word()),
            format!("attempt {}", data.attempts),
        ],
    ));
    out
}


/// `armada fleet tick` — one pass of the workflow loop.
///
/// **One row per Job, and the predicate is a column.** A reader watching a
/// fleet advance wants to know which gate settled a step, not only that it
/// settled: `failing_test_exists` on a `reproduce` row is the difference
/// between *"it moved on"* and *"it moved on because a test it wrote is failing
/// in the tree"*, and `job.rs`'s [`Gate`](armada_core::fleet::job::Gate) already
/// makes the same argument about the record.
///
/// **The word is the payload's**, in both audiences, for the reason every other
/// render in this file carries the schema's spelling: a reader who sees
/// `check_passes` can grep their workflow for it.
fn ticked(envelope: &Envelope<TickData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut table = Table::new(columns("job", "detail", false)).indent(2);
    for row in &data.results {
        table = table.row(vec![
            token(
                &row.did,
                match row.did.as_str() {
                    "advanced" | "finished" => Role::BeaconGreen,
                    "halted" | "retried" => Role::DistressRed,
                    "asked" => Role::SignalAmber,
                    _ => Role::SteelGrey,
                },
            ),
            Cell::painted(row.job.clone(), Role::NavalBlue),
            detail_cell(
                style,
                Some(&match &row.predicate {
                    Some(must) => format!("{} · {must} — {}", row.step, row.why),
                    None => format!("{} — {}", row.step, row.why),
                }),
            ),
        ]);
    }

    let mut out = table.render(style, width);
    if table.is_empty() {
        out.push_str("  no Jobs\n");
    }
    out.push('\n');
    out.push_str(&headlined(
        style,
        &style.strong(
            match data.moved {
                0 => Role::SteelGrey,
                _ => Role::BeaconGreen,
            },
            "TICK",
        ),
        &[
            match data.moved {
                1 => "1 Job moved".to_string(),
                moved => format!("{moved} Jobs moved"),
            },
            match data.results.len() {
                1 => "1 looked at".to_string(),
                seen => format!("{seen} looked at"),
            },
        ],
    ));
    out
}

// ---------------------------------------------------------------- M3: the fleet
//
// **The lead word on a Fleet summary line is a Job state, not a terminal
// state.** `RUNNING` there says what the *Job* is doing; the envelope's own
// `status` says how the *command* ended, and they are different questions
// (PLAN.md §14.3). This module's uppercase rule is kept rather than bent: the
// word is in the payload under `data.state`, spelled exactly as it is printed,
// so a reader can still grep for anything they saw — the same argument
// [`Headline`] carries for `NEEDS ATTENTION`.

/// A Job state, spelled as the payload spells it and coloured to agree.
fn job_state(state: JobState) -> Cell {
    Cell::painted(state.word().to_string(), Role::for_job_state(state))
}

/// The summary line for a verb that reports one Job.
fn job_summary(style: Style, state: JobState, facts: &[String]) -> String {
    headlined(
        style,
        &style.strong(Role::for_job_state(state), state.word()),
        facts,
    )
}

/// `armada fleet spawn` — the four things it did, and how to take the Job over.
///
/// **The confidence is on the screen and not only in the payload** (PLAN.md
/// §14.2): a guess has to be visible as a guess, and a classification nobody can
/// see is one nobody can override.
///
/// **And a low one is said in words, not left as a number in a column.** A real
/// spawn classified a task as `design` at `0.10` and proceeded silently. A tenth
/// is a coin flip, and nothing about printing `0.10` as one column among five
/// tells a reader that — they would have to know the threshold to know they had
/// been warned. So a guess gets a line of its own, naming the flag that replaces
/// it, on the same reasoning that gives `armada doctor` its fix lines: a report
/// that names a problem without the command that fixes it sends the reader to
/// the documentation.
///
/// # `--dry-run` says `WOULD`, and it is the same table
///
/// **A preview that reads as a receipt is the worst defect a preview can have.**
/// This function used to render a dry run and a real spawn identically —
/// `CREATED worktree <path>` for a directory that does not exist, `STARTED
/// drone job c099` for a process nobody started, and a `QUEUED … armada fleet
/// board <name>` footer offering a command that refuses, because there is no
/// Job. The run was correct; the report was not, and a reader had no way to tell
/// the two apart. That is the one thing a dry run must never be ambiguous about.
///
/// So the three steps that did not happen say **`WOULD`**, in `Role::FlareOrange`
/// — the word every other preview in this file already uses ([`init_dry`],
/// [`clean_dry`], [`up_dry`]) rather than a fourth vocabulary for a fifth verb.
/// The layout is untouched: same four columns, same four rows, same order, so a
/// reader who knows the real table can read this one.
///
/// **The classify row is the exception, because it is the one step that really
/// ran.** `--dry-run` still classifies — `commands/fleet/spawn.md` promises the
/// preview reports the classification, and the workflow is the whole substance
/// of the preview: one Haiku call is what stops a wrong workflow spending a
/// whole Job budget. So that row keeps its past tense and its real elapsed time,
/// which is also what makes the contrast legible: one thing happened, three
/// would.
///
/// **And the footer offers no action that cannot be taken.** `armada fleet board`
/// is gone, the `QUEUED` lead — a [`JobState`] for a Job with no record — is
/// gone, and the line is the convention the other previews close on:
/// `SKIPPED  <name>, dry run, nothing was spawned`.
fn spawn(envelope: &Envelope<SpawnData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut table = Table::new(columns_for(progress::Shape::Spawn)).indent(2);

    // **`SKIPPED` is the preview and `READY` is the spawn**, and they are the
    // only two statuses `verbs::fleet::spawn` returns — the dry-run arm and the
    // last line of the function. Read from the envelope rather than from a flag
    // threaded down beside it, because the envelope is what a `--json` reader
    // has to tell them apart by too, and a second signal is a second thing that
    // can disagree. `a_preview_and_a_spawn_are_told_apart_by_status` pins it.
    let dry = envelope.status == Status::Skipped;

    // Below the threshold Helm confirms at (PLAN.md §15.4). Fleet at the CLI has
    // nobody to ask, so it says so loudly instead.
    let guessed = data
        .confidence
        .is_some_and(|c| c < armada_core::fleet::classify::CONFIDENT);

    let (classified, role) = spawn_classified(&data.workflow, data.confidence);
    // Past tense even under `--dry-run`: the workflow really is settled, either
    // by a call that really was made or by the `--workflow` you passed.
    table = table.row(vec![
        token(progress::SpawnStep::Classify.done(), role),
        Cell::plain(progress::SpawnStep::Classify.id()),
        detail_cell(style, Some(&classified)),
        time_cell(data.classify_ms),
    ]);
    table = table.row(vec![
        done_or_would(dry, progress::SpawnStep::Worktree.done(), Role::BeaconGreen),
        Cell::plain(progress::SpawnStep::Worktree.id()),
        detail_cell(style, Some(&data.worktree)),
        // Nothing was prepared, so no interval is reported. `0.0s` next to
        // `WOULD` is a measurement of work that did not happen.
        time_cell(match dry {
            true => None,
            false => Some(data.prepare_ms),
        }),
    ]);
    table = table.row(vec![
        done_or_would(
            dry,
            progress::SpawnStep::Ports.done(),
            match data.port_block {
                Some(_) => Role::BeaconGreen,
                None => Role::SteelGrey,
            },
        ),
        Cell::plain(progress::SpawnStep::Ports.id()),
        detail_cell(
            style,
            data.port_block
                .map(|block| style.span(block.from, block.to))
                .as_deref(),
        ),
        time_cell(None),
    ]);
    table = table.row(vec![
        done_or_would(dry, progress::SpawnStep::Drone.done(), Role::BeaconGreen),
        Cell::plain(progress::SpawnStep::Drone.id()),
        detail_cell(
            style,
            Some(&match dry {
                // **No job id in a preview.** The uuid is minted before the
                // dry-run arm returns, but it is never saved — printing it
                // hands the reader an id that resolves to nothing.
                true => format!("{} step", data.step),
                false => format!(
                    "job {}, {} step",
                    armada_core::fleet::job::short(&data.uuid),
                    data.step
                ),
            }),
        ),
        time_cell(None),
    ]);

    let mut out = table.render(style, width);
    out.push('\n');
    out.push_str(&match dry {
        true => summary(
            style,
            envelope.status,
            &[
                // The name it *would* take, which is the second thing worth
                // previewing after the workflow — and the only place this table
                // says it, now that the footer no longer offers to board it.
                data.name.clone(),
                "dry run".to_string(),
                "nothing was spawned".to_string(),
            ],
        ),
        false => job_summary(
            style,
            data.state,
            &[
                data.name.clone(),
                format!("armada fleet board {} to take over", data.name),
            ],
        ),
    });
    // **The warning goes under the verdict, where a fix line goes.** A guess is
    // the one thing about a spawn a reader has to act on, and it is worth a line
    // rather than a decimal in a column.
    if guessed {
        out.push_str(&format!(
            "  {} {}\n",
            style.paint(Role::FlareOrange, style.arrow()),
            style.paint(
                Role::SteelGrey,
                &format!(
                    "low confidence: this may be the wrong workflow. \
                     --workflow {} respawns it",
                    armada_core::fleet::workflow::STARTERS.join("|")
                )
            )
        ));
    }
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error, style));
    }
    out
}

/// `armada fleet ls` — what is running, how long, what it has spent, and who
/// needs you.
///
/// **Every column is read off data Claude Code already emits** (PHASES.md §9.1
/// F2). The renderer rounds; it does not compute.
///
/// **`ID` is here because a name is not one** (`docs/reserved/005-inbox-label-
/// not-identity.md`). A name is a handle
/// [`armada_fleet::jobs::Store::free_name`] hands out again once the Job
/// holding it is over, so a listing of names alone cannot tell two Jobs called
/// `this-test` apart — and `armada fleet show this-test` then refuses as
/// ambiguous with no way, from this table, to see what to type instead. Eight
/// characters is what the refusal already prints and what a person successfully
/// types back, so it is what the column shows.
fn fleet_ls(envelope: &Envelope<FleetLsData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut table = Table::new(vec![
        Column::fixed("status"),
        Column::fixed("job"),
        Column::fixed("id"),
        Column::fixed("workflow"),
        Column::flexible("detail"),
        // **Right, always**: a column of right-aligned numbers can be compared
        // by eye without reading any of them (`render/table.rs`).
        Column::fixed("spent").right(),
        Column::fixed("time").right(),
    ])
    .indent(2);

    for row in &data.results {
        table = table.row(vec![
            job_state(row.state),
            // Naval blue is what the palette reserves for a Job identifier.
            Cell::painted(row.name.clone(), Role::NavalBlue),
            // **Muted, because it is the fallback and not the handle.** A name
            // is what a person types on an ordinary day; the id is what they
            // type on the day two Jobs share one.
            Cell::muted(armada_fleet::jobs::short(&row.uuid).to_string()),
            Cell::muted(row.workflow.clone()),
            detail_cell(style, Some(row.detail.as_str())),
            // **Nothing spent is a dash, not `$0.00`.** A zero in this column
            // reads as a measurement; a Job that has not run yet has not been
            // measured.
            Cell::muted(if row.cost_usd > 0.0 {
                format::money(row.cost_usd)
            } else {
                style.nothing().to_string()
            }),
            Cell::muted(if row.state == JobState::Queued {
                style.nothing().to_string()
            } else {
                format::elapsed(row.runtime_s * 1_000)
            }),
        ]);
    }

    let mut out = table.render(style, width);
    if table.is_empty() {
        // **An empty fleet says so.** A verb that printed nothing would be
        // indistinguishable from one that failed to run.
        out.push_str("  no Jobs\n");
    }
    out.push('\n');
    out.push_str(&summary(style, envelope.status, &ls_facts(data)));
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error, style));
    }
    out
}

/// What a fleet listing counts, in the order the agreed layout counts it.
fn ls_facts(data: &FleetLsData) -> Vec<String> {
    let mut facts = vec![format::count(data.results.len(), "job")];
    // Omitted at zero rather than printed as `0 need you`: the whole value of
    // this line is that "needs me" stays a signal (PLAN.md §15.4).
    if data.needs_you > 0 {
        facts.push(format!("{} need you", data.needs_you));
    }
    facts.push(format!("{} today", format::money(data.spent_usd)));
    facts
}

/// The Bridge's keys, **in priority order: the last ones drop first.**
///
/// **Named rather than glyphed.** The page draws `↵`; this writes `enter`,
/// because a key line is read by both audiences and a glyph that folds to ASCII
/// would give the two of them different words for the same key. Everything else
/// on the Bridge is a single character and needs no folding.
///
/// **`p`'s word is the one that varies**, because it is the one key whose verb
/// depends on the row under the cursor — `pause` over a running Job, `resume`
/// over a paused one. A second list of states here would let the line and the
/// binding drift, so both ask
/// [`armada_core::fleet::bridge::pause_key`], which is why this is a function
/// and not a `const`.
///
/// **`c chat` is not here.** The line must stay one line — `bridge.rs`'s tests
/// assert the frame does not change height — so space on it is the scarcest
/// thing the Bridge has, and an unbuilt verb does not get any of it while a
/// built one goes unadvertised. `c` is still bound and still answers.
///
/// **Order is priority and not importance-of-verb.** `enter` is first because it
/// is what a person tries on a table with a cursor; `d detail` is second for the
/// same reason, and because it is the only key that answers the one column that
/// is ever a call to action; `/ filter` is last because a filter over two rows
/// is worth nothing, and a fleet large enough to need one is a fleet whose
/// reader has already gone looking for `?`. What falls off the end at a given
/// width falls off in that order, and [`QUIT`] is never one of them.
///
/// **`d detail` is on the line now, and it is the overflow that pays for it.**
/// It was left unnamed while the line had no way to shed anything: eight pairs
/// is eighty-four columns, and the choice then was a wrapped line or a silent
/// key. Now what does not fit drops and `? keys` says so, so the key that
/// answers `NEEDS YOU: YES` is advertised wherever there is room for it and
/// reachable through `?` wherever there is not.
fn bridge_key_pairs(selected: Option<JobState>) -> [(&'static str, &'static str); 8] {
    [
        ("enter", "board"),
        ("d", "detail"),
        ("n", "new"),
        ("p", armada_core::fleet::bridge::pause_key(selected)),
        ("x", "abort"),
        ("a", "answer"),
        ("r", "reap"),
        ("/", "filter"),
    ]
}

/// The one key that never drops off the line.
///
/// **Because a full-screen program that does not say how to leave is a trap.**
/// Everything else on the line is a convenience; this one is the exit.
const QUIT: (&str, &str) = ("q", "quit");

/// What the line says when it could not carry everything.
///
/// **The honest overflow.** Nine pairs is eighty-two columns against a budget of
/// seventy-eight, so at some point a key line either wraps — changing the frame's
/// height, which the tests forbid — or stops listing everything. This is the
/// third option: it names itself, and `?` shows the rest.
const MORE: (&str, &str) = ("?", "keys");

/// Two spaces per gap, which is the one separator on this screen that is the
/// same for both audiences (see [`bridge_keys`]).
const KEY_GAP: usize = 2;

/// The two columns every table on this screen is indented by, which the key line
/// shares and therefore has to pay for out of its own budget.
const KEY_INDENT: usize = 2;

/// The reap preview's own keys, for the mode that has its own.
///
/// **A separate line for a separate screen.** The fleet's keys do nothing while
/// a preview is open, and printing them would advertise eight keys of which two
/// work.
const REAP_KEYS: [(&str, &str); 4] = [
    ("arrows", "move"),
    ("space", "toggle"),
    ("enter", "reap"),
    ("esc", "cancel"),
];

/// `armada bridge` — one frame of the live screen.
///
/// **This is the frame, not a second listing of it.** `--once` and the redrawn
/// screen show the same rows in the same columns; what the alternate screen adds
/// is a cursor and a keyboard, which is exactly the difference between watching
/// and reading (`commands/helm/bridge.md`).
///
/// **Three departures from the page's drawing, and each is this repository's own
/// rule winning.** The drawing puts `JOB` first, marks the needs-you column with
/// `●`, and boxes the whole thing. Status is first and always a word in every
/// table Armada draws; a symbol that only appears at a terminal gives the two
/// audiences different shapes, which `render_golden.rs` asserts of every
/// fixture; and a box drawn in text would have to be two different boxes for the
/// two audiences. The columns the page settled — the Job, its state, the task,
/// run time, spend and whether it needs you — are all here, in Armada's shape.
///
/// **There is no progress column, deliberately.** Nothing emits percent-complete
/// (PHASES.md §9.1 F2), and a bar computed from a turn count is a guess drawn as
/// a measurement.
fn bridge(envelope: &Envelope<BridgeData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut out = format!("{}\n\n", style.strong(Role::SignalAmber, "  ARMADA BRIDGE"));

    // **No row is under a cursor here, so the cursor column is dropped.** One
    // table describes both surfaces and the renderer decides which columns
    // earned their width (`docs/commands/render.md`), which is what stops the
    // screen and `--once` from becoming two layouts.
    let table = bridge_table(data, style, None);
    out.push_str(&table.render(style, width));
    if table.is_empty() {
        // **An empty fleet says so.** A screen that drew nothing would be
        // indistinguishable from one that failed to read the index.
        out.push_str(match data.filter {
            Some(_) => "  no Jobs match\n",
            None => "  no Jobs\n",
        });
    }

    out.push('\n');
    out.push_str(&bridge_summary(data, envelope.status, style));
    out.push('\n');
    // **No row is under a cursor in `--once`, so `p` prints its default word.**
    // The frame a pipe reads describes the fleet rather than a selection, and a
    // key line that guessed at a state nobody is standing on would be a claim
    // about a cursor that does not exist.
    out.push_str(&format!(
        "  {}\n",
        style.paint(Role::SteelGrey, &bridge_keys(None, width))
    ));
    out
}

/// The Bridge's table, described once for both surfaces.
///
/// **One table, two emitters** — the same split `render/live.rs` uses for the
/// run: `Table::render` paints escape sequences for `--once`, `Table::spans`
/// hands `ratatui` coloured pieces for the live screen, and both ask this
/// function which columns exist and how wide they are. Two descriptions would
/// drift in the first frame.
///
/// `cursor` is the row under the caret, or `None` when nobody is watching — and
/// a cursor column no row filled is dropped, header and all, which is why
/// `--once` shows no empty gutter.
pub fn bridge_table(data: &BridgeData, style: Style, cursor: Option<usize>) -> Table {
    let mut table = Table::new(vec![
        // The caret, for the surface that has one. **A character rather than a
        // colour**, for the reason `ask/select.rs` gives: a row told apart only
        // by being amber is a row a monochrome terminal cannot tell apart.
        Column::fixed(""),
        Column::fixed("status"),
        Column::fixed("job"),
        // **The step, and how long it has been on it.** One column rather than
        // two, and the width for it comes out of `TASK` — which is the trade,
        // stated: the task is already a truncation of a sentence the detail pane
        // carries whole, and the step is the fact no other column can be read
        // for. `DETAIL` in `fleet ls` folds the step together with an open
        // inbox body, so it is not a substitute.
        //
        // **A step and an elapsed time, and never a fraction.** *"On `implement`
        // for 12m"* is measured; *"three of five steps"* would be the progress
        // bar PHASES.md §9.1 F2 bans, drawn in words.
        Column::fixed("step"),
        Column::flexible("task"),
        Column::fixed("run").right(),
        Column::fixed("spent").right(),
        // **The only column that is ever a call to action**, and the only one
        // besides the caret that disappears when nothing fills it: a `NEEDS YOU`
        // header over a column of placeholders claims somebody is waiting.
        Column::fixed("needs you"),
    ])
    .indent(2);

    for (index, row) in data.results.iter().enumerate() {
        table = table.row(vec![
            match cursor == Some(index) {
                true => Cell::painted(style.caret(), Role::SignalAmber),
                false => Cell::empty(),
            },
            job_state(row.state),
            Cell::painted(row.name.clone(), Role::NavalBlue),
            step_cell(row),
            detail_cell(style, Some(row.task.as_str())),
            // A Job that has not run yet is a dash in both number columns, for
            // the reason `fleet ls` gives: a zero reads as a measurement, and
            // nothing has been measured.
            Cell::muted(if row.state == JobState::Queued {
                style.nothing().to_string()
            } else {
                format::elapsed(row.runtime_s * 1_000)
            }),
            Cell::muted(if row.cost_usd > 0.0 {
                format::money(row.cost_usd)
            } else {
                style.nothing().to_string()
            }),
            if row.needs_attention {
                Cell::painted("YES", Role::DistressRed)
            } else {
                Cell::nothing()
            },
        ]);
    }
    table
}

/// The step a Job is on, and how long it has been on it.
///
/// **Empty when there is no step, so the whole column goes with it.** A fleet of
/// queued Jobs draws no `STEP` header at all — the generalisation `render.md`
/// states, and the same one that drops the caret in `--once`.
///
/// **The time is omitted rather than zeroed when nothing measured it.** The
/// duration is a subtraction from a boundary a Drone reported crossing; a Job
/// whose Drone never reported one has no boundary, and `implement 0s` would be a
/// measurement nobody took.
fn step_cell(row: &armada_core::envelope::JobRow) -> Cell {
    match (row.step.as_str(), row.on_step_s) {
        ("", _) => Cell::empty(),
        (step, None) => Cell::muted(step.to_string()),
        (step, Some(on_step_s)) => {
            Cell::muted(format!("{step} {}", format::elapsed(on_step_s * 1_000)))
        }
    }
}

/// The Bridge's summary line, painted, for `--once`.
pub fn bridge_summary(data: &BridgeData, status: Status, style: Style) -> String {
    summary(style, status, &frame_facts(data))
}

/// The same line as **coloured pieces**, for the screen.
///
/// **The two must not drift**, which is what the test beside them asserts: a
/// terminal reading `4 jobs · 1 need you` on the screen and `4 jobs, 1 need you`
/// from `--once` would be one render behaving as two. So the separator, the lead
/// word and the facts are all decided here and only the emitting differs — the
/// screen cannot take escape sequences, because `ratatui` prints an SGR string
/// it finds in a value literally.
pub fn bridge_summary_pieces(data: &BridgeData, status: Status, style: Style) -> Vec<Span> {
    vec![
        Span {
            text: status.to_string(),
            role: Some(Role::for_status(status)),
            bold: true,
        },
        Span {
            text: "  ".to_string(),
            role: None,
            bold: false,
        },
        Span {
            text: frame_facts(data).join(style.between()),
            role: Some(Role::SteelGrey),
            bold: false,
        },
    ]
}

/// The key line's text, unpainted, for both surfaces.
///
/// **Two spaces between pairs, not [`Style::between`]** — which is the one
/// separator on this screen that is the same for both audiences. A middle dot
/// costs a column per gap and the line is eighty-one wide with them, so a person
/// at a standard terminal would read a wrapped key line while an agent read a
/// straight one. The drawing on `commands/helm/bridge.md` spaces them the same
/// way, for the same reason.
pub fn bridge_keys(selected: Option<JobState>, width: usize) -> String {
    spelled(&shown_keys(selected, width))
}

/// The keys the line could not carry, for the page `?` opens.
///
/// **Asked of the same function that trims the line**, so the overlay and the
/// line cannot disagree about which keys are hidden — which is the only way an
/// overflow key is worth having at all.
pub fn bridge_keys_hidden(selected: Option<JobState>, width: usize) -> Vec<(String, String)> {
    let shown = shown_keys(selected, width);
    bridge_key_pairs(selected)
        .iter()
        .filter(|pair| !shown.contains(pair))
        .map(|(key, does)| ((*key).to_string(), (*does).to_string()))
        .collect()
}

/// Every binding the Bridge has, for the page `?` opens — including the ones
/// that were never on the line because they have no verb behind them.
pub fn bridge_every_key(selected: Option<JobState>) -> Vec<(String, String)> {
    let mut all: Vec<(String, String)> = bridge_key_pairs(selected)
        .iter()
        .chain(std::iter::once(&QUIT))
        .map(|(key, does)| ((*key).to_string(), (*does).to_string()))
        .collect();
    all.push(("c".to_string(), "chat — entering is off".to_string()));
    all.push(("arrows, j k".to_string(), "move the cursor".to_string()));
    all.push((
        "esc".to_string(),
        "clear the filter, then leave".to_string(),
    ));
    all
}

/// The pairs that fit, with [`QUIT`] pinned last and [`MORE`] when any were cut.
///
/// **It drops rather than wraps.** A key line that wrapped would make the frame
/// one row taller, which moves everything above it — and the Bridge's own tests
/// assert the frame does not change height between redraws.
fn shown_keys(selected: Option<JobState>, width: usize) -> Vec<(&'static str, &'static str)> {
    let all = bridge_key_pairs(selected);
    let budget = width.saturating_sub(KEY_INDENT);
    let mut taken = all.len();
    loop {
        let mut line: Vec<(&str, &str)> = all[..taken].to_vec();
        if taken < all.len() {
            line.push(MORE);
        }
        line.push(QUIT);
        if spelled(&line).chars().count() <= budget || taken == 0 {
            return line;
        }
        taken -= 1;
    }
}

/// The reap preview's key line, in the same shape.
pub fn reap_keys() -> String {
    spelled(&REAP_KEYS)
}

fn spelled(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(key, does)| format!("{key} {does}"))
        .collect::<Vec<_>>()
        .join(&" ".repeat(KEY_GAP))
}

/// What a frame counts, in the order the drawing counts it.
///
/// **Over the rows on the screen, never over the whole fleet.** A filtered frame
/// reporting the fleet's totals would be answering a question nobody asked; the
/// filter and what it removed are said instead, so the smaller numbers are
/// accounted for rather than mysterious.
fn frame_facts(data: &BridgeData) -> Vec<String> {
    let mut facts = vec![format::count(data.results.len(), "job")];
    if data.needs_you > 0 {
        facts.push(format!("{} need you", data.needs_you));
    }
    facts.push(format!("{} today", format::money(data.spent_usd)));
    if let Some(filter) = &data.filter {
        facts.push(format!("filter {filter}"));
        facts.push(format!("{} hidden", data.hidden));
    }
    facts
}

/// `armada helm` — what was wired, and the command that would enter it.
///
/// **Four rows and then a command, in that order.** The rows are what changed on
/// the machine, which is what a reader has to be able to audit; the command is
/// what they would run, and it comes last because it is the thing they act on.
///
/// **`DETAIL` is the path and nothing else.** The first draft put the prose
/// beside it — *"live push: every inbox line arrives mid-turn"* — and at eighty
/// columns the path was what got truncated, which is the half a reader cannot
/// look up anywhere else. `WIRED` already names the role, `armada helm --help`
/// explains it, and the sentence stays in the envelope for a caller that wants
/// it. A cell that elides the one auditable fact in the row is worse than a
/// terse one.
///
/// **The command is on a line of its own rather than in a cell**, for the same
/// reason [`board`]'s `DETAIL` column is fixed: a truncated launch command is
/// not a shorter answer, it is the wrong one, and this verb exists to produce a
/// line somebody reads and pastes. It runs past eighty columns, which
/// `config scan`'s hand-over line already does and for the identical reason.
fn helm(envelope: &Envelope<HelmData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut table = Table::new(columns("wired", "detail", false)).indent(2);

    for row in &data.results {
        table = table.row(vec![
            token(
                row.state.word(),
                match row.state {
                    Wiring::Written => Role::BeaconGreen,
                    Wiring::Unchanged => Role::SteelGrey,
                },
            ),
            Cell::painted(row.what.clone(), Role::NavalBlue),
            detail_cell(style, Some(&row.at)),
        ]);
    }

    let mut out = table.render(style, width);
    out.push('\n');
    // **The command, whole, and never elided.** It is the one line here meant to
    // be copied, and a `…` in the middle of it produces an argv that starts an
    // unconfigured session rather than one that starts nothing.
    out.push_str(&format!(
        "  {} {}\n\n",
        style.strong(Role::SignalAmber, "enter with"),
        data.argv.join(" ")
    ));
    out.push_str(&summary(
        style,
        envelope.status,
        &[
            data.agent.clone(),
            format!("conversation {}", data.conversation.word().to_lowercase()),
            // **Said out loud, because the absence of a session is the point.**
            // A reader who assumed `armada helm` had opened one would sit
            // waiting for a prompt that is never coming — and one who assumed
            // `--exec` would open it would find out by typing it. The reason is
            // read from the one constant every surface reads it from, so this
            // line cannot drift from the refusal it describes.
            format!(
                "nothing started; {} is {}",
                crate::verbs::helm::ENTER,
                crate::verbs::helm::ENTER_IS_OFF
            ),
        ],
    ));
    out
}

// ------------------------------------------------------------------- fleet show
// one Job, and why it wants you — the view the Bridge's table cannot be

/// `armada fleet show` — the whole of one Job.
///
/// **Written once, for three audiences and two surfaces.** [`show_lines`] is the
/// only description of this view; this paints it for a terminal and a pipe, the
/// Bridge's detail pane hands the same pieces to `ratatui`, and `--json` emits
/// the payload. A second description would be a second layout by the following
/// milestone, which is the rule [`bridge_table`] already states.
fn show(envelope: &Envelope<ShowData>, style: Style, width: usize) -> String {
    let mut out = String::new();
    for line in show_lines(&envelope.data, style, width) {
        out.push_str(&paint_line(&line, style));
        out.push('\n');
    }
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error, style));
    }
    out
}

/// The detail view as **coloured pieces** — the one description of it.
///
/// **What the reader came for is first and in full.** The complaint this answers
/// is a `NEEDS YOU: YES` with no way to find out why, so the inbox entry that
/// raised it is above the task, the budget and the paths — and it is wrapped
/// prose rather than a cell, because the answer to *why* is a sentence and a
/// column would truncate it. So is the task, for the same reason: the `TASK`
/// column is a column, and this is what it was cut from.
///
/// **Nothing here is rephrased.** Every state word is [`JobState::word`], every
/// entry body is the inbox's own, and the step is the record's — the payload
/// gathers and this draws. Two components explaining one state in different
/// words is the failure `bridge_summary_pieces` exists to prevent, stated for a
/// second view.
///
/// **There is no progress bar and no percentage**, here least of all. A detail
/// view is exactly where one would look like a measurement; nothing emits
/// percent-complete (PHASES.md §9.1 F2), and what is honest — turns, tokens and
/// wall clock against their ceilings — is drawn as the numbers they are.
pub fn show_lines(data: &ShowData, style: Style, width: usize) -> Vec<Vec<Span>> {
    let mut lines: Vec<Vec<Span>> = Vec::new();

    // **The row he was looking at, unchanged.** The Bridge's table put him here;
    // opening with the same Job in the same shape is what makes the rest of the
    // page read as *more about this*, rather than as a second report.
    let identity = Table::new(vec![
        Column::fixed("status"),
        Column::fixed("job"),
        Column::fixed("workflow"),
        Column::flexible("step"),
        Column::fixed("spent").right(),
        Column::fixed("time").right(),
    ])
    .indent(2)
    .row(vec![
        job_state(data.state),
        Cell::painted(data.job.clone(), Role::NavalBlue),
        Cell::muted(data.workflow.clone()),
        detail_cell(style, Some(&step_and_attempt(data))),
        Cell::muted(match data.cost_usd > 0.0 {
            true => format::money(data.cost_usd),
            false => style.nothing().to_string(),
        }),
        Cell::muted(match data.state == JobState::Queued {
            true => style.nothing().to_string(),
            false => format::elapsed(data.runtime_s * 1_000),
        }),
    ]);
    lines.extend(identity.spans(style, width));

    lines.extend(asked_lines(data, style, width));
    lines.extend(task_lines(data, width));

    // **The three facts that disagree when something is wrong, side by side.**
    // What the record says, whether the Drone is still there and what the Job is
    // still holding are separate rows because they are separate questions — a
    // Job recorded `RUNNING` whose Drone is gone while its ports are still
    // claimed reads as healthy in every other view Armada draws.
    lines.push(Vec::new());
    lines.extend(facts_table(data, style).spans(style, width));

    lines.extend(transition_lines(data, style, width));
    lines.extend(progress_lines(data, style, width));

    lines.push(Vec::new());
    lines.push(show_summary_pieces(data, style));
    lines
}

/// The step, how many times it has been tried, and how long it has been on it.
///
/// **Never a position in the workflow.** The step *index* would mean reading the
/// workflow document for a number that decides nothing, and "step 3 of 5" is the
/// percentage PHASES.md §9.1 F2 bans wearing a different notation.
///
/// **The attempt count is per-step and is no longer drawn against the iteration
/// ceiling.** This used to read `implement, attempt 2 of 15`, pairing a per-step
/// count with a Job-wide ceiling that counts turns across every step — two
/// different quantities in one phrase, and the phrase answered neither question.
/// The ceiling is on the `budget` row below, as `4 of 15 turns`, where it is
/// beside the thing it actually bounds.
fn step_and_attempt(data: &ShowData) -> String {
    if data.step.is_empty() {
        return String::new();
    }
    let mut said = data.step.clone();
    if data.attempt > 1 {
        said.push_str(&format!(", attempt {}", data.attempt));
    }
    if let Some(on_step_s) = data.on_step_s {
        said.push_str(&format!(", {} on it", format::elapsed(on_step_s * 1_000)));
    }
    said
}

/// **Why it wants you** — the entries, each with its own words underneath.
///
/// **Every entry this Job raised, and the open ones are not separated out.** An
/// answered question is the record of what was already decided, and a reader
/// looking at a second question usually needs the first one to make sense of it.
/// The `STATUS` word says which is which.
fn asked_lines(data: &ShowData, style: Style, width: usize) -> Vec<Vec<Span>> {
    if data.asked.is_empty() {
        return Vec::new();
    }
    let mut table = Table::new(vec![
        Column::fixed("status"),
        Column::fixed("asked"),
        Column::fixed("time").right(),
    ])
    .indent(2);
    for row in &data.asked {
        table = table.row(vec![
            token(
                match row.answered.is_some() {
                    true => "answered",
                    false => &row.kind,
                },
                match (row.answered.is_some(), row.kind.as_str()) {
                    (true, _) => Role::SteelGrey,
                    (false, "BLOCKED") => Role::DistressRed,
                    (false, _) => Role::FlareOrange,
                },
            ),
            Cell::muted(row.uuid.clone()),
            Cell::muted(format::elapsed(row.waiting_s * 1_000)),
        ]);
    }

    // **The bodies are spliced under their own rows**, so one header names all
    // of them and each sentence still hangs off the entry it belongs to. The
    // table decided the column widths over every row before any of this, which
    // is why the rows still line up with prose between them.
    let rows = table.spans(style, width);
    let mut lines = vec![Vec::new()];
    let mut rest = rows.into_iter();
    if let Some(header) = rest.next() {
        lines.push(header);
    }
    for (spans, row) in rest.zip(&data.asked) {
        lines.push(spans);
        lines.extend(prose(&row.body, Role::Foreground, ASKED_INDENT, width));
        if let Some(answer) = &row.answered {
            // **Your answer is under the question**, in the same block: an
            // entry read a week later is a pair, and the two halves apart are
            // half a record.
            lines.extend(prose(
                &format!("you said: {answer}"),
                Role::SteelGrey,
                ASKED_INDENT,
                width,
            ));
        }
    }
    lines
}

/// **The task, whole.** A heading and then the sentence, because the `TASK`
/// column is a column and this is what it was cut from.
fn task_lines(data: &ShowData, width: usize) -> Vec<Vec<Span>> {
    if data.task.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![Vec::new(), vec![spaces(2), block_heading("TASK")]];
    lines.extend(prose(&data.task, Role::Foreground, 2, width));
    lines
}

/// What the record says, whether the Drone is there, and what is still held.
fn facts_table(data: &ShowData, style: Style) -> Table {
    let mut table = Table::new(columns("fact", "detail", false)).indent(2);

    // **Always drawn, even when it agrees.** The two states agreeing is the
    // ordinary case and the two disagreeing is the whole diagnosis; a row that
    // appeared only on disagreement would teach nobody where to look.
    table = table.row(vec![
        token("recorded", Role::SteelGrey),
        Cell::muted("state"),
        detail_cell(
            style,
            Some(&format!(
                "{}, as a verb last wrote it",
                data.recorded_state.word()
            )),
        ),
    ]);

    table = table.row(match (data.drone_pgid, data.drone_alive) {
        (Some(pgid), true) => vec![
            token("alive", Role::BeaconGreen),
            Cell::muted("drone"),
            detail_cell(style, Some(&format!("process group {pgid}"))),
        ],
        // **Red only while something still expects it to be running.** A Drone
        // that is gone because its Job finished is the ordinary end of a Job,
        // and colouring that as a fault would make the colour mean nothing on
        // the one row where it has to mean something.
        (Some(pgid), false) => vec![
            token(
                "gone",
                match data.recorded_state.is_over() {
                    true => Role::SteelGrey,
                    false => Role::DistressRed,
                },
            ),
            Cell::muted("drone"),
            detail_cell(
                style,
                Some(&format!("process group {pgid} is not Armada's any more")),
            ),
        ],
        (None, _) => vec![
            token("never", Role::SteelGrey),
            Cell::muted("drone"),
            detail_cell(style, Some("no Drone was ever started")),
        ],
    });

    // **What the step advances on** — the one fact that says *why it is still
    // here*. A step advances when its `verify: { must: … }` predicate holds, so
    // a reader looking at a Job that has not moved in an hour is otherwise
    // reading a symptom with the cause missing.
    //
    // **Drawn only when the workflow could be read**, because a guild can be
    // absent or half-synced and a defaulted `always` would invent the answer.
    if let Some(gate) = &data.gate {
        let mut said = format!("{} advances on {}", data.step, gate.must);
        if let Some(named) = gate.test.as_ref().or(gate.artifact.as_ref()) {
            // **`failing_test_exists` without its test is the case the
            // predicate exists to prevent** — *"a Drone 'fixes' a bug it never
            // reproduced and closes green"* — so what it names is drawn beside
            // it rather than dropped as detail.
            said.push_str(&format!(": {named}"));
        }
        if gate.answered_by_a_person {
            // **Said, not signalled.** `NEEDS YOU` and the inbox already carry
            // that somebody is waiting; this names which predicate is the reason
            // and does not become a second, differently-worded claim about it.
            said.push_str(", which is yours to answer");
        }
        table = table.row(vec![
            token("gated", Role::SteelGrey),
            Cell::muted("step"),
            detail_cell(style, Some(&said)),
        ]);
    }

    table = table
        .row(vec![
            token("spent", Role::SteelGrey),
            Cell::muted("budget"),
            detail_cell(
                style,
                Some(&format!(
                    "{} of {} turns, {} of {} tokens, {}",
                    data.turns,
                    data.budget.iterations,
                    token_count(data.tokens),
                    token_count(data.budget.tokens),
                    format::money(data.cost_usd),
                )),
            ),
        ])
        .row(vec![
            token("left", Role::SteelGrey),
            Cell::muted("budget"),
            detail_cell(
                style,
                Some(&format!(
                    "{} turns, {} tokens, {}",
                    data.budget_remaining.iterations,
                    token_count(data.budget_remaining.tokens),
                    // **[`format::elapsed`], the same spelling the `TIME` column
                    // uses.** `25m` beside a run time of `1h` compares; `25m 00s`
                    // beside it is the same fact in a second notation.
                    format::elapsed(data.budget_remaining.wall_clock_ms),
                )),
            ),
        ])
        .row(vec![
            token("since", Role::SteelGrey),
            Cell::muted("started"),
            detail_cell(style, Some(&data.created_at)),
        ]);

    // **What it is holding, which is what a stopped Job does not release.** Each
    // is a thing `armada fleet kill` would take back, and a Job whose Drone is
    // gone is holding all of them with nothing working on them.
    if let Some(block) = data.port_block {
        table = table.row(vec![
            token("held", Role::RadarCyan),
            Cell::muted("ports"),
            detail_cell(style, Some(&style.span(block.from, block.to))),
        ]);
    }
    table
        .row(vec![
            token("held", Role::RadarCyan),
            Cell::muted("worktree"),
            detail_cell(style, Some(&data.worktree)),
        ])
        .row(vec![
            token("held", Role::RadarCyan),
            Cell::muted("branch"),
            detail_cell(style, Some(&data.branch)),
        ])
        .row(vec![
            token("from", Role::SteelGrey),
            Cell::muted("repo"),
            detail_cell(style, Some(&data.repo)),
        ])
}

/// **The workflow, as it was actually walked** — every step boundary this Job
/// crossed, newest first.
///
/// **The detail pane is where the history belongs**, and it is why the Bridge's
/// `STEP` column can be one cell wide: eighty columns hold a step and a duration
/// and nothing more, and this has the room for the attempt number, the predicate
/// that settled each boundary and the exit codes it rested on.
///
/// **What happened and when — never how far through.** A workflow with five
/// steps sitting on step three is not "60% done" and nothing here says so; there
/// is no bar, no percentage and no estimated completion, for the reason
/// [`show_lines`] gives and [`armada_core::fleet::job::StepEvent`] states in
/// full. The rows are facts with timestamps, and the last one is where it is.
///
/// **The status word says who wrote the row**, which is the distinction the
/// whole record exists to keep: `ATTEMPTED` is the Drone saying it believes it
/// is finished, `COMPLETED` is the step's predicate holding. A Job whose last
/// two rows are `ATTEMPTED` then `FAILED` is a Drone that thought it was done
/// and a gate that disagreed — and that is unreadable from any view that stores
/// one word for both.
fn transition_lines(data: &ShowData, style: Style, width: usize) -> Vec<Vec<Span>> {
    if data.transitions.is_empty() {
        return Vec::new();
    }
    let mut table = Table::new(vec![
        Column::fixed("status"),
        Column::fixed("step"),
        Column::flexible("detail"),
        Column::fixed("time").right(),
    ])
    .indent(2);
    for crossing in &data.transitions {
        let mut said = format!("attempt {}", crossing.attempt);
        if let Some(must) = &crossing.must {
            said.push_str(&format!(", {must}"));
        }
        for evidence in &crossing.evidence {
            // **The exit code, not a sentence about it** (PLAN.md §14.3). The
            // number is the fact a verdict rests on, and a summary of it is the
            // assertion the rule refuses.
            said.push_str(&format!(", {} exited {}", evidence.scope, evidence.exit));
        }
        table = table.row(vec![
            token(
                &crossing.event,
                match crossing.event.as_str() {
                    "completed" => Role::BeaconGreen,
                    "failed" => Role::DistressRed,
                    // **An assertion is not painted as an outcome.** A Drone
                    // saying it is finished in green would be the screen
                    // agreeing with it before anything checked.
                    _ => Role::SteelGrey,
                },
            ),
            Cell::muted(crossing.step.clone()),
            detail_cell(style, Some(&said)),
            Cell::muted(format::elapsed(crossing.ago_s * 1_000)),
        ]);
    }
    let mut lines = vec![Vec::new()];
    lines.extend(table.spans(style, width));
    lines
}

/// **Recent activity — the Drone's own notes, and never its transcript.**
///
/// The orchestrator reads summaries and never raw transcripts (PLAN.md §15.2),
/// and a detail view is exactly the surface where that constraint would erode:
/// the transcript is right there and it is the easiest thing to print. These are
/// `fleet.report` notes, which the Drone wrote about itself.
///
/// **Truncated in a column, unlike the two blocks above, and deliberately.** A
/// note is a log line; the question and the task are the answer.
fn progress_lines(data: &ShowData, style: Style, width: usize) -> Vec<Vec<Span>> {
    if data.progress.is_empty() {
        return Vec::new();
    }
    let mut table = Table::new(vec![
        Column::fixed("status"),
        Column::fixed("step"),
        Column::flexible("detail"),
        Column::fixed("time").right(),
    ])
    .indent(2);
    for note in &data.progress {
        table = table.row(vec![
            token("reported", Role::SteelGrey),
            Cell::muted(note.step.clone()),
            detail_cell(style, Some(&note.body)),
            Cell::muted(format::elapsed(note.ago_s * 1_000)),
        ]);
    }
    let mut lines = vec![Vec::new()];
    lines.extend(table.spans(style, width));
    lines
}

/// The last line: the Job's own state, then what it is counted from.
fn show_summary_pieces(data: &ShowData, style: Style) -> Vec<Span> {
    let mut facts = vec![data.job.clone(), data.workflow.clone()];
    // Omitted at zero rather than printed as `0 open`, for the reason the Bridge
    // omits its needs-you count: the value of the line is that "needs me" stays
    // a signal (PLAN.md §15.4).
    if data.needs_attention {
        facts.push("needs you".to_string());
    }
    let open = data
        .asked
        .iter()
        .filter(|row| row.answered.is_none())
        .count();
    if open > 0 {
        facts.push(format!("{open} open"));
    }
    facts.push(format::money(data.cost_usd));
    vec![
        Span {
            text: data.state.word().to_string(),
            role: Some(Role::for_job_state(data.state)),
            bold: true,
        },
        Span {
            text: "  ".to_string(),
            role: None,
            bold: false,
        },
        Span {
            text: facts.join(style.between()),
            role: Some(Role::SteelGrey),
            bold: false,
        },
    ]
}

/// How far an entry's own words are set in from the margin.
///
/// **Under its row's second column**, so a sentence reads as hanging off the
/// entry that raised it rather than as a paragraph of its own.
const ASKED_INDENT: usize = 4;

/// Wrapped prose as spans, indented, in one role.
///
/// **No [`Style`], which is the point of putting it here.** Both audiences break
/// at the same words because the wrap is measured in characters and not in
/// anything a style decides — so a sentence that fits on two lines for a person
/// fits on two lines in a pipe (`wrap_prose`).
fn prose(text: &str, role: Role, indent: usize, width: usize) -> Vec<Vec<Span>> {
    wrap_prose(text, width.saturating_sub(indent))
        .into_iter()
        .map(|line| {
            vec![
                spaces(indent),
                Span {
                    text: line,
                    role: Some(role),
                    bold: false,
                },
            ]
        })
        .collect()
}

/// A heading spelled and coloured exactly as a table's own headers are, so a
/// block of prose sits under the same kind of label a column does.
fn block_heading(word: &str) -> Span {
    Span {
        text: word.to_uppercase(),
        role: Some(Role::SteelGrey),
        bold: true,
    }
}

fn spaces(n: usize) -> Span {
    Span {
        text: " ".repeat(n),
        role: None,
        bold: false,
    }
}

/// A token count, short enough to sit beside another one.
///
/// **Rounded down and marked, never rounded to a prettier number.** `119k` for
/// 119,900 says "at least this many", which is the direction a ceiling is read
/// in; a `120k` that was really 119,900 would be a budget line that overstates
/// what has been spent.
fn token_count(n: u64) -> String {
    match n {
        0..=9_999 => n.to_string(),
        10_000..=999_999 => format!("{}k", n / 1_000),
        _ => format!("{}.{}M", n / 1_000_000, (n % 1_000_000) / 100_000),
    }
}

/// Spans painted into one line, trailing padding removed.
///
/// **The trim is at the line and not at the span**, because a padding span in
/// the middle of a row is real spacing and only the run at the end is not. The
/// same rule `Table::render` follows: trailing whitespace is what makes a diff
/// of two captured outputs unreadable (`render/table.rs`).
fn paint_line(spans: &[Span], style: Style) -> String {
    let whole: String = spans.iter().map(|span| span.text.as_str()).collect();
    let keep = whole.trim_end().chars().count();
    let mut out = String::new();
    let mut seen = 0;
    for span in spans {
        if seen >= keep {
            break;
        }
        let text: String = span.text.chars().take(keep - seen).collect();
        seen += span.text.chars().count();
        out.push_str(&match (span.role, span.bold) {
            (Some(role), true) => style.strong(role, &text),
            (Some(role), false) => style.paint(role, &text),
            (None, _) => text,
        });
    }
    out
}

/// `armada fleet board` — the two facts needed to enter a Job.
///
/// **The DETAIL column is fixed here and flexible everywhere else**, because a
/// truncated resume command is not a shorter answer — it is the wrong one, and
/// this whole verb exists to be pasted.
fn board(envelope: &Envelope<BoardData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let table = Table::new(vec![Column::fixed("status"), Column::fixed("detail")])
        .indent(2)
        .row(vec![
            token("worktree", Role::NavalBlue),
            Cell::plain(data.worktree.clone()),
        ])
        .row(vec![
            token("resume", Role::BeaconGreen),
            Cell::plain(data.command.clone()),
        ]);

    let mut out = table.render(style, width);
    out.push('\n');
    out.push_str(&summary(
        style,
        envelope.status,
        &[data.job.clone(), format!("branch {}", data.branch)],
    ));
    out
}

/// `armada fleet kill` — what each Job released, and what became of its tree.
fn kill(envelope: &Envelope<KillData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut table = Table::new(columns("job", "detail", true)).indent(2);

    for killed in &data.results {
        table = table.row(vec![
            token(
                "cleaned",
                match killed.error {
                    Some(_) => Role::DistressRed,
                    None => Role::BeaconGreen,
                },
            ),
            Cell::painted(killed.job.clone(), Role::NavalBlue),
            detail_cell(style, Some(&released(style, killed))),
            time_cell(None),
        ]);
        table = table.row(vec![
            token(killed.worktree.word(), disposition_role(killed.worktree)),
            Cell::painted(killed.job.clone(), Role::NavalBlue),
            detail_cell(style, Some(&format!("worktree {}", killed.worktree_path))),
            time_cell(None),
        ]);
        table = table.row(vec![
            token(killed.branch.word(), disposition_role(killed.branch)),
            Cell::painted(killed.job.clone(), Role::NavalBlue),
            detail_cell(style, Some(&format!("branch {}", killed.branch_name))),
            time_cell(None),
        ]);
    }

    let mut out = table.render(style, width);
    if table.is_empty() {
        out.push_str("  no Jobs to kill\n");
    }
    out.push('\n');
    out.push_str(&summary(
        style,
        envelope.status,
        &[
            format::count(data.results.len(), "job"),
            // **The transcript is not deleted, and the line says so.** It lives
            // under ~/.claude/projects/ and is the record of what happened
            // (`commands/fleet/kill.md`).
            "transcripts kept".to_string(),
        ],
    ));
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error, style));
    }
    out
}

/// What one `kill` reclaimed, in one cell.
fn released(style: Style, killed: &armada_core::envelope::Killed) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (count, noun) in [
        (killed.released.containers, "container"),
        (killed.released.processes, "process"),
        (killed.released.networks, "network"),
        (killed.released.volumes, "volume"),
        (killed.released.images, "image"),
    ] {
        if count > 0 {
            parts.push(format::count(count, noun));
        }
    }
    if let Some(block) = killed.port_block {
        parts.push(format!("ports {}", style.span(block.from, block.to)));
    }
    if parts.is_empty() {
        // Stated rather than left blank: "it owned nothing" and "nobody looked"
        // read identically otherwise, and only one of them is a guarantee.
        parts.push("owned nothing".to_string());
    }
    parts.join(", ")
}

/// `armada fleet inbox` — what the fleet needs from you.
fn inbox(envelope: &Envelope<InboxData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut table = Table::new(columns("job", "detail", true)).indent(2);

    for row in &data.results {
        table = table.row(vec![
            token(
                &row.kind,
                // **Grey the moment it stops wanting you**, answered or closed
                // alike. A row that has ended reading in the same alarm colour
                // as a live question is the diluted signal PLAN.md §15.4 is
                // about, and it is how five entries against two dead Jobs went
                // on looking urgent.
                match (row.is_open(), row.kind.as_str()) {
                    (false, _) => Role::SteelGrey,
                    (true, "blocked") => Role::DistressRed,
                    (true, _) => Role::FlareOrange,
                },
            ),
            Cell::painted(row.job.clone(), Role::NavalBlue),
            detail_cell(style, Some(row.body.as_str())),
            Cell::muted(format::elapsed(row.waiting_s * 1_000)),
        ]);
    }

    let mut out = table.render(style, width);
    if table.is_empty() {
        // **An empty inbox is a normal state, not a failure**
        // (`commands/fleet/inbox.md`), so it is said in words.
        out.push_str("  nothing waiting\n");
    }
    out.push('\n');
    // **The action is offered only when something can take it**
    // (`docs/reserved/005-inbox-label-not-identity.md`). This line used to be
    // unconditional, so an inbox whose every entry belonged to a Job that had
    // ended still told the reader to answer one — an instruction that could
    // only fail, printed under a table of things that had already finished.
    let mut facts = vec![format!("{} open", data.open)];
    if data.open > 0 {
        facts.push("armada fleet answer <job> \"…\"".to_string());
    }
    out.push_str(&summary(style, envelope.status, &facts));
    out
}

/// `armada fleet answer` — the entry you closed, and what the Job did next.
fn answer(envelope: &Envelope<AnswerData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let table = Table::new(columns("job", "detail", true))
        .indent(2)
        .row(vec![
            token("answered", Role::BeaconGreen),
            Cell::painted(data.job.clone(), Role::NavalBlue),
            detail_cell(style, Some(&data.answer)),
            // **No time, because nothing was waited for.** An answer starts a
            // turn and returns; what it costs lands in the transcript and is
            // read by `armada fleet ls`.
            time_cell(None),
        ]);

    let mut out = table.render(style, width);
    out.push('\n');
    out.push_str(&job_summary(
        style,
        data.state,
        &[
            data.job.clone(),
            // **The budget was not reset**, and the line is where a reader sees
            // that: an answer is a continuation rather than a new run.
            format!(
                "{} remaining",
                format::count(data.budget_remaining.iterations as usize, "iteration")
            ),
        ],
    ));
    out
}

/// `armada fleet pause` — the Job that was held, and the Drone that stopped.
fn pause(envelope: &Envelope<PauseData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let table = Table::new(columns("job", "detail", true))
        .indent(2)
        .row(vec![
            token("paused", Role::SignalAmber),
            Cell::painted(data.job.clone(), Role::NavalBlue),
            detail_cell(
                style,
                Some(&match data.stopped {
                    Some(pgid) => format!("stopped the Drone, group {pgid}"),
                    // **Ordinary rather than a failure.** A Job between turns
                    // has no live Drone, and holding it is still a thing a
                    // person can ask for.
                    None => "no Drone was running".to_string(),
                }),
            ),
            time_cell(None),
        ]);

    let mut out = table.render(style, width);
    out.push('\n');
    out.push_str(&job_summary(
        style,
        data.state,
        &[
            data.job.clone(),
            // **What it has spent, not what it will**: the worktree, the branch
            // and the port block are all still held, which is the difference
            // between pausing and killing.
            format!("{} spent", format::money(data.spend.cost_usd)),
            "worktree kept".to_string(),
        ],
    ));
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error, style));
    }
    out
}

/// `armada fleet resume` — the Job that was continued, and its new Drone.
fn resume(envelope: &Envelope<ResumeData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let table = Table::new(columns("job", "detail", true))
        .indent(2)
        .row(vec![
            token("resumed", Role::BeaconGreen),
            Cell::painted(data.job.clone(), Role::NavalBlue),
            detail_cell(
                style,
                Some(&match data.pgid {
                    Some(pgid) => format!("started a Drone, group {pgid}"),
                    None => "started a Drone".to_string(),
                }),
            ),
            // **No time, for `answer`'s reason**: a resume starts a turn and
            // returns rather than waiting for one.
            time_cell(None),
        ]);

    let mut out = table.render(style, width);
    out.push('\n');
    out.push_str(&job_summary(
        style,
        data.state,
        &[
            data.job.clone(),
            // **The budget was not reset**, and this is where a reader sees it:
            // a resume continues the same session.
            format!(
                "{} remaining",
                format::count(data.budget_remaining.iterations as usize, "iteration")
            ),
        ],
    ));
    out
}

/// `armada fleet reap --dry-run` — every Job a reap offers, and what each holds.
///
/// **What it is holding is the column that makes this readable**, and it is why
/// the preview exists at all: a port block held by a Job whose Drone died months
/// ago is a span nothing can use and nothing else reports.
///
/// **`take` and `keep` are words rather than a tick and a cross**, for the rule
/// every table here follows: a glyph that only appears at a terminal gives the
/// two audiences different shapes.
fn reap_plan(envelope: &Envelope<ReapPlanData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut table = Table::new(vec![
        Column::fixed("status"),
        Column::fixed("job"),
        // **The uuid, because a name is not unique.** Two Jobs can share one —
        // `name_is_taken` only refuses to reuse a *live* Job's name — and it is
        // also what `--job` takes, so the row carries the handle the next
        // command needs.
        Column::fixed("uuid"),
        Column::fixed("state"),
        Column::flexible("holding"),
        Column::fixed("spent").right(),
    ])
    .indent(2);

    for row in &data.results {
        table = table.row(vec![
            match row.selected {
                true => token("take", Role::FlareOrange),
                // **Listed and left alone.** A state you might still act on is
                // not garbage, and hiding it would make the preview a shorter
                // list of a different question.
                false => token("keep", Role::SteelGrey),
            },
            Cell::painted(row.job.clone(), Role::NavalBlue),
            Cell::muted(armada_fleet::jobs::short(&row.uuid).to_string()),
            job_state(row.state),
            detail_cell(style, Some(&holding(style, row))),
            Cell::muted(match row.cost_usd > 0.0 {
                true => format::money(row.cost_usd),
                false => style.nothing().to_string(),
            }),
        ]);
    }

    let mut out = table.render(style, width);
    if table.is_empty() {
        out.push_str("  nothing to reap\n");
    }
    out.push('\n');
    out.push_str(&summary(
        style,
        envelope.status,
        &[
            format::count(data.results.len(), "job"),
            format!("{} to take", data.selected),
            // **Said every time, because a preview that reaped would be the
            // destructive path it was previewing** (`ARCHITECTURE.md` §2.1.2).
            "nothing reaped".to_string(),
        ],
    ));
    out
}

/// What one reap candidate is still holding, in one cell.
fn holding(style: Style, row: &armada_core::envelope::ReapCandidate) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(block) = row.port_block {
        parts.push(format!("ports {}-{}", block.from, block.to));
    }
    if row.worktree_exists {
        parts.push(format!("worktree {}", row.worktree_path));
    }
    parts.push(format!("branch {}", row.branch));
    parts.join(style.between())
}

// ---------------------------------------------------- M3: Armada's own failures

/// The colour a recorded failure's state is drawn in.
///
/// **`FIXING` is not green.** A Job on a bug is work in flight, not a bug fixed,
/// and green there would say the opposite of what the row means — the same
/// reason `armada fleet show` paints a departed Drone grey once its Job is over
/// and red while it is not.
fn failure_state(state: FailureState) -> Cell {
    token(
        state.word(),
        match state {
            FailureState::Open => Role::FlareOrange,
            FailureState::Fixing => Role::NavalBlue,
            FailureState::Cleared => Role::SteelGrey,
        },
    )
}

/// What a failure's row says, in the one place both listings read it from.
///
/// **The class leads and the count is second**, because the column truncates
/// from the right: the two facts a reader triages on have to survive a narrow
/// terminal, and the message is the part they can widen the window for.
///
/// **`x4` rather than `×4`**, ASCII, so the agent reading stdout and the person
/// at the terminal are given the same bytes in this cell (PLAN.md §3.1.1) — the
/// pair may differ in styling and never in width.
/// **Shared with the navigable listing**, which is why it is not private: a
/// person picking a row off `armada failures` at a terminal and the same person
/// reading it through a pipe are looking at one sentence, not two that agree
/// today.
/// **The lead is the class when Armada assigned one and the origin when it did
/// not.** A filed report has no class — Armada did not notice, so it attributed
/// nothing (`armada_core::failure::Origin`) — and the cell reads `reported,
/// the dry-run said CREATED and made nothing`. That is one column doing one
/// job: it says where the row came from, which is the first thing a reader
/// triaging a mixed list needs and the only thing the two halves differ on.
pub(crate) fn failure_detail(entry: &FailureEntry) -> String {
    let lead = entry.class.map_or_else(
        || entry.origin.word().to_string(),
        |class| class.to_string(),
    );
    match entry.count {
        0 | 1 => format!("{lead}, {}", entry.message),
        n => format!("{lead} x{n}, {}", entry.message),
    }
}

/// `armada failures`, and `armada failures clear` — Armada's own failures.
fn failures(envelope: &Envelope<FailuresData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut table = Table::new(columns("id", "detail", true)).indent(2);
    for entry in &data.results {
        table = table.row(vec![
            failure_state(entry.state),
            Cell::painted(entry.id.clone(), Role::NavalBlue),
            detail_cell(style, Some(&failure_detail(entry))),
            Cell::muted(format::elapsed(entry.age_s * 1_000)),
        ]);
    }

    let mut out = table.render(style, width);
    if table.is_empty() {
        // **An empty log is what a machine looks like when nothing has gone
        // wrong**, so it is said in words rather than left as a blank table.
        out.push_str("  nothing recorded\n");
    }
    out.push('\n');

    // **"2 failures" is wrong the moment a filed report is in the list**, and
    // wrong in the direction that matters: a person scanning the summary would
    // read their own report back as something Armada broke on. So the count
    // splits as soon as both kinds are present, and stays one word when only one
    // kind is — a machine that has never had a report filed on it reads exactly
    // as it did before.
    let reported = data
        .results
        .iter()
        .filter(|entry| entry.origin == armada_core::failure::Origin::Reported)
        .count();
    let mut facts = match reported {
        0 => vec![format::count(data.results.len(), "failure")],
        all if all == data.results.len() => vec![format::count(all, "report")],
        some => vec![
            format::count(data.results.len() - some, "failure"),
            format!("{some} reported"),
        ],
    };
    if data.open > 0 {
        facts.push(format!("{} open", data.open));
    }
    if !data.results.is_empty() {
        facts.push("armada failures show <id>".to_string());
    }
    out.push_str(&summary(style, envelope.status, &facts));
    out
}

/// What `armada report` gathered, as rows on the facts table the entry already
/// draws.
///
/// **Rows rather than a second table**, because every one of them is a single
/// fact about the machine the report was filed on — the same kind of thing
/// `typed` and `in` already are. The runs are the exception and get their own
/// table; see the call site.
///
/// **An absent diagnostic is printed rather than omitted.** "`claude --version`
/// did not answer" is a finding, and a row that vanished when the answer was
/// missing would make the most interesting case the invisible one.
fn diagnosed(
    mut facts: Table,
    diagnostics: &armada_core::failure::Diagnostics,
    style: Style,
) -> Table {
    facts = facts.row(vec![
        token("ran", Role::SteelGrey),
        Cell::muted("armada"),
        detail_cell(style, Some(&diagnostics.armada)),
    ]);
    facts = facts.row(vec![
        token("ran", Role::SteelGrey),
        Cell::muted("claude"),
        detail_cell(
            style,
            Some(diagnostics.claude.as_deref().unwrap_or("did not answer")),
        ),
    ]);
    facts = facts.row(vec![
        token("on", Role::SteelGrey),
        Cell::muted("system"),
        detail_cell(style, Some(&diagnostics.system)),
    ]);
    facts = facts.row(vec![
        token("in", Role::SteelGrey),
        Cell::muted("workspace"),
        detail_cell(
            style,
            Some(&match (&diagnostics.workspace, diagnostics.manifest) {
                (Some(at), true) => format!("{at}, with an armada.yml"),
                (Some(at), false) => format!("{at}, with no armada.yml"),
                (None, _) => "no workspace here".to_string(),
            }),
        ),
    ]);
    if !diagnostics.doctor.is_empty() {
        facts = facts.row(vec![
            token("said", Role::FlareOrange),
            Cell::muted("doctor"),
            detail_cell(style, Some(&diagnostics.doctor.join(style.between()))),
        ]);
    }
    if !diagnostics.failures.is_empty() {
        facts = facts.row(vec![
            token("open", Role::FlareOrange),
            Cell::muted("failures"),
            detail_cell(style, Some(&diagnostics.failures.join(style.between()))),
        ]);
    }
    if !diagnostics.jobs.is_empty() {
        facts = facts.row(vec![
            token("in", Role::NavalBlue),
            Cell::muted("flight"),
            detail_cell(style, Some(&diagnostics.jobs.join(style.between()))),
        ]);
    }
    facts
}

/// `armada failures show <id>` — one failure, whole.
///
/// **It reprints the failure exactly as the terminal printed it**, through the
/// same [`error_lines`] every failing verb ends in. A second phrasing of one
/// error is two vocabularies for one thing, and the whole promise of the record
/// is that what you come back to is what you saw.
fn failure(envelope: &Envelope<FailureData>, style: Style, width: usize) -> String {
    let Some(entry) = envelope.data.results.first() else {
        return summary(style, envelope.status, &[]);
    };

    let identity = Table::new(columns("id", "detail", true))
        .indent(2)
        .row(vec![
            failure_state(entry.state),
            Cell::painted(entry.id.clone(), Role::NavalBlue),
            // **The row from the listing, unchanged.** The table put the reader
            // here; opening with the same row in the same shape is what makes
            // the rest of the page read as *more about this* rather than as a
            // second report — the rule `armada fleet show` already follows.
            detail_cell(style, Some(&failure_detail(entry))),
            Cell::muted(format::elapsed(entry.age_s * 1_000)),
        ]);
    let mut out = identity.render(style, width);
    out.push('\n');

    // **A failure is reprinted; a report is quoted.** There is no envelope to
    // reprint for a filing — Armada did not fail, so there is no class, no
    // `where` and no next action — and pushing the person's sentence through
    // `error_lines` would dress their words up as Armada's own report of an
    // error that never happened.
    match entry.class {
        Some(class) => out.push_str(&error_lines(
            &ArmadaError {
                class,
                r#where: entry.r#where.clone(),
                message: entry.message.clone(),
                next_action: entry.next.clone(),
            },
            style,
        )),
        None => {
            out.push_str(&format!(
                "{}\n",
                style.paint(Role::SteelGrey, "  what was reported:")
            ));
            for line in wrap_prose(&entry.message, width.saturating_sub(4)) {
                out.push_str(&format!("    {line}\n"));
            }
        }
    }
    out.push('\n');

    let mut facts = Table::new(columns("fact", "detail", false)).indent(2);
    facts = facts.row(vec![
        token("seen", Role::SteelGrey),
        Cell::muted("count"),
        detail_cell(
            style,
            Some(&match entry.count {
                0 | 1 => format!("once, at {}", entry.last_at),
                n => format!("{n} times, {} to {}", entry.first_at, entry.last_at),
            }),
        ),
    ]);
    facts = facts.row(vec![
        token("typed", Role::SteelGrey),
        Cell::muted("command"),
        detail_cell(style, Some(&entry.argv)),
    ]);
    facts = facts.row(vec![
        token("in", Role::SteelGrey),
        Cell::muted("directory"),
        detail_cell(style, Some(&entry.cwd)),
    ]);
    if let Some(job) = &entry.job {
        facts = facts.row(vec![
            token("job", Role::NavalBlue),
            Cell::muted("spawned"),
            detail_cell(style, Some(job)),
        ]);
    }
    if let Some(diagnostics) = &entry.diagnostics {
        facts = diagnosed(facts, diagnostics, style);
    }
    out.push_str(&facts.render(style, width));
    out.push('\n');

    // **The runs are their own table because they are the attachment**, not a
    // fact about the filing. They are the answer to *"the thing that just
    // happened"* — the reason the ring buffer exists — and folding them into
    // the facts above as one comma-joined cell would lose the one column that
    // makes them readable: whether each one said it worked.
    if let Some(diagnostics) = &entry.diagnostics {
        if !diagnostics.recent.is_empty() {
            out.push_str(&format!(
                "{}\n",
                style.paint(Role::SteelGrey, "  what was run before it:")
            ));
            let mut runs = Table::new(columns("run", "detail", true)).indent(2);
            let newest = diagnostics.recent.first().map_or(0, |run| run.at_ms);
            for run in &diagnostics.recent {
                runs = runs.row(vec![
                    token(
                        run.word(),
                        match run.exit {
                            0 => Role::SteelGrey,
                            _ => Role::FlareOrange,
                        },
                    ),
                    Cell::painted(run.verb.clone(), Role::NavalBlue),
                    detail_cell(style, Some(&run.argv)),
                    Cell::muted(format::elapsed(newest.saturating_sub(run.at_ms))),
                ]);
            }
            out.push_str(&runs.render(style, width));
            out.push('\n');
        }
    }

    // **What would be sent, before it is sent.** The task leaves this machine
    // when a Job is spawned on it, so the one place to read it is here.
    out.push_str(&format!(
        "{}\n",
        style.paint(Role::SteelGrey, "  the task a Job would be given:")
    ));
    // **A line that fits is printed verbatim, and only a long one is wrapped.**
    // The task is a block with an indented, column-aligned envelope in the middle
    // of it, and [`wrap_prose`] splits on whitespace — so wrapping every line
    // unconditionally would strip that indent and collapse that alignment. This
    // is the one screen whose whole purpose is to show what will be sent, so what
    // can be shown unaltered is.
    let room = width.saturating_sub(4);
    for paragraph in envelope.data.task.lines() {
        if paragraph.trim().is_empty() {
            out.push('\n');
            continue;
        }
        if term::display_width(paragraph) <= room {
            out.push_str(&format!("    {paragraph}\n"));
            continue;
        }
        for line in wrap_prose(paragraph, room) {
            out.push_str(&format!("    {line}\n"));
        }
    }
    out.push('\n');

    out.push_str(&summary(
        style,
        envelope.status,
        &[
            match &entry.job {
                Some(job) => format!("armada fleet show {job}"),
                None => format!("armada failures fix {}", entry.id),
            },
            format!("armada failures clear {}", entry.id),
        ],
    ));
    out
}

// ------------------------------------------------------------- M2: the machine
// and the guild

/// The `STATUS · CHECK · DETAIL · TIME` table both machine verbs draw.
///
/// **`armada init` and `armada doctor` share it because they are asking one
/// question** — what is the state of this machine — and a reader who has met
/// one has met the other.
///
/// The `TIME` column is declared and never filled, because **nothing here is
/// timed** — and `render/table.rs` therefore drops it. It stays declared rather
/// than being left out at this call site so that the one rule about empty
/// columns lives in the table and not in each verb's opinion of its own columns.
fn machine_table(rows: &[Finding], style: Style) -> Table {
    let mut table = Table::new(columns("check", "detail", true)).indent(2);
    for row in rows {
        table = table.row(vec![
            token(row.status.word(), Role::for_health(row.status)),
            Cell::plain(row.check.clone()),
            detail_cell(style, Some(row.detail.as_str())),
            time_cell(None),
        ]);
    }
    table
}

/// A closed question **printed rather than drawn** — the form an agent reading
/// stdout gets, and the one a terminal that refuses raw mode falls back to.
///
/// It replaces a one-line menu of numbers that ended at a bare caret. That form
/// was compact and it was the thing a real reader could not interpret: *"has to
/// have a better UI for selecting options instead of just stopping and me
/// guessing it's asking me to input a number."* One option per line leaves room
/// for what each one does, and the last line says what is expected and what
/// enter takes.
///
/// Ends at the caret with a space and no newline, because that is where the
/// cursor sits and the terminal's own echo completes the line.
pub fn choice_list(question: &str, options: &[Choice], default: usize, style: Style) -> String {
    let mut out = format!("{}\n", style.paint(Role::SignalAmber, question));
    let mut table = Table::new(vec![
        Column::fixed(""),
        Column::fixed(""),
        Column::flexible(""),
    ])
    .headerless()
    .indent(2);
    for (index, choice) in options.iter().enumerate() {
        table = table.row(vec![
            Cell::painted((index + 1).to_string(), Role::NavalBlue),
            Cell::plain(choice.label.clone()),
            Cell::muted(choice.aside.clone()),
        ]);
    }
    out.push_str(&table.render(style, Terminal::FALLBACK_WIDTH));
    out.push_str("  ");
    out.push_str(&style.paint(
        Role::SteelGrey,
        &format!("a number, or enter for {default}"),
    ));
    out.push_str("  ");
    out.push_str(&style.paint(Role::SteelGrey, style.caret()));
    out.push(' ');
    out
}

/// What was picked, on one line, for the scrollback.
///
/// **A selector that clears itself leaves no record of the decision.** The
/// widget's viewport is gone the moment it closes; this is the line that stays,
/// and it is also how [`machine_init`] replays the choice into its transcript —
/// which is what makes the live conversation and the record the same sentence
/// rather than merely similar ones.
pub fn chosen_line(question: &str, label: &str, chosen: usize, style: Style) -> String {
    format!(
        "{}  {} {}\n",
        style.paint(Role::SignalAmber, question),
        style.paint(Role::SteelGrey, style.caret()),
        style.paint(Role::RadarCyan, &format!("{chosen} {label}")),
    )
}

/// How far every line of a question is indented, past the `n/7`.
const ASK_INDENT: usize = 5;

/// One interview question — **live**, as it is put to a person.
///
/// ```text
/// 2/7  When is work actually finished?
///      What must be true before an agent tells you it is done: tests
///      passing, a review, a branch, a changelog entry. → expectations.md
///
///      now  Tests pass and the diff has been read by someone.
///      enter keeps what import found  ›
/// ```
///
/// **Four things a real first run said were missing, and one it did not have to
/// say.** The purpose line, because the prompt alone did not say what answer was
/// wanted. The file, because each question writes one and knowing which changes
/// the answer you give. The `now` line, because *(enter to keep what import
/// found)* over an empty prompt is a default you cannot see. And the blank line
/// above it, because seven questions with no space between them ran together.
///
/// The one it did not have to say is that everything after the number is
/// indented to line up under the prompt: it all belongs to the question rather
/// than to the count.
pub fn interview_prompt(
    asked: &armada_core::envelope::Asked,
    style: Style,
    width: usize,
) -> String {
    let pad = " ".repeat(ASK_INDENT);
    let mut out = format!(
        "{}  {}\n",
        style.strong(Role::SignalAmber, &format!("{}/{}", asked.number, asked.of)),
        style.strong(Role::Foreground, &asked.prompt),
    );

    // The purpose and the file it writes read as one sentence, so they wrap as
    // one: the file stranded on a line of its own would look like a fix line.
    //
    // **Written out rather than joined with `→`.** The arrow is one column for a
    // person and two for an agent (`style.rs`), so a break point computed from
    // it would fall between different words in the two renders — and "same
    // columns, same order, same words" is the property the pair of fixtures
    // exists to prove.
    let sentence = format!("{} Writes {}.", asked.purpose, asked.writes);
    for line in wrap_prose(&sentence, width.saturating_sub(ASK_INDENT)) {
        out.push_str(&pad);
        out.push_str(&style.paint(Role::SteelGrey, &line));
        out.push('\n');
    }

    // **A prose question shows its default by holding it, not by quoting it.**
    //
    // There used to be a `now …` line here for every question. On a prose one it
    // was wrong twice: it truncated, so a long imported fragment could not be
    // read — which is the only reason to show it — and the text area drew it a
    // second time in a footer of its own that took no account of wrapping and
    // ran off the edge. The box opens holding the value instead, where it is
    // visible in full, scrollable and directly editable (`ask::editor`).
    //
    // The line stays for the two short structured questions, whose defaults fit
    // in it, and for a prose question with nothing to pre-fill — `nothing of
    // yours yet` rather than a `now` line that simply vanished, which would
    // leave the reader with `esc keeps it as it was` over nothing.
    let standing = match &asked.standing {
        Some(_) if asked.prose => String::new(),
        Some(standing) => standing.clone(),
        None if asked.prose => "nothing of yours yet".to_string(),
        None => String::new(),
    };
    if !standing.is_empty() {
        out.push('\n');
        out.push_str(&pad);
        out.push_str(&style.paint(Role::SteelGrey, "now  "));
        out.push_str(&style.paint(
            Role::RadarCyan,
            &term::truncate(&standing, width.saturating_sub(ASK_INDENT + 5)),
        ));
        out.push('\n');
    }

    out.push_str(&pad);
    out.push_str(&style.paint(Role::SteelGrey, &keys(asked, style)));
    // **Prose ends the block; a line ends at the caret.** The text area draws
    // its own frame under this and puts the cursor inside it, so a caret here
    // would be a second place to type.
    if asked.prose {
        out.push('\n');
    } else {
        out.push_str("  ");
        out.push_str(&style.paint(Role::RadarCyan, style.caret()));
        out.push(' ');
    }
    out
}

/// What to say when the text area could not be opened.
///
/// **The question has already been printed by then**, with the box's keys under
/// it, and the box is not coming — so this both corrects the instruction and
/// says why there is nothing to type into. Silence there would leave a person
/// looking at `ctrl-d saves` with no box and no caret, which is worse than the
/// single-line prompt this falls back to.
pub fn no_text_area(style: Style, width: usize) -> String {
    let pad = " ".repeat(ASK_INDENT);
    let mut out = format!(
        "{pad}{}\n",
        style.paint(
            Role::FlareOrange,
            &term::truncate(
                "this terminal will not open the box — one line, then enter",
                width.saturating_sub(ASK_INDENT),
            ),
        )
    );
    out.push_str(&pad);
    out.push_str(&style.paint(Role::RadarCyan, style.caret()));
    out.push(' ');
    out
}

/// The line above the text area when a **file** is what is being edited.
///
/// **It names the file and it names the keys**, because `armada guild edit` puts
/// no question — there is no `2/5` and no prompt above the box, so without this
/// a person would be handed a frame full of their own `SKILL.md` with nothing
/// saying how to leave it. The keys are the interview's prose keys, quoted from
/// the same list, so the two places this widget appears cannot drift apart.
pub fn editing(title: &str, style: Style, width: usize) -> String {
    let pad = " ".repeat(ASK_INDENT);
    let mut out = format!(
        "{pad}{}\n",
        style.strong(
            Role::SignalAmber,
            &term::truncate(title, width.saturating_sub(ASK_INDENT)),
        )
    );
    out.push_str(&format!(
        "{pad}{}\n\n",
        style.paint(
            Role::SteelGrey,
            &prose_keys("saves", "leaves it as it was", style.between()),
        )
    ));
    out
}

/// One guild item's content — `armada guild show`, and the terminal's *view*.
///
/// **The whole file, indented and unwrapped.** A viewer that wrapped a
/// `SKILL.md` to the terminal would show something that is not what is on disk,
/// and the reader is here precisely to see what is on disk. Long lines overhang,
/// which is the same choice [`wrap_prose`] makes about a word it cannot break.
///
/// **One renderer, and the terminal goes through it too.** `guild ls` at a
/// terminal draws a file through this function by building the same envelope
/// `guild show` returns, so what a person sees and what a pipe carries cannot be
/// two layouts maintained separately.
fn guild_item(envelope: &Envelope<GuildItemData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut out = format!(
        "  {}\n\n",
        style.strong(
            Role::SignalAmber,
            &term::truncate(&data.item.opens, width.saturating_sub(2))
        )
    );
    for line in data.body.lines() {
        // **A blank line stays blank rather than becoming two spaces.** The
        // indent is there to set the file apart from the frame around it, and
        // trailing whitespace on an empty line is invisible until somebody
        // copies the output into a diff.
        if line.is_empty() {
            out.push('\n');
            continue;
        }
        out.push_str(&format!("  {}\n", style.paint(Role::Foreground, line)));
    }
    if data.body.is_empty() {
        out.push_str(&format!(
            "  {}\n",
            style.paint(Role::SteelGrey, "nothing in it")
        ));
    }
    out.push('\n');

    // **Where, then what it is, then how big** — the same `at`-first summary
    // every other guild verb ends on. The kind is here because a reader who
    // arrived through `show <name>` never saw the listing row that carries it;
    // the *detail* is not, because it is a sentence and a fact list is not
    // where a sentence goes.
    out.push_str(&summary(
        style,
        envelope.status,
        &[
            data.at.clone(),
            data.item.kind.clone(),
            format::bytes(data.item.bytes),
        ],
    ));
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error, style));
    }
    out
}

/// The keys a question accepts, named for the way it is being answered.
///
/// **The question knows what its default is; only this knows what takes it.**
/// `enter` accepts the default at a single-line prompt and inserts a newline in
/// a text area, so a hint that said `enter keeps …` in both would be wrong in
/// one of them — and it is the one where the answer is three paragraphs long.
///
/// A prose hint says `esc keeps it as it was` rather than naming the default,
/// because the default is in the box in front of you: naming it as well would be
/// the preview this widget was pre-filled to remove.
fn keys(asked: &armada_core::envelope::Asked, style: Style) -> String {
    if asked.prose {
        prose_keys("saves", "keeps it as it was", style.between())
    } else {
        format!("enter keeps {}", asked.keeps)
    }
}

/// The three keys **every** text area in Armada accepts, and the one place they
/// are named.
///
/// ```text
/// enter for a new line · ctrl-d saves · esc keeps it as it was
/// ```
///
/// **Three surfaces put a box in front of somebody and all three must name the
/// same chords.** The interview's prose questions, `armada guild edit`, and the
/// Bridge's compose box — and the Bridge's was the one that named none of them,
/// which is exactly what a first reader reported: *"there is no help text, so I
/// didn't really know what to do … I guessed with control-d."* A second
/// convention would have been worse than either, so there is one, quoted from
/// here.
///
/// **What the two ways out *mean* is the caller's, and only that.** `ctrl-d` in
/// the interview saves a fragment and in the Bridge starts a Job; `esc` there
/// keeps a file as it was and here starts nothing. The keys never differ, which
/// is the half a reader learns once.
///
/// `between` is the caller's spacing rather than [`Style::between`] taken here:
/// a prompt block separates with a middle dot, and the Bridge's on-screen key
/// lines use two spaces so that both audiences read one width
/// (`commands/helm/bridge.md`).
pub fn prose_keys(saves: &str, leaves: &str, between: &str) -> String {
    [
        "enter for a new line".to_string(),
        format!("ctrl-d {saves}"),
        format!("esc {leaves}"),
    ]
    .join(between)
}

/// Greedy word wrap. **Not [`wrapped`]**, which spaces a run of items with a
/// separator whose two forms differ in width; this is prose, so both audiences
/// break at the same words and no measurement depends on the style.
fn wrap_prose(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if term::display_width(line) + 1 + term::display_width(word) <= width => {
                line.push(' ');
                line.push_str(word);
            }
            // A word longer than the line gets one of its own and overhangs.
            // Cutting a word is worse than a ragged edge.
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

/// `armada init` — set up **this machine**.
///
/// **The one verb whose render is a transcript**, because it is the one verb
/// that holds a conversation. The preflight table, the one question and what
/// was typed, what import adopted, each interview prompt as it was put, and the
/// verdict. `tests/golden/render/init-machine.plain` is the specification and
/// this follows it.
///
/// **The wordmark is not drawn here**, though `armada init` is one of its two
/// sites (`docs/commands/render.md`). It is drawn at the call site in `main`,
/// for a reason that is about the fixtures rather than about taste: the pair of
/// golden files is rendered at one width for both audiences, and a decoration
/// that appears in only one of them is not a *styling* difference the pair test
/// can express. Every suppression rule still lives in `render::banner`, so the
/// second call site cannot draw it under conditions the first refuses.
fn machine_init(envelope: &Envelope<MachineInitData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut out = machine_table(&data.results, style).render(style, width);

    if let Some(choice) = &data.guild {
        out.push('\n');
        // **The record, not the menu.** Live, the question is a selector drawn
        // on stderr and gone the moment it closes; what belongs in a transcript
        // is which option produced everything under it. Same line the selector
        // itself echoes, so the two cannot drift.
        out.push_str(&chosen_line(
            &choice.question,
            choice
                .options
                .get(choice.chosen.saturating_sub(1))
                .map(String::as_str)
                .unwrap_or_default(),
            choice.chosen,
            style,
        ));
    }

    if !data.imported.is_empty() {
        out.push('\n');
        out.push_str(&format!(
            "  {}\n",
            style.paint(Role::SteelGrey, &data.imported.join(style.between()))
        ));
    }

    // **What is now in effect, on its own line.** Whichever of the three
    // answers brought the guild here, `armada init` ends with it where Claude
    // Code reads it — and a machine that has never seen Armada getting a
    // *working* setup is this verb's whole done-when (`PHASES.md` §8.4).
    if let Some(projected) = &data.projected {
        let mut facts = vec![format!("projected into {}", projected.at)];
        facts.extend(projected.facts.clone());
        out.push_str(&format!(
            "  {}\n",
            style.paint(Role::SteelGrey, &facts.join(style.between()))
        ));
    }

    for asked in &data.asked {
        out.push('\n');
        // **The trailing space goes.** Live, it is where the cursor sits; in
        // the record it would be trailing whitespace, which is what makes a
        // diff of two captured outputs unreadable (`render/table.rs`).
        out.push_str(interview_prompt(asked, style, width).trim_end());
        out.push('\n');
    }

    out.push('\n');
    // **The question counts appear only when there was an interview.** Pulling
    // a guild from a remote asks nothing, and a count under a clone would be
    // describing something that did not happen.
    let mut facts = vec![format!("guild at {}", data.guild_path)];
    if data.questions > 0 {
        facts.extend(interview_facts(data.questions, data.answered));
    }
    out.push_str(&summary(style, envelope.status, &facts));
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error, style));
    }
    out
}

/// `armada doctor` — what this machine is missing.
///
/// **One table per check, not one flat table**, which is the one place this
/// diverges from `armada init` beside it. `init` ticks each check off once, so a
/// flat list *is* the grouping. `doctor` reports a check as many times as it has
/// something to say — `guild` is drift plus one row per fragment still as
/// imported — and a real report came back with three `guild` rows interleaved
/// with `docker` and `manifest.db` and nothing to tell the reader which belonged
/// together.
///
/// The check's name is hoisted out of the rows into a heading, because inside a
/// group it is the same word on every line. What is left needs no column names,
/// which is what [`Table::headerless`] is for, and the status column is given a
/// floor so that the groups line up with each other rather than each measuring
/// its own rows.
///
/// **The `→` line sits under its own row**, not in a block at the bottom. With
/// one problem the two are the same; with three, a list at the end makes the
/// reader match fixes to rows by eye.
fn doctor(envelope: &Envelope<DoctorData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut out = String::new();

    let widest = data
        .results
        .iter()
        .map(|row| row.status.word().len())
        .max()
        .unwrap_or(0);

    for check in checks_in_order(&data.results) {
        out.push_str(&format!("  {}\n", style.strong(Role::SignalAmber, &check)));
        let mut table = Table::new(vec![
            Column::fixed("").at_least(widest),
            Column::flexible(""),
        ])
        .headerless()
        .indent(4);
        for row in data.results.iter().filter(|row| row.check == check) {
            table = table.row_with_note(
                vec![
                    token(row.status.word(), Role::for_health(row.status)),
                    detail_cell(style, Some(row.detail.as_str())),
                ],
                row.remedy
                    .as_deref()
                    .map(|remedy| format!("{} {remedy}", style.arrow())),
            );
        }
        out.push_str(&table.render(style, width));
        out.push('\n');
    }

    out.push_str(&match data.headline {
        Some(word) => headline(style, word, &data.tally),
        None => summary(style, envelope.status, &data.tally),
    });
    out
}

/// Every check named in the rows, once each, in the order they first appear.
///
/// Order comes from the rows rather than from a list here, so a check added to
/// `verbs::doctor` appears without this file being told about it — and the run
/// order, which is deliberate, is the reading order.
fn checks_in_order(rows: &[Finding]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for row in rows {
        if !seen.contains(&row.check) {
            seen.push(row.check.clone());
        }
    }
    seen
}

/// `armada guild pull` and `armada guild push`.
///
/// **The rows describe the change set, and `applied` says whether any of it
/// landed.** On a divergence nothing is written — `guild/pull.md` states that
/// as an exit code — so the rows are what is *waiting*, and the summary line is
/// where a reader is told which of the two they are looking at. Reading the
/// rows as "what happened" when nothing happened is the one misreading this
/// layout can produce, and it is why `applied` exists in the envelope at all.
fn guild_sync(envelope: &Envelope<GuildSyncData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut table = Table::new(columns("item", "detail", true)).indent(2);
    for row in &data.results {
        table = table.row(vec![
            token(row.status.word(), Role::for_sync(row.status)),
            Cell::plain(row.item.clone()),
            detail_cell(style, Some(row.detail.as_str())),
            time_cell(None),
        ]);
    }
    let mut out = table.render(style, width);
    if !table.is_empty() {
        out.push('\n');
    }

    let conflicts = data
        .results
        .iter()
        .filter(|row| row.status == armada_core::envelope::Sync::Conflict)
        .count();

    out.push_str(&match data.headline {
        Some(word) => {
            // **The remedy is on the summary line rather than under it**, which
            // is the one place this differs from `doctor`: there is exactly one
            // thing to do about a conflicted guild, and a `→` line under a
            // one-item summary is a second line saying the same thing.
            let mut facts = Vec::new();
            if conflicts > 0 {
                facts.push(format::count(conflicts, "conflict"));
            }
            facts.extend(kept_facts(data.projected.as_ref()));
            if !data.applied {
                facts.push("resolve in ~/.armada/guild".to_string());
                facts.push("then armada guild push".to_string());
            }
            headline(style, word, &facts)
        }
        None => summary(style, envelope.status, &sync_facts(data)),
    });
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error, style));
    }
    out
}

/// What a sync that worked has to report: how far it moved, and where to.
fn sync_facts(data: &GuildSyncData) -> Vec<String> {
    let mut facts = Vec::new();
    if data.behind > 0 {
        facts.push(format!("pulled {}", format::count(data.behind, "commit")));
    }
    if data.ahead > 0 {
        facts.push(format!("pushed {}", format::count(data.ahead, "commit")));
    }
    if facts.is_empty() {
        facts.push("already in step".to_string());
    }
    match &data.remote {
        Some(remote) => facts.push(remote.clone()),
        // Sync off is the documented default and not a broken state, so it is
        // stated rather than left as an absence a reader has to notice.
        None => facts.push("no remote, export still works".to_string()),
    }
    // **A pulled guild that has not been projected is a guild that has not
    // taken effect**, and the gap between the two is a confusing hour
    // (`guild/pull.md`). The fact is stated on the line a reader is already
    // reading rather than left to `armada doctor` to discover later.
    if let Some(projected) = &data.projected {
        facts.push(format!("projected {}", projected.facts.join(", ")));
    }
    facts
}

/// What a projection that left something alone has to say on a summary line it
/// is sharing with another verb.
///
/// **The file is not named here.** One file is a name; a guild's worth is a
/// paragraph, and the verb that exists to list them is one word long.
fn kept_facts(projected: Option<&Projection>) -> Vec<String> {
    let Some(projected) = projected.filter(|done| done.kept > 0) else {
        return Vec::new();
    };
    vec![
        format!(
            "{} left as yours in {}",
            format::count(projected.kept, "file"),
            projected.at
        ),
        "armada guild project shows which".to_string(),
    ]
}

/// `armada guild project`, with or without `--remove`.
///
/// The same table `guild pull` draws, over a different tree — one row per area
/// of `~/.claude/`, in the order the `STATUS` column reads. **The reason it is
/// the same table is that it is the same question**: what moved, and what did
/// not.
fn guild_project(envelope: &Envelope<Projection>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut table = Table::new(columns("item", "detail", true)).indent(2);
    for row in &data.results {
        table = table.row(vec![
            token(row.status.word(), Role::for_sync(row.status)),
            Cell::plain(row.item.clone()),
            detail_cell(style, Some(row.detail.as_str())),
            time_cell(None),
        ]);
    }
    let mut out = table.render(style, width);
    if !table.is_empty() {
        out.push('\n');
    }

    // **Where, then what** — the same shape `guild init`'s summary has, because
    // a reader arriving at either wants the place before the count.
    let mut facts = vec![data.at.clone()];
    facts.extend(data.facts.clone());
    out.push_str(&match data.headline {
        // **A file left as yours is not a failure and it does need a person.**
        // It is the one outcome of this verb a reader has to decide about: the
        // guild has moved on and this machine's copy has not, and only he
        // knows which he wants. **The remedy is on the row rather than on this
        // line**, because it is the row that names the area it applies to —
        // unlike a conflicted pull, where there is one thing to do about the
        // whole guild.
        Some(word) => headline(style, word, &facts),
        None => summary(style, envelope.status, &facts),
    });
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error, style));
    }
    out
}

/// `armada guild init`.
fn guild_init(envelope: &Envelope<GuildInitData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut table = Table::new(columns("step", "detail", true)).indent(2);
    table = table.row(vec![
        token("imported", Role::BeaconGreen),
        Cell::plain("~/.claude/"),
        detail_cell(style, Some(&data.imported.join(", "))),
        time_cell(None),
    ]);
    // **No row when nothing was withheld.**
    //
    // There used to be one, on the argument that "Armada looked and found no
    // credentials" and "nobody looked" read identically as an absence. What it
    // actually printed was `withheld  0 values  no credential-shaped values
    // found`, and the reader it reached could not tell what had been checked,
    // against what, or why it was being told. A row that says nothing three
    // times is worse than no row: it is the one a reader learns to skip, and the
    // next time it says `1 value` he skips that too.
    //
    // The guarantee has not gone anywhere — the importer still refuses
    // credential-shaped values, `armada doctor` still reports how many are held
    // in `machine.yml`, and `guild/init.md` still states the rule. What is gone
    // is a line of output claiming to report it.
    if !data.withheld.is_empty() {
        table = table.row(vec![
            token("withheld", Role::FlareOrange),
            Cell::plain(format::count(data.withheld.len(), "value")),
            detail_cell(
                style,
                Some(&format!(
                    "{} -> machine.yml, which never syncs",
                    ids(&data.withheld, KEEP)
                )),
            ),
            time_cell(None),
        ]);
    }
    table = table.row(vec![
        token("wrote", Role::BeaconGreen),
        Cell::plain(format::count(data.wrote.len(), "file")),
        detail_cell(style, Some(&ids(&data.wrote, KEEP))),
        time_cell(None),
    ]);
    // **Only when it happened, and it happens once per machine.** A file Armada
    // rewrote without saying so is a file the next reader cannot trust, and
    // `machine.yml` is the one file here that is meant to be hand-edited. A row
    // that appeared every run would be the row nobody reads.
    if let Some(migrated) = &data.migrated {
        table = table.row(vec![
            token("migrated", Role::BeaconGreen),
            Cell::plain("machine.yml"),
            detail_cell(style, Some(migrated)),
            time_cell(None),
        ]);
    }
    table = table.row(vec![
        token("guild", Role::BeaconGreen),
        Cell::plain("initialised"),
        detail_cell(
            style,
            Some(match &data.remote {
                Some(remote) => remote.as_str(),
                None => "no remote: sync off, export still works",
            }),
        ),
        time_cell(None),
    ]);
    // **The row that says the guild is in effect and not merely written.**
    // Amber when something was left alone, because that is the only outcome of
    // the step a reader has to decide about.
    if let Some(projected) = &data.projected {
        table = table.row(vec![
            token(
                "projected",
                if projected.kept > 0 {
                    Role::FlareOrange
                } else {
                    Role::BeaconGreen
                },
            ),
            Cell::plain(projected.at.clone()),
            detail_cell(style, Some(&projected.facts.join(", "))),
            time_cell(None),
        ]);
    }

    let mut out = table.render(style, width);
    out.push('\n');
    let mut facts = vec![format!("guild at {}", data.guild_path)];
    facts.extend(interview_facts(data.questions, data.answered));
    out.push_str(&summary(style, envelope.status, &facts));
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error, style));
    }
    out
}

/// What an interview is summarised as: what you said, and what you kept.
///
/// **`4 kept as imported`, never `4 skipped`.** Pressing enter is what the hint
/// instructs and it accepts a value — so telling someone who followed the
/// instructions that he skipped four questions is telling him he did nothing,
/// which is what a real first run came back with. Both halves are stated because
/// each is a different fact about the guild: what is yours, and what is still a
/// machine's reading of your memory file.
fn interview_facts(questions: usize, answered: usize) -> Vec<String> {
    let kept = questions.saturating_sub(answered);
    let mut facts = vec![format!("{answered} answered")];
    if kept > 0 {
        facts.push(format!("{kept} kept as imported"));
    }
    facts
}

/// `armada guild ls` — **what is in your guild**.
///
/// **One row per thing, and the kind is the STATUS word.** Every other `guild`
/// verb groups by area because it is reporting what *moved*; this one is
/// reporting what *is*, and a reader who came to find a skill needs the skill's
/// name rather than the count `guild pull` prints. The kinds sort together and
/// the STATUS column is what they sort by, which is the same reason the change
/// set is keyed on its outcome first (`verbs/guild.rs`).
///
/// **This is what a person at a terminal navigates and what an agent reads.**
/// At a terminal `ls` draws these rows as a selector; without one they are
/// printed. Same rows, same words, same order — PLAN.md §3.1.1 applied to the
/// one verb most tempting to build for a terminal alone.
fn guild_list(envelope: &Envelope<GuildListData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut table = Table::new(columns("item", "detail", true)).indent(2);
    for row in &data.items {
        table = table.row(vec![
            token(&row.kind, kind_role(&row.kind)),
            Cell::plain(row.name.clone()),
            detail_cell(style, Some(row.detail.as_str())),
            time_cell(None),
        ]);
    }
    let mut out = table.render(style, width);
    if !table.is_empty() {
        out.push('\n');
    }

    // **Where, then what** — the same shape `guild init` and `guild project`
    // both end on, because a reader arriving at any of the three wants the
    // place before the count.
    let mut facts = vec![data.at.clone()];
    if data.items.is_empty() {
        // An empty guild is not a failure and it is not nothing to say: it is
        // the state `guild init` exists to leave behind, and naming the verb is
        // the whole of what a reader needs.
        facts.push("nothing in it yet".to_string());
        facts.push("armada guild init".to_string());
    } else {
        facts.extend(data.facts.clone());
    }
    out.push_str(&summary(style, envelope.status, &facts));
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error, style));
    }
    out
}

/// The colour a kind is spoken in, from the word the envelope carries.
///
/// **The envelope carries the word and not the enum**, because `--json` is a
/// contract with agents and an enum's variant names are a Rust detail. This
/// reads it back through Guild's own table, so the two can only disagree by a
/// kind being added and not named — which is a `_` arm's worth of grey rather
/// than a mismatch.
fn kind_role(word: &str) -> Role {
    use armada_guild::inventory::Kind;
    let kind = [
        Kind::Memory,
        Kind::Skill,
        Kind::Subagent,
        Kind::Workflow,
        Kind::Hook,
        Kind::Settings,
        Kind::Plugins,
        Kind::Mcp,
        Kind::Schema,
    ]
    .into_iter()
    .find(|kind| kind.word() == word);
    kind.map_or(Role::SteelGrey, Role::for_guild_kind)
}

/// `armada guild edit` and `armada guild delete` — one item, changed.
///
/// **The `committed` fact is on the summary line and it is not decoration.** The
/// guild is a git worktree that syncs between machines, so a change that is not
/// committed is a change the other machine will never see — and an edit that was
/// refused is one where the file moved and the history did not. Saying which is
/// the difference between a report and a guess.
fn guild_change(envelope: &Envelope<GuildChangeData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut table = Table::new(columns("item", "detail", true))
        .indent(2)
        .row(vec![
            token(data.outcome.word(), Role::for_guild_change(data.outcome)),
            Cell::plain(data.item.path.clone()),
            detail_cell(style, Some(data.reading.as_str())),
            time_cell(None),
        ]);
    // **What else names it is a row, not a footnote.** A workflow whose skill
    // has just been deleted fails on its next run, and the row is the only
    // place that connection is ever drawn.
    if !data.referenced_by.is_empty() {
        table = table.row(vec![
            token("referenced", Role::FlareOrange),
            Cell::plain(format::count(data.referenced_by.len(), "file")),
            detail_cell(style, Some(&ids(&data.referenced_by, KEEP))),
            time_cell(None),
        ]);
    }

    let mut out = table.render(style, width);
    out.push('\n');

    let mut facts = vec![data.at.clone()];
    facts.push(
        match (data.committed, data.outcome) {
            (true, _) => "committed, armada guild push sends it",
            // Nothing was written, so there is nothing to commit and saying
            // `not committed` would imply something was waiting.
            (false, armada_core::envelope::GuildChange::Viewed) => "nothing written",
            (false, armada_core::envelope::GuildChange::Unchanged) => "nothing changed",
            (false, _) => "not committed, git still holds the version before it",
        }
        .to_string(),
    );
    out.push_str(&summary(style, envelope.status, &facts));
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error, style));
    }
    out
}

/// `armada guild export` and `armada guild import`.
fn guild_bundle(envelope: &Envelope<GuildBundleData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    // **The bundle's column is flexible and every other verb's name column is
    // fixed**, because this one holds a *path* rather than an id. Measured: an
    // absolute path in a fixed column pushes DETAIL past eighty and the flexible
    // column truncates the one fact the row exists to carry. A path's tail is
    // the least valuable part of it, which is exactly what `Column::flexible`
    // is for.
    let mut table = Table::new(vec![
        Column::fixed("status"),
        Column::flexible("bundle"),
        Column::flexible("detail"),
        Column::fixed("time").right(),
    ])
    .indent(2);
    table = table.row(vec![
        token(
            if data.bytes.is_some() {
                "exported"
            } else {
                "imported"
            },
            Role::BeaconGreen,
        ),
        Cell::plain(data.path.clone()),
        detail_cell(style, Some(&data.contents.join(", "))),
        time_cell(None),
    ]);
    // **Reported either way.** "The file that never syncs did not sync" is the
    // fact `--include-secrets` exists to make checkable, and a line that only
    // appears when it went wrong is a line nobody learns to look for.
    table = table.row(vec![
        token(
            "secrets",
            if data.secrets {
                Role::FlareOrange
            } else {
                Role::BeaconGreen
            },
        ),
        Cell::plain(if data.secrets { "included" } else { "excluded" }),
        // **No em dash in a cell.** A typographic character in a table is one
        // the agent audience would also receive, since `Cell` text is not
        // styled — decoration that differs by audience goes through `Style`, and
        // there is no `Style` form of an aside. A colon says the same thing in
        // both renders.
        detail_cell(
            style,
            Some(if data.secrets {
                "machine.yml travelled: this machine, not you"
            } else {
                "machine.yml stays here"
            }),
        ),
        time_cell(None),
    ]);
    for skipped in &data.skipped {
        table = table.row(vec![
            token("skipped", Role::SteelGrey),
            Cell::plain(skipped.clone()),
            detail_cell(style, Some("this machine has its own")),
            time_cell(None),
        ]);
    }
    for conflict in &data.conflicts {
        table = table.row(vec![
            token("conflict", Role::DistressRed),
            Cell::plain(conflict.clone()),
            detail_cell(style, Some("edited here, left alone")),
            time_cell(None),
        ]);
    }

    let mut out = table.render(style, width);
    out.push('\n');
    out.push_str(
        &match (data.conflicts.is_empty(), envelope.error.is_some()) {
            (false, _) => headline(
                style,
                Headline::NeedsAttention,
                &[
                    format::count(data.conflicts.len(), "conflict"),
                    "left as they were".to_string(),
                ],
            ),
            _ => summary(
                style,
                envelope.status,
                &match data.bytes {
                    Some(bytes) => vec![format::bytes(bytes)],
                    None => Vec::new(),
                },
            ),
        },
    );
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error, style));
    }
    out
}

// ------------------------------------------------------------------- the parts
// every verb shares

/// The line every workspace-scoped verb opens with.
///
/// `armada` is the **brand**, not the repository's name — the wordmark in
/// miniature, so the eye finds the top of one command's output in a scrollback
/// of many. The workspace id sits beside it because that is the thing a reader
/// checks they are in, and `right` carries the one fact worth pinning to the
/// far edge.
fn header(
    style: Style,
    workspace: Option<&WorkspaceId>,
    where_it_is: Option<&str>,
    right: Option<String>,
    width: usize,
) -> String {
    let mut left = style.strong(Role::SignalAmber, "armada");
    let mut used = 6;
    if let Some(id) = workspace {
        let id = id.to_string();
        used += 2 + term::display_width(&id);
        left.push_str("  ");
        left.push_str(&style.paint(Role::SteelGrey, &id));
    }
    // **The path sits with the identity, not in a column.** It is the same value
    // on every row of the table below it, and a column that repeats itself is a
    // column that pushed a different one off the edge.
    if let Some(path) = where_it_is {
        used += 2 + term::display_width(path);
        left.push_str("  ");
        left.push_str(&style.paint(Role::SteelGrey, path));
    }
    match right {
        // Pinned to the far edge, with at least two spaces of air. A header that
        // wraps is worse than one that runs on, so a narrow terminal gets the
        // minimum gap rather than a second line.
        Some(right) => {
            let gap = width
                .saturating_sub(used + term::display_width(&right))
                .max(2);
            format!("{left}{}{right}\n\n", " ".repeat(gap))
        }
        None => format!("{left}\n\n"),
    }
}

/// The last line: the verb's own verdict, then what it is counted from.
fn summary(style: Style, status: Status, facts: &[String]) -> String {
    headlined(style, &verdict_strong(style, status), facts)
}

/// The same line, led by a render-only [`Headline`] instead of a [`Status`].
///
/// **Two verbs need this and no others.** `armada doctor` and `armada guild
/// pull` report the condition of a *machine* rather than the outcome of a
/// *run*, and `FAILED` there would say the command failed — which it did not.
/// The word is in the payload under `data.headline`, spelled exactly as it is
/// printed, which is what keeps this module's uppercase rule intact.
fn headline(style: Style, headline: Headline, facts: &[String]) -> String {
    headlined(
        style,
        &style.strong(Role::FlareOrange, &headline.to_string()),
        facts,
    )
}

fn headlined(style: Style, lead: &str, facts: &[String]) -> String {
    if facts.is_empty() {
        return format!("{lead}\n");
    }
    format!(
        "{lead}  {}\n",
        style.paint(Role::SteelGrey, &facts.join(style.between()))
    )
}

/// A terminal state, spelled as the envelope spells it and coloured to agree.
/// A terminal state, spelled as the envelope spells it and coloured to agree.
///
/// **Never padded here.** An escape sequence is characters a terminal does not
/// show, so `{:<8}` over a painted token pads to the wrong width — which is why
/// this returns a [`Cell`] for a table and a bare string only for a line that
/// has no columns after it.
fn verdict(status: Status) -> Cell {
    Cell::painted(status.to_string(), Role::for_status(status))
}

fn verdict_strong(style: Style, status: Status) -> String {
    style.strong(Role::for_status(status), &status.to_string())
}

/// A status word that is not an envelope `Status` — `REAPED`, `CLAIMED`,
/// `OWNS`, `WOULD`.
///
/// **The chokepoint that makes "one spelling" true rather than intended.**
/// Every status cell in the CLI that is not a `Status` or a `JobState` is built
/// here, so upper-casing is one line instead of fifty-three call sites each
/// remembering. A caller writes the word in whatever case reads best in the
/// source; the column decides how it is spelled.
fn token(word: &str, role: Role) -> Cell {
    Cell::painted(word.to_uppercase(), role)
}

/// The word `WOULD`, and the one rule for when a status cell says it.
///
/// **There is one conditional token in this file and this is it.** `init`, `up`
/// and `clean` each spell `token("would", Role::FlangeOrange)` inline in their
/// own preview renderer (`Role::FlareOrange`), which was tolerable while a
/// preview had a table shape
/// of its own to write anyway. `spawn` shares one table between the preview and
/// the real answer, so the choice happens per row — and a per-row choice typed
/// out four times is four chances to leave one row in the past tense, which is
/// the defect this exists to close.
///
/// A second conditional vocabulary would be worse than either: a reader who has
/// learned that `WOULD` means "not yet" should not have to learn `PLANNED` for
/// one verb.
fn done_or_would(dry: bool, done: &'static str, role: Role) -> Cell {
    match dry {
        true => token("would", Role::FlareOrange),
        false => token(done, role),
    }
}

/// The columns a verb that draws a **live** table uses — the live one and the
/// final one both.
///
/// **The single place either render learns its header.** `render/live.rs` says
/// same columns is the requirement rather than a nicety, and [`Table::spans`]
/// makes the *widths* structural; this makes the headers structural too. Before
/// it, the live table said `CHECK` and `fleet spawn`'s answer said `STEP`, and
/// nothing could notice because the two nouns were typed in two files.
fn columns_for(shape: progress::Shape) -> Vec<Column> {
    match shape {
        progress::Shape::Check => columns("check", "detail", true),
        progress::Shape::Spawn => columns("step", "detail", true),
    }
}

/// What the `classified` row says, and what colours it.
///
/// **Shared with the live table, because it is the row worth reading twice.**
/// The confidence is on the screen so a guess is visible as a guess
/// (`commands/fleet/spawn.md`), and a live table that omitted it would put the
/// warning only where it arrives last. Below the threshold Helm confirms at
/// (PLAN.md §15.4) the cell is not green — a low confidence is the one fact on
/// this table a reader has to act on.
pub(crate) fn spawn_classified(workflow: &str, confidence: Option<f64>) -> (String, Role) {
    let guessed = confidence.is_some_and(|c| c < armada_core::fleet::classify::CONFIDENT);
    let detail = match (confidence, guessed) {
        // **Said in the cell as well as below it.** A reader scanning the table
        // should not have to reach the summary to learn that the number beside
        // them is a coin flip.
        (Some(c), true) => format!("{workflow}, confidence {c:.2}, a guess"),
        (Some(c), false) => format!("{workflow}, confidence {c:.2}"),
        // **An override reports that you named it, not a confidence of 1.0.**
        // "You said so" and "the model was certain" are different facts and only
        // one of them is a measurement.
        (None, _) => format!("{workflow}, you named it"),
    };
    let role = match guessed {
        true => Role::FlareOrange,
        false => Role::BeaconGreen,
    };
    (detail, role)
}

/// The status cell for a row that has finished, in either vocabulary.
///
/// Both arms already existed and are unchanged — this is the one `match` that
/// picks between them, so the live table and the final one cannot spell or
/// colour a verdict differently.
fn verdict_cell(reached: progress::Verdict) -> Cell {
    match reached {
        progress::Verdict::Status(status) => verdict(status),
        progress::Verdict::Word(word, role) => token(word, role),
    }
}

/// The four columns, named for this verb.
fn columns(name: &str, detail: &str, time: bool) -> Vec<Column> {
    let mut columns = vec![
        Column::fixed("status"),
        Column::fixed(name),
        Column::flexible(detail),
    ];
    if time {
        columns.push(Column::fixed("time").right());
    }
    columns
}

/// A cell holding a duration, or nothing when none was measured.
///
/// **[`Cell::nothing`] rather than a painted placeholder**, so the table can
/// count the empties and drop a `TIME` column no row filled (`render/table.rs`).
/// A verb that measures some of its rows still shows the placeholder against the
/// rest, which is where it says something.
fn time_cell(ms: Option<u64>) -> Cell {
    match ms {
        Some(ms) => Cell::muted(format::duration(ms)),
        None => Cell::nothing(),
    }
}

/// A cell holding text, or a **visible** placeholder when the text is empty.
///
/// **Deliberately not [`Cell::nothing`], unlike [`time_cell`] beside it**, and
/// the difference is what the two absences mean. A missing duration is a
/// measurement nobody took, so a `TIME` column nobody filled says nothing and
/// goes. A missing detail is the row's answer — `owns  resources  —` is how a
/// reader tells "this workspace owns nothing" from "nobody looked", which is the
/// distinction `render.rs`'s own tests are most explicit about. Letting the
/// table drop `DETAIL` would delete that sentence rather than tidy it.
/// One colour per [`Disposition`], for every row that prints one.
///
/// **Matched exhaustively and on purpose.** This replaced two hand-written
/// matches — one on the worktree row, one on the branch row — that covered
/// different variants and each closed with a `_` arm. `Removed` fell through
/// the branch row's wildcard to grey, so `armada fleet kill` printed the same
/// word in the same column in two colours and the reader was left inferring a
/// distinction that did not exist.
///
/// `ARCHITECTURE.md` §1.2 states the rule for the scheduler's enums — *never
/// add a `_ =>` arm, it converts a compile error into silence* — and this is
/// that failure in the renderer. Adding a fourth `Disposition` must not compile
/// until someone has decided what colour it is.
fn disposition_role(disposition: Disposition) -> Role {
    match disposition {
        // Armada did the thing it said it would.
        Disposition::Removed => Role::BeaconGreen,
        // Left alone at your request — worth noticing, not worth alarm.
        Disposition::Kept => Role::FlareOrange,
        // Already gone. Not a failure, and not an action either.
        Disposition::Gone => Role::SteelGrey,
    }
}

fn detail_cell(style: Style, text: Option<&str>) -> Cell {
    match text.filter(|t| !t.is_empty()) {
        Some(text) => Cell::muted(text),
        None => Cell::muted(style.nothing()),
    }
}

/// How many ids a cell names before it starts counting instead.
///
/// **A fixed cap, not a width calculation.** Three ids is enough to recognise
/// what a workspace is holding; more is a list you read from `--json`. Fixing it
/// also keeps the render identical at every terminal width, which is what lets a
/// golden fixture pin it.
const KEEP: usize = 3;

/// A list of ids, cut to `keep` with the remainder counted rather than dropped.
///
/// **`+2` rather than `…`** (the agreed layout): an ellipsis says something was
/// cut, a count says how much, and the difference decides whether the reader
/// needs `--json`. `--json` always carries them all.
fn ids(items: &[String], keep: usize) -> String {
    if items.len() <= keep {
        return items.join(", ");
    }
    format!("{}, +{}", items[..keep].join(", "), items.len() - keep)
}

// ------------------------------------------------------------------------ init

/// The block, pinned to the far edge of a header — or a statement that there is
/// none.
///
/// **A workspace that needs no block says so rather than showing nothing.** The
/// absence is the interesting part: a reader who ran `init` in a repository that
/// declares no `ports:` is being told why there is no range to see, which is the
/// question a blank corner would leave them holding. It is also how the change
/// is visible at all — the old output printed `ports 5460–5469` for a workspace
/// that reserved nothing.
fn ports_pinned(style: Style, block: Option<PortBlock>) -> Option<String> {
    Some(match block {
        Some(block) => format!(
            "{} {}",
            style.paint(Role::SteelGrey, "ports"),
            style.paint(Role::NavalBlue, &style.span(block.from, block.to))
        ),
        None => style.paint(Role::SteelGrey, "no ports declared"),
    })
}

/// `armada manifest init`.
///
/// Two tables, because the envelope holds two grains: the components it ran
/// setup for, and the ports it assigned. Both are `STATUS · NAME · DETAIL ·
/// TIME`; neither invents a row the envelope does not have.
fn init(envelope: &Envelope<InitData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut out = header(
        style,
        envelope.workspace.as_ref(),
        None,
        ports_pinned(style, data.port_block),
        width,
    );

    let mut components = Table::new(columns("component", "detail", true)).indent(2);
    for row in &data.results {
        components = components.row(vec![
            verdict(row.status),
            Cell::plain(row.id.clone()),
            detail_cell(style, row.error.as_ref().map(|e| e.message.as_str())),
            time_cell(row.duration_ms),
        ]);
    }
    if !components.is_empty() {
        out.push_str(&components.render(style, width));
        out.push('\n');
    }

    let mut ports = Table::new(columns("port", "detail", false)).indent(2);
    for (name, port) in &data.ports {
        ports = ports.row(vec![
            token("claimed", Role::BeaconGreen),
            Cell::plain(name.clone()),
            Cell::painted(port.to_string(), Role::NavalBlue),
        ]);
    }
    if !ports.is_empty() {
        out.push_str(&ports.render(style, width));
        out.push('\n');
    }

    out.push_str(&reaped(&data.reaped, false, style, width));
    out.push_str(&summary(
        style,
        envelope.status,
        &[format::count(data.ports.len(), "port")],
    ));
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error, style));
    }
    out
}

fn init_dry(envelope: &Envelope<InitDryRun>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut out = header(style, envelope.workspace.as_ref(), None, None, width);

    // **The same table with `would` in the status column**, rather than a shape
    // of its own — one layout is one thing to learn (the agreed layout).
    let mut table = Table::new(columns("resource", "detail", false)).indent(2);
    if let Some(block) = data.would_claim {
        table = table.row(vec![
            token("would", Role::FlareOrange),
            Cell::plain("ports"),
            Cell::painted(style.span(block.from, block.to), Role::NavalBlue),
        ]);
    }
    for step in &data.would_run {
        table = table.row(vec![
            token("would", Role::FlareOrange),
            Cell::plain("setup"),
            Cell::muted(step.clone()),
        ]);
    }
    out.push_str(&table.render(style, width));
    if !table.is_empty() {
        out.push('\n');
    }
    out.push_str(&reaped(&data.would_reap, true, style, width));
    out.push_str(&summary(
        style,
        envelope.status,
        &["dry run".to_string(), "nothing was changed".to_string()],
    ));
    out
}

// ------------------------------------------------------------------ up / down

/// `armada manifest up` and `armada manifest down`.
///
/// **One renderer for both**, because the envelope is one shape and the words
/// in it already differ: a row says `UP` or `DOWN`, and the summary reads off
/// `envelope.status`. A second function would be a second place for the two
/// verbs to drift apart, which is the failure `render.rs` exists as one file to
/// prevent.
///
/// Three tables, and each earns its place:
///
/// 1. **The services**, `STATUS · NAME · DETAIL · TIME`. DETAIL is the row's
///    error when it has one and otherwise the ready-check that was waited on —
///    which is the first thing anyone asks of a `TIMEOUT`, and the thing that
///    makes a bare `UP` mean something.
/// 2. **The ports**, probed at report time and never remembered. `up`'s whole
///    output is worthless without the number a browser is pointed at.
/// 3. **The block**, spelled `kept`. `down` keeps it — that is the entire
///    distinction from `clean` — and a reader who cannot see that it was kept
///    has to run `status` to find out.
fn services(envelope: &Envelope<ServicesData>, style: Style, width: usize, noun: &str) -> String {
    let data = &envelope.data;
    let mut out = header(
        style,
        envelope.workspace.as_ref(),
        None,
        ports_pinned(style, data.port_block),
        width,
    );

    let mut table = Table::new(columns(noun, "detail", true)).indent(2);
    for row in &data.results {
        // The failure outranks the ready-check: a row that failed is being read
        // for why, and a row that did not is being read for what it waited on.
        let detail = row
            .error
            .as_ref()
            .map(|e| e.message.clone())
            .or_else(|| row.reason.clone());
        // **A log path only on a row that did not work**, the same rule `check`
        // applies: a path under every healthy service is a line nobody reads,
        // and it buries the one that matters.
        let note = row
            .log
            .clone()
            .filter(|_| !matches!(row.status, Status::Up | Status::Down));
        table = table.row_with_note(
            vec![
                verdict(row.status),
                Cell::plain(row.id.clone()),
                detail_cell(style, detail.as_deref()),
                time_cell(row.duration_ms),
            ],
            note,
        );
    }
    if !table.is_empty() {
        out.push_str(&table.render(style, width));
        out.push('\n');
    }

    let mut ports = Table::new(columns("port", "detail", false)).indent(2);
    for row in &data.results {
        for (name, report) in &row.ports {
            ports = ports.row(vec![
                token(port_word(report.state), Role::for_port(report.state)),
                Cell::plain(name.clone()),
                Cell::painted(report.port.to_string(), Role::NavalBlue),
            ]);
        }
    }
    if !ports.is_empty() {
        out.push_str(&ports.render(style, width));
        out.push('\n');
    }

    // **Stated, not implied.** `down` keeps the block so the next `up` gets the
    // same ports, which keeps URLs, bookmarks and `.env` files valid across a
    // restart — and a reader who cannot see that has to go and check.
    // A workspace holding no block has nothing to keep, and drawing a `kept`
    // row for it would claim it kept something.
    if let Some(block) = data.port_block {
        let kept = Table::new(columns("resource", "detail", false))
            .indent(2)
            .row(vec![
                token("kept", Role::BeaconGreen),
                Cell::plain("ports"),
                Cell::painted(style.span(block.from, block.to), Role::NavalBlue),
            ]);
        out.push_str(&kept.render(style, width));
        out.push('\n');
    }

    // **Counted against the row's success state, not the envelope's.** A
    // `PARTIAL` run reading "1 partial" would be counting the wrong thing: the
    // question is how many services reached the state the verb was asked for.
    let reached = data
        .results
        .iter()
        .filter(|row| matches!(row.status, Status::Up | Status::Down))
        .count();
    out.push_str(&summary(
        style,
        envelope.status,
        &[
            format::count(data.results.len(), noun),
            format!("{reached} {}", envelope.verb),
        ],
    ));
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error, style));
    }
    out
}

/// `armada manifest up --dry-run`.
///
/// **The wait is shown beside the spawn**, because it is the half that takes
/// the time: a preview naming the argv and hiding the ready-check would preview
/// the fast part of `up`.
fn up_dry(envelope: &Envelope<UpDryRun>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut out = header(style, envelope.workspace.as_ref(), None, None, width);

    let mut table = Table::new(columns("service", "detail", false)).indent(2);
    for (word, lines) in [("would", &data.would_run), ("wait", &data.would_wait)] {
        for line in lines {
            // `<service>: <the rest>` — the shell builds it that way so the
            // name lands in the NAME column rather than at the head of DETAIL.
            let (service, detail) = line.split_once(": ").unwrap_or(("", line.as_str()));
            table = table.row(vec![
                token(word, Role::FlareOrange),
                Cell::plain(service),
                Cell::muted(detail.trim()),
            ]);
        }
    }
    out.push_str(&table.render(style, width));
    if !table.is_empty() {
        out.push('\n');
    }
    out.push_str(&summary(
        style,
        envelope.status,
        &["dry run".to_string(), "nothing was started".to_string()],
    ));
    out
}

// ---------------------------------------------------------------------- status

/// `armada manifest status`.
///
/// One block per workspace in scope: a header, a component table, and — when
/// there is anything to say — what the workspace is holding.
///
/// **`OWNS` is not here yet, and its absence is deliberate rather than an
/// oversight.** The agreed layout carries real resource ids in that column;
/// `results[]` carries leases and unreclaimed commands but no owned-resource
/// ids, and adding them is a `--json` change this milestone is forbidden from
/// making. The ids Armada does hold appear on the `holds` table below, which is
/// where the envelope actually puts them.
fn status(envelope: &Envelope<StatusData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut out = String::new();

    for row in &data.results {
        out.push_str(&workspace_block(row, &data.unreclaimed, style, width));
    }
    if data.results.is_empty() {
        out.push_str(&header(
            style,
            envelope.workspace.as_ref(),
            None,
            None,
            width,
        ));
    }

    out.push_str(&summary(
        style,
        envelope.status,
        &[
            format::count(data.results.len(), "workspace"),
            format!("scope {}", data.scope),
        ],
    ));
    out
}

fn workspace_block(
    row: &ResultRow,
    unreclaimed: &[Unreclaimed],
    style: Style,
    width: usize,
) -> String {
    let id = WorkspaceId::from_stored(&row.id);
    let block = row.port_block.map(|b| {
        format!(
            "{} {}",
            style.paint(Role::SteelGrey, "ports"),
            style.paint(Role::NavalBlue, &style.span(b.from, b.to))
        )
    });
    let mut out = header(style, Some(&id), row.path.as_deref(), block, width);

    // **`PORT · DETAIL`, not `COMPONENT · PORT`.** Every row here comes from
    // `results[].ports`, which is keyed by *port name* — so the old header
    // called `redis` a component when the component was `cache`, and put the
    // number under a heading that named the thing beside it. `init` and `up`
    // both draw this table as `STATUS · PORT · DETAIL`; this one was the odd
    // one out, and one renderer means one answer.
    let mut components = Table::new(columns("port", "detail", false)).indent(2);
    for (name, report) in &row.ports {
        components = components.row(vec![
            // **Probed at report time, never remembered** (PLAN.md §3.1), and
            // spoken as the state of the component rather than of the socket:
            // `UP` is what a reader is asking about.
            token(port_word(report.state), Role::for_port(report.state)),
            Cell::plain(name.clone()),
            Cell::painted(report.port.to_string(), Role::NavalBlue),
        ]);
    }
    if !components.is_empty() {
        out.push_str(&components.render(style, width));
        out.push('\n');
    }

    let mut holds = Table::new(columns("resource", "detail", false)).indent(2);

    // **One row per resource, and the id is the whole point** (PLAN.md §3.1):
    // what `armada manifest clean` will remove, and what to go and look at by
    // hand. A count in a single cell would answer neither.
    //
    // **A row each rather than a list in one cell**, which the first attempt did
    // and which was wrong for a measurable reason: five ids do not fit an
    // eighty-column DETAIL cell, so the flexible column truncated — and the part
    // it cut was the trailing `+2`. The one fact that tells a reader whether they
    // need `--json` was the first thing to go. A row cannot lose its tail.
    for id in row.owns.iter().take(KEEP) {
        // The envelope's `<kind>:<reference>` grammar, split back into the two
        // columns it was always two of.
        let (kind, reference) = id.split_once(':').unwrap_or(("resource", id));
        holds = holds.row(vec![
            token("owns", Role::BeaconGreen),
            Cell::plain(kind),
            Cell::muted(reference),
        ]);
    }
    // **Always a row, even when there is nothing.** `owns  resources  -` says
    // Armada looked and found nothing; no row at all says nothing whatsoever,
    // and a reader cannot tell those apart. Same reasoning as `clean` keeping its
    // table when it had nothing to release.
    if row.owns.len() > KEEP || row.owns.is_empty() {
        holds = holds.row(vec![
            token("owns", Role::BeaconGreen),
            Cell::plain("resources"),
            detail_cell(
                style,
                (row.owns.len() > KEEP)
                    .then(|| format!("+{} more", row.owns.len() - KEEP))
                    .as_deref(),
            ),
        ]);
    }

    for lease in &row.leases {
        let cold = lease.ends_with("(cold)");
        holds = holds.row(vec![
            // A cold lease is a holder that died, which is a different fact from
            // a lease being held — and the reason `status` names it either way
            // rather than filtering it out.
            token(
                if cold { "cold" } else { "held" },
                if cold {
                    Role::FlareOrange
                } else {
                    Role::BeaconGreen
                },
            ),
            Cell::plain("lease"),
            Cell::muted(lease.trim_end_matches(" (cold)").to_string()),
        ]);
    }
    for external in unreclaimed.iter().filter(|u| u.workspace == id) {
        holds = holds.row(vec![
            // Recorded, reported, and never executed: a stale `DROP DATABASE` is
            // strictly more dangerous than a stale `kill` (PLAN.md §6.1).
            token("reported", Role::FlareOrange),
            Cell::plain(if external.workspace_exists {
                "release"
            } else {
                "release (gone)"
            }),
            Cell::muted(external.command.clone()),
        ]);
    }
    if !holds.is_empty() {
        out.push_str(&holds.render(style, width));
        out.push('\n');
    }
    out
}

/// What a probed port says about the component behind it.
///
/// Lowercase-rule exception: these are the envelope's own `PortState` spellings
/// translated into the question the reader is asking — is this service up. The
/// state itself stays available in `--json`, unchanged.
fn port_word(state: PortState) -> &'static str {
    match state {
        PortState::Listening => "UP",
        PortState::Reserved => "DOWN",
        PortState::Conflict => "CONFLICT",
    }
}

// ----------------------------------------------------------------------- check

/// `armada manifest check`.
///
/// The verb a gate calls, and the one an agent reads most. Three things the
/// agreed layout settles, each because of what it costs a reader:
///
/// 1. **A log path only on a row that failed.** Five paths under five passing
///    checks are five lines nobody reads, and they bury the one that matters.
/// 2. **Durations humanised** — `26.8s`, not `26754ms`. Milliseconds stay in
///    `--json`, where the reader is doing arithmetic rather than reading.
/// 3. **The full run id**, not a prefix. The agreed drawing shortens it, and
///    that is the one place this render does not follow it: the id is what
///    `armada manifest explain` is given, and a prefix that cannot be pasted
///    back is worse than a long line.
fn check(envelope: &Envelope<CheckData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    // **The run id leads, where the brand leads everywhere else.** `check` is
    // the verb whose output is read after the fact, out of a log or a CI pane,
    // and the first thing a reader needs is which run they are looking at.
    let left = format!(
        "{} {}",
        style.paint(Role::SteelGrey, "run"),
        style.paint(Role::NavalBlue, &data.run_id)
    );
    let right = style.paint(Role::SteelGrey, &format::count(data.results.len(), "check"));
    let gap = width
        .saturating_sub(4 + data.run_id.len() + term::display_width(&right))
        .max(2);
    let mut out = format!("{left}{}{right}\n\n", " ".repeat(gap));

    let mut table = Table::new(columns_for(progress::Shape::Check)).indent(2);
    for row in &data.results {
        let detail = row
            .error
            .as_ref()
            .map(|e| e.message.clone())
            .or_else(|| row.reason.clone())
            .or_else(|| {
                row.waiting_on
                    .as_ref()
                    .map(|w| serde_json::to_string(w).unwrap_or_default())
            });
        // **Only on failure**, which is the whole point of the rule.
        let note = row
            .log
            .clone()
            .filter(|_| !matches!(row.status, Status::Pass | Status::Skipped));
        table = table.row_with_note(
            vec![
                verdict(row.status),
                Cell::plain(row.id.clone()),
                detail_cell(style, detail.as_deref()),
                time_cell(row.duration_ms),
            ],
            note,
        );
    }
    out.push_str(&table.render(style, width));
    if !table.is_empty() {
        out.push('\n');
    }

    // **One row for the lot, cut with a count.** Reaping is reported and never
    // silent, but a run that reaped six directories should not open with six
    // lines about directories the reader did not ask about.
    if !data.reaped_runs.is_empty() {
        let reaped = Table::new(columns("reaped", "detail", false))
            .indent(2)
            .row(vec![
                token("reaped", Role::SteelGrey),
                Cell::plain(format::count(data.reaped_runs.len(), "run")),
                Cell::muted(ids(&data.reaped_runs, KEEP)),
            ]);
        out.push_str(&reaped.render(style, width));
        out.push('\n');
    }

    // **One row, and only for a run that is not this process.** It answers the
    // question `--status` is asked — is anything still deciding — and names the
    // log the detached invocation wrote, which is where a run that failed
    // before its first check says so.
    if let Some(detached) = &data.detached {
        let (word, role) = match detached.alive {
            true => ("running", Role::for_status(Status::Running)),
            false => ("stopped", Role::SteelGrey),
        };
        let group = Table::new(columns("detached", "detail", false))
            .indent(2)
            .row(vec![
                token(word, role),
                Cell::plain(format!("pgid {}", detached.pgid)),
                Cell::muted(detached.log.clone()),
            ]);
        out.push_str(&group.render(style, width));
        out.push('\n');
    }

    let facts = check_facts(&data.results);
    // **No aggregate time**, and that is the second place this render departs
    // from the drawing. The envelope carries one duration per check and no wall
    // clock for the run, so the only total available is the sum — which
    // over-reports a parallel run by however well it parallelised, on the verb
    // whose whole job is to be believed.
    out.push_str(&summary(style, envelope.status, &facts));

    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error, style));
    }
    out
}

/// The summary line's facts: how many rows reached each state.
///
/// **A state that is not `FAILED` is not counted as failed.** The line used to
/// read `ABORTED  3 passed · 2 failed` over a run whose second number included
/// the check the operator had just interrupted — a check that reached no verdict
/// at all. `implied_class` already draws that distinction in the core (an
/// `ABORTED` row "implies nothing"), and the render contradicted it.
///
/// **One state, one word, and the word is the status.** Rather than a fixed
/// vocabulary of buckets, every terminal state counts under its own name
/// lowercased — `2 aborted`, `1 timeout` — which is the same rule the rest of
/// the CLI follows: the human word is the enum's word. A state nobody reached
/// contributes nothing, so an ordinary run still reads `4 passed · 0 failed`.
///
/// `passed` and `failed` are always shown, even at zero: they are the question
/// the line is being read to answer, and a missing `0 failed` reads as an
/// omission rather than as a zero.
fn check_facts(results: &[ResultRow]) -> Vec<String> {
    // The order a reader wants them: the verdict first, then the ways a run can
    // end without one, then the rows that were never in question.
    const ORDER: [Status; 6] = [
        Status::Pass,
        Status::Failed,
        Status::Aborted,
        Status::Timeout,
        Status::Dead,
        Status::Skipped,
    ];
    let count = |status: Status| results.iter().filter(|r| r.status == status).count();
    let mut facts = Vec::new();
    for status in ORDER {
        let n = count(status);
        if n == 0 && !matches!(status, Status::Pass | Status::Failed) {
            continue;
        }
        let word = match status {
            Status::Pass => "passed".to_string(),
            Status::Failed => "failed".to_string(),
            other => other.to_string().to_lowercase(),
        };
        facts.push(format!("{n} {word}"));
    }
    // Anything `ORDER` does not name — `PARTIAL`, a state a later milestone
    // adds, and the two progress states a `--status` poll reports. Counted
    // rather than folded into `failed`, because being silently miscounted is
    // the defect this function exists to fix. **A finished run reaches none of
    // them**, so this adds nothing to the line an attached run prints.
    let mut rest: Vec<Status> = results
        .iter()
        .map(|r| r.status)
        .filter(|s| !ORDER.contains(s))
        .collect();
    rest.sort_by_key(ToString::to_string);
    rest.dedup();
    for status in rest {
        facts.push(format!(
            "{} {}",
            count(status),
            status.to_string().to_lowercase()
        ));
    }
    facts
}

fn check_dry(envelope: &Envelope<CheckDryRun>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut out = header(style, None, None, None, width);
    let mut table = Table::new(columns("check", "detail", false)).indent(2);
    for line in &data.would_run {
        table = table.row(vec![
            token("would", Role::FlareOrange),
            Cell::plain(line.clone()),
            Cell::muted(style.nothing()),
        ]);
    }
    for line in &data.would_skip {
        table = table.row(vec![
            token("skip", Role::SteelGrey),
            Cell::plain(line.clone()),
            Cell::muted(style.nothing()),
        ]);
    }
    for line in &data.would_reap {
        table = table.row(vec![
            token("reap", Role::SteelGrey),
            Cell::plain(format!("run {line}")),
            Cell::muted(style.nothing()),
        ]);
    }
    out.push_str(&table.render(style, width));
    if !table.is_empty() {
        out.push('\n');
    }
    out.push_str(&summary(
        style,
        envelope.status,
        &["dry run".to_string(), "nothing was run".to_string()],
    ));
    out
}

// ----------------------------------------------------------------------- clean

/// `armada manifest clean`.
///
/// **One table, and it is drawn even when nothing was owned.** That is the
/// requirement the agreed layout is most explicit about: a workspace with
/// nothing to release must not look like a workspace that failed to release
/// something, so the row is present with a `—` detail rather than absent.
///
/// The released tally is counts rather than ids because counts are what
/// `Released` carries. `status`'s settled "ids, not counts" is about a column
/// that says *what to go and look at*; this one says *what has already gone*.
fn clean(envelope: &Envelope<CleanData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut out = header(style, envelope.workspace.as_ref(), None, None, width);

    let mut table = Table::new(columns("workspace", "detail", false)).indent(2);
    for row in &data.results {
        let released = row.released.as_ref().map(tally).unwrap_or_default();
        table = table.row(vec![
            verdict(row.status),
            Cell::plain(row.id.clone()),
            detail_cell(style, released.as_deref()),
        ]);
    }
    for id in &data.skipped {
        table = table.row(vec![
            // A live lease is the guard, and saying so is the difference between
            // "nothing to do" and "deliberately left alone".
            token("skipped", Role::FlareOrange),
            Cell::plain(id.clone()),
            Cell::muted("holds a live lease"),
        ]);
    }
    for external in &data.unreclaimed {
        table = table.row(vec![
            token("reported", Role::FlareOrange),
            Cell::plain(external.workspace.to_string()),
            Cell::muted(external.command.clone()),
        ]);
    }
    out.push_str(&table.render(style, width));
    if !table.is_empty() {
        out.push('\n');
    }

    out.push_str(&reaped(&data.reaped, false, style, width));
    // A zero is left out of the summary rather than printed: `0 skipped` is a
    // fact nobody was asking about, on the verb that most needs to read as calm.
    let mut facts = vec![format::count(data.results.len(), "workspace")];
    if !data.skipped.is_empty() {
        facts.push(format!("{} skipped", data.skipped.len()));
    }
    out.push_str(&summary(style, envelope.status, &facts));
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error, style));
    }
    out
}

/// What a row released, in words, or `None` when it released nothing.
fn tally(released: &armada_core::envelope::Released) -> Option<String> {
    let mut parts = Vec::new();
    for (n, noun) in [
        (released.processes, "process"),
        (released.containers, "container"),
        (released.networks, "network"),
        (released.volumes, "volume"),
        (released.images, "image"),
        (released.files, "file"),
    ] {
        if n > 0 {
            parts.push(format::count(n, noun));
        }
    }
    if released.port_block {
        parts.push("ports released".to_string());
    }
    (!parts.is_empty()).then(|| parts.join(", "))
}

fn clean_dry(envelope: &Envelope<CleanDryRun>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut out = header(style, envelope.workspace.as_ref(), None, None, width);
    let mut table = Table::new(columns("resource", "detail", false)).indent(2);
    for (word, lines) in [
        ("release", &data.would_release),
        ("remove", &data.would_remove),
        ("delete", &data.would_delete),
        ("report", &data.would_report),
    ] {
        for line in lines {
            table = table.row(vec![
                token("would", Role::FlareOrange),
                Cell::plain(word),
                Cell::muted(line.clone()),
            ]);
        }
    }
    out.push_str(&table.render(style, width));
    if !table.is_empty() {
        out.push('\n');
    }
    out.push_str(&summary(
        style,
        envelope.status,
        &["dry run".to_string(), "nothing was changed".to_string()],
    ));
    out
}

// ----------------------------------------------------------------- config scan

/// `armada manifest config scan` — layer 1 of PLAN.md §5.
///
/// Three things the agreed layout settles here, each because of what it costs
/// the reader — who is, on this verb more than any other, the agent about to
/// author the config:
///
/// 1. **A row for every kind, present or not.** `absent  makefile  —` says
///    Armada looked; a missing row says nothing at all, and the author cannot
///    tell those apart.
/// 2. **No truncation in the sections.** All fourteen scripts print. Evidence
///    with a `…9 more` on it is evidence somebody has to fetch separately,
///    which is how the one script that mattered gets missed.
/// 3. **It ends by offering to hand over.** It has produced evidence and
///    evidence is not a config, so the last thing it prints is the choice.
///    `ARCHITECTURE.md` §1.9 permits that: the rule governs *inputs*, and
///    printing a choice is an output. Reading the answer is not Armada's —
///    nothing here consumes one.
fn scan(envelope: &Envelope<ScanData>, style: Style, width: usize) -> String {
    let data = &envelope.data;

    let here = if data.evidence.config_present {
        "armada.yml is here already"
    } else {
        "no armada.yml here"
    };
    let mut out = format!(
        "{}{}{}\n\n",
        style.paint(Role::FlareOrange, here),
        style.between(),
        style.paint(Role::SteelGrey, "this is evidence and not a config")
    );

    // **The kind list is the renderer's and the values are the envelope's.**
    // Which six kinds are drawn is layout, frozen by the fixture; whether each
    // one found anything is read from `results[]` and decided nowhere here.
    let mut kinds = Table::new(columns("kind", "detail", false)).indent(2);
    for kind in armada_core::scan::KINDS {
        let mut found = data.results.iter().filter(|row| row.id == kind).peekable();
        if found.peek().is_none() {
            kinds = kinds.row(vec![
                token("absent", Role::SteelGrey),
                Cell::plain(kind),
                detail_cell(style, None),
            ]);
            continue;
        }
        for row in found {
            kinds = kinds.row(vec![
                token("found", Role::BeaconGreen),
                Cell::plain(kind),
                detail_cell(style, row.reason.as_deref()),
            ]);
        }
    }
    out.push_str(&kinds.render(style, width));

    let evidence = &data.evidence;
    for source in &evidence.scripts {
        let mut table = pairs();
        for script in &source.scripts {
            table = table.row(vec![
                Cell::painted(script.name.clone(), Role::RadarCyan),
                Cell::muted(script.cmd.clone()),
            ]);
        }
        out.push_str(&section(
            style,
            width,
            &format!("{} scripts", source.file),
            Some("verbatim and not interpreted"),
            &table,
        ));
    }

    let mut tools = pairs();
    for pyproject in &evidence.pyproject {
        tools = tools.row(vec![
            Cell::painted(pyproject.file.clone(), Role::RadarCyan),
            Cell::muted(pyproject.tools.join(", ")),
        ]);
    }
    out.push_str(&section(
        style,
        width,
        "pyproject tool sections",
        None,
        &tools,
    ));

    let mut targets = pairs();
    for makefile in &evidence.makefiles {
        targets = targets.row(vec![
            Cell::painted(makefile.file.clone(), Role::RadarCyan),
            Cell::muted(makefile.targets.join(", ")),
        ]);
    }
    out.push_str(&section(style, width, "makefile targets", None, &targets));

    // **A section per compose file, not one merged list.** The first version
    // merged them, and against a repository with two compose files that printed
    // `postgres` and `redis` twice with nothing saying which file either came
    // from — the same fact-destroying merge that made the scanner blind to
    // `backend/` in the first place, one level down.
    for compose in &evidence.compose {
        let mut services = pairs();
        for service in &compose.services {
            services = services.row(vec![
                Cell::painted(service.name.clone(), Role::RadarCyan),
                Cell::muted(service.ports.join(", ")),
            ]);
        }
        out.push_str(&section(
            style,
            width,
            &format!("{} services", compose.file),
            None,
            &services,
        ));
    }

    let runs: Vec<String> = evidence
        .ci
        .iter()
        .flat_map(|workflow| workflow.runs.clone())
        .collect();
    if !runs.is_empty() {
        out.push('\n');
        out.push_str(&heading(
            style,
            "ci steps",
            Some("the best existing evidence of what you actually run"),
        ));
        // **Not a table cell, because a flexible column truncates** and the one
        // rule this verb has is that evidence is never cut.
        out.push_str(&wrapped(style, &runs, width));
    }

    let mut globs = pairs();
    for workspace in &evidence.workspace_globs {
        globs = globs.row(vec![
            Cell::painted(workspace.file.clone(), Role::RadarCyan),
            Cell::muted(workspace.globs.join(", ")),
        ]);
    }
    out.push_str(&section(style, width, "workspace globs", None, &globs));

    // **The layout, stated once.** `backend/` holding `uv.lock` and
    // `pyproject.toml` while `web/` holds `pnpm-lock.yaml` is the single most
    // important fact about a polyglot repository, and reading it off the flat
    // lists above is work the author should not have to do.
    // **Drawn only when there is a layout to draw.** A repository with one
    // package at the root has no monorepo structure, and a section saying
    // `.  package.json, pnpm-lock.yaml` restates the table above it. `--json`
    // carries `packages` either way, because a consumer asking "what is here"
    // wants the answer whichever shape the repository is.
    let mut packages = pairs();
    for package in evidence
        .packages
        .iter()
        .filter(|_| evidence.packages.iter().any(|p| !p.dir.is_empty()))
    {
        let mut held = package.manifests.clone();
        held.extend(package.lockfiles.iter().cloned());
        packages = packages.row(vec![
            Cell::painted(
                if package.dir.is_empty() {
                    ".".to_string()
                } else {
                    package.dir.clone()
                },
                Role::RadarCyan,
            ),
            Cell::muted(held.join(", ")),
        ]);
    }
    out.push_str(&section(
        style,
        width,
        "packages",
        Some("a directory with a manifest of its own"),
        &packages,
    ));

    // **Reported, never decided** (PLAN.md §4.6). The fact underneath is that
    // each of these resolves its own dependencies — a manifest *and* a lockfile
    // of its own, claimed by nobody's workspace glob — which is what tells a
    // separate product from a member of one. Whether to declare it is the
    // author's call.
    let candidates: Vec<String> = evidence
        .packages
        .iter()
        .filter(|package| package.independent)
        .map(|package| package.dir.clone())
        .collect();
    if !candidates.is_empty() {
        out.push('\n');
        out.push_str(&heading(
            style,
            "workspaces: candidates",
            Some("each resolves its own dependencies"),
        ));
        out.push_str(&wrapped(style, &candidates, width));
    }

    out.push_str(&format!(
        "\n{} {}\n",
        style.strong(Role::SignalAmber, "Evidence only."),
        style.paint(
            Role::SteelGrey,
            "Armada does not guess which of these you actually run."
        )
    ));
    // **The blank line belongs to the hand-over, not to the sentence above it.**
    // Two of the three hand-overs now draw nothing — one is a selector on
    // stderr, the other is `--json` — and a report that ended in a blank line
    // whenever nothing followed it would be a trailing newline nobody asked for.
    let next = handover(style, &data.handover);
    if !next.is_empty() {
        out.push('\n');
        out.push_str(&next);
    }
    out
}

/// The choice `scan` ends on — or the command it would have run, for a reader
/// who cannot answer one.
///
/// **Which of the two is in the payload and is not decided here**
/// ([`armada_core::scan::Handover`]). It follows from facts only the entrypoint
/// has — whether each stream is a terminal, whether there is a skill to hand
/// over to — and a renderer that worked it out for itself would give the two
/// human audiences different *content*, which is the one thing they may not
/// differ in.
fn handover(style: Style, choice: &Handover) -> String {
    match choice {
        // Nothing at all: `--json` is a parser waiting for one payload.
        Handover::Silent => String::new(),
        // **Nothing here either, and that is the change.** The choice used to be
        // printed as part of this report — a list of numbers that then waited
        // silently on stdin, which is the thing a real reader could not
        // interpret. It is a selector now (`ask::select`), drawn on stderr
        // *below* this report so the evidence he has just read stays on the
        // screen, and echoed afterwards so the scrollback records what he
        // picked. `Ask` is only ever produced when a person is at a terminal, so
        // there is no audience left for a printed menu here.
        Handover::Ask => String::new(),
        // **The command, so a reader who cannot answer a menu still learns the
        // next step.** A prompt drawn for an agent is worse than no prompt: it
        // is a question it cannot satisfy, in the place an instruction belongs.
        //
        // **Not a table cell, for the same reason the CI steps are not one.** A
        // flexible column truncates, and a truncated command is not a command —
        // the one promise this line makes is that pasting it works. A long line
        // overhangs, which is honest, and every terminal will still select it
        // whole.
        Handover::Tell { why, command } => {
            let aside = match why {
                TellWhy::NotATerminal => "paste this to hand the repository to an agent",
                // Printed anyway: it is what `armada guild init` makes work, and
                // a reader told only "no" learns nothing about reaching yes.
                TellWhy::NoSkill => "no onboarding skill in your guild — `armada guild init`",
            };
            format!(
                "{}    {}\n",
                heading(style, "next", Some(aside)),
                style.paint(Role::SteelGrey, command)
            )
        }
    }
}

/// A titled block of evidence, or nothing at all when there is none of it.
fn section(style: Style, width: usize, title: &str, aside: Option<&str>, table: &Table) -> String {
    if table.is_empty() {
        return String::new();
    }
    format!(
        "\n{}{}",
        heading(style, title, aside),
        table.render(style, width)
    )
}

/// A run of items, separated and **wrapped rather than cut**.
///
/// The only place in the renderer that wraps, and it exists because this is the
/// only place that may not truncate: a repository whose CI runs twelve commands
/// has twelve pieces of evidence, and both of the usual answers are wrong here
/// — a flexible column would drop the tail, and one line would run to seven
/// hundred columns.
///
/// **The break points are computed from the wider separator, not from the one
/// this audience gets.** `·` and `, ` differ by a column, so a greedy fit
/// measured per audience would break at different items and the two renders
/// would stop being one render twice — which is the property
/// `render_golden.rs` asserts. Measuring both against the wider form costs a
/// column of slack in the plain render and keeps the two identical.
fn wrapped(style: Style, items: &[String], width: usize) -> String {
    const INDENT: usize = 4;
    /// The wider of the two separators, in columns.
    const SEPARATOR: usize = 3;

    let budget = width.saturating_sub(INDENT);
    let mut lines: Vec<Vec<&str>> = Vec::new();
    let mut used = 0;
    for item in items {
        let cost = term::display_width(item);
        match lines.last_mut() {
            // A single item wider than the line still gets a line of its own
            // and overhangs, which is honest: it is evidence, and cutting it is
            // the one thing this section may not do.
            Some(line) if used + SEPARATOR + cost <= budget => {
                line.push(item);
                used += SEPARATOR + cost;
            }
            _ => {
                lines.push(vec![item]);
                used = cost;
            }
        }
    }

    lines
        .into_iter()
        .map(|line| {
            format!(
                "{}{}\n",
                " ".repeat(INDENT),
                style.paint(Role::SteelGrey, &line.join(style.between()))
            )
        })
        .collect()
}

/// A section title, and the half-sentence that says what the section is for.
fn heading(style: Style, title: &str, aside: Option<&str>) -> String {
    let mut line = format!("  {}", style.paint(Role::SignalAmber, title));
    if let Some(aside) = aside {
        line.push_str(style.between());
        line.push_str(&style.paint(Role::SteelGrey, aside));
    }
    line.push('\n');
    line
}

/// A name and its value, aligned — the one shape every evidence section takes.
fn pairs() -> Table {
    Table::new(vec![Column::fixed(""), Column::flexible("")])
        .headerless()
        .indent(4)
}

// --------------------------------------------------------------- config verify

/// `armada manifest config verify` — layer 3 of PLAN.md §5.
///
/// Two blocks, because there are two passes and the reader needs to know which
/// one they are looking at: pass 1 is static and takes seconds, pass 2 is the
/// check suite run for real and takes as long as the repository's checks take.
///
/// Three things the agreed layout settles:
///
/// 1. **`unchecked` has a row rather than a footnote.** It is the honest cost of
///    `shell: true` — there is no `argv[0]` to resolve in a shell string, so
///    verify counts those entries rather than guessing or silently passing them
///    — and it is worth seeing.
/// 2. **`pass 2 not attempted` rather than `skipped`.** Pass 1 short-circuits,
///    and "skipped" would read as a choice somebody made about pass 2.
/// 3. **A fix line under the summary for every finding.** A check that reports a
///    problem without the command that fixes it sends the reader to the
///    documentation, which is most of what this verb exists to save.
fn verify(envelope: &Envelope<VerifyData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut out = format!(
        "{}{}{}{}{}\n\n",
        style.paint(Role::SignalAmber, "pass 1"),
        style.between(),
        style.paint(Role::SteelGrey, "static"),
        style.between(),
        style.paint(Role::SteelGrey, "nothing is executed")
    );

    let mut table = Table::new(columns("check", "detail", true)).indent(2);
    for row in &data.results {
        let detail = row
            .error
            .as_ref()
            .map(|e| e.message.clone())
            .or_else(|| row.reason.clone());
        table = table.row(vec![
            verdict(row.status),
            Cell::plain(row.id.clone()),
            detail_cell(style, detail.as_deref()),
            time_cell(row.duration_ms),
        ]);
    }
    // **A render-only word, because the envelope has no status that means
    // this.** `unchecked` is not a verdict — it is a count of what could not be
    // established either way — so it is derived from `data.unchecked` and
    // spelled lowercase, exactly as `claimed` and `owns` are.
    table = table.row(vec![
        token("unchecked", Role::FlareOrange),
        Cell::plain("shell entries"),
        Cell::muted(format!("{}, no argv[0] to resolve", data.unchecked)),
        time_cell(None),
    ]);
    out.push_str(&table.render(style, width));
    out.push('\n');

    if let Some(run) = &data.pass_2 {
        out.push_str(&format!(
            "{}{}{}\n\n",
            style.paint(Role::SignalAmber, "pass 2"),
            style.between(),
            style.paint(Role::SteelGrey, "the check suite, run for real")
        ));
        let mut suite = Table::new(columns("check", "detail", true)).indent(2);
        for row in &run.results {
            let detail = row
                .error
                .as_ref()
                .map(|e| e.message.clone())
                .or_else(|| row.reason.clone());
            suite = suite.row(vec![
                verdict(row.status),
                Cell::plain(row.id.clone()),
                detail_cell(style, detail.as_deref()),
                time_cell(row.duration_ms),
            ]);
        }
        out.push_str(&suite.render(style, width));
        out.push('\n');
    }

    let facts = match (&data.pass_2, envelope.status) {
        (None, Status::Pass) => vec![
            "pass 2 not attempted".to_string(),
            "nothing to run".to_string(),
        ],
        (None, _) => vec![
            "pass 2 not attempted".to_string(),
            "fix pass 1 first".to_string(),
        ],
        (Some(run), _) => vec![
            "pass 1 and pass 2".to_string(),
            format::count(run.results.len(), "check"),
        ],
    };
    out.push_str(&summary(style, envelope.status, &facts));

    // **Every finding's fix, not just the aggregate's.** The row carries one
    // line of detail and `--json` carries them all; these are what a reader
    // acts on, so a config with three problems gets three of them.
    for row in &data.results {
        if let Some(next) = row.error.as_ref().and_then(|e| e.next_action.as_deref()) {
            out.push_str(&format!(
                "  {} {}\n",
                style.paint(Role::SteelGrey, style.arrow()),
                style.paint(Role::SteelGrey, next)
            ));
        }
    }
    out
}

// ---------------------------------------------------------------------- skills

/// `armada manifest skills`, and `skills show <name>`.
///
/// **`declared` is a render-only word**, lowercase for the reason this module's
/// header gives: the envelope has no status that means it. Listing a skill says
/// the repository declares it, not that anything about it passed — whether its
/// `uses:` and `verify.check` resolve is `armada manifest config verify`'s
/// answer, on a different verb, so a word here that read as a verdict would be
/// claiming something this verb never checked.
///
/// **The grant table is drawn only for `show`**, and it is the same shape
/// `status` draws its holdings with: a lowercase word, the thing, and the
/// reference. `uses:` is expanded to what each command actually runs, because
/// the one question a reader has about a grant is what it lets the skill do.
fn skills(envelope: &Envelope<SkillsData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut out = header(style, envelope.workspace.as_ref(), None, None, width);

    let mut table = Table::new(columns("skill", "detail", false)).indent(2);
    for row in &data.results {
        table = table.row(vec![
            token("declared", Role::BeaconGreen),
            Cell::plain(row.id.clone()),
            detail_cell(style, row.reason.as_deref()),
        ]);
    }
    out.push_str(&table.render(style, width));
    if !table.is_empty() {
        out.push('\n');
    }

    // **One skill means `show`**, which is the only shape that has room for the
    // grants: on a listing they would be four columns nobody can read at eighty.
    if let [skill] = data.skills.as_slice() {
        let mut grants = Table::new(columns("name", "detail", false)).indent(2);
        for granted in &skill.uses {
            grants = grants.row(vec![
                token("grants", Role::RadarCyan),
                Cell::plain(granted.name.clone()),
                detail_cell(style, Some(granted.cmd.as_str())),
            ]);
        }
        for scope in &skill.verify {
            grants = grants.row(vec![
                token("verifies", Role::BeaconGreen),
                Cell::plain("check"),
                Cell::muted(scope.clone()),
            ]);
        }
        grants = grants.row(vec![
            // **`reads` and never `holds`.** Armada holds the path and reads
            // nothing; the row says what the *skill's* reader will open.
            token("reads", Role::SteelGrey),
            Cell::plain("doc"),
            Cell::muted(skill.doc.clone()),
        ]);
        for glob in &skill.touches {
            grants = grants.row(vec![
                // Advisory, and the word says so: `touches:` feeds the scope
                // lens and lets a review step notice edits far outside it. It
                // is not enforced anywhere.
                token("touches", Role::FlareOrange),
                Cell::plain("glob"),
                Cell::muted(glob.clone()),
            ]);
        }
        out.push_str(&grants.render(style, width));
        out.push('\n');
    }

    // **A grant that resolved to nothing is counted, not hidden.** It is a
    // `config verify` failure, and this verb is not that one — but a reader
    // looking at a list of grants should not have to run a second command to
    // find out that one of them names nothing.
    let unresolved = data
        .skills
        .iter()
        .flat_map(|skill| skill.uses.iter())
        .filter(|granted| granted.cmd.is_empty())
        .count();
    out.push_str(&summary(
        style,
        envelope.status,
        &[
            format::count(data.skills.len(), "skill"),
            format::count(unresolved, "unresolved reference"),
        ],
    ));
    out
}

/// `armada manifest components` — what `--component <name>` can be given.
///
/// **The same shape as `skills`**, because it answers the same kind of question
/// about the same document, and a reader who has met one has met the other.
///
/// The `STATUS` column says whether a component takes part in `up` and `down`,
/// which is the one fact about it that changes what a caller does next. `RUNS`
/// against a component with a `run:`, `DECLARED` against one without — the
/// second is not a lesser state and is not painted as one; a component that is
/// only a set of checks is an ordinary and common thing.
fn components(envelope: &Envelope<ComponentsData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut out = header(style, envelope.workspace.as_ref(), None, None, width);

    let mut table = Table::new(columns("component", "checks", false)).indent(2);
    for component in &data.components {
        let checks = component.checks.join(", ");
        table = table.row(vec![
            match component.runs {
                true => token("runs", Role::BeaconGreen),
                false => token("declared", Role::SteelGrey),
            },
            Cell::plain(component.name.clone()),
            detail_cell(style, (!checks.is_empty()).then_some(checks.as_str())),
        ]);
    }
    out.push_str(&table.render(style, width));
    if !table.is_empty() {
        out.push('\n');
    }

    let checks: usize = data.components.iter().map(|c| c.checks.len()).sum();
    out.push_str(&summary(
        style,
        envelope.status,
        &[
            format::count(data.components.len(), "component"),
            format::count(checks, "check"),
        ],
    ));
    // **The line that makes the list actionable.** A reader is here because they
    // were about to type `--component` and did not know what to put after it;
    // ending with the shape of that call saves them working it out.
    out.push_str(&format!(
        "\n{}\n",
        style.paint(
            Role::SteelGrey,
            "`armada manifest check --component <name>`, or `<name>:<check>` for one check."
        )
    ));
    out
}

/// `armada manifest commands` — the verbs this repository declares.
///
/// **The same shape as `skills` and `components`**, and deliberately: the three
/// answer the same kind of question about the same document, and a reader who
/// has met one has met the others. `declared` is a render-only word for the
/// reason the other two give — the envelope has no status that means it, and a
/// word that read as a verdict would claim something this verb never checked.
///
/// **The trailer says whose verbs these are**, because that is the confusion
/// this listing exists to end: nothing in `armada.yml` is Armada's, and a
/// reader who has just been shown a table of verbs has every reason to wonder
/// which of them Armada would have had anyway. The answer is none of them.
fn commands(envelope: &Envelope<CommandsData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut out = header(style, envelope.workspace.as_ref(), None, None, width);

    let mut table = Table::new(columns("command", "detail", false)).indent(2);
    for command in &data.commands {
        table = table.row(vec![
            token("declared", Role::BeaconGreen),
            Cell::plain(command.name.clone()),
            detail_cell(style, Some(crate::verbs::commands::detail(command))),
        ]);
    }
    out.push_str(&table.render(style, width));
    if !table.is_empty() {
        out.push('\n');
    }

    // **The grant count is a fact about the listing, not a state of a row**, so
    // it goes where `skills` puts its unresolved-reference count rather than
    // into the status column. "What can reach this token" was previously
    // answered by grepping the config; this is the same answer, counted.
    let granted = data
        .commands
        .iter()
        .filter(|command| !command.secrets.is_empty())
        .count();
    let mut facts = vec![format::count(data.commands.len(), "command")];
    if !data.commands.is_empty() {
        facts.push(format!("{granted} with a secrets grant"));
    }
    out.push_str(&summary(style, envelope.status, &facts));

    // **A repository that declares none gets different lines, not the same
    // lines against an empty table.** "These are this repository's verbs" said
    // over nothing is answering a question nobody could have asked; what that
    // reader needs is what a `commands:` entry is made of.
    let trailer: [&str; 2] = match data.commands.is_empty() {
        true => [
            "No commands: block in armada.yml, so this repository declares no verbs.",
            "An entry is a name, a cmd: to run, and a help: line saying what it is for.",
        ],
        false => [
            "`armada manifest <command>` runs one; everything after the name is its own.",
            "These are this repository's own verbs; `armada manifest --help` lists Armada's.",
        ],
    };
    out.push_str(&format!(
        "\n{}\n{}\n",
        style.paint(Role::SteelGrey, trailer[0]),
        style.paint(Role::SteelGrey, trailer[1])
    ));
    out
}

// ------------------------------------------------------------------- the parts
// more than one verb prints

/// What a reap pass did. **Reported, never silent** — a tool that removes things
/// without saying so is worse than one that does not remove them.
///
/// **`dry` is the same distinction [`spawn`] draws, and it is here for the same
/// reason.** [`init_dry`] renders `data.would_reap` through this function, so a
/// preview of `armada manifest init` said `REAPED workspace <id>, directory
/// gone` for a workspace still on disk — under a summary reading `dry run,
/// nothing was changed`, which is the report contradicting itself in five lines.
/// The rows a preview *keeps* rather than reclaims already read conditionally
/// (`KEPT`, `UNSWEPT`), so only the reclaiming ones needed the word.
fn reaped(plan: &ReapPlan, dry: bool, style: Style, width: usize) -> String {
    let mut table = Table::new(columns("reaped", "detail", false)).indent(2);
    for id in &plan.workspaces {
        table = table.row(vec![
            done_or_would(dry, "reaped", Role::BeaconGreen),
            Cell::plain("workspace"),
            Cell::muted(format!("{id}, directory gone")),
        ]);
    }
    for target in &plan.resources {
        table = table.row(vec![
            done_or_would(dry, "reaped", Role::BeaconGreen),
            Cell::plain(target.kind.to_string()),
            Cell::muted(format!("{}, {}", target.reference, target.workspace)),
        ]);
    }
    for lease in &plan.leases {
        table = table.row(vec![
            done_or_would(dry, "reaped", Role::BeaconGreen),
            Cell::plain("lease"),
            Cell::muted(format!("{lease}, heartbeat cold")),
        ]);
    }
    for report in &plan.reported {
        table = table.row(vec![
            token("kept", Role::FlareOrange),
            Cell::plain(report.kind.to_string()),
            Cell::muted(format!(
                "{}, {}, {}",
                report.reference,
                report.workspace,
                serde_json::to_string(&report.reason)
                    .unwrap_or_default()
                    .trim_matches('"')
            )),
        ]);
    }
    for skipped in &plan.skipped {
        table = table.row(vec![
            token("unswept", Role::FlareOrange),
            Cell::plain("resources"),
            Cell::muted(skipped.clone()),
        ]);
    }
    if table.is_empty() {
        return String::new();
    }
    format!("{}\n", table.render(style, width))
}

/// A dispatched `commands:` entry.
///
/// The child wrote its own output; Armada adds nothing. Saying "exited 0" after
/// a command that already printed its result is noise, and saying it on stdout
/// would corrupt a pipeline the repo owns.
fn dispatch(envelope: &Envelope<DispatchData>, style: Style) -> String {
    match &envelope.error {
        Some(error) => error_lines(error, style),
        None => String::new(),
    }
}

/// The error, in the shape PLAN.md §3.2.1 prints it.
///
/// **Only the word `error:` is painted, not the message.** A failure goes to
/// stderr, which is frequently a log file even when stdout is a terminal, and a
/// message wrapped in escapes is the part a reader most needs to copy.
pub fn error_lines(error: &ArmadaError, style: Style) -> String {
    let mut out = format!(
        "{} {}\n",
        style.strong(Role::DistressRed, "error:"),
        error.message
    );
    out.push_str(&format!("  where: {}\n", error.r#where));
    out.push_str(&format!("  class: {}\n", error.class));
    if let Some(next) = &error.next_action {
        out.push_str(&format!("  next:  {next}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::envelope::{PortReport, Released};
    use armada_core::error::ErrClass;
    use armada_core::ports::PortBlock;
    use std::collections::BTreeMap;

    fn workspace() -> WorkspaceId {
        WorkspaceId::from_stored("a3f91c02")
    }

    fn failure() -> ArmadaError {
        ArmadaError {
            class: ErrClass::ToolFailed,
            r#where: "api".to_string(),
            message: "`npm ci` exited 1".to_string(),
            next_action: Some("run it by hand".to_string()),
        }
    }

    fn a_check(id: &str, status: Status, ms: u64) -> ResultRow {
        let mut row = ResultRow::new(id, status);
        row.duration_ms = Some(ms);
        row.log = Some(format!(
            ".armada/run/01M00WRY/logs/{}.log",
            id.replace(':', ".")
        ));
        row
    }

    fn check_envelope(status: Status, rows: Vec<ResultRow>) -> Envelope<CheckData> {
        Envelope {
            schema_version: armada_core::envelope::SCHEMA_VERSION,
            verb: "check".to_string(),
            workspace: Some(workspace()),
            status,
            error: None,
            data: CheckData {
                run_id: "01M00WRY00CYTZ44".to_string(),
                results: rows,
                reaped_runs: Vec::new(),
                detached: None,
            },
        }
    }

    /// The same envelope, for a run this process is not carrying out.
    fn detached_envelope(status: Status, rows: Vec<ResultRow>, alive: bool) -> Envelope<CheckData> {
        let mut envelope = check_envelope(status, rows);
        envelope.data.detached = Some(armada_core::envelope::DetachedView {
            pgid: 4212,
            alive,
            log: ".armada/run/01M00WRY00CYTZ44/detach.log".to_string(),
        });
        envelope
    }

    fn rendered(output: &Output, style: Style) -> String {
        human(output, style, Terminal::at(88))
    }

    /// **A detached run says whether anything is still deciding**, which is the
    /// question `--status` is asked and the one an attached run never has.
    #[test]
    fn a_detached_run_names_its_group_and_says_whether_it_is_still_there() {
        let mut running = ResultRow::new("armada:test", Status::Running);
        running.duration_ms = Some(4_100);
        let text = rendered(
            &Output::Check(Box::new(detached_envelope(
                Status::Running,
                vec![running],
                true,
            ))),
            Style::plain(),
        );
        assert!(text.contains("RUNNING"), "no verdict word:\n{text}");
        assert!(text.contains("pgid 4212"), "the group is unnamed:\n{text}");
        assert!(
            text.contains("detach.log"),
            "the detached run's own output is not pointed at:\n{text}"
        );
        // **The line counts what is in flight.** `0 passed · 0 failed` alone
        // over a run that has not finished reads as a run that did nothing.
        assert!(
            text.contains("1 running"),
            "the summary hid the check in flight:\n{text}"
        );
    }

    /// A group that has gone is said to have gone, in the same row and the same
    /// place — a reader polling twice compares one word.
    #[test]
    fn a_finished_detached_run_reports_its_group_as_stopped() {
        let text = rendered(
            &Output::Check(Box::new(detached_envelope(
                Status::Pass,
                vec![a_check("armada:test", Status::Pass, 4_100)],
                false,
            ))),
            Style::plain(),
        );
        assert!(
            text.contains("STOPPED"),
            "the group still reads live:\n{text}"
        );
        assert!(text.contains("1 passed"), "the verdict is missing:\n{text}");
    }

    /// **An attached run draws no detach row at all.** The absence is the
    /// answer: the run is this process, which the reader is already waiting on.
    #[test]
    fn an_attached_run_says_nothing_about_a_process_group() {
        let text = rendered(
            &Output::Check(Box::new(check_envelope(
                Status::Pass,
                vec![a_check("armada:test", Status::Pass, 4_100)],
            ))),
            Style::plain(),
        );
        assert!(
            !text.contains("pgid"),
            "a foreground run grew a pgid:\n{text}"
        );
    }

    /// **A log path appears under a failed check and under no other kind.**
    /// Five paths for five passing checks are five lines nobody reads, and they
    /// bury the one that matters.
    #[test]
    fn a_log_path_prints_only_for_a_check_that_failed() {
        let envelope = check_envelope(
            Status::Failed,
            vec![
                a_check("armada:fmt", Status::Pass, 282),
                a_check("armada:test", Status::Failed, 26_754),
            ],
        );
        let text = rendered(&Output::Check(Box::new(envelope)), Style::plain());
        assert!(
            !text.contains("armada.fmt.log"),
            "a passing check kept its log path:\n{text}"
        );
        assert!(
            text.contains("armada.test.log"),
            "a failing check lost its log path:\n{text}"
        );
    }

    /// The agreed durations, on the agreed verb.
    #[test]
    fn check_prints_humanised_durations_and_the_full_run_id() {
        let envelope = check_envelope(
            Status::Pass,
            vec![a_check("armada:test", Status::Pass, 26_754)],
        );
        let text = rendered(&Output::Check(Box::new(envelope)), Style::plain());
        assert!(text.contains("26.8s"), "{text}");
        assert!(
            !text.contains("26754"),
            "milliseconds belong to --json: {text}"
        );
        assert!(
            text.contains("01M00WRY00CYTZ44"),
            "the run id is the value a reader pastes back: {text}"
        );
    }

    /// **Status is the first column, always, and always a word.** A symbol that
    /// only appears at a terminal would give the two audiences different shapes.
    #[test]
    fn every_row_opens_with_a_status_word_and_no_symbol() {
        let envelope = check_envelope(
            Status::Failed,
            vec![
                a_check("armada:fmt", Status::Pass, 282),
                a_check("armada:test", Status::Failed, 26_754),
            ],
        );
        let text = rendered(&Output::Check(Box::new(envelope)), Style::painted());
        for symbol in ['✓', '✗', '✔', '✘', '●', '×'] {
            assert!(
                !text.contains(symbol),
                "a {symbol} reached the render:\n{text}"
            );
        }
        let body: Vec<&str> = text.lines().filter(|l| l.starts_with("  ")).collect();
        assert!(!body.is_empty());
    }

    /// **A clean run that owned nothing keeps its table**, so "nothing was
    /// owned" never reads as "something failed".
    #[test]
    fn a_clean_that_released_nothing_still_draws_its_row() {
        let envelope = Envelope::ok(
            "clean",
            Some(workspace()),
            Status::Clean,
            CleanData {
                reaped: ReapPlan::default(),
                results: vec![ResultRow::new("a3f91c02", Status::Clean)],
                unreclaimed: Vec::new(),
                skipped: Vec::new(),
            },
        );
        let text = rendered(&Output::Clean(Box::new(envelope)), Style::plain());
        assert!(text.contains("CLEAN"), "{text}");
        assert!(text.contains("a3f91c02"), "the row is present: {text}");
        assert!(!text.contains("error"), "{text}");
    }

    #[test]
    fn a_clean_that_released_something_counts_it() {
        let mut row = ResultRow::new("a3f91c02", Status::Clean);
        row.released = Some(Released {
            processes: 1,
            containers: 2,
            networks: 0,
            volumes: 4,
            images: 0,
            port_block: true,
            files: 0,
        });
        let envelope = Envelope::ok(
            "clean",
            Some(workspace()),
            Status::Clean,
            CleanData {
                reaped: ReapPlan::default(),
                results: vec![row],
                unreclaimed: Vec::new(),
                skipped: Vec::new(),
            },
        );
        let text = rendered(&Output::Clean(Box::new(envelope)), Style::plain());
        assert!(
            text.contains("1 process, 2 containers, 4 volumes, ports released"),
            "{text}"
        );
        assert!(
            !text.contains("0 network"),
            "an empty count is left out: {text}"
        );
    }

    /// **The two human audiences differ in styling and in nothing else**
    /// (PLAN.md §3.1.1) — so the painted render, with every escape removed and
    /// every typographic character folded back to ASCII, is the plain render.
    #[test]
    fn painted_and_plain_differ_only_in_escapes_and_typography() {
        let mut row = ResultRow::new("a3f91c02", Status::Ok);
        row.path = Some("/scratch/repo".to_string());
        row.port_block = Some(PortBlock {
            from: 5460,
            to: 5469,
        });
        row.ports = BTreeMap::from([(
            "api".to_string(),
            PortReport {
                port: 5460,
                state: PortState::Conflict,
            },
        )]);
        row.leases = vec!["run:a3f91c02".to_string()];
        let envelope = || {
            Envelope::ok(
                "status",
                Some(workspace()),
                Status::Ok,
                StatusData {
                    scope: "workspace".to_string(),
                    results: vec![row.clone()],
                    unreclaimed: Vec::new(),
                },
            )
        };

        let plain = rendered(&Output::Status(Box::new(envelope())), Style::plain());
        let painted = rendered(&Output::Status(Box::new(envelope())), Style::painted());
        assert!(!plain.contains('\x1b'));
        assert!(painted.contains('\x1b'));
        assert_eq!(fold(&strip_ansi(&painted)), plain);
    }

    /// A `status` envelope carrying `owns`, and nothing else.
    fn owning(owns: &[&str]) -> Envelope<StatusData> {
        let mut row = ResultRow::new("a3f91c02", Status::Ok);
        row.owns = owns.iter().map(|s| s.to_string()).collect();
        Envelope::ok(
            "status",
            Some(workspace()),
            Status::Ok,
            StatusData {
                scope: "workspace".to_string(),
                results: vec![row],
                unreclaimed: Vec::new(),
            },
        )
    }

    /// **Real ids, never a count.** "3 containers" sends the reader to `docker
    /// ps` to find out which three, which is the work the column exists to save.
    #[test]
    fn owns_names_the_resources_rather_than_counting_them() {
        let text = rendered(
            &Output::Status(Box::new(owning(&[
                "container:armada-a3f91c02-api",
                "volume:pgdata",
            ]))),
            Style::plain(),
        );
        assert!(
            has_row(&text, &["OWNS", "container", "armada-a3f91c02-api"]),
            "{text}"
        );
        assert!(has_row(&text, &["OWNS", "volume", "pgdata"]), "{text}");
        assert!(
            !text.contains("2 resources"),
            "a count is not an id: {text}"
        );
    }

    /// **Owning nothing is stated, not implied by an absence.** A missing row
    /// and an empty one read identically to a reader, and only one of them means
    /// "Armada looked". Same reasoning as `clean` keeping its table.
    #[test]
    fn a_workspace_that_owns_nothing_says_so_rather_than_leaving_a_gap() {
        let text = rendered(&Output::Status(Box::new(owning(&[]))), Style::plain());
        assert!(
            has_row(&text, &["OWNS", "resources", "-"]),
            "the placeholder row is absent, so a reader cannot tell \
             `nothing is owned` from `nobody looked`:\n{text}"
        );
        assert!(
            !text.contains("FAILED"),
            "owning nothing is not a failure: {text}"
        );
    }

    /// **The overflow count survives, because it is the part that decides
    /// whether the reader needs `--json`.** It gets a row of its own for exactly
    /// that reason: the first attempt put the whole list in one cell, and at
    /// eighty columns the flexible column truncated the trailing `+2` away.
    #[test]
    fn a_long_owns_list_is_capped_and_says_how_many_it_did_not_name() {
        let text = rendered(
            &Output::Status(Box::new(owning(&[
                "container:one",
                "container:two",
                "container:three",
                "container:four",
                "volume:five",
            ]))),
            Style::plain(),
        );
        assert!(
            has_row(&text, &["OWNS", "resources", "+2", "more"]),
            "{text}"
        );
        assert!(has_row(&text, &["OWNS", "container", "one"]), "{text}");
        assert!(
            !text.contains("four"),
            "past the cap, only the count: {text}"
        );
        for line in text.lines() {
            assert!(!line.contains('…'), "the count was truncated away: {line}");
        }
    }

    /// A port's probed state is spoken as the question the reader is asking.
    #[test]
    fn status_speaks_a_port_state_as_the_component_it_belongs_to() {
        let mut row = ResultRow::new("a3f91c02", Status::Ok);
        row.ports = BTreeMap::from([
            (
                "api".to_string(),
                PortReport {
                    port: 5460,
                    state: PortState::Listening,
                },
            ),
            (
                "web".to_string(),
                PortReport {
                    port: 5461,
                    state: PortState::Reserved,
                },
            ),
        ]);
        let envelope = Envelope::ok(
            "status",
            Some(workspace()),
            Status::Ok,
            StatusData {
                scope: "workspace".to_string(),
                results: vec![row],
                unreclaimed: Vec::new(),
            },
        );
        let text = rendered(&Output::Status(Box::new(envelope)), Style::plain());
        // Through `has_row`, because column widths belong to the golden
        // fixtures: pinning them here too fails twice for one change and says
        // nothing the fixture did not.
        assert!(has_row(&text, &["UP", "api", "5460"]), "{text}");
        assert!(has_row(&text, &["DOWN", "web", "5461"]), "{text}");
        // **The header names what the rows actually are.** These come from
        // `results[].ports`, keyed by port name — so a `COMPONENT` heading
        // called `api` a component when the component was something else.
        assert!(text.contains("STATUS  PORT  DETAIL"), "{text}");
    }

    /// **Wrapped and never cut**, which is the one rule `config scan` has: a
    /// repository whose CI runs twelve commands has twelve pieces of evidence,
    /// and the author reading them is the one who has to find the one that
    /// mattered.
    #[test]
    fn evidence_too_wide_for_a_line_wraps_and_loses_nothing() {
        let items: Vec<String> = (0..8)
            .map(|n| format!("pnpm run task-number-{n}"))
            .collect();
        let text = wrapped(Style::plain(), &items, 80);
        assert!(text.lines().count() > 1, "one line: {text}");
        for line in text.lines() {
            assert!(line.len() <= 80, "{line:?}");
        }
        for item in &items {
            assert!(text.contains(item.as_str()), "{item} was cut: {text}");
        }
        assert!(!text.contains('…'), "evidence was truncated: {text}");
    }

    /// **The two audiences break at the same items**, because the break points
    /// are measured against the wider separator rather than against the one
    /// this audience gets. Without that, `·` and `, ` would wrap differently
    /// and the two renders would stop being one render twice.
    #[test]
    fn wrapping_breaks_at_the_same_places_for_both_audiences() {
        let items: Vec<String> = (0..9)
            .map(|n| format!("command-{n} --with-a-flag"))
            .collect();
        let plain = wrapped(Style::plain(), &items, 80);
        let painted = wrapped(Style::painted(), &items, 80);
        assert_eq!(fold(&strip_ansi(&painted)), plain);
    }

    /// A single item wider than the line gets a line of its own and overhangs.
    /// Cutting it is the one thing this section may not do.
    #[test]
    fn one_item_too_wide_for_any_line_overhangs_rather_than_being_cut() {
        let long = "x".repeat(120);
        let text = wrapped(Style::plain(), std::slice::from_ref(&long), 80);
        assert_eq!(text, format!("    {long}\n"));
    }

    /// A long list is cut with a count rather than an ellipsis: the count is
    /// what tells a reader whether they need `--json`.
    #[test]
    fn a_truncated_id_list_says_how_many_it_dropped() {
        let items: Vec<String> = ["a", "b", "c", "d"].iter().map(|s| s.to_string()).collect();
        assert_eq!(ids(&items, 4), "a, b, c, d");
        assert_eq!(ids(&items, 2), "a, b, +2");
    }

    /// The error class is spelled exactly as the envelope spells it, and the
    /// remedy line is omitted rather than printed empty when there is none.
    #[test]
    fn an_error_without_a_remedy_prints_three_lines_and_not_four() {
        let text = error_lines(
            &ArmadaError {
                class: ErrClass::BadInvocation,
                r#where: "--force-rebuild".to_string(),
                message: "not built yet".to_string(),
                next_action: None,
            },
            Style::plain(),
        );
        assert_eq!(
            text,
            "error: not built yet\n  where: --force-rebuild\n  class: bad_invocation\n"
        );
    }

    /// A dispatched command that ran prints **nothing**: the child already wrote
    /// its own output, and "exited 0" on top of it is noise — on stdout, it is
    /// corruption of a pipeline the repo owns.
    #[test]
    fn a_dispatched_command_that_ran_adds_nothing_of_armadas_own() {
        let ran = Envelope::ok(
            "commands",
            Some(workspace()),
            Status::Ok,
            DispatchData {
                command: "echoer".to_string(),
                dispatched: true,
                child_exit: Some(0),
                argv: vec!["echo".to_string()],
            },
        );
        assert_eq!(
            rendered(&Output::Dispatch(Box::new(ran)), Style::plain()),
            ""
        );

        let refused = Envelope::failed(
            "commands",
            Some(workspace()),
            failure(),
            DispatchData {
                command: "missing".to_string(),
                dispatched: false,
                child_exit: None,
                argv: Vec::new(),
            },
        );
        assert_eq!(
            rendered(&Output::Dispatch(Box::new(refused)), Style::plain()),
            "error: `npm ci` exited 1\n  where: api\n  class: tool_failed\n  next:  run it by hand\n"
        );
    }

    // ------------------------------------------------------------------ M2

    fn a_guild_init(withheld: &[&str], remote: Option<&str>, answered: usize) -> Output {
        Output::GuildInit(Box::new(Envelope::ok(
            "guild init",
            None,
            Status::Ready,
            armada_core::envelope::GuildInitData {
                guild_path: "~/.armada/guild".to_string(),
                imported: vec!["19 skills".to_string(), "12 hooks".to_string()],
                withheld: withheld.iter().map(|s| s.to_string()).collect(),
                wrote: vec![
                    "voice.md".to_string(),
                    "expectations.md".to_string(),
                    "how-i-work.md".to_string(),
                    "workflows/bug.yml".to_string(),
                ],
                migrated: None,
                remote: remote.map(str::to_string),
                questions: 5,
                answered,
                projected: None,
            },
        )))
    }

    /// **The header above the text area names the file and the keys.**
    /// `armada guild edit` puts no question, so without this a person is handed
    /// a frame full of their own `SKILL.md` and nothing saying how to leave it.
    #[test]
    fn editing_a_file_says_which_file_and_how_to_get_out() {
        let head = editing("workflows/bug.yml", Style::plain(), 80);
        assert!(head.contains("workflows/bug.yml"), "{head}");
        assert!(head.contains("ctrl-d saves"), "{head}");
        assert!(head.contains("esc leaves it as it was"), "{head}");
        assert!(head.ends_with("\n\n"), "the block does not close: {head:?}");
    }

    /// **Every box in Armada names the same three chords.** The Bridge's
    /// compose box had none at all, and the person who met it first guessed
    /// `ctrl-d` from having used the interview — a guess that happened to be
    /// right is still a guess, and a second convention would have been worse
    /// than either. So all three surfaces quote [`prose_keys`], and this holds
    /// them against each other rather than against a string typed twice.
    #[test]
    fn every_text_area_names_the_same_three_keys_however_it_spaces_them() {
        let interview = interview_prompt(
            &armada_core::envelope::Asked {
                number: 1,
                of: 7,
                prompt: "What is this repository for?".to_string(),
                purpose: "one paragraph".to_string(),
                writes: "project.md".to_string(),
                keeps: "what import found".to_string(),
                standing: None,
                prose: true,
            },
            Style::plain(),
            80,
        );
        let file = editing("workflows/bug.yml", Style::plain(), 80);
        // The Bridge spaces its key lines with two spaces rather than a
        // separator whose width differs between the two audiences.
        let bridge = prose_keys("starts it", "starts nothing", "  ");

        for surface in [&interview, &file, &bridge] {
            for chord in ["enter", "ctrl-d", "esc"] {
                assert!(surface.contains(chord), "`{chord}` unnamed in:\n{surface}");
            }
            assert!(surface.contains("enter for a new line"), "{surface}");
        }
        // The order never varies either: what you press to finish is always the
        // middle one, between the two that do not.
        for surface in [&interview, &file, &bridge] {
            let enter = surface.find("enter for a new line").unwrap();
            let save = surface.find("ctrl-d").unwrap();
            let leave = surface.find("esc ").unwrap();
            assert!(enter < save && save < leave, "out of order:\n{surface}");
        }
    }

    /// **`guild show` shows the file, not a rendering of it.** A reader is here
    /// to see what is on disk, so long lines overhang rather than wrap — the
    /// same choice `wrap_prose` makes about a word it cannot break.
    #[test]
    fn showing_an_item_shows_the_file_as_it_is() {
        let body = "---\nname: add-migration\n---\n\n# Add a migration\n";
        let shown = rendered(
            &an_item("skills/add-migration/SKILL.md", body),
            Style::plain(),
        );
        assert!(
            shown.starts_with("  skills/add-migration/SKILL.md\n\n"),
            "{shown}"
        );
        for line in body.lines() {
            assert!(shown.contains(line), "`{line}` is missing:\n{shown}");
        }
        // An empty file says so rather than drawing nothing, which reads as a
        // command that did not run.
        assert!(rendered(&an_item("voice.md", ""), Style::plain()).contains("nothing in it"));
    }

    /// One `guild show` envelope, for the two tests that read it.
    fn an_item(opens: &str, body: &str) -> Output {
        Output::GuildItem(Box::new(Envelope::ok(
            "guild show",
            None,
            Status::Ready,
            GuildItemData {
                at: "~/.armada/guild".to_string(),
                item: armada_core::envelope::GuildItemRow {
                    kind: "skill".to_string(),
                    name: "add-migration".to_string(),
                    path: "skills/add-migration".to_string(),
                    opens: opens.to_string(),
                    detail: "Write a migration and its rollback.".to_string(),
                    bytes: body.len() as u64,
                },
                body: body.to_string(),
            },
        )))
    }

    /// **The row that says the guild is in effect and not merely written.**
    /// Without it, `guild init` reports a guild nothing reads — which is what it
    /// did, and what `PHASES.md` §8.4 records as the milestone's broken path.
    #[test]
    fn guild_init_says_where_the_guild_was_projected() {
        let quiet = rendered(&a_guild_init(&[], None, 0), Style::plain());
        assert!(!quiet.to_lowercase().contains("projected"), "{quiet}");

        let mut output = a_guild_init(&[], None, 0);
        if let Output::GuildInit(envelope) = &mut output {
            envelope.data.projected = Some(Projection {
                at: "~/.claude/".to_string(),
                results: Vec::new(),
                facts: vec!["20 placed".to_string()],
                kept: 0,
                headline: None,
            });
        }
        let text = rendered(&output, Style::plain());
        assert!(text.to_lowercase().contains("projected"), "{text}");
        assert!(text.contains("~/.claude/"), "{text}");
        assert!(text.contains("20 placed"), "{text}");
    }

    /// **A file left alone is a fact the summary line carries**, because a
    /// reader who did not scroll back to the table would otherwise be told the
    /// pull worked and never learn that his own copy is the one still in effect.
    #[test]
    fn a_pull_that_left_a_file_alone_says_so_on_the_line_it_reports_on() {
        assert!(kept_facts(None).is_empty());
        let facts = kept_facts(Some(&Projection {
            at: "~/.claude/".to_string(),
            results: Vec::new(),
            facts: Vec::new(),
            kept: 2,
            headline: None,
        }));
        assert_eq!(facts[0], "2 files left as yours in ~/.claude/");
        assert!(facts[1].contains("armada guild project"));
    }

    /// **The migration row appears only when there was a migration**, and it
    /// names the keys that moved: `machine.yml` is hand-edited, so "two keys
    /// moved" leaves the reader with nothing to go and look at.
    #[test]
    fn guild_init_says_when_it_rewrote_machine_yml_and_nothing_when_it_did_not() {
        let quiet = rendered(&a_guild_init(&[], None, 0), Style::plain());
        // Case-insensitively: the status token is SCREAMING like every other
        // one, and a test that hard-codes its casing breaks the next time that
        // rule is applied rather than the next time the behaviour changes.
        assert!(!quiet.to_lowercase().contains("migrated"), "{quiet}");

        let mut output = a_guild_init(&[], None, 0);
        if let Output::GuildInit(envelope) = &mut output {
            envelope.data.migrated = Some("cpu_slots moved under `manifest:`".to_string());
        }
        let text = rendered(&output, Style::plain());
        assert!(text.to_lowercase().contains("migrated"), "{text}");
        assert!(text.contains("machine.yml"), "{text}");
        assert!(text.contains("cpu_slots"), "{text}");
    }

    /// **No row at all when nothing was withheld.** `withheld  0 values  no
    /// credential-shaped values found` says nothing three times and does not say
    /// what was checked or against what — a reader learns to skip it, and then
    /// skips the day it says `1 value`.
    #[test]
    fn guild_init_draws_no_withheld_row_when_it_withheld_nothing() {
        let text = rendered(&a_guild_init(&[], None, 0), Style::plain());
        assert!(!text.contains("withheld"), "{text}");
        assert!(!text.contains("credential-shaped"), "{text}");
    }

    /// **`kept as imported`, never `skipped`.** Pressing enter is what the hint
    /// instructs and it accepts a value; the old wording told someone who had
    /// followed the instructions that he had done nothing.
    #[test]
    fn an_accepted_default_is_kept_rather_than_skipped() {
        let text = rendered(&a_guild_init(&[], None, 1), Style::plain());
        assert!(text.contains("1 answered, 4 kept as imported"), "{text}");
        assert!(!text.contains("skipped"), "{text}");

        let all = rendered(&a_guild_init(&[], None, 5), Style::plain());
        assert!(all.contains("5 answered"), "{all}");
        assert!(
            !all.contains("kept"),
            "0 kept is a fact about nothing: {all}"
        );
    }

    /// A withheld value is named by **key**, and the destination is stated: the
    /// one fact a reader needs is where to go and look.
    #[test]
    fn guild_init_names_the_withheld_key_and_where_it_went() {
        let text = rendered(
            &a_guild_init(&["settings.json:env.GITHUB_TOKEN"], None, 0),
            Style::plain(),
        );
        assert!(text.contains("settings.json:env.GITHUB_TOKEN"), "{text}");
        assert!(text.contains("machine.yml"), "{text}");
    }

    /// **Sync off is stated, not left as an absence.** It is the documented
    /// default and `export` still works, so a reader must be able to tell it
    /// from a remote that failed to be recorded.
    #[test]
    fn a_guild_with_no_remote_says_sync_is_off_rather_than_leaving_a_gap() {
        let text = rendered(&a_guild_init(&[], None, 0), Style::plain());
        assert!(text.contains("no remote"), "{text}");
        assert!(text.contains("export still works"), "{text}");

        let with = rendered(
            &a_guild_init(&[], Some("git@example.com:me/guild.git"), 0),
            Style::plain(),
        );
        assert!(with.contains("git@example.com:me/guild.git"), "{with}");
    }

    fn a_bundle(secrets: bool, conflicts: &[&str]) -> Output {
        Output::GuildBundle(Box::new(Envelope::ok(
            "guild export",
            None,
            Status::Ready,
            armada_core::envelope::GuildBundleData {
                path: "./guild.tar.zst".to_string(),
                bytes: Some(421_888),
                contents: vec!["19 skills".to_string(), "4 workflows".to_string()],
                secrets,
                skipped: Vec::new(),
                conflicts: conflicts.iter().map(|s| s.to_string()).collect(),
            },
        )))
    }

    /// **Reported either way.** "The file that never syncs did not sync" is the
    /// fact `--include-secrets` exists to make checkable, and a line that only
    /// appears when it went wrong is a line nobody learns to look for.
    #[test]
    fn a_bundle_states_whether_machine_yml_travelled_in_both_cases() {
        let without = rendered(&a_bundle(false, &[]), Style::plain());
        assert!(has_row(
            &without,
            &["SECRETS", "excluded", "machine.yml", "stays", "here"]
        ));
        assert!(without.contains("412 KB"), "{without}");

        let with = rendered(&a_bundle(true, &[]), Style::plain());
        assert!(with.contains("included"), "{with}");
        assert!(
            with.contains("this machine, not you"),
            "including it says what that means: {with}"
        );
        // A table cell is not styled, so a typographic character in one reaches
        // the agent audience too. There is no `Style` form of an aside, so
        // there are no asides in cells.
        for glyph in ['—', '–', '…', '›', '→'] {
            assert!(!with.contains(glyph), "a {glyph} reached a cell: {with}");
        }
    }

    /// A merge that skipped a file ends on `NEEDS ATTENTION`, because a file
    /// that did not land is a thing a person has to decide about.
    #[test]
    fn a_bundle_with_a_conflict_needs_a_person() {
        let text = rendered(&a_bundle(false, &["voice.md"]), Style::plain());
        assert!(text.contains("NEEDS ATTENTION"), "{text}");
        assert!(text.contains("1 conflict"), "{text}");
        assert!(text.contains("left as they were"), "{text}");
    }

    /// A sync with nothing to do says so, and says where it syncs to — the two
    /// facts a reader is checking when they run `pull` and nothing happens.
    #[test]
    fn a_sync_that_moved_nothing_still_reports_the_remote() {
        let facts = sync_facts(&armada_core::envelope::GuildSyncData {
            remote: Some("git@example.com:me/guild.git".to_string()),
            ahead: 0,
            behind: 0,
            results: Vec::new(),
            applied: true,
            headline: None,
            projected: None,
        });
        assert_eq!(
            facts,
            vec![
                "already in step".to_string(),
                "git@example.com:me/guild.git".to_string()
            ]
        );
    }

    /// Whether some line is exactly these words, whatever the padding between
    /// them. Column widths belong to the golden fixtures; a unit test asserting
    /// them too would fail twice for one change and say nothing new.
    fn has_row(text: &str, words: &[&str]) -> bool {
        text.lines()
            .any(|line| line.split_whitespace().collect::<Vec<_>>() == words)
    }

    /// Everything a terminal would not display, removed.
    fn strip_ansi(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut chars = text.chars();
        while let Some(c) = chars.next() {
            if c != '\x1b' {
                out.push(c);
                continue;
            }
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        }
        out
    }

    /// Every typographic character folded back to the ASCII the agent audience
    /// gets. One column each way, so folding cannot move a column.
    fn fold(text: &str) -> String {
        text.replace(['—', '–'], "-").replace(" · ", ", ")
    }
}
