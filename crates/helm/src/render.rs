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
//! **Two spellings of a status, and the case says which.** `UPPERCASE` is the
//! envelope's own `Status` — `PASS`, `FAILED`, `READY` — so the human spelling
//! is the JSON spelling (`ARCHITECTURE.md` §1.6). `lowercase` is a render-only
//! word for something the envelope states structurally rather than as a status:
//! `claimed` for a port in `data.ports`, `removed` for a non-zero count in
//! `released`. A reader can tell which is which, and nothing lowercase can be
//! mistaken for a field they could have grepped from `--json`.

pub mod banner;
pub mod format;
pub mod help;
pub mod palette;
pub mod progress;
pub mod style;
pub mod table;
pub mod term;

use armada_core::envelope::{
    CheckData, CheckDryRun, CleanData, CleanDryRun, DispatchData, Envelope, InitData, InitDryRun,
    ResultRow, StatusData, Unreclaimed,
};
use armada_core::error::{ArmadaError, Status};
use armada_core::id::WorkspaceId;
use armada_core::ports::PortState;
use armada_core::reap::ReapPlan;

use crate::verbs::Output;
use palette::Role;
use style::Style;
use table::{Cell, Column, Table};
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
        Output::Clean(envelope) => clean(envelope, style, width),
        Output::CleanDryRun(envelope) => clean_dry(envelope, style, width),
        Output::Status(envelope) => status(envelope, style, width),
        Output::Check(envelope) => check(envelope, style, width),
        Output::CheckDryRun(envelope) => check_dry(envelope, style, width),
        Output::Dispatch(envelope) => dispatch(envelope, style),
    }
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
    if facts.is_empty() {
        return format!("{}\n", verdict_strong(style, status));
    }
    format!(
        "{}  {}\n",
        verdict_strong(style, status),
        style.paint(Role::SteelGrey, &facts.join(style.between()))
    )
}

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

/// A render-only status word: lowercase, because the envelope has no field
/// spelling it. See this module's header for why the case carries that.
fn token(word: &str, role: Role) -> Cell {
    Cell::painted(word.to_string(), role)
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

/// A cell holding a duration, or the placeholder when there is none.
fn time_cell(style: Style, ms: Option<u64>) -> Cell {
    match ms {
        Some(ms) => Cell::muted(format::duration(ms)),
        None => Cell::muted(style.nothing()),
    }
}

/// A cell holding text, or the placeholder when the text is empty.
fn detail_cell(style: Style, text: Option<&str>) -> Cell {
    match text.filter(|t| !t.is_empty()) {
        Some(text) => Cell::muted(text),
        None => Cell::muted(style.nothing()),
    }
}

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

/// `armada manifest init`.
///
/// Two tables, because the envelope holds two grains: the components it ran
/// setup for, and the ports it assigned. Both are `STATUS · NAME · DETAIL ·
/// TIME`; neither invents a row the envelope does not have.
fn init(envelope: &Envelope<InitData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let block = style.span(data.port_block.from, data.port_block.to);
    let mut out = header(
        style,
        envelope.workspace.as_ref(),
        None,
        Some(format!(
            "{} {}",
            style.paint(Role::SteelGrey, "ports"),
            style.paint(Role::NavalBlue, &block)
        )),
        width,
    );

    let mut components = Table::new(columns("component", "detail", true)).indent(2);
    for row in &data.results {
        components = components.row(vec![
            verdict(row.status),
            Cell::plain(row.id.clone()),
            detail_cell(style, row.error.as_ref().map(|e| e.message.as_str())),
            time_cell(style, row.duration_ms),
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

    out.push_str(&reaped(&data.reaped, style, width));
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
    out.push_str(&reaped(&data.would_reap, style, width));
    out.push_str(&summary(
        style,
        envelope.status,
        &["dry run".to_string(), "nothing was changed".to_string()],
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

    let mut components = Table::new(vec![
        Column::fixed("status"),
        Column::fixed("component"),
        Column::flexible("port"),
    ])
    .indent(2);
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

    let mut table = Table::new(columns("check", "detail", true)).indent(2);
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
                time_cell(style, row.duration_ms),
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
                Cell::muted(ids(&data.reaped_runs, 3)),
            ]);
        out.push_str(&reaped.render(style, width));
        out.push('\n');
    }

    let passed = data
        .results
        .iter()
        .filter(|r| r.status == Status::Pass)
        .count();
    let failed = data
        .results
        .iter()
        .filter(|r| {
            r.status.is_terminal() && r.status != Status::Pass && r.status != Status::Skipped
        })
        .count();
    let skipped = data
        .results
        .iter()
        .filter(|r| r.status == Status::Skipped)
        .count();
    let mut facts = vec![format!("{passed} passed"), format!("{failed} failed")];
    if skipped > 0 {
        facts.push(format!("{skipped} skipped"));
    }
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

    out.push_str(&reaped(&data.reaped, style, width));
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

// ------------------------------------------------------------------- the parts
// more than one verb prints

/// What a reap pass did. **Reported, never silent** — a tool that removes things
/// without saying so is worse than one that does not remove them.
fn reaped(plan: &ReapPlan, style: Style, width: usize) -> String {
    let mut table = Table::new(columns("reaped", "detail", false)).indent(2);
    for id in &plan.workspaces {
        table = table.row(vec![
            token("reaped", Role::BeaconGreen),
            Cell::plain("workspace"),
            Cell::muted(format!("{id}, directory gone")),
        ]);
    }
    for target in &plan.resources {
        table = table.row(vec![
            token("reaped", Role::BeaconGreen),
            Cell::plain(target.kind.to_string()),
            Cell::muted(format!("{}, {}", target.reference, target.workspace)),
        ]);
    }
    for lease in &plan.leases {
        table = table.row(vec![
            token("reaped", Role::BeaconGreen),
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
            },
        }
    }

    fn rendered(output: &Output, style: Style) -> String {
        human(output, style, Terminal::at(88))
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
        assert!(text.contains("UP      api        5460"), "{text}");
        assert!(text.contains("DOWN    web        5461"), "{text}");
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
