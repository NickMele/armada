# `armada helm`

The one agent you talk to.

> **Status: shipped.** The launch is assembled and verified, the inbox is wired, the conversation
> is remembered — and **`armada helm` enters it, once this machine has said yes.** Entering is off
> by default; [`enable.md`](enable.md) is the switch, and a fresh install has not flipped it.

**Helm is a conversation, not a screen.** It is a Claude Code session, which is the whole
design: it needs no interface work, so it ships with Fleet instead of after everything else. The
screen is the [Bridge](bridge.md), and it is a separate thing you can run or not run — nothing
below Helm moves either way.

> **There is no `helm` binary.** Kubernetes owns that name and Armada runs on machines that have
> it. Helm is a subcommand, never a program on `PATH`
> ([`../glossary.md`](../../glossary.md)).

## Synopsis

```sh
armada helm [--agent <name>] [--new]
armada helm --print-command
armada helm --json
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--new` | flag | off | Start a fresh Helm conversation instead of resuming. |
| `--agent <name>` | subagent name | `helm` | Use a different persona from `~/.armada/guild/subagents/`. |
| `--print-command` | flag | off | Print the launch command and **enter nothing**. |
| `--exec` | flag | off | Become the session — the explicit spelling of the default. |

Asking for `--print-command` and `--exec` in one line is `bad_invocation`: they ask for opposite
things, and a precedence rule would silently drop one of the two.

## Entering is what this verb does; the machine is what permits it

| Invocation | What happens |
|---|---|
| `armada helm`, `helm.enter` on | enters the session |
| `armada helm`, `helm.enter` off | refused by name, and nothing is written |
| `armada helm --print-command` | the pasteable line; never enters |
| `armada helm --json` | the envelope, `argv` and all; never enters |
| `armada helm --exec` | enters — kept as an explicit synonym |

Entering spends and assembling does not:

| | Costs |
|---|---|
| Assembling the command | nothing |
| Entering the session | a real budget, against a real account, for as long as it is open |

A verb that opened a Claude Code session as a side effect of being run can be reached by a
script, by a shell alias, by a test harness and by a mistyped line, and each of those spends. So
there is a lock: `helm.enter` in `~/.armada/machine.yml` — off on a fresh install, flipped by
[`armada helm enable`](enable.md) and put back by `armada helm disable`. Off, entering is refused
by name:

```
armada helm
-> exit 2
   bad_invocation  entering is off on this machine: `armada helm` — and `armada helm --exec`,
                   which is the same act spelled out — opens a Claude Code session, and this
                   machine has not said yes to that yet
   next: `armada helm enable` turns it on here; `armada helm --print-command` prints the
         line to paste and enters nothing
```

**There used to be a second lock, and it was the defect.** Entering was also behind `--exec`, so
a reader who had already set `helm.enter: true` typed `armada helm`, was handed a command to
paste, and reported it — *"it didn't dump me into a Claude Code session. It asked me to run the
command, which was weird."* Flipping the machine switch **is** the yes. Asking for it a second
time does not make the first answer more considered; it makes it look ignored.

**The printed line was kept and moved rather than deleted.** It is how a person reads the argv
without spending anything to see it, and what a caller opening the session somewhere else pastes.
`--print-command` is where it lives now.

**`--json` does not enter either**, for the same reason: a caller asking for the envelope is
asking for output, and a process replaced by `claude` never prints one. Every script that reads
`data.argv` is such a caller. `--exec --json` is how to say *enter anyway*.

**Refused by name, never as an unknown flag.** A caller told *unknown flag* concludes Armada is
broken or that they typed it wrong, and goes looking for the spelling that works. Told that it is
off and how to turn it on, they either run `enable` or ask for the line and paste it themselves —
both of which `next:` offers.

**On, the process is replaced.** The argv, the documents and the conversation's record are built
and verified exactly the same way on every row of that table; entering records that the
conversation has started and then execs `claude`, and this process does not come back.

The flag words and the reason live in one place each — `verbs::helm::ENTER`, `PRINT` and
`ENTER_IS_OFF` — which the parser, this page, the render's summary line and the refusal all read.
A gate whose reason is retyped per call site says three different things by the third edit, and
the one that reads as an accident is the one somebody works around.

> **Bare `armada` does not enter Helm.** [`../../reserved/020-the-tui-decided.md`](../../reserved/020-the-tui-decided.md)
> §8 gives the bare word a menu of the modules instead, which supersedes
> [`PLAN.md`](../../PLAN.md) §15.1 on that point. `armada helm` is how you enter.

## What the session may do without asking

Helm launches under `--permission-mode`, and the value is `helm.mode` in `~/.armada/machine.yml`
— **`auto` by default**.

| | Mode | Why |
|---|---|---|
| Helm | `auto` | you are at the terminal, so a prompt is a question that gets answered |
| A Drone | `dontAsk` | nobody is there to answer, and a prompt is a stall that costs the whole Job |

That contrast is the decision. **Passing no mode at all was the bug it replaced**: the launch
inherited Claude Code's own default and the first real Helm session had to have every tool call
approved by hand, which is not an orchestrator.

Any of the six modes Claude Code offers may be set — `acceptEdits`, `auto`, `bypassPermissions`,
`manual`, `dontAsk`, `plan`. Anything else is `bad_config`, named, before a launch is assembled:

```
armada helm
-> exit 3
   bad_config  machine.yml helm.mode
               `yolo` is not a permission mode — Claude Code offers acceptEdits, auto,
               bypassPermissions, manual, dontAsk, plan
   next: set `helm.mode:` in ~/.armada/machine.yml to one of them (`auto` is the default),
         then retry unchanged
```

**Checked here rather than at `exec`.** `--permission-mode` is validated at argument-parse time,
so an unrecognised value is a session that dies the instant it is entered, printing Claude Code's
usage error over whatever you were doing and naming no file. The list is the one
`crates/core/src/fleet/drone.rs` already holds a Drone's posture against — one list, because
there is one flag.

## How it works

Assembles one command, and prints it:

```sh
claude --agent helm \
       --append-system-prompt "$(cat ~/.armada/helm/system-prompt.md)" \
       --mcp-config      ~/.armada/helm/mcp.json \
       --plugin-dir      ~/.armada/helm/plugin \
       --settings        ~/.armada/helm/settings.json \
       --permission-mode auto \
       --session-id      <uuid>     # the first launch, which mints it
```

`--resume <uuid>` replaces `--session-id <uuid>` on every launch after the first, which is the
whole of *"the same conversation each day"*. The two are not interchangeable: `--resume` against
a uuid Claude Code has never seen fails with *no conversation found*, and `--session-id` against
one it has seen is a second conversation wearing the first one's name.

| Piece | Comes from |
|---|---|
| The persona | `~/.armada/guild/subagents/helm.md` — **yours**, editable, synced. Seeded from [`templates/guild/subagents/helm.md`](../../../templates/guild/subagents/helm.md); the five behaviours it fixes and why are in [`PLAN.md`](../../PLAN.md) §15.4. |
| The toolbelt | [`mcp.md`](mcp.md) — `fleet.*` and `manifest.*`, registered as `--mcp-config` |
| Armada's own skill | compiled into the binary — how to use Armada, and to raise a manifest change rather than make one ([`../../reserved/008-armada-injects-its-own-skills.md`](../../reserved/008-armada-injects-its-own-skills.md)) |
| Your voice | `~/.armada/guild/voice.md`, `expectations.md` and `how-i-work.md` — **appended to the same system prompt**, after Armada's half, see below |
| The conversation | `~/.armada/helm/session.json` — a uuid, the persona it belongs to, and whether it has run |
| Awareness | [`inbox.md`](inbox.md) — a monitor plus a `Stop` hook, both written to `~/.armada/helm/` |

Its job is **decompose → delegate → aggregate → report**. Classification is *not* its job; that
belongs to Fleet ([`../fleet/spawn.md`](../fleet/spawn.md)), because a Job must be
classifiable before Helm exists.

### Your own words are injected, not read

The persona used to *instruct* Helm to read the three memory fragments at the start of a
session. **It could not.** Its `tools:` list is `mcp__armada__*` and holds no `Read` —
deliberately, because *never do the work* is enforced by the absence of `Read`, `Edit` and
`Bash` rather than by a paragraph. So the instruction was inert, and Helm spoke in nobody's
voice while a file on the machine said exactly how to speak.

**Granting `Read` was the cheaper repair and the worse one.** It widens a toolbelt that is
narrow on purpose, spends a turn per file, and leaves the outcome to a session remembering to do
it — and a session that forgets is a session that ignores you. So `armada helm` reads them and
appends them, which is the same repair [`config scan`'s hand-over](../manifest/config.md)
already made: a session cannot be relied on to find a file Armada could simply hand it.

| Case | What the launch does |
|---|---|
| A fragment you have written | appended, under a heading naming the file it came from |
| A fragment still holding Armada's example text | **skipped** — boilerplate made binding in your name is worse than none, and the persona's own defaults already cover it |
| A fragment missing, unreadable or blank | skipped, the same way |
| All three still Armada's | none of your words appended, and a row saying so — the flag is still there, carrying Armada's own skill |
| A fragment past **24 KB** | cut at a line boundary, with a note in the prompt and in the row |

**Precedence is settled in the prose, not by position.** The appended half opens by saying these
are your words and that they outrank the persona; the persona says the same from its side. A
flag's place in an argv is not something a model reads.

**24 KB, because a real limit exists whether or not Armada names one.** A single argv element is
capped at 128 KiB on Linux and argv plus environ at 256 KiB on macOS; past either, `--exec`
fails with *Argument list too long* at the moment you were expecting a session, naming no file.
Cutting here is visible; discovering it at `exec` is not.

### What it writes

Everything lands under `~/.armada/helm/`, which never syncs — every file in it names a path on
*this* machine ([`PLAN.md`](../../PLAN.md) §13.1).

| File | What it is |
|---|---|
| `mcp.json` | the toolbelt, as a `--mcp-config` document |
| `plugin/` | a session-scoped plugin carrying one monitor |
| `settings.json` | the `Stop` hook, registered for this session and not for the machine |
| `stop-inbox.sh` | what that hook runs |
| `system-prompt.md` | Armada's own skill, then your three memory fragments — what `--append-system-prompt` carries |
| `session.json` | the conversation |

> **`system-prompt.md` is generated, and named so it cannot be mistaken for its sources.**
> `~/.armada/guild/voice.md` is yours, hand-edited and synced; this is rewritten on every launch
> like `mcp.json` beside it. It exists so the printed command can read it back with `"$(cat …)"`
> instead of pasting kilobytes into one line. Edit the guild's files, not this one.

> **It holds two halves, and it was `guild-voice.md` until it held the second.** Armada's own
> instructions to the agents it runs come first
> ([`../../reserved/008-armada-injects-its-own-skills.md`](../../reserved/008-armada-injects-its-own-skills.md)),
> then your fragments under the header that says yours win. **One flag, not two**: `claude
> --help` spells it `--append-system-prompt <prompt>`, singular, so a second occurrence would
> keep only the last and drop the first without a word. On a machine where nothing of yours is
> written the file is still there, carrying Armada's half — which is why the row now says *none
> of yours yet* rather than *none yet*.

**Rewritten on every launch rather than written once**, because each names a path — the `armada`
binary, the inbox — and a machine whose home directory moved would otherwise keep a registration
pointing at a binary that is not there. A file already holding exactly those bytes reports
`unchanged`, so a reader who edited one by hand can see that it was replaced.

### Two structural rules

**Helm reads summaries, never raw transcripts.** Reading Drone transcripts fills its context in
about three days of work, after which it starts forgetting the fleet — the exact failure it exists
to prevent. This is a design constraint, not a tuning knob. It reaches the `Stop` hook too: the
backstop reports *how many* entries are unread and names `fleet.inbox`, rather than pasting the
bodies into Helm's window at the end of every turn.

**Probe never interrupts a Drone.** `fleet.probe` summarises a transcript with a cheap model.
Messaging a busy agent to ask how it is going costs you the thing you were measuring.

## Output

**There is none on the path that enters** — the process is replaced before anything could be
printed. This is `--print-command`'s (and `--json`'s) report: five rows and the command.

```
  STATUS     WIRED         DETAIL
  WRITTEN    toolbelt      ~/.armada/helm/mcp.json
  WRITTEN    monitor       ~/.armada/helm/plugin
  WRITTEN    backstop      ~/.armada/helm/stop-inbox.sh
  WRITTEN    voice         ~/.armada/helm/system-prompt.md
  UNCHANGED  conversation  ~/.armada/helm/session.json

  enter with claude --agent helm --append-system-prompt "$(cat …)" --mcp-config …

OK  helm · conversation new · nothing started; entering is off on this machine — `armada helm enable`
```

**The `voice` row is there on every launch, including the ones with nothing to say.** A launch
that quietly appended nothing looks exactly like one that appended everything — which is how the
persona's *read these three files* instruction went unnoticed for as long as it did. On a guild
nobody has written yet the row reads `none yet — voice.md, expectations.md, how-i-work.md are
Armada's words; armada guild edit voice.md`.

The last line is load-bearing — and it names the actual state. On a machine that has run
`armada helm enable` it reads `entering is on — armada helm alone enters` instead, which is how a
reader who asked for the line learns the switch did take; either way, four `WRITTEN` rows and a
launch command, without it, read exactly like a Helm that is now running.

`--json` carries the same facts plus `argv` as a vector — a `$HOME` with a space in it is
ordinary on macOS, and a consumer that had to split the printed line back into words would break
on exactly those machines. It also carries `command`, which is that vector rendered as the one
line above: the two differ only in the appended prompt, which is bytes in `argv` and
`"$(cat …)"` in `command`, and `command` is derived from `argv` so the two cannot drift.

## Dependencies

| On | Why |
|---|---|
| `claude` | It is a Claude Code session. |
| An initialised guild | The persona lives there. |
| A projected guild | `--agent` names a persona Claude Code has to be able to find, and `~/.armada/guild/` is not on its load path until [`../guild/project.md`](../guild/project.md) puts it there. |
| Fleet | Its tools are Fleet's verbs. |
| An MCP server registration | [`mcp.md`](mcp.md) |

## Exit codes

`0` assembled — `--print-command` or `--json`, on any machine. On the entering path this process
is replaced by `claude` and the exit code is whatever the session exits with; there is no
envelope from that launch, because nothing that could print one is still running.

`2` `bad_invocation` — entering, on a machine that has not run `armada helm enable`; or
`--print-command` and `--exec` in the same line.

`3` `bad_config` — no guild, no such persona, a persona that is not on Claude Code's load path,
or a `helm.mode` Claude Code does not have. They are reported separately, because different
things fix them.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`enable.md`](enable.md) · [`bridge.md`](bridge.md) · [`mcp.md`](mcp.md) · [`inbox.md`](inbox.md) · [`../fleet/spawn.md`](../fleet/spawn.md)
