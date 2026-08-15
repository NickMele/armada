# Output and colour

How Armada renders for a person, for an agent, and for a parser.

> **Status: built — M1.5.** Specified in [`../PLAN.md`](../PLAN.md) §3.1.1, and frozen by the
> golden pairs in `tests/golden/render/` — one `.tty` and one `.plain` per case, rendered at
> one width, so the only difference between them is styling.

## Three audiences

| Audience | Detected by | Gets |
|---|---|---|
| A person at a terminal | stdout is a TTY | Colour, aligned tables, progress on stderr |
| **An agent reading stdout** | stdout is not a TTY | Same structure, no ANSI, no progress |
| A parser | `--json` | The envelope and nothing else |

**Non-TTY human output is a first-class mode, not a degraded one.** Agents call this CLI
constantly and most do not pass `--json` — they run the verb and read what comes back. Same
columns, same order, same words, minus the styling.

## Flags and environment

| | Effect |
|---|---|
| `--color auto` | **Default.** Colour when stdout is a TTY *and* `NO_COLOR` is unset. |
| `--color always` | Colour even when piped. For a pager. |
| `--color never` | Never. Same output a non-TTY gets. |
| `NO_COLOR` | Honoured whatever its value, per the standard. |

**Progress goes to stderr, always.** A live table on stdout means `armada manifest check | jq`
receives frames of animation, and the one consumer the envelope exists for is the one that
breaks. A run redraws its table in an **inline viewport**, never the alternate screen: the
run's own output has to stay in the scrollback, which is where anyone looks for it.

## Tables

Every table is `STATUS · NAME · DETAIL · TIME`, status first and always a word.

**Every word a `STATUS` column holds is SCREAMING**, whether or not the envelope has a field
spelling it — `PASS`, `FAILED`, `REAPED`, `CLAIMED`, `OWNS`, `WOULD`. One column, one meaning,
one spelling, in the payload and on the screen alike.

Case used to carry a second meaning: lowercase marked a render-only word, so a reader could
tell which words they could have grepped out of `--json`. It was a real distinction and it was
dropped, because the column reads worse than the distinction was worth — `ABORTED` beside
`reaped` looks like two kinds of thing when it is one, and the question a reader is actually
asking is answered by the word and its colour.

**A column no row filled is dropped, header and all.** A verb declares the four columns; the
renderer decides which of them earned their width. `armada doctor` times nothing, so its `TIME`
header used to stand over a column of placeholders — a header is a claim that something was
measured, and four em dashes are not a measurement.

The rule is about a **column**, never a row. A table where one row of five has a duration keeps
`TIME` and shows the placeholder against the other four, because there the absence is the
answer. That is also why `DETAIL` keeps its placeholder rather than emptying: `owns  resources
—` is how a reader tells *this workspace owns nothing* from *nobody looked*.

## The banner

```
 █████╗ ██████╗ ███╗   ███╗ █████╗ ██████╗  █████╗
██╔══██╗██╔══██╗████╗ ████║██╔══██╗██╔══██╗██╔══██╗
███████║██████╔╝██╔████╔██║███████║██║  ██║███████║
██╔══██║██╔══██╗██║╚██╔╝██║██╔══██║██║  ██║██╔══██║
██║  ██║██║  ██║██║ ╚═╝ ██║██║  ██║██████╔╝██║  ██║
╚═╝  ╚═╝╚═╝  ╚═╝╚═╝     ╚═╝╚═╝  ╚═╝╚═════╝ ╚═╝  ╚═╝
```

Six lines, 51 columns, signal amber.

| Shows on | Never shows on |
|---|---|
| `armada` with no arguments — entering Helm | any verb |
| `armada init` | `--help` |
| | non-TTY stdout |
| | `--json`, or `--color never` |

**Two places, both of which are moments of orientation rather than work.** A banner on a verb
is a banner in the way; a banner on `--help` is in the way of the one thing you reach for when
you are in a hurry.

**It is suppressed below 51 columns** rather than wrapped. A wrapped wordmark is worse than no
wordmark, and a terminal that narrow has more pressing problems.

**It never reaches non-TTY output.** That is not a nicety — an agent running `armada init` in a
worktree reads stdout, and six lines of block characters at the top of that is noise it has to
learn to skip. Same rule as progress: anything decorative is for the person, not the parser.

## Palette


| Role | Hex | Used for |
|---|---|---|
| void / bg | `#0A0E14` | background |
| foreground | `#E6E8EB` | body text |
| signal amber | `#FFA940` | `RUNNING`, headings, the prompt |
| naval blue | `#4C9FE8` | Job and Drone identifiers |
| beacon green | `#3DDC84` | `OK`, `PASS`, live indicator |
| distress red | `#FF5C5C` | `BLOCKED`, `FAILED` |
| flare orange | `#FFB454` | `STALLED` |
| radar cyan | `#5CE1E6` | `QUEUED`, tool calls |
| stasis purple | `#C792EA` | `PAUSED` |
| abort pink | `#FF7AB6` | `ABORTED` |
| steel grey | `#9CA3AF` | box drawing, muted text |
| deep slate | `#1E2530` | borders, fills |

**Truecolor is the target and there is no 16-colour fallback.** Signal amber and flare orange
are one ANSI step apart; at 16 colours both become bright yellow and the `RUNNING` / `STALLED`
distinction disappears — precisely where a fallback would need to work. Terminals in scope here
are truecolor. On a terminal that is not, the Bridge degrades to monochrome with the state word
carrying the meaning, since state is already spelled out in text rather than signalled by colour
alone.

### Truecolor only

**There is no 16-colour fallback**, and that is deliberate: signal amber and flare orange are
one ANSI step apart, so at 16 colours both become bright yellow and the `RUNNING` / `STALLED`
distinction disappears — precisely where a fallback would need to work. A terminal that cannot
do truecolor gets the no-colour path, which is a supported mode rather than a broken one.

## The interview's text area

[`guild/init.md`](guild/init.md) questions 1–3 want paragraphs, so they open an editable box
**inline**, in the terminal you are already in: wrapping, arrow keys, and a paste of several
paragraphs that arrives whole. It is drawn with `ratatui` — already the decided crate for the
Bridge ([`../PHASES.md`](../PHASES.md) §8.5) — and `Viewport::Inline` is what keeps everything
above it in the scrollback rather than taking the screen.

| Key | Does |
|---|---|
| `enter` | a new line |
| `ctrl-d` | done |
| `esc`, `ctrl-c` | keep the default |

It takes the terminal into raw mode, which is a thing to do to a person at a keyboard and not to
a pipe: a run whose stderr is not a TTY reads paragraphs a line at a time, and a terminal that
refuses raw mode takes the documented default like every other question there. Raw mode and
bracketed paste are restored on drop **and by a panic hook** — the one moment a panic message
matters is the one moment raw mode would make it unreadable.

It draws from the same palette as everything else, through `Role::rgb`. There is no second table
of colours.

## Dependencies

TTY detection and terminal width come from the standard library and one small crate; no curses,
no terminfo database. `ratatui` is the interview's text area and nothing else draws with it — see
`crates/helm/Cargo.toml` for why it is the one exception.

## Exit codes

Rendering never changes an exit code. It is `f(error.class)` regardless of how the result was
displayed ([`reference.md`](reference.md)).

## See also

[`helm/bridge.md`](helm/bridge.md) · [`../glossary.md`](../glossary.md) ·
[`reference.md`](reference.md)
