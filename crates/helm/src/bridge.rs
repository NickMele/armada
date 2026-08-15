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
use armada_core::error::ArmadaError;
use armada_core::fleet::bridge::{self, Departure, Filter, Frame, Key, Mode, Pressed, Screen};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};

use crate::ask::terminal::Restore;
use crate::render::palette::Role;
use crate::render::style::Style;
use crate::render::table::Span;
use crate::render::term::Terminal;
use crate::render::{self, live};
use crate::verbs::bridge::Options;
use crate::verbs::fleet::Where;

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
        // **`ctrl-c` leaves, and no other control chord is a Bridge key.** A
        // `ctrl-d` swallowed here would be a key the reader pressed and nothing
        // answered.
        return matches!(code, KeyCode::Char('c')).then_some(Key::Interrupt);
    }
    match code {
        KeyCode::Up => Some(Key::Up),
        KeyCode::Down => Some(Key::Down),
        KeyCode::Enter => Some(Key::Enter),
        KeyCode::Esc => Some(Key::Esc),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::Char(c) => Some(Key::Char(c)),
        _ => None,
    }
}

/// Everything one redraw puts on the screen, as coloured pieces.
///
/// **Built without a terminal**, which is the point: a frame is a value, so what
/// the Bridge draws is asserted in a unit test instead of photographed.
pub fn paint(
    frame: &Frame,
    screen: &Screen,
    status: armada_core::error::Status,
    style: Style,
    width: usize,
) -> Vec<Vec<Span>> {
    let data = crate::verbs::bridge::data(frame.clone());
    let mut lines: Vec<Vec<Span>> = vec![
        vec![
            plain("  "),
            bold("ARMADA BRIDGE", Role::SignalAmber),
            // **Beacon green is the live indicator** (`docs/commands/render.md`),
            // and it is a word rather than a bullet: a screen reader and a
            // monochrome terminal lose emphasis and no information.
            plain("   "),
            piece("LIVE", Role::BeaconGreen),
        ],
        Vec::new(),
    ];

    lines.extend(render::bridge_table(&data, style, Some(screen.cursor.at())).spans(style, width));
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

    lines.push(Vec::new());
    lines.push(render::bridge_summary_pieces(&data, status, style));

    // **One line for whatever the screen has to say back**, kept even when it is
    // empty so the key line does not walk up and down as notices come and go.
    lines.push(match &screen.mode {
        Mode::Filtering(typed) => vec![
            plain("  "),
            piece("filter ", Role::SignalAmber),
            plain(format!("{typed}{}", style.caret())),
        ],
        _ => match &screen.notice {
            Some(notice) => vec![plain("  "), piece(notice.clone(), Role::FlareOrange)],
            None => Vec::new(),
        },
    });

    lines.push(Vec::new());
    lines.push(vec![
        plain("  "),
        piece(render::bridge_keys(), Role::SteelGrey),
    ]);
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
pub fn watch<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    options: &Options,
    filter: Option<&Filter>,
    style: Style,
    terminal: Terminal,
) -> Result<(Frame, Departure), ArmadaError> {
    let raw = Restore::install().map_err(|_| crate::verbs::bridge::no_screen())?;
    let alt = Alt::enter().map_err(|_| crate::verbs::bridge::no_screen())?;
    let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
    let mut view: ratatui::Terminal<ratatui::backend::CrosstermBackend<Stdout>> =
        ratatui::Terminal::new(backend).map_err(|_| crate::verbs::bridge::no_screen())?;

    let mut screen = Screen {
        filter: filter.cloned(),
        ..Screen::default()
    };
    let interval = Duration::from_secs(options.interval_s);
    let outcome = loop {
        let frame = crate::verbs::bridge::read(run, now, place, screen.filter.as_ref())?;
        screen.cursor.clamp(frame.rows.len());
        draw(&mut view, &frame, &screen, style, terminal);

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
            match bridge::press(&mut screen, &frame.rows, pressed) {
                Pressed::Leave(departure) => break Some(departure),
                Pressed::Stay => draw(&mut view, &frame, &screen, style, terminal),
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

fn draw(
    view: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<Stdout>>,
    frame: &Frame,
    screen: &Screen,
    style: Style,
    terminal: Terminal,
) {
    let status = crate::verbs::bridge::status_of(frame);
    let lines: Vec<ratatui::text::Line<'static>> =
        paint(frame, screen, status, style, terminal.usable_width())
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
            task: format!("do the {name} thing"),
            runtime_s: 840,
            cost_usd: 2.10,
            tokens: 1_000,
            turns: 3,
            budget_remaining: Remaining {
                iterations: 9,
                tokens: 1,
                wall_clock_ms: 1,
            },
            needs_attention: needs,
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

    fn drawn(screen: &Screen) -> Vec<String> {
        text(&paint(
            &frame(),
            screen,
            Status::Running,
            Style::plain(),
            80,
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
            (KeyCode::Char('q'), Some(Key::Char('q'))),
            (KeyCode::Char('/'), Some(Key::Char('/'))),
            (KeyCode::F(4), None),
            (KeyCode::Home, None),
        ] {
            assert_eq!(key_of(code, KeyModifiers::NONE), expected, "{code:?}");
        }
    }

    /// **`ctrl-c` leaves and no other chord is a key.** A `ctrl-d` answered here
    /// would be a keypress the reader made and nothing responded to.
    #[test]
    fn only_ctrl_c_is_a_control_chord() {
        assert_eq!(
            key_of(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Some(Key::Interrupt)
        );
        assert_eq!(key_of(KeyCode::Char('d'), KeyModifiers::CONTROL), None);
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
        assert!(all.contains("ARMADA BRIDGE"), "{all}");
        assert!(all.contains("LIVE"), "{all}");
        for header in ["STATUS", "JOB", "TASK", "RUN", "SPENT", "NEEDS YOU"] {
            assert!(all.contains(header), "no {header} column:\n{all}");
        }
        assert!(all.contains("RUNNING"), "{all}");
        assert!(all.contains("2 jobs"), "{all}");
        assert!(all.contains("1 need you"), "{all}");
        assert!(all.contains("q quit"), "{all}");
        // **No progress column, deliberately** — nothing emits percent-complete.
        assert!(!all.contains("PROGRESS"), "{all}");
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
            };
            let drawn = text(&paint(
                &empty,
                &Screen::default(),
                Status::Ok,
                Style::plain(),
                80,
            ));
            assert!(drawn.join("\n").contains(expected), "{drawn:#?}");
        }
    }
}
