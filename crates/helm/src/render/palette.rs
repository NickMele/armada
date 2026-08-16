//! The palette, and nothing else.
//!
//! **One page, shared by every coloured surface** — the CLI today, the Bridge
//! when it exists (`docs/commands/render.md`). A second table of hexes is how
//! two surfaces start disagreeing about what `RUNNING` looks like.
//!
//! **Truecolor is the target and there is no 16-colour fallback**, deliberately:
//! signal amber and flare orange are one ANSI step apart, so at 16 colours both
//! collapse to bright yellow and the `RUNNING` / `STALLED` distinction
//! disappears — precisely where a fallback would need to work. A terminal that
//! cannot do truecolor gets the no-colour path, which is a supported mode rather
//! than a broken one, because **the state word already carries the meaning**:
//! `FAILED` is spelled out, never signalled by red alone.

use armada_core::envelope::{Health, Sync};
use armada_core::error::Status;
use armada_core::ports::PortState;

/// End every attribute. Written after each painted span rather than once at the
/// end of a line, so a truncated or interleaved line cannot leave a terminal
/// wearing a colour.
pub const RESET: &str = "\x1b[0m";

/// Bold, for the one thing per screen that has to be found first.
pub const BOLD: &str = "\x1b[1m";

/// A role from `docs/commands/render.md`'s palette table.
///
/// **Roles, not colours.** Nothing outside this module names a hex, so moving
/// `STALLED` off flare orange is one line here rather than a search for
/// `\x1b[38;2;255;180;84m`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// `#0A0E14` — background. Never painted by the CLI; the terminal owns its
    /// own background, and a tool that repaints it fights every theme.
    Void,
    /// `#E6E8EB` — body text.
    Foreground,
    /// `#FFA940` — `RUNNING`, headings, the prompt, the wordmark.
    SignalAmber,
    /// `#4C9FE8` — Job and Drone identifiers.
    NavalBlue,
    /// `#3DDC84` — `OK`, `PASS`, live indicator.
    BeaconGreen,
    /// `#FF5C5C` — `BLOCKED`, `FAILED`.
    DistressRed,
    /// `#FFB454` — `STALLED`.
    FlareOrange,
    /// `#5CE1E6` — `QUEUED`, tool calls.
    RadarCyan,
    /// `#C792EA` — `PAUSED`.
    StasisPurple,
    /// `#FF7AB6` — `ABORTED`.
    AbortPink,
    /// `#9CA3AF` — box drawing, muted text.
    ///
    /// **Raised from `#6B7280`, which was not readable.** Against the `Void`
    /// background that value is a 4.0:1 contrast ratio, under WCAG AA's 4.5:1
    /// floor for body text — and this is not a decorative role: it paints every
    /// table header, every `DETAIL` cell, every duration, every `—`, and the
    /// gloss under every line of `--help`. Most of the words on a screen of
    /// Armada output are this colour, and they were being reported as hard to
    /// read because they were.
    ///
    /// `#9CA3AF` is 7.6:1, which clears AAA, while [`Role::Foreground`] is
    /// 15.8:1 — so muted is still visibly half as loud as body text and the
    /// hierarchy the role exists for survives. Quiet has to stay legible;
    /// `SKIPPED` being unmistakably not-green is what keeps it from reading as
    /// approval, and that is a matter of hue rather than of dimness.
    SteelGrey,
    /// `#1E2530` — borders, fills.
    DeepSlate,
}

impl Role {
    /// The hex from the palette table, as documentation carries it.
    pub const fn hex(self) -> &'static str {
        match self {
            Role::Void => "#0A0E14",
            Role::Foreground => "#E6E8EB",
            Role::SignalAmber => "#FFA940",
            Role::NavalBlue => "#4C9FE8",
            Role::BeaconGreen => "#3DDC84",
            Role::DistressRed => "#FF5C5C",
            Role::FlareOrange => "#FFB454",
            Role::RadarCyan => "#5CE1E6",
            Role::StasisPurple => "#C792EA",
            Role::AbortPink => "#FF7AB6",
            Role::SteelGrey => "#9CA3AF",
            Role::DeepSlate => "#1E2530",
        }
    }

    /// The SGR sequence that sets this role as the **foreground** colour.
    ///
    /// A literal per role rather than a formatted string: this is on every cell
    /// of every table, and the test below proves each literal is the hex above.
    pub const fn fg(self) -> &'static str {
        match self {
            Role::Void => "\x1b[38;2;10;14;20m",
            Role::Foreground => "\x1b[38;2;230;232;235m",
            Role::SignalAmber => "\x1b[38;2;255;169;64m",
            Role::NavalBlue => "\x1b[38;2;76;159;232m",
            Role::BeaconGreen => "\x1b[38;2;61;220;132m",
            Role::DistressRed => "\x1b[38;2;255;92;92m",
            Role::FlareOrange => "\x1b[38;2;255;180;84m",
            Role::RadarCyan => "\x1b[38;2;92;225;230m",
            Role::StasisPurple => "\x1b[38;2;199;146;234m",
            Role::AbortPink => "\x1b[38;2;255;122;182m",
            Role::SteelGrey => "\x1b[38;2;156;163;175m",
            Role::DeepSlate => "\x1b[38;2;30;37;48m",
        }
    }

    /// The three channels, for a surface that takes a colour rather than an
    /// escape sequence.
    ///
    /// **The interview's text area is drawn by `ratatui`**, which wants a
    /// `Color::Rgb` and not an SGR string — and the Bridge will want the same.
    /// This is the third spelling of one colour and the test below holds all
    /// three together, which is the whole reason nothing outside this module
    /// names a hex.
    pub const fn rgb(self) -> (u8, u8, u8) {
        match self {
            Role::Void => (10, 14, 20),
            Role::Foreground => (230, 232, 235),
            Role::SignalAmber => (255, 169, 64),
            Role::NavalBlue => (76, 159, 232),
            Role::BeaconGreen => (61, 220, 132),
            Role::DistressRed => (255, 92, 92),
            Role::FlareOrange => (255, 180, 84),
            Role::RadarCyan => (92, 225, 230),
            Role::StasisPurple => (199, 146, 234),
            Role::AbortPink => (255, 122, 182),
            Role::SteelGrey => (156, 163, 175),
            Role::DeepSlate => (30, 37, 48),
        }
    }

    /// The colour a terminal state is spoken in.
    ///
    /// **The word is the meaning and the colour only agrees with it.** Every
    /// state is spelled out in text, so a monochrome terminal loses emphasis and
    /// no information — which is what makes "no 16-colour fallback" a supported
    /// mode rather than a broken one.
    pub const fn for_status(status: Status) -> Role {
        match status {
            // The verb did what it was asked.
            Status::Ready | Status::Up | Status::Clean | Status::Pass | Status::Ok => {
                Role::BeaconGreen
            }
            // Nothing ran, or nothing is there. Muted rather than green: a
            // `SKIPPED` that reads as approval is the failure `check`'s
            // all-skipped rule exists to prevent (PLAN.md §3.1).
            Status::Skipped | Status::Down => Role::SteelGrey,
            Status::Failed | Status::Dead => Role::DistressRed,
            // `PARTIAL` and `TIMEOUT` are the two "it stalled part-way" answers,
            // which is what flare orange means on the Bridge.
            Status::Partial | Status::Timeout => Role::FlareOrange,
            Status::Aborted => Role::AbortPink,
            Status::Running => Role::SignalAmber,
            Status::Waiting => Role::RadarCyan,
            // **`QUEUED` shares radar cyan with `WAITING`** — it is the colour
            // the palette already gives *not running yet*, and `020` §7 changed
            // the word rather than the meaning of the row.
            Status::Queued => Role::RadarCyan,
        }
    }

    /// The colour a machine-condition word is spoken in — `armada init`'s
    /// preflight and every `armada doctor` check.
    ///
    /// **One word, one colour, across both verbs.** The reviewed drawing in
    /// `docs/reference-output/command-output.html` paints `missing` amber under
    /// `armada init` and red under `armada doctor`, and this does not follow it
    /// there. The layout it froze is followed exactly — the `.plain` fixtures are
    /// unchanged — but a word that means two things depending on which verb
    /// printed it is the defect `glossary.md` opens by naming, and colour is not
    /// exempt from it. `missing` is red in both, and the DETAIL cell carries the
    /// severity: *"not required by every repo"* is what tells a reader the red
    /// row under `init` is survivable.
    pub const fn for_health(health: Health) -> Role {
        match health {
            // It is there. Whether Armada found it or made it is the word's job.
            Health::Ok | Health::Found | Health::Created => Role::BeaconGreen,
            Health::Missing => Role::DistressRed,
            // The three "it is there and not right" answers, which is what
            // flare orange means everywhere else in this palette.
            Health::Stale | Health::Partial | Health::Offline => Role::FlareOrange,
        }
    }

    /// The colour a setting's [`Locality`](armada_core::envelope::Locality) is
    /// spoken in.
    ///
    /// **Two colours for two answers, and neither is red or green.** Neither
    /// side of `PLAN.md` §13.1's line is a problem — a setting is not wrong for
    /// being machine-local — so this is the one distinction on the page that is
    /// not a verdict: amber for what stays here, blue for what travels with you.
    pub const fn for_locality(locality: armada_core::envelope::Locality) -> Role {
        use armada_core::envelope::Locality;
        match locality {
            Locality::Machine => Role::SignalAmber,
            Locality::Synced => Role::NavalBlue,
        }
    }

    /// The colour a sync outcome is spoken in.
    ///
    /// **`conflict` is red and nothing else is.** It is the one row on a `guild
    /// pull` that stops the command and needs a person, and the whole design of
    /// that verb is that it surfaces rather than resolves.
    pub const fn for_sync(sync: Sync) -> Role {
        match sync {
            Sync::Added => Role::BeaconGreen,
            Sync::Changed | Sync::Removed => Role::FlareOrange,
            Sync::Conflict => Role::DistressRed,
            // Nothing happened to it. Muted, for the same reason `SKIPPED` is:
            // a green `unchanged` would read as an approval of something.
            Sync::Unchanged => Role::SteelGrey,
        }
    }

    /// The colour one guild item's kind is spoken in.
    ///
    /// **The three fragments are amber and everything else is one of two
    /// quieter colours**, because a listing of forty rows in nine colours is a
    /// listing nobody can scan. `voice.md`, `expectations.md` and `how-i-work.md`
    /// are the half of a guild that is *you* — they are what `armada doctor`
    /// complains about being unwritten, and they are what a reader opening this
    /// verb came to check. The things you author are cyan; the things Armada or
    /// a tool maintains are grey.
    ///
    /// **Colour agrees with the word and never replaces it** (`render.rs`): the
    /// kind is spelled out in the STATUS column either way, and a monochrome
    /// terminal loses nothing but the grouping.
    pub const fn for_guild_kind(kind: armada_guild::inventory::Kind) -> Role {
        use armada_guild::inventory::Kind;
        match kind {
            Kind::Memory => Role::SignalAmber,
            Kind::Skill | Kind::Subagent | Kind::Workflow | Kind::Hook => Role::RadarCyan,
            // **`permissions` is amber with the memory fragments**, and not
            // grey with the other three YAML files. It is a decision you made
            // about what an unattended agent may do on your machine, which
            // makes it the half of a guild that is *you* — and it is the row a
            // reader scanning this listing after a Job did something surprising
            // came for.
            Kind::Permissions => Role::SignalAmber,
            Kind::Settings | Kind::Plugins | Kind::Mcp | Kind::Schema => Role::SteelGrey,
        }
    }

    /// The colour a change to one guild item is spoken in.
    ///
    /// **`REFUSED` is orange and not red.** Nothing failed and nothing was lost:
    /// the edit is on disk and git still holds the version before it. Red would
    /// say the command broke, which is the misreading this verb most needs to
    /// avoid — a person who thinks his edit is gone will retype it.
    pub const fn for_guild_change(change: armada_core::envelope::GuildChange) -> Role {
        use armada_core::envelope::GuildChange;
        match change {
            GuildChange::Edited => Role::BeaconGreen,
            GuildChange::Deleted => Role::FlareOrange,
            GuildChange::Refused => Role::FlareOrange,
            GuildChange::Viewed | GuildChange::Unchanged => Role::SteelGrey,
        }
    }

    /// The colour a Job state is spoken in.
    ///
    /// **This is the palette table read straight down** (`docs/commands/render.md`):
    /// signal amber is `RUNNING`, flare orange is `STALLED`, radar cyan is
    /// `QUEUED`, stasis purple is `PAUSED`, abort pink is `ABORTED` and distress
    /// red is `BLOCKED`. Those five rows were written for the Bridge and are
    /// spent here first, which is the point of one palette: the CLI and the
    /// screen cannot disagree about what `RUNNING` looks like.
    ///
    /// `DONE` is the one the table does not name, and it takes beacon green for
    /// the reason `PASS` does — it is the state that means the thing worked.
    pub const fn for_job_state(state: armada_core::fleet::JobState) -> Role {
        use armada_core::fleet::JobState;
        match state {
            JobState::Queued => Role::RadarCyan,
            JobState::Running => Role::SignalAmber,
            JobState::Paused => Role::StasisPurple,
            JobState::Stalled => Role::FlareOrange,
            // **`SILENT` shares flare orange with `STALLED`**, because they are
            // the same class of failure to a reader scanning a table — a Job
            // that is not going to move on its own — and the word is what tells
            // them apart. Minting a seventh colour to distinguish two rows that
            // want the same reaction is how a palette stops meaning anything.
            JobState::Silent => Role::FlareOrange,
            JobState::Blocked => Role::DistressRed,
            JobState::Aborted => Role::AbortPink,
            JobState::Done => Role::BeaconGreen,
        }
    }

    /// The colour an in-flight action is spoken in
    /// (`docs/reserved/020-the-tui-decided.md` §5).
    ///
    /// **No new colour, and that is deliberate.** `Acting` is a fourth enum but
    /// it is not a fourth kind of thing to a reader scanning a column: every one
    /// of its three words is a Job on its way to a state the palette already
    /// paints, so each borrows the colour of where it is going. A row that turns
    /// pink when you press `x` and stays pink when the abort lands is one
    /// colour changing once; minting a colour for the *-ING* form would make it
    /// change twice for one event.
    ///
    /// **`REAPING` shares abort pink with `ABORTING`** for the reason `SILENT`
    /// shares flare orange with `STALLED` above: both take a Job and what it
    /// holds away, a reader wants the same thing from both rows, and the word is
    /// what tells them apart.
    pub const fn for_acting(acting: armada_core::fleet::Acting) -> Role {
        use armada_core::fleet::Acting;
        match acting {
            Acting::Aborting | Acting::Reaping => Role::AbortPink,
            Acting::Pausing => Role::StasisPurple,
        }
    }

    /// The colour a probed port state is spoken in.
    pub const fn for_port(state: PortState) -> Role {
        match state {
            // Assigned, nothing bound — the expected state after `init`.
            PortState::Reserved => Role::SteelGrey,
            PortState::Listening => Role::BeaconGreen,
            // Bound by something Armada did not start.
            PortState::Conflict => Role::DistressRed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVERY_ROLE: [Role; 12] = [
        Role::Void,
        Role::Foreground,
        Role::SignalAmber,
        Role::NavalBlue,
        Role::BeaconGreen,
        Role::DistressRed,
        Role::FlareOrange,
        Role::RadarCyan,
        Role::StasisPurple,
        Role::AbortPink,
        Role::SteelGrey,
        Role::DeepSlate,
    ];

    /// **The escape and the hex are two spellings of one colour, and this is
    /// what keeps them one.** The literals exist so painting a cell costs no
    /// allocation; without this test a typo in a decimal triple is invisible
    /// until someone photographs a terminal.
    #[test]
    fn every_roles_escape_is_the_truecolor_form_of_its_documented_hex() {
        for role in EVERY_ROLE {
            let hex = role.hex();
            let channel = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).unwrap();
            let expected = format!("\x1b[38;2;{};{};{}m", channel(1), channel(3), channel(5));
            assert_eq!(role.fg(), expected, "{hex} is painted wrong");
            // The third spelling, for a surface that takes channels rather than
            // an escape — the interview's text area today, the Bridge next.
            assert_eq!(
                role.rgb(),
                (channel(1), channel(3), channel(5)),
                "{hex} has a different value as channels than as an escape"
            );
        }
    }

    /// **Every colour Armada writes text in is readable on the background it
    /// writes it against.**
    ///
    /// `SteelGrey` was `#6B7280`, a 4.0:1 ratio, and the report was simply that
    /// it could not be read — which is what 4.0 means. WCAG AA asks 4.5:1 of
    /// body text and that is the floor asserted here, so the next quiet colour
    /// someone reaches for cannot land under it by eye.
    ///
    /// `Void` and `DeepSlate` are exempt because they are surfaces rather than
    /// text: the CLI never paints a word in either, and a background is supposed
    /// to disappear against itself.
    #[test]
    fn every_colour_text_is_written_in_is_readable_on_the_background() {
        /// sRGB channel to linear, per WCAG 2.
        fn linear(channel: u8) -> f64 {
            let c = f64::from(channel) / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        fn luminance(role: Role) -> f64 {
            let (r, g, b) = role.rgb();
            0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
        }

        let background = luminance(Role::Void);
        for role in EVERY_ROLE {
            if matches!(role, Role::Void | Role::DeepSlate) {
                continue;
            }
            let text = luminance(role);
            let ratio = (text.max(background) + 0.05) / (text.min(background) + 0.05);
            assert!(
                ratio >= 4.5,
                "{} is {ratio:.2}:1 on {} — under AA, which is the report that \
                 raised steel grey",
                role.hex(),
                Role::Void.hex()
            );
        }
    }

    /// Muted is quieter than body text, which is the only thing the role is for.
    /// Raising it for legibility must not raise it to the point where nothing on
    /// the screen is emphasised.
    #[test]
    fn muted_is_still_quieter_than_the_foreground() {
        let brightness = |role: Role| {
            let (r, g, b) = role.rgb();
            u32::from(r) + u32::from(g) + u32::from(b)
        };
        assert!(brightness(Role::SteelGrey) < brightness(Role::Foreground));
    }

    /// Signal amber and flare orange are one step apart at 16 colours, which is
    /// the whole argument for truecolor-or-nothing. They must at least be two
    /// different truecolor values.
    #[test]
    fn the_two_ambers_are_distinct_at_truecolor() {
        assert_ne!(Role::SignalAmber.fg(), Role::FlareOrange.fg());
    }

    /// A verdict and a failure never share a colour, which is the one confusion
    /// the palette exists to prevent.
    #[test]
    fn passing_and_failing_are_not_the_same_colour() {
        assert_ne!(
            Role::for_status(Status::Pass),
            Role::for_status(Status::Failed)
        );
        assert_ne!(
            Role::for_status(Status::Pass),
            Role::for_status(Status::Skipped),
            "a skipped check must not read as an approval"
        );
    }

    /// **One word, one colour, whichever verb printed it.** The drawing paints
    /// `missing` amber under `armada init` and red under `armada doctor`; a word
    /// that means two things depending on its caller is the defect the glossary
    /// opens by naming, and this is the test that keeps it one.
    #[test]
    fn a_machine_condition_word_has_one_colour_across_both_verbs() {
        assert_eq!(Role::for_health(Health::Missing), Role::DistressRed);
        assert_eq!(Role::for_health(Health::Found), Role::BeaconGreen);
        assert_eq!(Role::for_health(Health::Created), Role::BeaconGreen);
        assert_eq!(Role::for_health(Health::Ok), Role::BeaconGreen);
        for health in [Health::Stale, Health::Partial, Health::Offline] {
            assert_eq!(Role::for_health(health), Role::FlareOrange, "{health:?}");
        }
    }

    /// `conflict` is the one row on a `guild pull` that stops the command, so
    /// it is the one that is red — and `unchanged` must not read as approval.
    #[test]
    fn a_conflict_is_the_only_red_row_on_a_sync() {
        assert_eq!(Role::for_sync(Sync::Conflict), Role::DistressRed);
        for sync in [Sync::Added, Sync::Changed, Sync::Removed, Sync::Unchanged] {
            assert_ne!(Role::for_sync(sync), Role::DistressRed, "{sync:?}");
        }
        assert_eq!(Role::for_sync(Sync::Unchanged), Role::SteelGrey);
    }
}
