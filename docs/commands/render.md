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

**Progress goes to stderr, always.** A spinner on stdout means `armada manifest check | jq`
receives frames of animation, and the one consumer the envelope exists for is the one that
breaks.

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
| steel grey | `#6B7280` | box drawing, muted text |
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

## Dependencies

None. TTY detection and terminal width come from the standard library and one small crate; no
curses, no terminfo database.

## Exit codes

Rendering never changes an exit code. It is `f(error.class)` regardless of how the result was
displayed ([`reference.md`](reference.md)).

## See also

[`helm/bridge.md`](helm/bridge.md) · [`../glossary.md`](../glossary.md) ·
[`reference.md`](reference.md)
