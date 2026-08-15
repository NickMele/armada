//! The run, redrawn in place: **one row per check, not one spinning line**.
//!
//! What it replaced said `2/5` and listed the ids still running, on one line.
//! The report that ended it was a run with a check stuck for seven minutes:
//! the operator could not see *which* check that was, nor that a different one
//! had already failed, because neither fact fits on a line that has to be short
//! enough to erase. A run's shape is a table, and it always was — the final
//! output has been one from the beginning.
//!
//! So this draws that table live, in the same columns, and the final one
//! replaces it when the run ends. **Same columns is the requirement**, not a
//! nicety: a reader who watched `STATUS · CHECK · DETAIL · TIME` go by and is
//! then handed a differently-shaped answer has to find their place twice.
//! [`Table::spans`] is what makes that structural rather than a matter of two
//! call sites agreeing — the columns are measured once, by the code that
//! measures them for the final table.
//!
//! **An inline viewport, never the alternate screen.** The alternate screen
//! takes the terminal, and gives it back wiped: the run's own output would not
//! be in the scrollback afterwards, which is where anyone looks for it. An
//! inline viewport reserves its lines at the bottom of the terminal you are
//! already in, and everything above it stays.
//!
//! **And raw mode is never entered.** The editor in `ask/` does, because it
//! reads keys. This reads nothing — and a run that swallowed `ctrl-c` would be
//! a run you cannot stop, which is the opposite of what watching it is for.
//!
//! **The audience rule is stderr's, exactly as the spinner's was.** Progress
//! belongs on stderr, so `armada manifest check | jq` gets a live table *and* a
//! clean payload. Not a terminal, or `--json`, and there is no table at all.

use std::collections::BTreeMap;
use std::io::Stderr;

use armada_core::error::Status;

use super::palette::Role;
use super::progress::{Planned, Progress, Shape, Verdict};
use super::style::Style;
use super::table::{Cell, Span, Table};
use super::term::Terminal;
use super::{columns_for, format, verdict_cell};

/// The most rows the viewport will ever take.
///
/// **A cap rather than "as many as there are".** A hundred-check run would
/// reserve a hundred lines, which is every line of most terminals — the live
/// table would *be* the screen, and the scrollback it exists to preserve would
/// be pushed off it. Twenty-four is a conventional terminal's height, so the
/// table can fill a small one and still leave a large one mostly free.
const MAX_ROWS: usize = 24;

/// Where one check has got to.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RowState {
    /// Planned, not yet spawned.
    Waiting,
    /// Spawned, at this monotonic reading.
    Running { since_mono: u64 },
    /// Answered.
    Done {
        reached: Verdict,
        elapsed_ms: u64,
        detail: Option<String>,
    },
}

impl RowState {
    /// The verdict this row shows now.
    ///
    /// **The two unfinished states are `Status`es in either table.** `WAITING`
    /// and `RUNNING` are the progress states PLAN.md §3.1 defines, and they mean
    /// the same thing for a check and for a spawn's worktree — only the word a
    /// row ends on is per-verb.
    fn reached(&self) -> Verdict {
        match self {
            RowState::Waiting => Verdict::Status(Status::Waiting),
            RowState::Running { .. } => Verdict::Status(Status::Running),
            RowState::Done { reached, .. } => *reached,
        }
    }

    fn is_done(&self) -> bool {
        matches!(self, RowState::Done { .. })
    }
}

/// The state of a run, as a table — **and nothing that touches a terminal**.
///
/// Split from [`Live`] on purpose. Everything interesting here is which row is
/// in which state and how the overflow is chosen, and none of it should need a
/// terminal to test. `Live` is the twenty lines that cannot be tested without
/// one.
pub struct Run {
    /// Which table this is. Handed in, never inferred — see
    /// [`super::progress::Shape`].
    shape: Shape,
    /// In plan order, which is the order the final table uses.
    rows: Vec<Row>,
    index: BTreeMap<String, usize>,
    /// The last monotonic reading anyone handed in.
    now_mono: u64,
}

/// One row.
struct Row {
    id: String,
    /// Its own deadline, carried so a running row can say what it is, or `None`
    /// for a step nobody bounded.
    timeout_ms: Option<u64>,
    state: RowState,
}

impl Run {
    /// A run of these rows, none of them started.
    pub fn new(shape: Shape, planned: &[Planned<'_>], now_mono: u64) -> Run {
        let rows: Vec<Row> = planned
            .iter()
            .map(|planned| Row {
                id: planned.id.to_string(),
                timeout_ms: planned.timeout_ms,
                state: RowState::Waiting,
            })
            .collect();
        let index = rows
            .iter()
            .enumerate()
            .map(|(at, row)| (row.id.clone(), at))
            .collect();
        Run {
            shape,
            rows,
            index,
            now_mono,
        }
    }

    /// How many lines the viewport needs: the rows it will show, plus a header.
    pub fn height(&self) -> u16 {
        let rows = self.rows.len().min(MAX_ROWS);
        // At least one row, so a run with no checks still has somewhere to draw
        // the header rather than a zero-height viewport.
        u16::try_from(rows.max(1) + 1).unwrap_or(u16::MAX)
    }

    /// Advance the clock. The only clock this module has, and it is handed in —
    /// the real one is injected, and a renderer is not where one is read.
    pub fn tick(&mut self, now_mono: u64) {
        self.now_mono = now_mono;
    }

    /// A check was spawned.
    pub fn started(&mut self, id: &str) {
        let now = self.now_mono;
        if let Some(row) = self.row(id) {
            *row = RowState::Running { since_mono: now };
        }
    }

    /// A check reached a verdict.
    ///
    /// **Elapsed is measured from the spawn, not from the plan.** A check that
    /// waited four minutes for a lease and ran for two did not take six, and the
    /// final table's `TIME` means the same thing.
    pub fn finished(&mut self, id: &str, reached: Verdict, detail: Option<&str>) {
        let now = self.now_mono;
        let Some(row) = self.row(id) else { return };
        let elapsed_ms = match row {
            RowState::Running { since_mono } => now.saturating_sub(*since_mono),
            // A verdict for something never spawned — skipped, or refused
            // before it started. It took no time, and saying `0ms` is more
            // honest than inventing the wait.
            _ => 0,
        };
        *row = RowState::Done {
            reached,
            elapsed_ms,
            detail: detail.map(str::to_string),
        };
    }

    fn row(&mut self, id: &str) -> Option<&mut RowState> {
        let at = *self.index.get(id)?;
        self.rows.get_mut(at).map(|row| &mut row.state)
    }

    /// Which rows are drawn, in plan order.
    ///
    /// **Finished rows are what gets dropped when there is not room**, and
    /// nothing that is still running ever is. That is the whole complaint this
    /// module answers: the row you need to see is the one that has not come
    /// back, and a table that hid it to make space for four that had would be
    /// the single-line spinner again with more lines.
    ///
    /// A finished row is not lost by being dropped — the final table prints
    /// every one of them a moment later.
    fn visible(&self) -> Vec<usize> {
        if self.rows.len() <= MAX_ROWS {
            return (0..self.rows.len()).collect();
        }
        let mut room = MAX_ROWS;
        let mut keep = vec![false; self.rows.len()];
        // Two passes over the same order, so what survives is still in plan
        // order and a row does not move under the reader's eye as others end.
        for pass in [false, true] {
            for (at, row) in self.rows.iter().enumerate() {
                if room == 0 {
                    break;
                }
                if keep[at] || row.state.is_done() != pass {
                    continue;
                }
                keep[at] = true;
                room -= 1;
            }
        }
        keep.iter()
            .enumerate()
            .filter(|(_, kept)| **kept)
            .map(|(at, _)| at)
            .collect()
    }

    /// The table, in the columns the final one uses.
    fn table(&self) -> Table {
        let mut table = Table::new(columns_for(self.shape)).indent(2);
        for at in self.visible() {
            let row = &self.rows[at];
            let (detail, elapsed) = match &row.state {
                RowState::Waiting => (None, None),
                // **A running row says what its own deadline is.** Elapsed
                // alone cannot be read: seven minutes is alarming against a
                // one-minute budget and unremarkable against fifteen, and the
                // run that prompted this was the second and looked like the
                // first. Nothing here is a hang nobody bounded, and now the row
                // says so instead of leaving the reader to open `machine.yml`.
                //
                // **A row with no deadline says nothing rather than `0s`.** Not
                // every step has a configured ceiling — a spawn's are the
                // worktree and the Drone — and a budget of zero is a claim, not
                // an absence.
                RowState::Running { since_mono } => (
                    row.timeout_ms
                        .map(|ms| format!("timeout {}", format::duration(ms))),
                    Some(self.now_mono.saturating_sub(*since_mono)),
                ),
                RowState::Done {
                    elapsed_ms, detail, ..
                } => (detail.clone(), Some(*elapsed_ms)),
            };
            table = table.row(vec![
                verdict_cell(row.state.reached()),
                Cell::plain(row.id.clone()),
                match detail {
                    Some(text) => Cell::muted(text),
                    None => Cell::nothing(),
                },
                match elapsed {
                    Some(ms) => Cell::muted(format::duration(ms)),
                    None => Cell::nothing(),
                },
            ]);
        }
        // **The count of what is not shown is a row, not a footnote.** A table
        // that silently drew twenty-four of ninety rows would be wrong in the
        // way a truncated column never is: nothing on the screen would say so.
        let hidden = self.rows.len() - self.visible().len();
        if hidden > 0 {
            table = table.row(vec![
                Cell::painted("DONE", Role::SteelGrey),
                Cell::muted(format::count(hidden, "more")),
                Cell::nothing(),
                Cell::nothing(),
            ]);
        }
        table
    }

    /// The frame, as coloured pieces — what [`Live`] paints and what a test
    /// reads.
    pub fn frame(&self, style: Style, width: usize) -> Vec<Vec<Span>> {
        self.table().spans(style, width)
    }
}

/// The viewport, and the run being drawn in it.
pub struct Live {
    run: Run,
    style: Style,
    terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<Stderr>>,
    width: usize,
}

impl Live {
    /// Reserve the lines and draw the first frame.
    ///
    /// `None` when the terminal will not take an inline viewport, which is the
    /// same answer the interview's editor gives: a surface that cannot be drawn
    /// is not drawn, and the run carries on. Progress is never the answer, so
    /// failing here would fail a check because a terminal went away.
    pub fn start(run: Run, style: Style, terminal: Terminal) -> Option<Live> {
        let height = run.height();
        let backend = ratatui::backend::CrosstermBackend::new(std::io::stderr());
        let inner = ratatui::Terminal::with_options(
            backend,
            ratatui::TerminalOptions {
                viewport: ratatui::Viewport::Inline(height),
            },
        )
        .ok()?;
        let mut live = Live {
            run,
            style,
            terminal: inner,
            width: terminal.usable_width(),
        };
        live.draw();
        Some(live)
    }

    /// The run being drawn, for the caller to step.
    pub fn run(&mut self) -> &mut Run {
        &mut self.run
    }

    /// Repaint. **A failed write is dropped**, for the same reason the spinner
    /// dropped one: the run has still done its work.
    pub fn draw(&mut self) {
        let style = self.style;
        let lines: Vec<ratatui::text::Line<'static>> = self
            .run
            .frame(style, self.width)
            .iter()
            .map(|spans| {
                ratatui::text::Line::from(
                    spans
                        .iter()
                        .map(|span| paint(span, style))
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        let _ = self.terminal.draw(|f| {
            let area = f.area();
            f.render_widget(ratatui::widgets::Paragraph::new(lines), area);
        });
    }

    /// Give the lines back, leaving nothing behind — the final table arrives on
    /// a clean stream, exactly as it did after the spinner erased its line.
    pub fn finish(mut self) {
        let _ = self.terminal.clear();
    }
}

/// The [`Progress`] implementation a person gets.
///
/// **The viewport is reserved at `begin` and not before**, because its height
/// is the number of checks and nothing knows that until the plan is resolved.
/// Holding an `Option<Live>` rather than constructing eagerly is also what makes
/// "the terminal would not take a viewport" a silence rather than a failure: the
/// run carries on and prints its table at the end, exactly as it does through a
/// pipe.
pub struct Watcher {
    style: Style,
    terminal: Terminal,
    live: Option<Live>,
}

impl Watcher {
    /// A watcher that will draw at `terminal`'s width when the run begins.
    pub fn new(style: Style, terminal: Terminal) -> Watcher {
        Watcher {
            style,
            terminal,
            live: None,
        }
    }
}

impl Progress for Watcher {
    fn begin(&mut self, shape: Shape, planned: &[Planned<'_>], now_mono: u64) {
        self.live = Live::start(
            Run::new(shape, planned, now_mono),
            self.style,
            self.terminal,
        );
    }

    fn started(&mut self, id: &str) {
        if let Some(live) = self.live.as_mut() {
            live.run().started(id);
            live.draw();
        }
    }

    fn finished(&mut self, id: &str, reached: Verdict, detail: Option<&str>) {
        if let Some(live) = self.live.as_mut() {
            live.run().finished(id, reached, detail);
            live.draw();
        }
    }

    fn tick(&mut self, now_mono: u64) {
        if let Some(live) = self.live.as_mut() {
            live.run().tick(now_mono);
            live.draw();
        }
    }

    fn finish(&mut self) {
        if let Some(live) = self.live.take() {
            live.finish();
        }
    }
}

/// One of the table's pieces, as `ratatui` wants it.
///
/// **The palette is read through [`Role::rgb`]**, which is the whole reason
/// that method exists: no surface names a hex, so the colour a live `PASS` is
/// drawn in cannot drift from the colour the final one is drawn in.
fn paint(span: &Span, style: Style) -> ratatui::text::Span<'static> {
    let text = span.text.clone();
    if !style.enabled() {
        return ratatui::text::Span::raw(text);
    }
    let mut rata = ratatui::style::Style::default();
    if let Some(role) = span.role {
        let (r, g, b) = role.rgb();
        rata = rata.fg(ratatui::style::Color::Rgb(r, g, b));
    }
    if span.bold {
        rata = rata.add_modifier(ratatui::style::Modifier::BOLD);
    }
    ratatui::text::Span::styled(text, rata)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(run: &Run) -> String {
        run.frame(Style::plain(), 80)
            .into_iter()
            .map(|line| {
                line.into_iter()
                    .map(|span| span.text)
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The machine's documented default, which is what the reported run had.
    const FIFTEEN_MINUTES: u64 = 900_000;

    fn run_of(n: usize) -> Run {
        let ids: Vec<String> = (0..n).map(|i| format!("api:check{i}")).collect();
        let planned: Vec<Planned<'_>> = ids
            .iter()
            .map(|id| Planned {
                id: id.as_str(),
                timeout_ms: Some(FIFTEEN_MINUTES),
            })
            .collect();
        Run::new(Shape::Check, &planned, 0)
    }

    /// A `fleet spawn`'s four rows, none of them bounded by a deadline.
    fn spawn_run() -> Run {
        let planned: Vec<Planned<'_>> = super::super::progress::SpawnStep::ALL
            .iter()
            .map(|step| Planned {
                id: step.id(),
                timeout_ms: None,
            })
            .collect();
        Run::new(Shape::Spawn, &planned, 0)
    }

    /// A convenience for the tests that still speak `check`'s vocabulary.
    fn reached(status: Status) -> Verdict {
        Verdict::Status(status)
    }

    /// **Every planned check has a row before anything has started**, which is
    /// the first thing the single line could not say: how many there are and
    /// what they are called.
    #[test]
    fn every_check_has_a_row_from_the_start() {
        let run = run_of(3);
        let frame = text(&run);
        for id in ["api:check0", "api:check1", "api:check2"] {
            assert!(frame.contains(id), "{id} has no row:\n{frame}");
        }
        assert_eq!(frame.matches("WAITING").count(), 3, "{frame}");
    }

    /// The columns are the final table's columns, headed the same way.
    #[test]
    fn the_header_is_the_final_tables_header() {
        let mut run = run_of(2);
        run.started("api:check0");
        run.tick(1_500);
        let frame = text(&run);
        let header = frame.lines().next().expect("a header");
        assert!(header.contains("STATUS"), "{header}");
        assert!(header.contains("CHECK"), "{header}");
        assert!(header.contains("TIME"), "{header}");
    }

    /// **The hung check is the point.** A check that has been running for seven
    /// minutes says so, by name, next to the one that already failed — the two
    /// facts the operator could not get out of `2/5`.
    #[test]
    fn a_long_running_check_shows_its_name_and_how_long() {
        let mut run = run_of(3);
        run.started("api:check0");
        run.started("api:check1");
        run.tick(1_300);
        run.finished("api:check0", reached(Status::Pass), None);
        run.tick(423_000);
        run.finished("api:check2", reached(Status::Failed), Some("exited 101"));

        let frame = text(&run);
        assert!(frame.contains("PASS"), "{frame}");
        assert!(frame.contains("RUNNING"), "{frame}");
        assert!(frame.contains("FAILED"), "{frame}");
        assert!(frame.contains("exited 101"), "{frame}");
        // 423s from its spawn, still going, and named.
        let running = frame
            .lines()
            .find(|line| line.contains("RUNNING"))
            .expect("a running row");
        assert!(running.contains("api:check1"), "{running}");
        assert!(running.contains("7m"), "{running}");
    }

    /// **A long-running row says what it is measured against.**
    ///
    /// The second half of the same report: seven minutes looked like a hang
    /// nobody had bounded, and it was a check with thirteen minutes still to go.
    /// The row carries its own deadline so the reader does not have to open
    /// `machine.yml` to find out which of those two it is looking at.
    #[test]
    fn a_running_row_states_the_budget_it_is_measured_against() {
        let mut run = run_of(1);
        run.started("api:check0");
        run.tick(423_000);
        let frame = text(&run);
        assert!(frame.contains("7m 03s"), "{frame}");
        assert!(frame.contains("timeout 15m"), "{frame}");
        // And it goes once there is a verdict — the budget mattered while the
        // answer was still outstanding.
        run.finished("api:check0", reached(Status::Pass), None);
        assert!(!text(&run).contains("timeout"), "{}", text(&run));
    }

    /// Elapsed runs from the spawn, not from the plan: a check that waited for
    /// a lease and then ran did not take the sum of both.
    #[test]
    fn elapsed_is_measured_from_the_spawn() {
        let mut run = run_of(1);
        run.tick(60_000);
        run.started("api:check0");
        run.tick(62_000);
        run.finished("api:check0", reached(Status::Pass), None);
        let frame = text(&run);
        assert!(frame.contains("2.0s"), "counted the wait: {frame}");
    }

    /// A small run reserves exactly what it needs, and a large one is capped
    /// rather than taking the screen the scrollback lives on.
    #[test]
    fn the_viewport_is_the_rows_plus_a_header_and_is_capped() {
        assert_eq!(run_of(3).height(), 4);
        assert_eq!(run_of(0).height(), 2);
        assert_eq!(run_of(200).height(), (MAX_ROWS + 1) as u16);
    }

    /// **Nothing still running is ever hidden**, and what was dropped is
    /// counted on the last row rather than disappearing.
    #[test]
    fn a_capped_table_drops_finished_rows_and_never_a_running_one() {
        let mut run = run_of(MAX_ROWS + 5);
        for index in 0..MAX_ROWS + 5 {
            run.started(&format!("api:check{index}"));
        }
        run.tick(1_000);
        // Everything but the last two has come back.
        for index in 0..MAX_ROWS + 3 {
            run.finished(&format!("api:check{index}"), reached(Status::Pass), None);
        }
        let frame = text(&run);
        for index in [MAX_ROWS + 3, MAX_ROWS + 4] {
            assert!(
                frame.contains(&format!("api:check{index}")),
                "a running check was hidden:\n{frame}"
            );
        }
        assert!(frame.contains("more"), "the hidden rows are not counted");
    }

    /// Rows keep their places as others finish: a table whose lines reorder
    /// under the eye is harder to read than one that does not move.
    #[test]
    fn rows_stay_in_plan_order() {
        let mut run = run_of(4);
        run.started("api:check2");
        run.tick(500);
        run.finished("api:check2", reached(Status::Pass), None);
        let frame = text(&run);
        let order: Vec<usize> = (0..4)
            .map(|i| frame.find(&format!("api:check{i}")).expect("a row"))
            .collect();
        assert!(
            order.windows(2).all(|pair| pair[0] < pair[1]),
            "the rows reordered:\n{frame}"
        );
    }

    /// A verdict for a check that was never spawned does not invent a duration
    /// out of the run's start.
    #[test]
    fn a_check_that_never_started_is_not_credited_with_the_runs_elapsed() {
        let mut run = run_of(1);
        run.tick(500_000);
        run.finished("api:check0", reached(Status::Skipped), Some("no fix:"));
        assert!(text(&run).contains("SKIPPED"));
        assert!(!text(&run).contains("8m"), "{}", text(&run));
    }

    /// A verdict for something not in the plan is ignored rather than added:
    /// the plan is what the viewport was sized for.
    #[test]
    fn an_unplanned_id_does_not_grow_the_table() {
        let mut run = run_of(1);
        run.started("web:e2e");
        run.finished("web:e2e", reached(Status::Pass), None);
        assert!(!text(&run).contains("web:e2e"));
        assert_eq!(run.height(), 2);
    }

    /// A check with its own `timeout:` shows its own, not the machine's.
    #[test]
    fn a_check_that_declares_a_deadline_shows_the_one_it_declared() {
        let mut run = Run::new(
            Shape::Check,
            &[
                Planned {
                    id: "api:quick",
                    timeout_ms: Some(30_000),
                },
                Planned {
                    id: "api:e2e",
                    timeout_ms: Some(FIFTEEN_MINUTES),
                },
            ],
            0,
        );
        run.started("api:quick");
        run.started("api:e2e");
        let frame = text(&run);
        assert!(frame.contains("timeout 30.0s"), "{frame}");
        assert!(frame.contains("timeout 15m"), "{frame}");
    }

    /// **`fleet spawn` gets the same table, headed the way its answer is
    /// headed.** The column is `STEP` rather than `CHECK` because
    /// [`super::super::columns_for`] is the one place either render learns it —
    /// invert this to `CHECK` and it fails, which is the drift the shared
    /// component exists to prevent.
    #[test]
    fn a_spawn_draws_the_same_table_under_its_own_noun() {
        let run = spawn_run();
        let frame = text(&run);
        let header = frame.lines().next().expect("a header");
        assert!(header.contains("STATUS"), "{header}");
        assert!(header.contains("STEP"), "{header}");
        assert!(!header.contains("CHECK"), "{header}");
        // STATUS is the first column here as it is everywhere: the universal
        // rule won over `STEP` first (`commands/render.md`).
        assert!(
            header.trim_start().starts_with("STATUS"),
            "STATUS is not first: {header}"
        );
        for step in super::super::progress::SpawnStep::ALL {
            assert!(
                frame.contains(step.id()),
                "{} has no row:\n{frame}",
                step.id()
            );
        }
        assert_eq!(frame.matches("WAITING").count(), 4, "{frame}");
    }

    /// **A step nobody bounded says nothing about a deadline**, where a check
    /// says what its own is. Inverted — asserting the running row contains
    /// `timeout` — this fails, which is the whole distinction: a spawn's
    /// worktree has no configured ceiling, and `timeout 0s` would report one it
    /// does not have.
    #[test]
    fn an_unbounded_step_claims_no_deadline() {
        let mut run = spawn_run();
        run.started("worktree");
        run.tick(4_200);
        let frame = text(&run);
        assert!(frame.contains("RUNNING"), "{frame}");
        assert!(!frame.contains("timeout"), "{frame}");
        assert!(frame.contains("4.2s"), "{frame}");
    }

    /// **The live rows end on the words the final table ends on.** A spawn
    /// answers in `CLASSIFIED` / `CREATED` / `CLAIMED` / `STARTED`, not in
    /// `PASS` — a reader who watched four rows go by is handed the same four.
    #[test]
    fn a_finished_spawn_row_says_what_the_answer_will_say() {
        let mut run = spawn_run();
        for step in super::super::progress::SpawnStep::ALL {
            run.started(step.id());
            run.finished(
                step.id(),
                Verdict::Word(step.done(), Role::BeaconGreen),
                None,
            );
        }
        let frame = text(&run);
        for word in ["CLASSIFIED", "CREATED", "CLAIMED", "STARTED"] {
            assert!(frame.contains(word), "{word} is missing:\n{frame}");
        }
        assert!(!frame.contains("PASS"), "{frame}");
    }
}
