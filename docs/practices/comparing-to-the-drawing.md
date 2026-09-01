# Comparing a screen to its drawing

**Kind:** practice. **Governs:** `pnpm shoot`, the tool that screenshots a
drawing, the component gallery and Bridge's own screens, and compares them. Read
before changing a screen, and before reviewing one.

A screen shipped about thirty differences from its drawing with every gate
green. An unimported stylesheet rendered a component as a vertical stack and a
`null` stopped a whole screen drawing; typecheck, the tests, Storybook and the
docs gates all passed. Nobody had turned a render into an image and put it
beside the drawing.

---

## The three commands

| Command | What it does |
|---|---|
| `pnpm shoot` | Builds the gallery, captures every marked screen story to `.shots/app/` |
| `pnpm shoot --bridge` | Builds Bridge's own screens from `apps/desktop`, captures them to `.shots/bridge/` |
| `pnpm shoot --design <file.dc.html>` | Captures every marked frame of a drawing to `.shots/design/`, caching the drawing beside them |
| `pnpm shoot --design <file> --suggest` | Proposes a mark for each unmarked frame instead of refusing |
| `pnpm shoot --sheet [sides]` | Compares every side that has been captured, into `.shots/sheet.html` and `.shots/rows/`. `design:bridge` narrows it |

**It needs nothing but an installed workspace.** No network, no running Fleet,
no built app. The browser is Bridge's own Electron.

### `app` is the components, and `bridge` is the app

**The side called `app` is the component library's gallery, and for a long time
that was the only side there was.** It answers whether a component is drawn
right. It cannot answer whether the screen the app assembles out of them is,
because the gallery's screen stories build their own arrangements: `Screens/
Inside a job` hand-builds the Job header out of four `<Button>`s. That is a
drawing of the screen written in React. A change to the app's own header
could not move it, so a header could ship rebuilt and this tool would report the
old one with everything green — which is the same class of miss the tool was
built to end, one level up.

**`--bridge` captures the app's own compositions.** Every figure on that side
imports the component it is a shot of, and a screen is a screen: `JobDetail` at
one state, not its header and not one of its buttons. Everything on the shot —
the badge, the field run, the tree, the panel, the acts — is derived by the
app's own code from one fixture, the way it is derived from one wire read at
runtime. A shot of a control in isolation proves the control and says nothing
about the screen, which is the question this side exists to answer.

**The stage is Bridge's own window, 1280×800, and a screen may name a second
width.** `width` on a screen entry draws the same composition at another size,
which is how "designed for resize rather than for the size it was built at"
stops being a claim nobody checks.

**The page loads the stylesheet Bridge ships, read out of its own build.** Not a
list assembled by the tool: the renderer imports Tailwind between the tokens and
the components, so a page that followed only the component stylesheets had no
`box-sizing: border-box` and every screen overflowed by its own padding. That
looked exactly like a layout defect in the app and was reported as one. **Check
the page loads what the app loads before believing what it shows.**

### Adding or changing a Bridge screen

A screen lives in a `*.screens.tsx` beside the code it draws. The file exports
`title` and `screens`; each entry is a mark, a name, the render it is one of,
and one element. Nothing runs — a screen that needed a Fleet is a screen nobody
captures — so a fixture is the values the wire would carry, written down.

```tsx
export const title = "Inside a job";

export const screens: Screen[] = [
  {
    mark: "inside-a-job-killed",   // what it lines up by. Chosen, not derived
    name: "Killed",                // what the page prints above it
    render: "stopped",             // which of renderFor's arrangements
    width: 900,                    // optional. 1280, Bridge's own, otherwise
    element: <JobDetail job={...} watched={...} {...INERT} />,
  },
];
```

Then:

```sh
pnpm shoot --bridge            # capture, and rewrite the snapshots
```

**Look at the PNGs it names.** That is the step; everything else exists to make
it cheap. The snapshots are a diff, not a substitute — markup says what is
there, and only the image says whether it is drawn right.

**A mark is written by hand rather than derived.** There is no story export name
to derive one from, and a mark that a drawing has to match is not something to
leave to a transform. Two screens claiming one mark is refused at build.

### What keeps it current

**`.shots/` is ignored, so the images are not a baseline.** Two things stand in
for one:

| | Catches | Where it runs |
|---|---|---|
| `apps/desktop/screens/snapshots/` | A screen that changed. The markup is checked in, so a rebuilt header is a changed file on the pull request whether or not anybody ran the tool | `pnpm shoot --bridge` writes them; `--check` fails on drift without writing |
| The gate rule | A screen that does not exist. Every variant of `Render` has a screen, every screen has a snapshot, and neither list is one somebody maintains — `Render` is the app's own union | `cargo xtask verify-foundations` |

**Neither of them looks at anything.** They make an unlooked-at change visible;
a person or an agent still has to open the image. Two of the five renders had no
screen at all when the rule was written, and nobody had noticed.

**Everything it writes is under `.shots/`, which is ignored.** Regenerating both
sides takes about a minute.

## Reading the output

| Output | For | Says |
|---|---|---|
| `.shots/sheet.html` | A person | One page, one row per state, every captured side across it at equal width |
| `.shots/rows/<state>.png` | An agent | Every side of one state in one image, to hold in context and describe |
| `.shots/sheet.json` | A caller | The same comparison, machine-readable |
| `.shots/<side>/shots.json` | A caller | What that side captured, and from what |

**A state on one side only is the finding, not an omission.** `design-only` is a
screen drawn and never built; `app-only` is a screen built with nothing drawn to
check it against.

**A height gap over a tenth of the taller shot is flagged**, and never under
16 CSS pixels. Below that a difference is a font metric or a border radius;
above it something is present on one side and absent on the other.

## How a state is marked

**The mark is `data-shot`, and both sides carry it.** Pairing is by that value
and by nothing else.

### The build marks itself

**The gallery stamps `data-shot` on every story under `Screens/`**, derived from
the story's export name — `WaitingOnYou` becomes `waiting-on-you`. Nothing is
marked by hand and nothing else is marked at all: a component has no frame in a
drawing to pair with.

**Where two screens export a story of the same name, both marks are qualified by
their screen** — `inside-a-job-running` and `the-list-running`. One flat mark
cannot hold both, and the overwrite would be silent.

### A drawing is marked by hand

**A drawing carries `data-shot` on the element that is the screen**, put there
by whoever draws it. Nothing on the design side can be made to require it.

**An unmarked drawing is refused, by frame.** That refusal is the enforcement: a
drawing that cannot be paired blocks the implementation it was drawn for.
`--suggest` turns the refusal into an attribute line per frame, so marking one
is typing rather than a return trip.

**A partly marked drawing is refused on the same terms.** Capturing what it can
would report the unmarked frames as built-and-not-drawn, which is a different
defect and a false one.

## What it does not do

**It does not compare pixels.** It puts the sides side by side and reports which
states are on all of them and how far the heights are apart; reading the
difference is a person's job or an agent's. Where more than two sides have a
state, the widest disagreement is reported as the pair it is between — "these
two differ" is something to go and look at, and "the spread is 25%" is not.

**It captures resting states only.** Hover, focus and open menus need a pointer.
A split button is captured closed, so what its menu holds is not on the shot —
read that in the markup or in the code.

**A sheet is every side that was captured, in one run.** Drawn, arranged,
assembled — what the screen should be, how the component library puts it
together, and what the app actually mounts, left to right. A disagreement
between any two is a finding, and which two says whose it is.

**It was two sides and a named pair, and that was wrong.** Verifying one screen
took three runs and somebody remembering the third, so a person runs the first
pair, sees green and stops — which is how a header shipped reading `Needs you`
against a drawing that said `Escalated`, with a pair that agreed with itself
because both halves came from the gallery. `--sheet design:bridge` still narrows
the columns when a question really is about two of them; narrowing is the
exception now rather than the only mode.

**A side with nothing captured is left out rather than drawn as a column of
absences.** An empty column says "you did not run it", which is not what the
page is for.

**Only one finding is a refusal, and it needs the drawing.** A state drawn and
built by nothing is a screen somebody drew and nobody made. Everything else the
sheet reports is two renderings of one screen disagreeing — neither the gallery
nor the app is the authority over the other, so that is drift, and drift is read
rather than refused.

**It pairs one drawing at a time.** `--design` replaces `.shots/design/`.

**It does not compare a branch against `base`.** #209 wants that from the same
harness. The comparison is written over named sides so a second source can be
added; nothing captures a `base` side yet.
