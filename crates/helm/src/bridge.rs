//! The live screen: the alternate screen, the redraw loop, and the keyboard.
//!
//! **This one owns the alternate screen, and it is the only thing in Armada
//! that does.** The interview's text area and the selector both draw in an
//! inline viewport, because the output above them is a report the reader has
//! just finished reading (`docs/commands/render.md`). The Bridge is the
//! opposite: a persistent view of a fleet that keeps changing, with no
//! scrollback worth keeping — `htop`, not a prompt.
//!
//! **Everything is restored on the way out, including on a panic.** A TUI that
//! dies leaving raw mode on is worse than no TUI: the shell stops echoing what
//! you type and the panic message itself steps diagonally down the screen.
//! [`Alt`] and [`Restore`] are both drop guards and both install a panic hook,
//! so the terminal comes back whichever way the process leaves.
//!
//! **Nothing here decides anything.** Which key means what, which rows survive
//! a filter and where the cursor goes are all
//! [`armada_core::fleet::bridge`]'s; what lives here is reading an event,
//! reading a frame on a cadence, and drawing.

use std::io::Stdout;
use std::time::{Duration, Instant};

use armada_core::ctx::{Clock, Run};
use armada_core::envelope::ShowData;
use armada_core::error::ArmadaError;
use armada_core::fleet::bridge::{
    self, Action, Departure, Done, Frame, Key, Mode, Pressed, Screen,
};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};

use crate::ask::terminal::Restore;
use crate::render::palette::Role;
use crate::render::style::Style;
use crate::render::table::Span;
use crate::render::term::Terminal;
use crate::render::{self, frame, live};
use crate::verbs::bridge::{BridgeView, Options};
use crate::verbs::fleet::Where;

/// **The threshold between 033's two mock-ups.** At or above it, all seven
/// boxes draw; below it, whole panels drop rather than shed columns — MANIFEST,
/// GUILD and SYSTEM disappear and only ARMADA, JOBS, INBOX and KEYS remain,
/// matching the 96-column mock exactly (measured: the 138-column mock is the
/// narrowest width every box's own minimum content fits inside without
/// overhanging).
///
/// **This lives here, not in `render/frame.rs`.** Which boxes exist at a given
/// width is a decision about Jobs, Inbox, Manifest, Guild and System — the
/// four modules `frame.rs` "must know nothing about" (`ARCHITECTURE.md`
/// §1.9) — so the width branch is the caller's, and `frame.rs`'s own
/// `shed_to_narrow` stays scoped to the one thing it already does: trimming a
/// single key line.
const WIDE: usize = 138;

/// The alternate screen, given back on drop **and on panic**.
///
/// **A separate guard from [`Restore`] rather than a flag on it**, because the
/// two are wanted separately: every other widget in Armada takes raw mode and
/// deliberately does *not* take the screen. Composing them is what keeps that
/// true — this is the one caller that wants both.
struct Alt;

impl Alt {
    /// Take the screen, or say why not.
    fn enter() -> Result<Alt, std::io::Error> {
        execute!(std::io::stdout(), EnterAlternateScreen)?;
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
            previous(info);
        }));
        Ok(Alt)
    }
}

impl Drop for Alt {
    fn drop(&mut self) {
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        let _ = std::panic::take_hook();
    }
}

/// One `crossterm` key as one of the Bridge's, or nothing.
///
/// **The whole of the mapping, and it decides nothing** — what each [`Key`]
/// then means is the core's, which is what lets the bindings be unit tests
/// rather than a person at a keyboard.
pub fn key_of(code: KeyCode, modifiers: KeyModifiers) -> Option<Key> {
    if modifiers.contains(KeyModifiers::CONTROL) {
        // **Two chords, and no others.** `ctrl-c` leaves. `ctrl-d` is the
        // text area's *done* everywhere else in Armada (`ask/editor.rs`), so it
        // is carried through here for the compose box and ignored by every
        // other mode — a key the reader pressed and nothing answered is what
        // this used to be, on the one screen that now has a box to answer it.
        return match code {
            KeyCode::Char('c') => Some(Key::Interrupt),
            KeyCode::Char('d') => Some(Key::Save),
            _ => None,
        };
    }
    match code {
        KeyCode::Up => Some(Key::Up),
        KeyCode::Down => Some(Key::Down),
        KeyCode::Enter => Some(Key::Enter),
        KeyCode::Esc => Some(Key::Esc),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::Tab => Some(Key::Tab),
        KeyCode::Char(c) => Some(Key::Char(c)),
        _ => None,
    }
}

/// Everything one redraw puts on the screen, as coloured pieces.
///
/// **Built without a terminal**, which is the point: a frame is a value, so what
/// the Bridge draws is asserted in a unit test instead of photographed.
pub fn paint(
    view: &BridgeView,
    screen: &Screen,
    detail: Option<&ShowData>,
    workspace_id: &str,
    cwd: &str,
    style: Style,
    width: usize,
) -> Vec<Vec<Span>> {
    let frame = &view.fleet;

    // **The detail view covers the whole screen rather than sitting beside
    // it.** The question it answers — *why does this one need me* — is about
    // one Job, and 033's own mock-up gives it a screen of its own.
    if let Mode::Detail(job) = &screen.mode {
        return detail_pane(job.as_str(), detail, style, width);
    }

    // **The preview replaces the command centre rather than sitting under
    // it.** It is a different question — what would be deleted — and drawing
    // both would put two cursors on one screen.
    if let Mode::Reaping(reap) = &screen.mode {
        let mut lines = preview(reap, style, width);
        lines.push(Vec::new());
        lines.push(vec![
            plain("  "),
            piece(render::reap_keys(), Role::SteelGrey),
        ]);
        return lines;
    }

    // **Every binding, because the line could not carry them all.** The line
    // drops its lowest-priority pairs rather than wrapping — wrapping would make
    // the frame one row taller — and this is where the ones it dropped are.
    if screen.mode == Mode::Keys {
        let selected = screen.cursor.selected(&frame.rows).map(|row| row.state);
        let mut table = crate::render::table::Table::new(vec![
            crate::render::table::Column::fixed("key", "key"),
            crate::render::table::Column::flexible("does", "does"),
        ])
        .indent(2);
        for (key, does) in render::bridge_every_key(selected) {
            table = table.row(vec![
                crate::render::table::Cell::painted(key, Role::SignalAmber),
                crate::render::table::Cell::muted(does),
            ]);
        }
        let mut lines = vec![
            vec![plain("  "), bold("KEYS", Role::SignalAmber)],
            Vec::new(),
        ];
        lines.extend(table.spans(style, width));
        lines.push(Vec::new());
        lines.push(vec![
            plain("  "),
            piece("any key returns to the fleet", Role::SteelGrey),
        ]);
        return lines;
    }

    // **The compose box takes the KEYS box's place and leaves every other box
    // standing.** `n` used to end the screen to ask for a task, and what a
    // first reader reported was *"it took me to a screen that felt like it
    // took me out of the bridge"* — writing a task is a thing you do while
    // watching, not a different screen. Its own key line already names
    // `ctrl-d`/`esc`, so drawing the command centre's KEYS box too would be
    // two key lines answering one question.
    let composing = matches!(screen.mode, Mode::Composing(_));
    let mut lines = watching_screen(view, screen, workspace_id, cwd, style, width, !composing);

    if let Mode::Composing(typed) = &screen.mode {
        lines.push(Vec::new());
        lines.extend(compose_box(typed, style, width));
    }
    lines
}

/// The command centre, watching — 033's two mock-ups, chosen by [`WIDE`].
#[allow(clippy::too_many_arguments)]
fn watching_screen(
    view: &BridgeView,
    screen: &Screen,
    workspace_id: &str,
    cwd: &str,
    style: Style,
    width: usize,
    keys: bool,
) -> Vec<Vec<Span>> {
    let frame = &view.fleet;
    let data = crate::verbs::bridge::data(frame.clone());
    let cursor = Some(screen.cursor.at());

    let mut lines = render::armada_header(&frame.windows, workspace_id, cwd, width);
    lines.push(Vec::new());

    if width >= WIDE {
        let jobs_inbox = frame::hjoin(
            render::jobs_box(&data, style, cursor, jobs_width(width)),
            render::inbox_box(&view.inbox, style, width - jobs_width(width) - GAP),
            GAP,
            width,
        );
        lines.extend(jobs_inbox);
        lines.push(Vec::new());

        let third = (width - 2 * GAP) / 3;
        let manifest_guild = frame::hjoin(
            render::manifest_box(third),
            render::guild_box(&view.guild, style, third),
            GAP,
            2 * third + GAP,
        );
        let three = frame::hjoin(
            manifest_guild,
            render::system_box(&view.system, style, width - 2 * third - 2 * GAP),
            GAP,
            width,
        );
        lines.extend(three);
        if keys {
            lines.push(Vec::new());
            lines.extend(render::command_centre_keys_wide(width));
        }
    } else {
        lines.extend(render::jobs_box(&data, style, cursor, width));
        lines.extend(render::inbox_box(&view.inbox, style, width));
        if keys {
            lines.extend(render::command_centre_keys_narrow(width));
        }
    }

    if frame.rows.is_empty() {
        lines.push(vec![
            plain("  "),
            piece(
                match frame.filter {
                    Some(_) => "no Jobs match",
                    None => "no Jobs",
                },
                Role::SteelGrey,
            ),
        ]);
    }

    // **One line for whatever the screen has to say back**, and the filter box
    // while it is open — kept even when empty so nothing above it moves.
    match &screen.mode {
        Mode::Filtering(typed) => lines.push(vec![
            plain("  "),
            piece("filter ", Role::SignalAmber),
            plain(format!("{typed}{}", style.caret())),
        ]),
        // **One line reserved either way**, empty when there is nothing to
        // say — the same rule the single-table Bridge followed, so the key
        // line below never walks up and down as a notice comes and goes.
        _ => lines.push(match &screen.notice {
            Some(notice) => vec![plain("  "), piece(notice.clone(), Role::FlareOrange)],
            None => Vec::new(),
        }),
    }

    lines
}

/// Two spaces between boxes, the width `hjoin` inserts between the two it
/// joins.
const GAP: usize = 2;

/// JOBS gets the wider half of the top row — it carries the most columns.
fn jobs_width(width: usize) -> usize {
    (width - GAP) * 3 / 5
}

/// The new-Job box: what is being asked, what has been typed, and the keys.
///
/// **The keys are named, and they are the interview's** — [`render::prose_keys`]
/// is the one place all three are spelled, so the box in the Bridge and the box
/// in `armada init` cannot come to mean different chords. This surface had none
/// at all, and the person who met it first guessed `ctrl-d` from having used the
/// interview: a guess that happened to be right is still a guess.
fn compose_box(typed: &str, style: Style, width: usize) -> Vec<Vec<Span>> {
    let mut lines = vec![vec![
        plain("  "),
        bold("NEW JOB", Role::SignalAmber),
        plain("   "),
        piece("what should it do?", Role::SteelGrey),
    ]];

    // **Hard-wrapped, not word-wrapped, so the caret lands where the next
    // character will.** `ask/editor.rs` measures its own cursor the same way,
    // and a task half-typed has no spaces to break on yet.
    let room = width.saturating_sub(4).max(1);
    let mut rows: Vec<String> = Vec::new();
    for line in typed.split('\n') {
        let mut chars = line.chars().peekable();
        loop {
            let row: String = chars.by_ref().take(room).collect();
            let last = chars.peek().is_none();
            rows.push(row);
            if last {
                break;
            }
        }
    }
    // The caret goes on the last row, which is where typing continues.
    if let Some(last) = rows.last_mut() {
        last.push_str(style.caret());
    }
    for row in rows {
        lines.push(vec![plain("  "), piece(row, Role::RadarCyan)]);
    }

    lines.push(Vec::new());
    lines.push(vec![
        plain("  "),
        // **Two spaces between pairs, like every other key line on this
        // screen**, rather than the interview's middle dot: the dot costs a
        // column per gap and is one character for a terminal and two for a
        // pipe, and the Bridge's lines are measured (`render.rs`).
        piece(
            render::prose_keys("starts it", "starts nothing", "  "),
            Role::SteelGrey,
        ),
    ]);
    lines
}

/// One Job, in full — 033's own mock-up: identity, then `hjoin(WORKFLOW,
/// NEEDS YOU)`, then `TIMELINE` full width, then `hjoin(REPORTS, FACTS)`,
/// then its own `KEYS` box.
///
/// **Fed only by `ShowData`**, still "one description emitted three ways"
/// (PLAN.md §3.1.1): `armada fleet show <job>` at a terminal, the same
/// command through a pipe, and this screen all read the one payload.
fn detail_pane(job: &str, detail: Option<&ShowData>, style: Style, width: usize) -> Vec<Vec<Span>> {
    let Some(data) = detail else {
        // **A Job that went while the pane was open says so.** `kill` in
        // another shell is the ordinary way this happens, and a pane that
        // simply emptied would read as a failed redraw.
        return vec![
            vec![
                plain("  "),
                bold("ARMADA BRIDGE", Role::SignalAmber),
                plain("   "),
                piece(job.to_string(), Role::NavalBlue),
            ],
            Vec::new(),
            vec![
                plain("  "),
                piece(
                    format!("`{job}` is no longer in the fleet"),
                    Role::SteelGrey,
                ),
            ],
        ];
    };

    let mut lines = render::detail_identity(data, width);
    lines.push(Vec::new());

    let half = (width - GAP) / 2;
    lines.extend(frame::hjoin(
        render::workflow_box(data, style, half),
        render::needs_you_box(data, width - half - GAP),
        GAP,
        width,
    ));
    lines.push(Vec::new());
    lines.extend(render::timeline_box(&data.transitions, style, width));
    lines.push(Vec::new());
    lines.extend(frame::hjoin(
        render::reports_box(&data.progress, style, half),
        render::facts_box(data, style, width - half - GAP),
        GAP,
        width,
    ));
    lines.push(Vec::new());
    lines.extend(render::detail_keys(width));
    lines
}

/// The reap preview, drawn.
///
/// **Words rather than a tick and a cross**, for the rule every table in Armada
/// follows: a glyph that only appears at a terminal gives the two audiences
/// different shapes. `take` and `keep` fold to themselves.
fn preview(reap: &bridge::Reap, style: Style, width: usize) -> Vec<Vec<Span>> {
    let mut table = crate::render::table::Table::new(vec![
        crate::render::table::Column::fixed("", ""),
        crate::render::table::Column::fixed("status", "status"),
        crate::render::table::Column::fixed("job", "job"),
        // **The uuid is on the row, because a name is not unique.** Two Jobs can
        // share one — `name_is_taken` only refuses to reuse a *live* Job's name
        // — and a preview of what is about to be deleted that cannot tell two
        // rows apart is a preview that cannot be read.
        crate::render::table::Column::fixed("uuid", "uuid"),
        crate::render::table::Column::fixed("state", "state"),
        crate::render::table::Column::flexible("holding", "holding"),
    ])
    .indent(2);

    for (index, row) in reap.rows.iter().enumerate() {
        table = table.row(vec![
            match reap.cursor.is_on(index) {
                true => crate::render::table::Cell::painted(style.caret(), Role::SignalAmber),
                false => crate::render::table::Cell::empty(),
            },
            match row.selected {
                true => crate::render::table::Cell::painted("take", Role::FlareOrange),
                false => crate::render::table::Cell::painted("keep", Role::SteelGrey),
            },
            crate::render::table::Cell::painted(row.target.job.clone(), Role::NavalBlue),
            crate::render::table::Cell::muted(row.target.short().to_string()),
            crate::render::table::Cell::painted(
                row.state.word(),
                render::palette::Role::for_job_state(row.state),
            ),
            crate::render::table::Cell::muted(row.holding.clone()),
        ]);
    }

    let mut lines = vec![
        vec![
            plain("  "),
            bold("REAP", Role::FlareOrange),
            plain("   "),
            piece(
                format!(
                    "{} of {} ticked",
                    reap.rows.iter().filter(|row| row.selected).count(),
                    reap.rows.len()
                ),
                Role::SteelGrey,
            ),
        ],
        Vec::new(),
    ];
    lines.extend(table.spans(style, width));
    if reap.rows.is_empty() {
        lines.push(vec![plain("  "), piece("nothing to reap", Role::SteelGrey)]);
    }
    lines
}

fn piece(text: impl Into<String>, role: Role) -> Span {
    Span {
        text: text.into(),
        role: Some(role),
        bold: false,
    }
}

fn bold(text: impl Into<String>, role: Role) -> Span {
    Span {
        text: text.into(),
        role: Some(role),
        bold: true,
    }
}

fn plain(text: impl Into<String>) -> Span {
    Span {
        text: text.into(),
        role: None,
        bold: false,
    }
}

/// Take the screen and watch until something says to stop.
///
/// **The read is on a cadence and the keyboard is not.** `event::poll` waits up
/// to whatever is left of the interval, so a keypress is answered immediately
/// and the fleet is re-read exactly as often as `--interval` says — rather than
/// the Bridge sleeping through a keystroke or spinning on an empty queue.
// Eight, and each is a seam rather than a parameter: the two `ctx` ports, where
// the fleet is, what was asked for, the filter, the two halves of the terminal,
// and what to do when a key acts. Bundling them would be one struct per call
// site with no other use.
#[allow(clippy::too_many_arguments)]
pub fn watch<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    options: &Options,
    screen: &mut Screen,
    style: Style,
    terminal: Terminal,
    act: &mut dyn FnMut(&Action) -> Done,
) -> Result<(Frame, Departure), ArmadaError> {
    let raw = Restore::install().map_err(|_| crate::verbs::bridge::no_screen())?;
    let alt = Alt::enter().map_err(|_| crate::verbs::bridge::no_screen())?;
    let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
    let mut view: ratatui::Terminal<ratatui::backend::CrosstermBackend<Stdout>> =
        ratatui::Terminal::new(backend).map_err(|_| crate::verbs::bridge::no_screen())?;

    // **Read once, not once per frame.** `WorkspaceId::derive` hashes a path —
    // no IO — so this costs nothing to compute every redraw, but it is the
    // same value every time and reads better named once.
    let workspace_id = armada_core::id::WorkspaceId::derive(&place.cwd);
    let workspace_id = workspace_id.as_str();
    let cwd = place.cwd.display().to_string();

    let interval = Duration::from_secs(options.interval_s);
    let outcome = loop {
        let centre = crate::verbs::bridge::read_all(run, now, place, screen.filter.as_ref())?;
        let frame = centre.fleet.clone();
        screen.cursor.clamp(frame.rows.len());
        // **Re-read on the same cadence as the frame.** An open detail pane is
        // the Bridge, and a Bridge that froze one Job while the rest kept moving
        // would be the one screen in Armada that lies about being live.
        let mut detail = detail_of(run, now, place, screen);
        draw(
            &mut view,
            &centre,
            screen,
            detail.as_deref(),
            workspace_id,
            &cwd,
            style,
            terminal,
        );

        let deadline = Instant::now() + interval;
        let departure = loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break None;
            }
            match event::poll(left) {
                Ok(false) => break None,
                // A terminal whose event stream has died is a terminal that is
                // gone; leaving is the only honest answer.
                Err(_) => break Some(Departure::Quit),
                Ok(true) => {}
            }
            let Ok(Event::Key(key)) = event::read() else {
                continue;
            };
            if key.kind == KeyEventKind::Release {
                continue;
            }
            let Some(pressed) = key_of(key.code, key.modifiers) else {
                continue;
            };
            let showing = screen.filter.clone();
            match bridge::press(screen, &frame.rows, pressed) {
                Pressed::Leave(departure) => break Some(departure),
                Pressed::Stay => {
                    // The key may have just opened the pane, or moved it to
                    // another row; either way what it shows is read now rather
                    // than at the next tick, for the reason a committed filter
                    // re-reads now.
                    detail = detail_of(run, now, place, screen);
                    draw(
                        &mut view,
                        &centre,
                        screen,
                        detail.as_deref(),
                        workspace_id,
                        &cwd,
                        style,
                        terminal,
                    );
                }
                // **The action runs here, on the screen, and its failure is a
                // line under the table.** This is the whole of the fix: an
                // abort that could not remove a worktree used to end the Bridge
                // and print into a shell the reader was no longer looking at,
                // taking away the view of four other Jobs to say one thing
                // about a fifth.
                Pressed::Act(action) => {
                    let done = act(&action);
                    screen.notice = Some(done.notice);
                    if let Some(mode) = done.mode {
                        screen.mode = mode;
                    }
                    // The fleet has just changed underneath the frame in hand,
                    // so it is re-read now rather than at the next tick.
                    break None;
                }
            }
            // **A changed filter re-reads now rather than at the next tick.**
            // The frame in hand was built under the old expression, so drawing
            // it again would show the rows the filter just excluded and a
            // summary counting them — for up to a whole interval, which reads
            // as the filter having been ignored.
            if screen.filter != showing {
                break None;
            }
        };
        if let Some(departure) = departure {
            break (frame, departure);
        }
    };

    // **Given back before anything is printed.** The frame the caller renders
    // belongs in the scrollback the Bridge was covering, not on top of it.
    let _ = view.clear();
    drop(alt);
    drop(raw);

    Ok(outcome)
}

/// What the open pane is showing, or `None` when no pane is open.
///
/// **One call to the verb, and no read of its own.** `armada fleet show <job>`
/// is the whole of it, so the pane cannot drift from what the same command
/// prints in a shell — and a Job that has gone comes back as `None` rather than
/// as an error that would tear down the screen.
fn detail_of<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    screen: &Screen,
) -> Option<Box<ShowData>> {
    let Mode::Detail(job) = &screen.mode else {
        return None;
    };
    match crate::verbs::fleet::show(run, now, place, job) {
        Ok(crate::verbs::Output::Show(envelope)) => Some(Box::new(envelope.data)),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw(
    view: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<Stdout>>,
    centre: &BridgeView,
    screen: &Screen,
    detail: Option<&ShowData>,
    workspace_id: &str,
    cwd: &str,
    style: Style,
    terminal: Terminal,
) {
    let lines: Vec<ratatui::text::Line<'static>> = paint(
        centre,
        screen,
        detail,
        workspace_id,
        cwd,
        style,
        terminal.usable_width(),
    )
    .iter()
    .map(|spans| {
        ratatui::text::Line::from(
            spans
                .iter()
                .map(|span| live::paint(span, style))
                .collect::<Vec<_>>(),
        )
    })
    .collect();
    // A failed write is dropped: the next redraw is two seconds away and the
    // fleet is unaffected either way.
    let _ = view.draw(|f| {
        f.render_widget(ratatui::widgets::Paragraph::new(lines), f.area());
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::envelope::JobRow;
    use armada_core::error::Status;
    use armada_core::fleet::job::Remaining;
    use armada_core::fleet::JobState;

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
                attempts: 2,
                cost_usd: 1.0,
                wall_clock_ms: 1,
            },
            needs_attention: needs,
            acting: None,
            acting_for_s: None,
        }
    }

    fn frame() -> Frame {
        Frame {
            rows: vec![
                row("rate-limit", JobState::Running, false),
                row("release-merge", JobState::Blocked, true),
            ],
            running: 1,
            needs_you: 1,
            spent_usd: 4.20,
            hidden: 0,
            filter: None,
            windows: Vec::new(),
        }
    }

    fn text(lines: &[Vec<Span>]) -> Vec<String> {
        lines
            .iter()
            .map(|spans| {
                spans
                    .iter()
                    .map(|span| span.text.as_str())
                    .collect::<String>()
            })
            .collect()
    }

    /// An empty inbox, guild and system reading, over a given fleet frame —
    /// what every test that is not itself about a panel other than JOBS wants.
    fn view(fleet: Frame) -> BridgeView {
        BridgeView {
            fleet,
            inbox: armada_core::envelope::InboxData::default(),
            guild: armada_core::envelope::GuildListData {
                at: String::new(),
                items: Vec::new(),
                facts: Vec::new(),
                template: None,
            },
            system: armada_core::envelope::DoctorData {
                results: Vec::new(),
                headline: None,
                tally: Vec::new(),
            },
        }
    }

    fn drawn(screen: &Screen) -> Vec<String> {
        drawn_at(&frame(), screen, 80)
    }

    /// The same, at a stated width — for the layout that grows into a wide
    /// terminal, which is the one thing a fixed-width helper cannot show.
    fn drawn_at(fleet: &Frame, screen: &Screen, width: usize) -> Vec<String> {
        text(&paint(
            &view(fleet.clone()),
            screen,
            None,
            "c24a68b6",
            "~/Development/armada",
            Style::plain(),
            width,
        ))
    }

    /// **Every binding the page names, and nothing that is not one.**
    #[test]
    fn each_crossterm_key_maps_to_the_bridges_own() {
        for (code, expected) in [
            (KeyCode::Up, Some(Key::Up)),
            (KeyCode::Down, Some(Key::Down)),
            (KeyCode::Enter, Some(Key::Enter)),
            (KeyCode::Esc, Some(Key::Esc)),
            (KeyCode::Backspace, Some(Key::Backspace)),
            (KeyCode::Tab, Some(Key::Tab)),
            (KeyCode::Char('q'), Some(Key::Char('q'))),
            (KeyCode::Char('/'), Some(Key::Char('/'))),
            (KeyCode::F(4), None),
            (KeyCode::Home, None),
        ] {
            assert_eq!(key_of(code, KeyModifiers::NONE), expected, "{code:?}");
        }
    }

    /// **Two chords and no others.** `ctrl-c` leaves; `ctrl-d` is the text
    /// area's *done*, carried through for the compose box and ignored by every
    /// mode that has no box open. Any other chord is a keypress the reader made
    /// and nothing responded to, so it is not a key at all.
    #[test]
    fn ctrl_c_leaves_ctrl_d_saves_and_no_other_chord_is_a_key() {
        assert_eq!(
            key_of(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Some(Key::Interrupt)
        );
        assert_eq!(
            key_of(KeyCode::Char('d'), KeyModifiers::CONTROL),
            Some(Key::Save)
        );
        assert_eq!(key_of(KeyCode::Char('a'), KeyModifiers::CONTROL), None);
        // A bare `c` is the chat key, not an interrupt.
        assert_eq!(
            key_of(KeyCode::Char('c'), KeyModifiers::NONE),
            Some(Key::Char('c'))
        );
    }

    /// The frame is the agreed columns, the summary and the keys — and the
    /// cursor is a character rather than a colour, so a monochrome terminal
    /// still knows which row it is on.
    #[test]
    fn a_frame_draws_the_columns_the_summary_and_the_keys() {
        let drawn = drawn(&Screen::default());
        let all = drawn.join("\n");
        assert!(all.contains("ARMADA"), "{all}");
        assert!(all.contains("c24a68b6"), "{all}");
        // **`TASK` is not in this list any more**, and its absence at eighty
        // columns is the trade `render.rs`'s `SHED` states: a row needs you, so
        // the question is on the line and the task is one keypress away.
        for header in ["STATUS", "JOB", "ID", "RUN", "SPENT", "NEEDS YOU"] {
            assert!(all.contains(header), "no {header} column:\n{all}");
        }
        assert!(all.contains("RUNNING"), "{all}");
        assert!(all.contains("2 jobs"), "{all}");
        assert!(all.contains("1 need you"), "{all}");
        assert!(all.contains("q quit"), "{all}");
        // **No progress column, deliberately** — nothing emits percent-complete.
        assert!(!all.contains("PROGRESS"), "{all}");
    }

    /// **The id is on every row, because a generated handle is not readable.**
    /// `now-that` is what deriving two significant words out of a sentence
    /// produces; the handle stays for typing and the id is what a person checks
    /// a row against — and it is the same eight characters `armada fleet ls`
    /// prints, so the two listings can be matched by eye.
    #[test]
    fn every_row_carries_the_short_id_that_fleet_ls_prints() {
        let all = drawn(&Screen::default()).join("\n");
        assert!(all.contains(" ID "), "no ID column:\n{all}");
        for row in &frame().rows {
            let short = armada_fleet::jobs::short(&row.uuid);
            assert_eq!(short.chars().count(), 8, "the short form changed length");
            assert!(
                all.contains(short),
                "`{short}` is not on the screen:\n{all}"
            );
        }
    }

    /// **The table grows into a wide terminal and sheds in a stated order**
    /// (`020` §"Also decided"): `WORKFLOW` first, then `TURNS`, then `TASK`.
    ///
    /// The assertion is on the *order* rather than on three widths, because the
    /// widths themselves depend on how long the Jobs happen to be called — what
    /// must not change is that a column never survives one that outranks it.
    ///
    /// **Checked within each of 033's two regimes, not across their boundary.**
    /// The command centre gives JOBS the *whole* screen under [`WIDE`] and
    /// three-fifths of it at or above — a deliberate jump at the one width
    /// where a whole different box (INBOX) starts sharing the row, not a
    /// continuous shrink. `no_row_overhangs_the_terminal_at_any_width` still
    /// covers the property that actually has to hold across the jump: nothing
    /// ever draws off the edge.
    #[test]
    fn the_table_sheds_workflow_then_turns_then_task_as_the_terminal_narrows() {
        let frame = frame();
        let seen = |width: usize| {
            let all = drawn_at(&frame, &Screen::default(), width).join("\n");
            (
                all.contains("WORKFLOW"),
                all.contains("TURNS"),
                all.contains("TASK"),
            )
        };
        // Wide enough for everything, and narrow enough for none of it.
        assert_eq!(seen(400), (true, true, true), "a wide terminal sheds");
        assert_eq!(seen(60), (false, false, false), "sixty carried an extra");

        // Nothing survives a column that outranks it, at any width within a
        // regime — checked separately either side of `WIDE`.
        for regime in [
            (WIDE..=400).rev().collect::<Vec<_>>(),
            (60..WIDE).rev().collect(),
        ] {
            let mut previous = seen(*regime.first().unwrap());
            for width in regime {
                let (workflow, turns, task) = seen(width);
                assert!(
                    !(workflow && !turns) && !(turns && !task),
                    "at {width} the shed order broke: \
                     workflow={workflow} turns={turns} task={task}"
                );
                let (was_workflow, was_turns, was_task) = previous;
                assert!(
                    !(workflow && !was_workflow) && !(turns && !was_turns) && !(task && !was_task),
                    "a column came back at {width}, going narrower"
                );
                previous = (workflow, turns, task);
            }
        }
    }

    /// **No row overhangs the screen at any width**, which is what the shedding
    /// is *for*: `ratatui` clips a line that is too long, and a clipped table is
    /// one whose right-hand columns silently stop existing.
    #[test]
    fn no_row_overhangs_the_terminal_at_any_width() {
        let frame = frame();
        for width in [80, 100, 120, 160, 200] {
            for line in drawn_at(&frame, &Screen::default(), width) {
                assert!(
                    line.chars().count() <= width,
                    "a line is {} wide at {width}: {line}",
                    line.chars().count()
                );
            }
        }
    }

    /// **`NEEDS YOU` carries the question, not `YES`**, so most answers need no
    /// second screen — and `YES` survives only where there is no question to
    /// show, which is a Job whose state wants a person with nothing raised
    /// against it.
    #[test]
    fn needs_you_shows_the_question_and_falls_back_to_yes_without_one() {
        let asked = "the CI timeout is 30s and the flake needs 90s. Raise it?";
        let mut asking = frame();
        asking.rows[1].detail = asked.to_string();
        // Wide enough that the JOBS box's own share of a wide screen still
        // carries the whole sentence, even split beside INBOX.
        let all = drawn_at(&asking, &Screen::default(), 400).join("\n");
        assert!(
            all.contains(asked),
            "the question is not on the row:\n{all}"
        );
        assert!(
            !all.contains("YES"),
            "the question and `YES` are both drawn:\n{all}"
        );

        // The fallback: `detail` is the step, which `STEP` already says, so
        // repeating it in this column would put one fact on the row twice.
        let all = drawn_at(&frame(), &Screen::default(), 400).join("\n");
        assert!(
            all.contains("YES"),
            "no fallback for a bare needs-you:\n{all}"
        );
    }

    /// **Counts over several Jobs, and no word derived from the worst row**
    /// (`020` §6). *"What's the point of an aggregate status when multiple jobs
    /// are running?"* — a word taken off one row describes no Job in particular,
    /// and the count of stalled Jobs is the fact it used to hide.
    #[test]
    fn several_jobs_are_counted_rather_than_summarised_by_one_word() {
        let mut frame = frame();
        frame
            .rows
            .push(row("xlsx-report", JobState::Stalled, false));
        let all = drawn_at(&frame, &Screen::default(), 200).join("\n");
        let line = all
            .lines()
            .find(|line| line.contains("3 jobs"))
            .expect("a summary line");
        assert!(line.contains("1 need you"), "{line}");
        assert!(line.contains("1 stalled"), "{line}");
        assert!(
            !line.starts_with("RUNNING"),
            "an aggregate word still leads the line: {line}"
        );
    }

    /// **A single Job keeps its status word, because there it means something.**
    /// It is that Job's state rather than a summary of anything, and dropping it
    /// would take a fact off the line to satisfy a rule about a case this is not.
    ///
    /// **The command centre's JOBS box states it on the row and not on a
    /// second summary line** — the `STATUS` column 033's own mock draws —
    /// which is the same fact the single-table Bridge's summary line used to
    /// lead with for exactly one Job, said once rather than twice.
    #[test]
    fn one_job_keeps_the_status_word() {
        let frame = Frame {
            rows: vec![row("rate-limit", JobState::Running, false)],
            needs_you: 0,
            ..frame()
        };
        let all = drawn_at(&frame, &Screen::default(), 200).join("\n");
        let line = all
            .lines()
            .find(|line| line.contains("rate-limit"))
            .expect("the Job's own row");
        assert!(line.contains("RUNNING"), "{line}");
    }

    /// **Window usage leads and spend follows** (`020` §4). Claude Code has
    /// reported the reader's window position on every exchange since the spike
    /// measured it and Armada discarded it; what stops him working outranks what
    /// it cost.
    ///
    /// **And it is not the banned progress bar.** The percentage is the
    /// service's own `utilization` floored, not a fraction of a turn count
    /// (PHASES.md §9.1 F2) — which is also why it is absent rather than
    /// estimated when the event did not carry one.
    ///
    /// **The window is in the `ARMADA` header now, not the summary line.**
    /// 033's mock-up draws both usage windows once, above every box, rather
    /// than repeating them on a per-Job summary — so "ahead of the spend" is
    /// now "above the JOBS box that states the spend", which this asserts by
    /// checking each line rather than one line's own ordering.
    #[test]
    fn the_window_is_drawn_ahead_of_the_spend_and_omits_what_was_not_measured() {
        let mut frame = frame();
        frame.windows = vec![armada_core::envelope::Window {
            kind: "five_hour".to_string(),
            used_percent: Some(71),
            resets_in_s: Some(2 * 3_600 + 14 * 60),
        }];
        let drawn = drawn_at(&frame, &Screen::default(), 200);
        let all = drawn.join("\n");
        let window_at = drawn
            .iter()
            .position(|line| line.contains("5h 71% resets 2h14m"))
            .expect("the window line");
        let spend_at = drawn
            .iter()
            .position(|line| line.contains("$4.20"))
            .expect("the spend line");
        assert!(
            window_at < spend_at,
            "the window is not ahead of the spend:\n{all}"
        );

        // A window the service reported without a percentage says what it has.
        frame.windows = vec![armada_core::envelope::Window {
            kind: "five_hour".to_string(),
            used_percent: None,
            resets_in_s: Some(43 * 60),
        }];
        let all = drawn_at(&frame, &Screen::default(), 200).join("\n");
        let line = all
            .lines()
            .find(|line| line.contains("resets"))
            .expect("a summary line");
        assert!(line.contains("5h resets 43m"), "{line}");
        assert!(
            !line.contains('%'),
            "a percentage nobody measured was drawn: {line}"
        );
    }

    /// The caret sits on the selected row and on no other.
    #[test]
    fn the_cursor_marks_exactly_one_row() {
        let mut screen = Screen::default();
        let first = drawn(&screen);
        let marked: Vec<&String> = first.iter().filter(|line| line.contains('>')).collect();
        assert_eq!(marked.len(), 1, "{first:#?}");
        assert!(marked[0].contains("rate-limit"), "{marked:#?}");

        screen.cursor.next(2);
        let second = drawn(&screen);
        let marked: Vec<&String> = second.iter().filter(|line| line.contains('>')).collect();
        assert_eq!(marked.len(), 1, "{second:#?}");
        assert!(marked[0].contains("release-merge"), "{marked:#?}");
    }

    /// The filter box shows what has been typed, and a notice shows instead
    /// when there is no box — one line either way, so the keys do not move.
    #[test]
    fn the_line_under_the_table_is_the_box_or_the_notice_and_is_always_there() {
        let plain = drawn(&Screen::default());

        let typing = Screen {
            mode: Mode::Filtering("state=run".to_string()),
            ..Screen::default()
        };
        let typed = drawn(&typing);
        assert_eq!(typed.len(), plain.len(), "the frame changed height");
        assert!(
            typed.iter().any(|line| line.contains("filter state=run")),
            "{typed:#?}"
        );

        let noticed = Screen {
            notice: Some("pause/resume is not built yet".to_string()),
            ..Screen::default()
        };
        let shown = drawn(&noticed);
        assert_eq!(shown.len(), plain.len(), "the frame changed height");
        assert!(
            shown
                .iter()
                .any(|line| line.contains("pause/resume is not built yet")),
            "{shown:#?}"
        );
    }

    /// **The screen and `--once` write one summary line, not two.** They have
    /// to be emitted differently — `ratatui` prints an SGR sequence it finds in
    /// a value literally — and a terminal reading `4 jobs · 1 need you` on the
    /// screen and `4 jobs, 1 need you` from `--once` would be one render
    /// behaving as two.
    #[test]
    fn the_screen_and_once_write_the_same_summary_line() {
        fn strip(text: &str) -> String {
            let mut out = String::new();
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

        for style in [Style::plain(), Style::painted()] {
            let data = crate::verbs::bridge::data(frame());
            let pieces: String = render::bridge_summary_pieces(&data, Status::Running, style)
                .iter()
                .map(|span| span.text.as_str())
                .collect();
            let line = render::bridge_summary(&data, Status::Running, style);
            assert_eq!(pieces, strip(&line).trim_end_matches('\n'));
        }
    }

    /// **The pane answers the question the table could not**, and it is
    /// `render::show_lines` that answers it — the same words `armada fleet show`
    /// prints in a shell.
    #[test]
    fn the_detail_pane_draws_the_reason_the_task_and_its_own_keys() {
        let screen = Screen {
            mode: Mode::Detail("release-merge".to_string()),
            ..Screen::default()
        };
        let shown = detail();
        let all = text(&paint(
            &view(frame()),
            &screen,
            Some(&shown),
            "c24a68b6",
            "~/Development/armada",
            Style::plain(),
            80,
        ))
        .join("\n");

        // The defect, closed: the entry's own words, whole.
        assert!(
            all.contains("the CI timeout is 30s and the flake needs 90s"),
            "{all}"
        );
        // And the task, whole — not the column's worth of it.
        assert!(all.contains(&shown.task), "{all}");
        assert!(all.contains("TASK"), "{all}");
        // The three facts that disagree when something is wrong.
        for fact in ["RECORDED", "ALIVE", "HELD"] {
            assert!(all.contains(fact), "no {fact} row:\n{all}");
        }
        // Its own key line, because none of the Bridge's eight work from here.
        assert!(all.contains("esc back"), "{all}");
        assert!(
            !all.contains("n new"),
            "the Bridge's keys are named:\n{all}"
        );
        // **Still no progress column**, on the surface most tempted by one.
        assert!(!all.contains("PROGRESS"), "{all}");
    }

    /// The table is **covered, not squeezed beside**: a pane sharing the width
    /// with six columns would truncate the one sentence it exists to show.
    #[test]
    fn the_detail_pane_replaces_the_table_rather_than_joining_it() {
        let screen = Screen {
            mode: Mode::Detail("release-merge".to_string()),
            ..Screen::default()
        };
        let all = text(&paint(
            &view(frame()),
            &screen,
            Some(&detail()),
            "c24a68b6",
            "~/Development/armada",
            Style::plain(),
            80,
        ))
        .join("\n");
        assert!(
            !all.contains("rate-limit"),
            "the fleet table is still drawn:\n{all}"
        );
        assert!(
            !all.contains("JOBS ("),
            "the JOBS box is still drawn:\n{all}"
        );
    }

    /// **`TIMELINE` and `REPORTS` never source a row from the same field.**
    /// 033's own section "The detail view names its two tables": one carries
    /// the gate's decision — the predicate and exit code that settled it —
    /// and the other carries an agent's own summary. Collapsing them is how a
    /// Drone's claim gets read as evidence, so this asserts the two tables
    /// draw from `transitions` and `progress` and never swap.
    #[test]
    fn timeline_and_reports_read_from_different_fields() {
        let mut shown = detail();
        shown.transitions = vec![armada_core::envelope::TransitionRow {
            at: "2026-08-09T14:49:11Z".to_string(),
            ago_s: 60,
            step: "implement".to_string(),
            event: "failed".to_string(),
            attempt: 1,
            must: Some("check_passes".to_string()),
            evidence: vec![armada_core::envelope::Evidence {
                kind: "check".to_string(),
                scope: "orders:test".to_string(),
                exit: 1,
            }],
        }];
        shown.progress = vec![armada_core::envelope::NoteRow {
            at: "2026-08-09T14:50:11Z".to_string(),
            ago_s: 30,
            step: "implement".to_string(),
            body: "reproduced the cold-start failure locally".to_string(),
        }];

        let screen = Screen {
            mode: Mode::Detail("release-merge".to_string()),
            ..Screen::default()
        };
        let all = text(&paint(
            &view(frame()),
            &screen,
            Some(&shown),
            "c24a68b6",
            "~/Development/armada",
            Style::plain(),
            160,
        ))
        .join("\n");

        let timeline_at = all.find("TIMELINE").expect("a TIMELINE box");
        let reports_at = all.find("REPORTS").expect("a REPORTS box");
        let gate_evidence_at = all
            .find("check_passes")
            .expect("the gate's predicate is drawn");
        let drone_words_at = all
            .find("reproduced the cold-start failure locally")
            .expect("the Drone's own words are drawn");

        // The gate's predicate is on the TIMELINE side, before REPORTS starts.
        assert!(
            timeline_at < gate_evidence_at && gate_evidence_at < reports_at,
            "the gate's predicate is not inside TIMELINE:\n{all}"
        );
        // The Drone's own words are on the REPORTS side, after TIMELINE ends.
        assert!(
            drone_words_at > reports_at,
            "the Drone's words are not inside REPORTS:\n{all}"
        );
        // Neither field's content appears on the other's side.
        assert!(
            !all[timeline_at..reports_at].contains("reproduced the cold-start"),
            "REPORTS' content leaked into TIMELINE:\n{all}"
        );
    }

    /// A Job that went while the pane was open says so. `armada fleet kill` in
    /// another shell is the ordinary way this happens, and a pane that simply
    /// emptied would read as a failed redraw.
    #[test]
    fn a_detail_pane_whose_job_has_gone_says_so() {
        let screen = Screen {
            mode: Mode::Detail("release-merge".to_string()),
            ..Screen::default()
        };
        let all = text(&paint(
            &view(frame()),
            &screen,
            None,
            "c24a68b6",
            "~/Development/armada",
            Style::plain(),
            80,
        ))
        .join("\n");
        assert!(
            all.contains("`release-merge` is no longer in the fleet"),
            "{all}"
        );
    }

    /// One Job with a question open against it.
    fn detail() -> armada_core::envelope::ShowData {
        armada_core::envelope::ShowData {
            job: "release-merge".to_string(),
            uuid: "release-merge-uuid".to_string(),
            workflow: "feature".to_string(),
            state: JobState::Blocked,
            recorded_state: JobState::Running,
            acting: None,
            acting_for_s: None,
            drone_pgid: Some(48122),
            drone_alive: true,
            step: "implement".to_string(),
            attempt: 2,
            on_step_s: Some(720),
            gate: None,
            transitions: Vec::new(),
            task: "raise the nightly CI timeout so the flake stops failing the run".to_string(),
            runtime_s: 840,
            created_at: "2026-08-09T14:02:11Z".to_string(),
            cost_usd: 2.10,
            tokens: 1_000,
            turns: 3,
            budget: armada_core::fleet::workflow::Budget {
                attempts: 3,
                cost_usd: 10.0,
                wall_clock_ms: 90 * 60 * 1_000,
                on_exhausted: armada_core::fleet::workflow::OnExhausted::NeedsHuman,
            },
            budget_remaining: Remaining {
                attempts: 2,
                cost_usd: 7.90,
                wall_clock_ms: 60 * 60 * 1_000,
            },
            repo: "orders".to_string(),
            branch: "armada/release-merge".to_string(),
            worktree: "~/.armada/workspaces/orders/release-merge".to_string(),
            port_block: None,
            needs_attention: true,
            asked: vec![armada_core::envelope::InboxRow {
                uuid: "e1".to_string(),
                job_uuid: Some("7f2ab618-58d3-4c07-b9e4-1a6c39fd80ae".to_string()),
                job: "release-merge".to_string(),
                kind: "NEEDS_HUMAN".to_string(),
                raised_at: "2026-08-09T14:12:11Z".to_string(),
                waiting_s: 9 * 60,
                body: "the CI timeout is 30s and the flake needs 90s. Raise it?".to_string(),
                answered: None,
                closed: None,
            }],
            progress: Vec::new(),
            steps: Vec::new(),
        }
    }

    /// **The key line reads the row the cursor is on.** A line that always said
    /// `pause` over a `PAUSED` Job advertised the one thing that key would not
    /// do, and left the reader with no way at all to start a held Job again.
    ///
    /// **The command centre's `KEYS` box is 033's own fixed legend, and this is
    /// the trade-off named plainly.** The single-table Bridge's key line read
    /// the selected row's state and said `p resume` over a paused Job; 033's
    /// mock-up draws one static legend for the whole screen (`p pause`, no
    /// per-row variant), and this box follows the mock rather than carrying
    /// the personalization forward.
    #[test]
    fn the_keys_box_names_pause_regardless_of_which_row_is_selected() {
        let frame = Frame {
            rows: vec![
                row("rate-limit", JobState::Running, false),
                row("xlsx-report", JobState::Paused, false),
            ],
            ..frame()
        };
        // Wide enough (still under `WIDE`) that `p pause` survives the
        // narrow legend's own shedding — the movement legend and `q quit`
        // alone already cost about 65 columns.
        let line = |screen: &Screen| {
            let drawn = drawn_at(&frame, screen, 120);
            drawn
                .iter()
                .find(|line| line.contains("q quit"))
                .cloned()
                .expect("a key line")
        };

        let mut screen = Screen::default();
        assert!(line(&screen).contains("p pause"), "{}", line(&screen));

        screen.cursor.next(2);
        assert!(line(&screen).contains("p pause"), "{}", line(&screen));
    }

    /// **The key line stays one line and never wraps**, at every width the
    /// two audiences meet — which is what keeps the frame's height constant.
    /// What does not fit drops, in the priority order `render.rs` states, and
    /// `q quit` is never what drops: a full-screen program that does not say how
    /// to leave is a trap.
    #[test]
    fn the_key_line_fits_its_width_at_every_width_and_never_loses_quit() {
        for width in [40, 60, 72, 74, 80, 100, 120] {
            for state in [None, Some(JobState::Paused)] {
                let line = render::bridge_keys(state, width);
                assert!(
                    line.chars().count() + 2 <= width,
                    "the key line is {} wide at {width}: {line}",
                    line.chars().count() + 2
                );
                assert!(line.contains("q quit"), "quit fell off at {width}: {line}");
                // Anything it could not carry is reachable, and the line says so.
                let hidden = render::bridge_keys_hidden(state, width);
                assert_eq!(
                    line.contains("? keys"),
                    !hidden.is_empty(),
                    "at {width} the line and the overflow disagree: {line}"
                );
            }
        }
    }

    /// **Every binding is on the page `?` opens**, including the ones the line
    /// dropped and the one that has no verb behind it. An overflow key that did
    /// not list what overflowed would be worse than no overflow key.
    #[test]
    fn the_keys_page_lists_every_binding_the_line_could_not_carry() {
        let drawn = drawn(&Screen {
            mode: Mode::Keys,
            ..Screen::default()
        });
        let all = drawn.join("\n");
        for word in ["board", "new", "pause", "abort", "answer", "filter", "reap"] {
            assert!(
                all.contains(word),
                "`{word}` is not on the keys page:\n{all}"
            );
        }
        assert!(all.contains("chat"), "an unbound-looking key is unlisted");
        assert!(all.contains("quit"), "{all}");
        // The fleet table is not underneath it: it is a page, not a pane.
        assert!(!all.contains("rate-limit"), "{all}");
    }

    /// **The reap preview says what each Job is holding**, which is the whole
    /// point of previewing: the cost of reaping and the cost of *not* reaping
    /// have to be on the same row. And `take`/`keep` are words rather than a
    /// tick and a cross, so both audiences read the same shape.
    #[test]
    fn the_reap_preview_shows_what_each_job_holds_and_which_rows_are_ticked() {
        let screen = Screen {
            mode: Mode::Reaping(bridge::Reap {
                rows: vec![
                    bridge::ReapRow {
                        target: bridge::Target {
                            job: "rate-limit".to_string(),
                            uuid: "rate-limit-uuid".to_string(),
                        },
                        state: JobState::Done,
                        selected: true,
                        holding: "ports 5470-5479, worktree".to_string(),
                    },
                    bridge::ReapRow {
                        target: bridge::Target {
                            job: "xlsx-report".to_string(),
                            uuid: "xlsx-report-uuid".to_string(),
                        },
                        state: JobState::Paused,
                        selected: false,
                        holding: "branch armada/xlsx-report".to_string(),
                    },
                ],
                cursor: Default::default(),
            }),
            ..Screen::default()
        };
        let drawn = drawn(&screen);
        let all = drawn.join("\n");
        assert!(all.contains("REAP"), "{all}");
        assert!(all.contains("1 of 2 ticked"), "{all}");
        // **The uuid is on every row**, because two Jobs can share a name and a
        // preview of what is about to be deleted has to be readable when they
        // do.
        assert!(all.contains("rate-lim"), "no uuid column:\n{all}");
        assert!(all.contains("xlsx-rep"), "no uuid column:\n{all}");
        assert!(all.contains("ports 5470-5479"), "{all}");
        assert!(all.contains("take"), "{all}");
        assert!(all.contains("keep"), "{all}");
        // No tick and no cross: a glyph that only appears at a terminal gives
        // the two audiences different shapes.
        assert!(
            !all.contains('\u{2713}') && !all.contains('\u{2717}'),
            "{all}"
        );
        assert!(all.contains("space toggle"), "{all}");
        assert!(all.contains("esc cancel"), "{all}");
    }

    /// **The compose box names its keys, and the fleet is still on the
    /// screen.** Both halves are the defect: the box that `n` used to open had
    /// no help text at all — *"I guessed with control-d"* — and it opened on a
    /// screen with no Jobs on it, which is what made it feel like leaving.
    #[test]
    fn the_compose_box_names_the_interviews_keys_over_a_table_that_is_still_there() {
        let screen = Screen {
            mode: Mode::Composing("raise the nightly CI timeout".to_string()),
            ..Screen::default()
        };
        let drawn = drawn(&screen);
        let all = drawn.join("\n");

        assert!(all.contains("NEW JOB"), "{all}");
        assert!(all.contains("raise the nightly CI timeout"), "{all}");
        // The keys, and they are the interview's — one convention, one source.
        assert!(all.contains("enter for a new line"), "{all}");
        assert!(all.contains("ctrl-d starts it"), "{all}");
        assert!(all.contains("esc starts nothing"), "{all}");
        assert_eq!(
            all.contains("ctrl-d"),
            render::prose_keys("starts it", "starts nothing", "  ").contains("ctrl-d"),
            "the box and the one list of keys have drifted"
        );
        // **The table is underneath, not replaced.** This is the difference
        // between the compose box and the two panes that cover the table.
        assert!(all.contains("rate-limit"), "the fleet went away:\n{all}");
        assert!(all.contains("2 jobs"), "{all}");
        // And the Bridge's own key line is not also drawn: `n` is not `q` here.
        assert!(!all.contains("q quit"), "two key lines:\n{all}");
    }

    /// **A box with nothing in it still draws a caret**, so there is somewhere
    /// visible to type — and it wraps rather than running off the edge.
    #[test]
    fn the_compose_box_shows_a_caret_when_empty_and_wraps_when_long() {
        let empty = drawn(&Screen {
            mode: Mode::Composing(String::new()),
            ..Screen::default()
        });
        assert!(
            empty.iter().any(|line| line.trim() == ">"),
            "no caret to type at:\n{empty:#?}"
        );

        let long = text(&paint(
            &view(frame()),
            &Screen {
                mode: Mode::Composing("x".repeat(200)),
                ..Screen::default()
            },
            None,
            "c24a68b6",
            "~/Development/armada",
            Style::plain(),
            80,
        ));
        for line in &long {
            assert!(
                line.chars().count() <= 80,
                "the box overhangs at 80: {} wide",
                line.chars().count()
            );
        }
        assert!(
            long.iter().filter(|line| line.contains("xxxx")).count() >= 3,
            "200 characters did not wrap:\n{long:#?}"
        );
    }

    /// An empty fleet says so rather than drawing nothing, and says which of the
    /// two empties it is.
    #[test]
    fn an_empty_frame_says_which_kind_of_empty_it_is() {
        for (filter, expected) in [
            (None, "no Jobs"),
            (Some("needs=you".to_string()), "no Jobs match"),
        ] {
            let empty = Frame {
                rows: Vec::new(),
                running: 0,
                needs_you: 0,
                spent_usd: 0.0,
                hidden: 0,
                filter,
                windows: Vec::new(),
            };
            let drawn = text(&paint(
                &view(empty),
                &Screen::default(),
                None,
                "c24a68b6",
                "~/Development/armada",
                Style::plain(),
                80,
            ));
            assert!(drawn.join("\n").contains(expected), "{drawn:#?}");
        }
    }
}
