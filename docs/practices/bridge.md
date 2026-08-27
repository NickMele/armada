# Bridge practices

Bridge is Armada's desktop application: an Electron shell under `apps/desktop`,
built with electron-vite, shipped from the same pnpm workspace and the same
commit as Fleet. Read this before you write TypeScript here. Almost nothing
exists yet — a window that shows nothing but its own title, a preload that
exposes one function that returns a constant, no install, no lockfile. That is
not an invitation to fill the gap with whatever gets a screen on the page. Every
rule below exists because the alternative was tried, in v1, and it lost people
time or trust. Where that's a measured failure rather than a hunch, this doc
says so.

## The three-process split

Electron gives you three processes and they are not interchangeable conveniences
— they are a security boundary and a data-flow boundary at once, and the two
line up on purpose.

| Process | Runs | May hold |
|---|---|---|
| Main | `src/main/index.ts`, full Node | Window lifecycle, the Fleet connection (WebSocket + HTTP), the runtime-file lookup, filesystem and OS access |
| Preload | `src/preload/index.ts`, Node + a sandboxed DOM | Nothing of its own — a translation layer, `contextBridge.exposeInMainWorld` calls only |
| Renderer | `src/renderer/`, React, no Node | UI state, components, everything the user looks at |

`electron.vite.config.ts` builds these as three separate bundles precisely
because the preload is the only thing that crosses between them — it is not an
import path, it's a wire. Nothing the renderer needs from Fleet skips it.

The rule that matters day to day: **every function you add to the preload is a
capability the renderer can call, unconditionally, from any script that ends up
running in that window.** contextIsolation means the renderer can't reach into
Node directly, but it can call anything you handed it through
`contextBridge`. A preload surface is not "the API we intend the UI to use," it
is "the API the UI is physically capable of using." Those are the same set only
if you keep the preload narrow. A correctly narrow surface looks like this —
typed, small enough to read end to end, and shaped around what the renderer is
actually allowed to initiate:

```ts
// src/preload/index.ts
contextBridge.exposeInMainWorld('armada', {
  protocolVersion: (): number => PROTOCOL_VERSION,
  jobs: {
    list: (): Promise<JobSummary[]> => ipcRenderer.invoke('jobs:list'),
    subscribe: (onEvent: (e: JobEvent) => void): (() => void) => {
      const handler = (_: unknown, e: JobEvent) => onEvent(e)
      ipcRenderer.on('jobs:event', handler)
      return () => ipcRenderer.removeListener('jobs:event', handler)
    },
  },
  fleet: {
    status: (): Promise<FleetStatus> => ipcRenderer.invoke('fleet:status'),
  },
})
```

No raw `ipcRenderer`, no `require`, no filesystem handle, no arbitrary-channel
`invoke`. Each entry is one operation with a typed return, not a general-purpose
pipe. If you find yourself writing `invoke(channel: string, ...args: any[])` to
save time, you've handed the renderer the whole main process under a thin
name — stop and expose the specific operations instead.

## Security posture

`src/main/index.ts` already sets `contextIsolation: true`, `nodeIntegration:
false`, `sandbox: true`, and `index.html` already carries
`Content-Security-Policy: default-src 'self'`. None of these are decorative,
and none of them are there because Electron's docs suggest them — each closes
a specific hole:

- **`contextIsolation: true`** keeps the renderer's JavaScript world and the
  preload's JavaScript world as two separate global objects. Without it, a page
  script can walk the prototype chain the preload used and reach Node
  primitives the preload never intended to expose — the preload's *intent*
  stops mattering, only what it touched.
- **`nodeIntegration: false`** means the renderer has no `require`, no `fs`, no
  `child_process`, full stop. If content ever renders in that window that
  Bridge didn't author byte-for-byte — a pasted URL, a rendered Markdown
  review comment, anything from Fleet's event stream — it cannot escalate.
- **`sandbox: true`** runs the renderer in the OS-level sandbox Chromium itself
  uses, which caps the blast radius even of a Chromium bug, not just of code
  Bridge wrote.
- **The CSP** (`default-src 'self'`) is what stops all three of the above from
  being undone by network access instead of Node access. It has to keep
  reaching only `'self'` — no CDN, no remote font, no external image src — because
  the moment a directive reaches off-app, a compromised or malicious payload
  in a review or an event has somewhere to exfiltrate to or fetch a payload
  from. **Bridge talks to Fleet and nothing else; the CSP is that promise
  enforced by the browser engine, not just stated in a doc.**

These four are load-bearing together. Loosening any one of them to make a
feature easier is a security review, not a local decision — say so explicitly
in review rather than quietly relaxing a flag to unblock yourself.

## The component constraint

A Job Board row, a diff view, a legend — every visible surface in Bridge is
built from `packages/tokens` and shadcn primitives, with `lucide-react` for
icons, and nothing else. Not "prefer" — the constraint is absolute because
`packages/tokens` is shared: it is also Doctor's pass/warn/fail palette, and a
one-off color picked to make a component look right in Bridge is a value
Doctor never agreed to and a second place that palette now has to be kept in
sync by hand.

**Nothing invented** means what it says. If the component you need — a
resizable split pane, a virtualized log viewer, whatever the review-and-reply
surface turns out to need — doesn't exist in shadcn, adding a hand-rolled
`<div>` with inline styles or a one-off CSS module is not a shortcut, it's a
component the design system doesn't know about and nobody else can reuse or
retheme. Raise it as a design conversation instead: does shadcn have a
primitive to compose this from, does the token set need a new value before the
component can be built correctly, or is this genuinely new and belongs in a
design doc before it becomes code. `packages/tokens` and the shadcn set are
still being stood up — as they grow, gaps like this are expected, not a sign
you're doing something wrong. The rule is about what you do when you hit one.

## State and data flow

Bridge talks to **one peer**, in the main process, to Armada API. That was
"exactly one connection" until observing a Drone landed, and the number was
never the point — the rule is that the renderer holds none and main holds all
of them. A second socket is opened per Job to carry one Drone's turns, because
the Board's channel is one bounded queue for every Job and transcript rows on it
would evict state changes. See `protocol.md`, the second socket.

The Board's connection:
WebSocket for events, HTTP for queries and commands. That's it — there is no
second connection, no per-component fetch, and **Bridge never talks to a
Drone**. Every Drone interaction is mediated by Fleet; if a feature seems to
need Bridge to reach a Drone directly, the design is wrong, not the
constraint.

For a component, this means data never arrives by that component reaching
into the network itself. A component doesn't own a WebSocket handle or an
`http.get`; it reads from state that the main process populated through the
one connection and the preload handed across. Practically: main owns the
socket and the HTTP client, preload exposes typed operations
(`jobs.list()`, `jobs.subscribe(cb)`) and the renderer's job is to render
what comes back and to call back through those same typed operations for
commands. A component that wants data it doesn't have is missing a preload
call or a piece of shared state, not a fetch of its own — if you're about to
add `fetch()` inside a renderer component, that's the connection getting
duplicated, and duplicated it drifts from Fleet's actual event stream instead
of reflecting it.

## Reconnection and lifetime

Bridge and Fleet have independent lifetimes. Jobs keep progressing with the
window closed — Fleet is a daemon, not a subprocess of the window — and
reopening Bridge should reconnect to whatever Fleet is already running rather
than spawn a second one. Bridge finds that daemon through a runtime file
carrying its port, its pid, and the protocol version it speaks, and **it
verifies the pid before it trusts the file** — a stale runtime file left by a
crashed Fleet points at a port nothing is listening on, or worse, a port
something *else* now owns. Skipping the pid check turns "Fleet isn't running"
into a hung connection attempt with no way to tell it apart from "Fleet is
running and the network is just being slow," which is exactly the ambiguity
this check exists to resolve. A bare connection timeout cannot distinguish
those two cases; the pid check can, before a socket is even opened.

The runtime file's exact location and format are not decided yet — that's an
open question for whoever lands the connection layer, not something this doc
should presume.

What the UI shows has to name the actual state, not paper over it with a spinner:

| State | What's true | What Bridge shows |
|---|---|---|
| No runtime file | Fleet has never started, or its runtime file was cleaned up | "Fleet is not running" plus a way to start it |
| Runtime file present, pid dead | Fleet crashed or was killed without cleanup | "Fleet is not running" (not "unreachable" — the pid check already told you which one this is) |
| Runtime file present, pid alive, socket refuses or times out | Fleet is running but something between Bridge and it is broken | "Fleet is running and unreachable" — distinct copy, distinct next step, because restarting Fleet is the wrong fix here |
| Connected, protocol versions match | Normal | Full UI |
| Connected, minor version skew | Fleet is behind an additive-only bump | Full UI plus a persistent "Fleet is behind, restart when idle" banner — safe only because minor bumps never remove or retype a field |
| Connected, major version skew | Wire-incompatible | The v0 lifeboat: a frozen, hand-written recovery screen — list Jobs with status, kill a Job, stop Fleet, report Fleet's version, nothing else. It has no dependency to break because breaking is the one case it exists to survive |

The lifeboat is deliberately minimal and Bridge-side code should not try to
make it richer — its entire value is being the one path guaranteed to still
work when the rest of the protocol has changed underneath it.

## The v1 failure this app exists to escape

v1 kept a failure log; 77 entries, 11 readable, 9 of those 11 the same
complaint restated: the surface froze, the layout broke on resize, the legend
was illegible, columns flip-flopped between renders. That's not "Electron is
slow," it's "a UI that renders unbounded lists and reflows without a plan for
either." The fix is process, not vibes:

- **Nothing renders an unbounded list directly.** A Job Board with more rows
  than fit on screen, a log pane, a diff with more than a couple hundred
  lines — virtualize it. "It was fine with 20 Jobs in dev" is not evidence it
  survives 200 in the field; the freeze in v1's log was not observed until
  someone was actually running enough Drones to hit it.
- **Every layout is designed for resize, not just for its initial size.**
  Flex and grid with real min-widths and defined overflow behavior, not a
  fixed-pixel layout that happens to look right at 1280×800 because that's the
  size someone had open when they built it. "Layout breaks on resize" was
  its own line item in the failure log, not a subset of "freezing" — treat it
  as a separate thing to verify, not a side effect you'll notice for free.
- **A legend, key, or status color is read against the actual token contrast,
  not eyeballed.** Illegible legend was specific enough to be its own
  complaint, not "hard to use" in general — check contrast against the token
  set, not against whatever the editor's default background happened to be.
- **Column order is stable across a render.** Column identity and order live
  in state that does not change when new rows arrive, and resize logic stays
  separate from data-arrival logic.

  **Width is a different question here, and the v1 entry does not settle it.**
  That complaint was logged against a ratatui terminal, where a column is a
  count of characters and a recompute is a hard reflow of a fixed grid with no
  layout engine underneath it. This app has one. Content-sized tracks are how
  CSS grid is meant to be used, and forbidding them produces the opposite
  defect — values truncated to an ellipsis beside empty space, which is what
  fixed widths transcribed from a drawing actually shipped.

  So: a column carries a **floor**, never a fixed width, and a list sizes its
  columns **once for the whole list** rather than per row — `subgrid`, with the
  tracks declared on the list and borrowed by each row. That is what keeps a
  column aligned down the list without pinning it to a number somebody guessed.
  A `minmax(0, 1fr)` filler absorbs the slack so growth lands in one deliberate
  place rather than spreading every value apart.

None of this is generic React advice offered because it's good practice in
the abstract. It's the specific list of things that were measured to break
in the predecessor of this exact app.

## Review and reply is one loop

Reviewing a Drone's output and replying to it is one continuous interaction —
read the diff, decide, say what's wrong or approve it, and that has to reach
the Drone — not two surfaces and not two panels either. A design that puts the
diff in one view and the reply box in another, even side by side in the same
window, recreates v1's problem inside Electron: it turns one decision into a
context switch, and a context switch is exactly the kind of friction the
failure log's freezing and flip-flopping already made people distrust the
surface for. If a review screen is ever designed where "leave feedback" is a
separate route, tab, or modal from "look at the diff," that's the thing to
push back on before it's built, not after.

## Everything else specific to this app

**The build is three bundles from one config, not one.** `electron.vite.config.ts`
defines `main`, `preload`, and `renderer` as separate `lib`/`root` targets
because the preload's build is what actually enforces the process boundary —
importing across them at the source level would silently blur exactly the line
`contextIsolation` exists to hold. If a change makes you want to import
renderer code from main or vice versa, that want is a sign the code belongs in
`packages/`, not that the import should happen.

**What Bridge does with a failure is not decided here.** The wire fields it can
rely on, the unknown-code fallback, and the rule that where a message appears is
chosen by blast radius rather than severity all live in
`docs/contracts/error-contract.md`.

**`protocol-version.toml` at the repo root is the one number both sides check**,
Rust and TypeScript alike (`crates/ipc/build.rs` on Fleet's side, a generated
TS constant on Bridge's). Bridge should read the generated constant, never a
hardcoded literal — a hand-typed `1` in Bridge code is a second source of
truth the day the file changes.

**`packages/` stays empty until something is actually shared** — the generated
IPC types will be its first real occupant once the protocol crate exists.
Don't create a package pre-emptively for Bridge alone; a package with one
consumer is a folder with an import path, per `packages/README.md`.

**pnpm workspace, not npm or yarn**, `packages: ["apps/*", "packages/*"]`
per `pnpm-workspace.yaml` — Fleet and Bridge version together in one repo and
one commit, so don't introduce a second lockfile or a second package manager
for anything under `apps/desktop`.

## Open questions

Name these rather than deciding them by writing code that assumes an answer:

- **[runtime-file-format]** The runtime file's exact path, filename, and
  on-disk format (JSON? TOML?) — not fixed anywhere in the repo yet.
- **[fleet-ports]** Which HTTP/WebSocket ports Fleet binds to, and whether
  that's fixed or Fleet-chosen-and-recorded-in-the-runtime-file.
- **[list-virtualization]** The virtualization approach for lists, logs and
  diffs. No library chosen.
- **[shadcn-and-tokens]** How shadcn primitives take their values from the
  token set. Tailwind 4 is installed and `packages/tokens` is the only
  spellable scale, but no primitive has been added yet, so nothing has proved
  the two compose.
