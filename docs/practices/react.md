# React in Bridge

**Kind:** practice. **Governs:** how a component in `apps/desktop` and
`packages/components` is written. Read before writing one.

`bridge.md` says where data comes from and what a component may be built out
of. This says how the component itself is written, and it exists because those
are different mistakes.

---

## What does not apply here

Most React performance advice is written for a page served over a network.
Bridge is not one, and following it would spend effort on problems this app
does not have.

| Common advice | Why it is not ours |
|---|---|
| Eliminate fetch waterfalls | The renderer never fetches. One connection lives in the main process |
| Cut the initial bundle for time-to-interactive | The app loads from disk |
| Server rendering, hydration, streaming | There is no server and no hydration |
| A client fetching library for deduplication | A component that fetches is a second connection, which `bridge.md` forbids |

**What does apply** is everything about re-rendering and about drawing: this is
a window that updates from a live event stream and stays open all day.

## Data arrives, it is not fetched

**A component reads state the preload handed it and calls back through the same
typed operations.** No `fetch`, no socket handle, no client in a hook.

**No `useEffect` fetches data.** An effect that loads on mount is the
duplicated connection wearing a hook, and it drifts from Fleet's event stream
rather than reflecting it.

**An effect synchronises with something outside React, or it should not exist.**
Subscribing to the preload's event stream is one. Deriving a value from props
is not — compute it during render.

## The list is the hard part

The Job Board is a list that reorders while somebody is reading it, and every
rule below exists because of that.

### Key by identity, never by position

**A row's key is its Job id.** An index key makes React reuse the wrong row
when the list reorders, so a status badge changes under the cursor while the
text does not.

### Nothing remounts on an update

**An event that changes one Job must not remount the others.** A remount
restarts every transition and loses focus, and the reading rule that a live
value may pulse but rows may not move depends on it.

### Derived vocabulary is derived

**A status's verb, glyph and token come from the generated module and are read
during render.** Never held in state, never copied into a component, never
written by hand — that is the drift `lib/job-states.js` was deleted for.

### A long list is bounded, and says so

**A list that renders everything freezes**, which is a v1 failure. Bound it and
render what was left out as a line the reader can see, rather than truncating
quietly.

## Re-rendering

**Measure before memoising.** A `useMemo` around a cheap computation costs more
than it saves and hides which values are actually expensive.

**Read state where it is used.** A value read high and passed down re-renders
everything between; read at the point of use, only that subtree re-renders.

**Narrow an effect's dependencies to what it reads.** An effect depending on a
whole object runs on every change to any field of it.

**Initialise expensive state lazily.** `useState(() => build())` runs once;
`useState(build())` runs on every render and throws the result away.

## Motion, and why React is involved

`design-system.md` forbids entrance animation on a data surface and permits one
continuous animation: the running mark's pulse. Two React consequences follow.

**A row must keep its identity across updates** or its transition restarts —
see keys and remounting above.

**The pulse is CSS on a stable element.** Driving it from state re-renders a row
on every frame, and `prefers-reduced-motion` then has to be handled in two
places rather than one.

## Failure

**A Job that cannot be rendered must not blank the window.** An error boundary
around the list keeps the rest of the app usable, and the failure is a row
saying what it could not read.

**A missing verb or glyph renders the wire spelling and says it is missing.**
Never a blank cell, never an invented word — the registry carries variants with
neither, and a blank hides that.

## Stories are the tests

**Every story is a test the moment it exists.** `@storybook/addon-vitest`
mounts each one in headless Chromium, so a story that throws fails without
anyone writing an assertion.

**A `play` function is where a story asserts what a person should see.** A
handful per surface, on the behaviour that surface exists to express — never
one per story.

### What earns a play

| Earns one | Where |
|---|---|
| A keyboard contract written by hand | `Dialog`'s Enter, `Tabs`'s arrows |
| A rule about what does not happen | `Sheet`'s scrim takes no press |
| Controlled state that must stay inert | `PhaseStrip`'s `pinnedStage` |
| A callback falling through to another | `ActivityLogSheet`'s `Show me` |
| Unmounted versus hidden | `JobRecord`'s closed sections |
| A roving cursor, and its clamp | `ActiveJobsList` |

**A variant, a measurement, a colour or a sum earns none.** A `play` that reads
what the args already say is ceremony, and one that computes is a unit test
paying a browser's price.

### Assert on roles, names and text

**Never a class name and never a `data-` attribute.** A test naming a
stylesheet's hook fails on every refactor and says nothing about what a person
saw or heard.

**Where no accessible property carries the fact, add the one that should.**
`JobDiffSheet`'s rail took `aria-current` for this reason: which file is open
was in `data-on` alone, so a screen reader was never told.

**A prop callback is asserted with `fn()` from `storybook/test`.** Prefer a
rendered outcome wherever one exists — a spy proves the component reported, not
that anybody could see it.

### Two traps

**Move the pointer off a control before asserting the layer closed.** Hover and
a pin are two reasons one card is open, and a press leaves the cursor on what
it pressed.

**Locate an element with no role through the structure, not its class.** A
scrim is the panel's `parentElement`; dispatch with `fireEvent` where there is
nothing to hit-test.

### Verify a play by breaking what it guards

**A new `play` is run once against a deliberately broken component.** One that
passes either way asserts nothing, and reads exactly like one that works.

**Arithmetic is not tested here.** A pure function belongs in `packages/screens`,
where a hundred cases cost what one costs.

## TypeScript

**Props are a named type, exported beside the component.** A component whose
props are inline is one nothing else can wrap.

**No `any` at the preload boundary.** That boundary is where wire types become
app types, and it is the one place a wrong shape is caught before it spreads.

## Open questions

- **[react-list-virtualization]** At what length does the Job list need
  windowing rather than a bound, and does windowing survive the rule that a row
  must not remount while the list reorders? The list is bounded today and names
  what it left out, which is honest but is not an answer.
