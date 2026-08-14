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
    CheckData, CheckDryRun, CleanData, CleanDryRun, DispatchData, DoctorData, Envelope, Finding,
    GuildBundleData, GuildInitData, GuildSyncData, Headline, InitData, InitDryRun, MachineInitData,
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
        Output::MachineInit(envelope) => machine_init(envelope, style, width),
        Output::Doctor(envelope) => doctor(envelope, style, width),
        Output::GuildSync(envelope) => guild_sync(envelope, style, width),
        Output::GuildInit(envelope) => guild_init(envelope, style, width),
        Output::GuildBundle(envelope) => guild_bundle(envelope, style, width),
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
                Cell::muted(ids(&data.reaped_runs, KEEP)),
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

// ------------------------------------------------------------- M2: the machine
// and the guild

/// The `STATUS · CHECK · DETAIL · TIME` table both machine verbs draw.
///
/// **`armada init` and `armada doctor` share it because they are asking one
/// question** — what is the state of this machine — and a reader who has met
/// one has met the other. The `TIME` column is present and empty on every row:
/// nothing here is timed, and the column stays so the four columns of the
/// agreed layout are the same four columns everywhere (`render.rs`'s header).
fn machine_table(rows: &[Finding], style: Style, width: usize) -> Table {
    let mut table = Table::new(columns("check", "detail", true)).indent(2);
    for row in rows {
        table = table.row(vec![
            token(row.status.word(), Role::for_health(row.status)),
            Cell::plain(row.check.clone()),
            detail_cell(style, Some(row.detail.as_str())),
            time_cell(style, None),
        ]);
    }
    let _ = width;
    table
}

/// The `→` lines. **The point of `armada doctor`**: a check that reports a
/// problem without the command that fixes it sends the reader to the
/// documentation, which is most of what the verb exists to save.
fn fix_lines(rows: &[Finding], style: Style) -> String {
    let mut out = String::new();
    for remedy in rows.iter().filter_map(|row| row.remedy.as_deref()) {
        out.push_str(&format!(
            "  {}\n",
            style.paint(Role::SteelGrey, &format!("{} {remedy}", style.fix_arrow()))
        ));
    }
    out
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
    let mut out = machine_table(&data.results, style, width).render(style, width);

    if let Some(choice) = &data.guild {
        out.push('\n');
        out.push_str(&format!(
            "{}\n",
            style.paint(Role::SignalAmber, &choice.question)
        ));
        // **The options and the answer on one line**, because the question has
        // three answers and a menu of three is a list you scan rather than read.
        let mut line = String::from("  ");
        for (index, option) in choice.options.iter().enumerate() {
            line.push_str(&style.paint(Role::NavalBlue, &(index + 1).to_string()));
            line.push(' ');
            line.push_str(option);
            line.push_str("  ");
        }
        line.push_str(&style.paint(Role::SteelGrey, style.caret()));
        line.push(' ');
        line.push_str(&style.paint(Role::RadarCyan, &choice.chosen.to_string()));
        out.push_str(&format!("{line}\n"));
    }

    if !data.imported.is_empty() {
        out.push('\n');
        out.push_str(&format!(
            "  {}\n",
            style.paint(Role::SteelGrey, &data.imported.join(style.between()))
        ));
    }

    for asked in &data.asked {
        out.push('\n');
        out.push_str(&format!(
            "{}  {}\n",
            style.paint(Role::SignalAmber, &format!("{}/{}", asked.number, asked.of)),
            style.paint(Role::SteelGrey, &asked.prompt)
        ));
        // Indented to line up under the prompt rather than under the number:
        // the hint belongs to the question, not to the count.
        out.push_str(&format!(
            "     {} {}\n",
            style.paint(Role::SteelGrey, &asked.hint),
            style.paint(Role::RadarCyan, style.caret())
        ));
    }

    out.push('\n');
    out.push_str(&summary(
        style,
        envelope.status,
        &[
            format!("guild at {}", data.guild_path),
            format::count(data.questions, "question"),
            format!("{} skipped", data.skipped),
        ],
    ));
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error, style));
    }
    out
}

/// `armada doctor` — what this machine is missing.
fn doctor(envelope: &Envelope<DoctorData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut out = machine_table(&data.results, style, width).render(style, width);
    out.push('\n');
    out.push_str(&match data.headline {
        Some(word) => headline(style, word, &data.tally),
        None => summary(style, envelope.status, &data.tally),
    });
    out.push_str(&fix_lines(&data.results, style));
    out
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
            time_cell(style, None),
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
            let mut facts = vec![format::count(conflicts, "conflict")];
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
        None => facts.push("no remote — export still works".to_string()),
    }
    facts
}

/// `armada guild init`.
fn guild_init(envelope: &Envelope<GuildInitData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut table = Table::new(columns("step", "detail", true)).indent(2);
    table = table.row(vec![
        token("imported", Role::BeaconGreen),
        Cell::plain("~/.claude/"),
        detail_cell(style, Some(&data.imported.join(", "))),
        time_cell(style, None),
    ]);
    // **Always a row, even when nothing was withheld.** "Armada looked and
    // found no credentials" and "nobody looked" read identically otherwise, and
    // only one of them is a guarantee — the same reasoning `clean` keeps its
    // table for.
    table = table.row(vec![
        token(
            "withheld",
            if data.withheld.is_empty() {
                Role::SteelGrey
            } else {
                Role::FlareOrange
            },
        ),
        Cell::plain(format::count(data.withheld.len(), "value")),
        detail_cell(
            style,
            Some(&if data.withheld.is_empty() {
                "no credential-shaped values found".to_string()
            } else {
                format!("{} -> machine.yml", ids(&data.withheld, KEEP))
            }),
        ),
        time_cell(style, None),
    ]);
    table = table.row(vec![
        token("wrote", Role::BeaconGreen),
        Cell::plain(format::count(data.wrote.len(), "file")),
        detail_cell(style, Some(&ids(&data.wrote, KEEP))),
        time_cell(style, None),
    ]);
    table = table.row(vec![
        token("guild", Role::BeaconGreen),
        Cell::plain("initialised"),
        detail_cell(
            style,
            Some(match &data.remote {
                Some(remote) => remote.as_str(),
                None => "no remote — sync off, export still works",
            }),
        ),
        time_cell(style, None),
    ]);

    let mut out = table.render(style, width);
    out.push('\n');
    out.push_str(&summary(
        style,
        envelope.status,
        &[
            format!("guild at {}", data.guild_path),
            format::count(data.questions, "question"),
            format!("{} skipped", data.skipped),
        ],
    ));
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error, style));
    }
    out
}

/// `armada guild export` and `armada guild import`.
fn guild_bundle(envelope: &Envelope<GuildBundleData>, style: Style, width: usize) -> String {
    let data = &envelope.data;
    let mut table = Table::new(columns("bundle", "detail", true)).indent(2);
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
        time_cell(style, None),
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
        time_cell(style, None),
    ]);
    for skipped in &data.skipped {
        table = table.row(vec![
            token("skipped", Role::SteelGrey),
            Cell::plain(skipped.clone()),
            detail_cell(style, Some("this machine has its own")),
            time_cell(style, None),
        ]);
    }
    for conflict in &data.conflicts {
        table = table.row(vec![
            token("conflict", Role::DistressRed),
            Cell::plain(conflict.clone()),
            detail_cell(style, Some("edited here — left alone")),
            time_cell(style, None),
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
            has_row(&text, &["owns", "container", "armada-a3f91c02-api"]),
            "{text}"
        );
        assert!(has_row(&text, &["owns", "volume", "pgdata"]), "{text}");
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
            has_row(&text, &["owns", "resources", "-"]),
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
            has_row(&text, &["owns", "resources", "+2", "more"]),
            "{text}"
        );
        assert!(has_row(&text, &["owns", "container", "one"]), "{text}");
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

    // ------------------------------------------------------------------ M2

    fn a_guild_init(withheld: &[&str], remote: Option<&str>, skipped: usize) -> Output {
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
                remote: remote.map(str::to_string),
                questions: 5,
                skipped,
            },
        )))
    }

    /// **The withheld row is drawn even when nothing was withheld**, because
    /// "Armada looked and found no credentials" and "nobody looked" read
    /// identically as an absence — and only one of them is a guarantee. Same
    /// reasoning as `clean` keeping its table.
    #[test]
    fn guild_init_says_so_when_it_withheld_nothing() {
        let text = rendered(&a_guild_init(&[], None, 5), Style::plain());
        assert!(
            has_row(
                &text,
                &[
                    "withheld",
                    "0",
                    "values",
                    "no",
                    "credential-shaped",
                    "values",
                    "found",
                    "-"
                ]
            ),
            "{text}"
        );
        assert!(text.contains("5 questions, 5 skipped"), "{text}");
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
            &["secrets", "excluded", "machine.yml", "stays", "here", "-"]
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
