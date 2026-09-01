# Comparing a screen to its drawing

**Kind:** practice. **Governs:** `pnpm shoot`, the tool that screenshots a
drawing, the component gallery and Bridge's own screens, and pairs them. Read
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
| `pnpm shoot --sheet` | Pairs what has been captured into `.shots/sheet.html` and `.shots/pairs/` |

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
imports the component it is a shot of. The screens are declared in
`*.screens.tsx` beside the code they draw: a file exports a `title` and a
`screens` array, and each entry states its own `data-shot` mark, the name to
print, and one element. Fixtures are written in the file — enough of the wire's
shape for the composition to decide what it draws, and nothing else. Nothing
runs; a screen that needed a Fleet is a screen nobody captures.

**A mark is written by hand here rather than derived.** There is no story export
name to derive one from, and a mark that a drawing has to match is not something
to leave to a transform. Two screens claiming one mark is refused at build.

**Everything it writes is under `.shots/`, which is ignored.** Regenerating both
sides takes about a minute.

## Reading the output

| Output | For | Says |
|---|---|---|
| `.shots/sheet.html` | A person | One page, drawing left, build right, equal width |
| `.shots/pairs/<state>.png` | An agent | Both halves in one image, to hold in context and describe |
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

**It does not compare pixels.** It puts two images side by side and reports
which states pair and how far the heights are apart; reading the difference is a
person's job or an agent's.

**It captures resting states only.** Hover, focus and open menus need a pointer.
A split button is captured closed, so what its menu holds is not on the shot —
read that in the markup or in the code.

**`--bridge` captures and does not pair.** `--sheet` pairs the drawing against
`app`. Pairing a drawing against `bridge` needs the drawing's frames to carry
the same marks, and no drawing does yet.

**It pairs one drawing at a time.** `--design` replaces `.shots/design/`.

**It does not pair a branch against `base`.** #209 wants that pair from the same
harness. The pairing is written over named sides so a second source can be
added; nothing captures a `base` side yet.
