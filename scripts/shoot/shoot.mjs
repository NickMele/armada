/* shoot — turn a screen into an image, and put it beside its drawing.
 *
 * A screen shipped about thirty differences from its drawing with every gate
 * green. Two of them — a stylesheet nothing imported, so a component rendered
 * as a vertical stack, and a `null` that stopped a whole screen drawing — were
 * invisible to typecheck, to the tests, to Storybook and to the docs gates, and
 * obvious the instant somebody looked at the screen. Nobody had ever turned a
 * render into an image and put it beside the drawing. That is the entire gap
 * this closes.
 *
 * It works because both sides are HTML: a drawing is a `.dc.html` file, the
 * gallery renders every story to one page, and one browser reads both. They are
 * paired by a shared attribute, `data-shot`.
 *
 * ## Written to be invoked, not only typed
 *
 * #209 puts visual evidence behind a harness the repository declares and Fleet
 * runs. So every command writes a manifest beside its images and never asks a
 * caller to scrape this terminal:
 *
 *   .shots/app/shots.json      what was captured from the build, and where
 *   .shots/design/shots.json   the same for a drawing, plus what was cached
 *   .shots/sheet.json          the comparison — the file a caller reads
 *
 * Two per-side manifests and one comparison, because a side is captured on its
 * own and is worth reading on its own; the comparison is a third fact that
 * needs both and is the one thing a reviewer actually asked for.
 *
 * Nothing here knows what a browser is. That lives behind `browser.mjs`.
 *
 * Documented in `docs/practices/comparing-to-the-drawing.md`. */
import { spawn } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { browse } from "./browser.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../..");
const shots = join(root, ".shots");

/* Two shots differing in height by more than this fraction of the taller one
 * are flagged. A tenth is roughly one row of a list or one line of a header —
 * below that the difference is a font metric or a rounded border, above it
 * something is present on one side and not the other. Never flagged under
 * sixteen CSS pixels, so a chip is not reported over a hairline. */
const HEIGHT_TOLERANCE = 0.1;
const HEIGHT_FLOOR = 16;

/* A side is a directory of PNGs named by state. Pairing knows nothing else
 * about either side, which is what lets a second left-hand side arrive later —
 * #209 pairs a branch against `base`, this pairs a build against a drawing,
 * and they are the same pairing over different sources. Adding `against: base`
 * is a third entry here and a fourth line in AGAINST, not a rewrite. */
const SIDES = {
  design: {
    dir: join(shots, "design"),
    label: "Drawing",
    absent: "Not drawn. The build has a state the drawing does not.",
  },
  app: {
    dir: join(shots, "app"),
    label: "App",
    absent: "Not built. The drawing has a state the build does not.",
  },
  /* Bridge's own screens, assembled by `apps/desktop` rather than by a story
     that imitates it. A third side and not a replacement for `app`: the
     gallery answers whether a component is drawn right, and this answers
     whether the screen the app assembles out of them is. */
  bridge: {
    dir: join(shots, "bridge"),
    label: "Bridge",
    absent: "Not assembled. The drawing has a state Bridge does not.",
  },
};

/* Which two sides a sheet compares, keyed `<left>:<right>`.
 *
 * **Two sides, named, and never three.** A state both the gallery and Bridge
 * render has three pictures, and a sheet that silently dropped one of them
 * would rebuild the blind spot this side was added to end — the pair that
 * agreed with itself and proved nothing. So the pair is chosen rather than
 * inferred: a run says which two it is comparing, and the third picture is a
 * second run away.
 *
 * **`app:bridge` needs no drawing.** It asks whether the gallery's arrangement
 * of a screen and the app's assembly of it are the same screen — which is the
 * question that had no asker when a story's fixture said `Needs you` and the
 * pair beside it agreed, because both halves came from the gallery. */
const AGAINST = {
  "design:app": {
    left: "design",
    right: "app",
    absent: {
      design: "Not drawn. The build has a state the drawing does not.",
      app: "Not built. The drawing has a state the build does not.",
    },
    only: { left: "drawn, not built", right: "built, not drawn" },
    blocking: "Drawn and not built",
  },
  "design:bridge": {
    left: "design",
    right: "bridge",
    absent: {
      design: "Not drawn. Bridge assembles a state the drawing does not.",
      bridge: "Not assembled. The drawing has a state Bridge does not.",
    },
    only: { left: "drawn, not assembled", right: "assembled, not drawn" },
    blocking: "Drawn and not assembled",
  },
  "app:bridge": {
    left: "app",
    right: "bridge",
    absent: {
      app: "No story. Bridge assembles a screen the gallery does not arrange.",
      bridge: "No screen. The gallery arranges one the app does not assemble here.",
    },
    only: { left: "in the gallery, not in the app", right: "in the app, not in the gallery" },
    // Neither side of this pair is the authority, so nothing here blocks the
    // way a drawing does. What it finds is drift, and drift is read rather
    // than refused.
    blocking: null,
  },
};

const DEFAULT_PAIR = "design:app";

const USAGE = `shoot — screenshot a screen and its drawing, and pair them

  pnpm shoot                        the components: build the gallery, capture
                                    every marked screen story to .shots/app/
  pnpm shoot --bridge               the app: build Bridge's own screens from
                                    apps/desktop and capture them to
                                    .shots/bridge/
  pnpm shoot --design <file.dc.html>
                                    a drawing: capture every [data-shot] frame
                                    to .shots/design/, and cache the source
  pnpm shoot --design <file> --suggest
                                    propose a mark for each unmarked frame
                                    instead of refusing
  pnpm shoot --sheet [pair]         pair what has been captured into
                                    .shots/sheet.html and .shots/pairs/.
                                    pair is design:app (the default),
                                    design:bridge or app:bridge

Everything it writes is under .shots/, which is ignored.
`;

// ------------------------------------------------------------------- the shell

const die = (...lines) => {
  for (const line of lines) console.error(line);
  process.exit(1);
};

const plural = (n, one, many) => `${n} ${n === 1 ? one : many}`;

const now = () => new Date().toISOString();

/** PNG carries its size in the IHDR chunk, at a fixed offset. No decoder. */
function pngSize(file) {
  const head = readFileSync(file).subarray(0, 24);
  return { width: head.readUInt32BE(16), height: head.readUInt32BE(20) };
}

const pngsIn = (dir) =>
  existsSync(dir)
    ? Object.fromEntries(
        readdirSync(dir)
          .filter((f) => f.endsWith(".png"))
          .map((f) => [f.slice(0, -4), join(dir, f)]),
      )
    : {};

/* A side, read back off disk. Sizes come from the side's own manifest, in CSS
 * pixels, so every number this tool prints is in the same unit the drawing and
 * the stylesheets are written in. A PNG is captured at two device pixels per
 * CSS pixel, so reading its header instead would double every figure — that is
 * the fallback, used only when the manifest is missing, and it says so. */
function sideShots(dir) {
  const files = pngsIn(dir);
  let css = {};
  try {
    css = Object.fromEntries(
      JSON.parse(readFileSync(join(dir, "shots.json"), "utf8")).shots.map((s) => [s.state, s.css]),
    );
  } catch {
    css = {};
  }
  return Object.fromEntries(
    Object.entries(files).map(([state, file]) => [
      state,
      css[state]
        ? { file, ...css[state], measured: "css" }
        : { file, ...pngSize(file), measured: "device pixels" },
    ]),
  );
}

const manifest = (file, body) => {
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, `${JSON.stringify(body, null, 2)}\n`, "utf8");
  return file;
};

/** One row per shot, the same shape on both sides. */
const shotRows = (read) =>
  [...read.written]
    .sort((a, b) => a.mark.localeCompare(b.mark))
    .map((w) => ({
      state: w.mark,
      file: relative(shots, w.file),
      css: { width: Math.round(w.width), height: Math.round(w.height) },
      pixels: pngSize(w.file),
    }));

// The path is printed because a shot is meant to be opened, and a name with no
// path makes a person go and find it. Absolute, because most terminals make an
// absolute path clickable and a relative one nothing at all.
const printShots = (rows, where) => {
  console.log(`\n${plural(rows.length, "state", "states")} captured to ${where}`);
  const pad = Math.max(8, ...rows.map((r) => r.state.length));
  const size = Math.max(9, ...rows.map((r) => `${r.css.width}×${r.css.height}`.length));
  for (const r of rows) {
    const dims = `${r.css.width}×${r.css.height}`;
    console.log(`  ${r.state.padEnd(pad)}  ${dims.padStart(size)}  ${resolve(shots, r.file)}`);
  }
};

// ---------------------------------------------------------------------- the app

async function shootApp() {
  console.log("Building the gallery");
  const built = await new Promise((done) =>
    spawn(process.execPath, [join(root, "packages/components/gallery/build.mjs")], {
      stdio: "inherit",
    }).on("exit", done),
  );
  if (built !== 0) die("The gallery did not build, so there is nothing to capture.");

  const into = SIDES.app.dir;
  rmSync(into, { recursive: true, force: true });

  const page = join(root, "packages/components/gallery/dist/gallery.html");
  const read = await browse({ page, capture: true, into, width: 1440, height: 1200 });

  if (!read.written.length)
    die(
      "No story is marked, so there is nothing to capture.",
      "The gallery stamps data-shot on stories under Screens/ — if that tree is empty, so is this.",
    );

  const rows = shotRows(read);
  printShots(rows, ".shots/app/");
  if (read.failures.length) {
    console.log("\nThe page reported:");
    for (const f of read.failures) console.log(`  ${f}`);
  }

  manifest(join(into, "shots.json"), {
    tool: "shoot",
    side: "app",
    captured_at: now(),
    source: { kind: "gallery", page: relative(root, page) },
    shots: rows,
    page_errors: read.failures,
  });
  console.log("\n.shots/app/shots.json — what was captured, for a caller");
}

// ------------------------------------------------------------------- bridge

/* The app's own screens, and the reason this side exists.
 *
 * The gallery's `Screens/Inside a job` hand-builds its header out of four
 * buttons. It is a drawing of the screen written in React, so a change to
 * `Acts.tsx` cannot move it — the header could ship rebuilt and this tool would
 * report the old one, green. Every figure on this side imports the component it
 * is a shot of. */
async function shootBridge() {
  console.log("Building Bridge's screens");
  const built = await new Promise((done) =>
    spawn(process.execPath, [join(root, "apps/desktop/screens/build.mjs")], {
      stdio: "inherit",
    }).on("exit", done),
  );
  if (built !== 0) die("The screens did not build, so there is nothing to capture.");

  const into = SIDES.bridge.dir;
  rmSync(into, { recursive: true, force: true });

  const page = join(root, "apps/desktop/screens/dist/screens.html");
  const read = await browse({ page, capture: true, into, width: 1440, height: 1200 });

  if (!read.written.length)
    die(
      "No screen is marked, so there is nothing to capture.",
      "A `*.screens.tsx` exports `title` and `screens`, and each screen states its own mark.",
    );

  const rows = shotRows(read);
  printShots(rows, ".shots/bridge/");
  if (read.failures.length) {
    console.log("\nThe page reported:");
    for (const f of read.failures) console.log(`  ${f}`);
  }

  manifest(join(into, "shots.json"), {
    tool: "shoot",
    side: "bridge",
    captured_at: now(),
    source: { kind: "screens", page: relative(root, page) },
    shots: rows,
    page_errors: read.failures,
  });
  console.log("\n.shots/bridge/shots.json — what was captured, for a caller");
}

// ------------------------------------------------------------------ the drawing

/** Every relative asset a file reaches for, followed into CSS. */
function assetsOf(file) {
  const text = readFileSync(file, "utf8");
  const specs = new Set();
  const local = (s) => s && !/^[a-z]+:/i.test(s) && !s.startsWith("/") && !s.startsWith("#");
  for (const m of text.matchAll(/(?:href|src)\s*=\s*["']([^"']+)["']/g))
    if (local(m[1])) specs.add(m[1]);
  for (const m of text.matchAll(/@import\s+["']([^"']+)["']/g)) if (local(m[1])) specs.add(m[1]);
  for (const m of text.matchAll(/url\(\s*["']?([^"')]+)["']?\s*\)/g))
    if (local(m[1])) specs.add(m[1]);
  return [...specs];
}

/* A drawing is compared against a build that will have moved on by the time
 * anybody reads the comparison, so the drawing is copied in beside the shots.
 * It is copied with what it reaches for: a `.dc.html` file carries no values of
 * its own, it links the design workspace's token sheet, and one cached without
 * that sheet renders as unstyled text and screenshots as garbage. */
function cacheSource(file) {
  const cache = join(SIDES.design.dir, "_source");
  rmSync(cache, { recursive: true, force: true });
  mkdirSync(cache, { recursive: true });

  const from = dirname(file);
  const missing = [];
  const copied = [];
  const copy = (spec) => {
    const src = join(from, spec);
    const dst = join(cache, spec);
    if (!existsSync(src) || !statSync(src).isFile()) return missing.push(spec);
    mkdirSync(dirname(dst), { recursive: true });
    copyFileSync(src, dst);
    copied.push(spec);
    if (/\.css$/i.test(src)) for (const nested of assetsOf(src)) copy(join(dirname(spec), nested));
  };

  copyFileSync(file, join(cache, basename(file)));
  for (const spec of assetsOf(file)) copy(spec);

  return { page: join(cache, basename(file)), cache, copied, missing };
}

const unmarkedLines = (frames) =>
  frames.map((f) => `  #${f.id}${f.heading ? `  ${f.heading}` : ""}`);

async function shootDesign(file, { suggest }) {
  if (!existsSync(file)) die(`No such drawing: ${file}`);

  const cached = cacheSource(resolve(file));
  console.log(`Cached ${basename(file)} to ${relative(root, cached.cache)}/`);
  if (cached.missing.length) {
    console.log(
      `\nIt links ${plural(cached.missing.length, "file that is", "files that are")} not beside it:`,
    );
    for (const m of cached.missing) console.log(`  ${m}`);
    console.log("Without its token sheet a drawing renders as unstyled text.");
  }

  const page = cached.page;
  const read = await browse({ page, capture: false, width: 1440, height: 1200 });
  const unmarked = read.frames.filter((f) => !f.marked);

  if (suggest) return propose(unmarked, resolve(file));

  /* Refusing is the enforcement. There is nothing on the design side that can
     be made to require a mark, and a drawing that cannot be paired blocks the
     implementation it was drawn for — so a partly-marked drawing is refused on
     the same terms as an unmarked one. Capturing what it can would report the
     unmarked frames as built-and-not-drawn, which is a different defect and a
     false one. */
  if (unmarked.length)
    die(
      read.marks.length
        ? `${basename(file)} is partly marked, and a partly marked drawing pairs partly.`
        : `${basename(file)} carries no data-shot, so nothing in it can be paired.`,
      "",
      `${plural(unmarked.length, "frame has", "frames have")} no mark:`,
      ...unmarkedLines(unmarked),
      "",
      "Run again with --suggest for a line to paste onto each one.",
    );

  const into = SIDES.design.dir;
  for (const png of Object.values(pngsIn(into))) rmSync(png, { force: true });
  const captured = await browse({ page, capture: true, into, width: 1440, height: 1200 });

  const rows = shotRows(captured);
  printShots(rows, ".shots/design/");

  manifest(join(into, "shots.json"), {
    tool: "shoot",
    side: "design",
    captured_at: now(),
    source: {
      kind: "drawing",
      name: basename(file),
      cached: relative(shots, cached.page),
      assets_cached: cached.copied,
      assets_missing: cached.missing,
    },
    shots: rows,
    page_errors: captured.failures,
  });
  console.log("\n.shots/design/shots.json — what was captured, for a caller");
}

const kebab = (s) =>
  s
    .split(/\s+—\s+/)[0]
    .replace(/([a-z0-9])([A-Z])/g, "$1-$2")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");

/** Print an attribute to paste, per unmarked frame. Marking is then typing. */
function propose(unmarked, page) {
  if (!unmarked.length) return console.log("\nEvery frame is marked. Nothing to suggest.");

  console.log(
    `\n${plural(unmarked.length, "frame has", "frames have")} no mark. Paste each attribute onto the element shown:\n`,
  );
  for (const f of unmarked) {
    const mark = kebab(f.heading || f.id);
    console.log(`  #${f.id}${f.heading ? `  ${f.heading}` : ""}`);
    if (!f.candidate) {
      console.log("      nothing inside it to mark — the frame is empty\n");
      continue;
    }
    const opening = f.candidate.opening;
    console.log(`      data-shot="${mark}"`);
    console.log(`      on  ${opening.slice(0, 96)}${opening.length > 96 ? "…" : ""}`);
    console.log(
      f.candidate.guessed
        ? "      guessed — no div in this frame paints var(--bg-base), so this is its biggest child\n"
        : "      the div that paints var(--bg-base), so it is the screen\n",
    );
  }
  console.log(`The drawing is ${page}`);
  console.log("Mark it, then run shoot --design again without --suggest.");
}

// --------------------------------------------------------------------- the sheet

async function sheet(against = DEFAULT_PAIR) {
  const pair = AGAINST[against];
  const { left, right } = pair;
  const sides = { [left]: sideShots(SIDES[left].dir), [right]: sideShots(SIDES[right].dir) };

  if (!Object.keys(sides[left]).length && !Object.keys(sides[right]).length)
    die(
      `Neither side of ${against} has been captured.`,
      "Capture them first: `pnpm shoot` for app, `pnpm shoot --bridge` for bridge, " +
        "`pnpm shoot --design <file>` for design.",
    );

  const states = [...new Set([...Object.keys(sides[left]), ...Object.keys(sides[right])])].sort();
  const rows = states.map((state) => {
    const at = (side) => sides[side][state] ?? null;
    const l = at(left);
    const r = at(right);
    const taller = l && r ? Math.max(l.height, r.height) : 0;
    const gap = l && r ? Math.abs(l.height - r.height) : 0;
    return {
      state,
      left: l,
      right: r,
      kind: l && r ? "paired" : l ? `${left}-only` : `${right}-only`,
      gap,
      fraction: taller ? gap / taller : 0,
      flagged: !!(l && r) && gap > HEIGHT_FLOOR && gap / taller > HEIGHT_TOLERANCE,
    };
  });

  writeSheetHtml(rows, left, right, pair.absent);
  const paired = rows.filter((r) => r.kind === "paired");
  if (paired.length) await writePairs(paired, left, right);

  const file = manifest(join(shots, "sheet.json"), {
    tool: "shoot",
    against,
    compared_at: now(),
    left: { side: left, label: SIDES[left].label },
    right: { side: right, label: SIDES[right].label },
    threshold: { height_fraction: HEIGHT_TOLERANCE, height_floor_css_px: HEIGHT_FLOOR },
    sheet: "sheet.html",
    summary: {
      paired: paired.length,
      [`${left}_only`]: rows.filter((r) => r.kind === `${left}-only`).length,
      [`${right}_only`]: rows.filter((r) => r.kind === `${right}-only`).length,
      flagged: rows.filter((r) => r.flagged).length,
    },
    states: rows.map((r) => ({
      state: r.state,
      kind: r.kind,
      [left]: r.left && { file: relative(shots, r.left.file), size_css_px: sizeOf(r.left) },
      [right]: r.right && { file: relative(shots, r.right.file), size_css_px: sizeOf(r.right) },
      pair: r.kind === "paired" ? `pairs/${r.state}.png` : null,
      height_gap_px: r.gap,
      height_gap_fraction: Number(r.fraction.toFixed(3)),
      flagged: r.flagged,
    })),
  });

  report(rows, left, right, file, pair);
}

/* CSS pixels, matching what the side manifests record. A PNG header would
   read double — capture is at two device pixels per CSS pixel. */
const sizeOf = (s) => ({ width: s.width, height: s.height });

const esc = (s) =>
  String(s).replace(
    /[&<>"]/g,
    (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c],
  );

const tokens = () => readFileSync(join(root, "packages/tokens/tokens.css"), "utf8");

const SHEET_CSS = `
body { background: var(--bg-base); color: var(--fg-default); font-family: var(--font-sans);
  font-size: var(--text-sm); line-height: var(--leading-sm); margin: 0; padding: var(--space-6); }
h1 { font-size: var(--text-xl); line-height: var(--leading-xl); font-weight: var(--weight-heading); margin: 0 0 var(--space-2); }
.lede { color: var(--fg-muted); margin: 0 0 var(--space-8); max-width: 74ch; }
.pair { margin-bottom: var(--space-10); }
.head { display: flex; align-items: baseline; flex-wrap: wrap; gap: var(--space-3); border-bottom: var(--border-width) solid var(--border-subtle); padding-bottom: var(--space-2); margin-bottom: var(--space-3); }
.head h2 { font-size: var(--text-lg); line-height: var(--leading-lg); font-weight: var(--weight-heading); margin: 0; }
.tag { font-family: var(--font-mono); font-size: var(--text-2xs); letter-spacing: var(--tracking-caps); text-transform: uppercase; padding: 2px 6px; border-radius: var(--radius-sm); background: var(--bg-sunken); color: var(--fg-muted); }
.tag[data-flag] { background: var(--status-failed); color: var(--bg-base); }
.tag[data-only] { background: var(--status-waiting); color: var(--bg-base); }
/* Equal width, always. A comparison where one side is wider than the other is
   a comparison of two different things. */
.cols { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: var(--space-4); align-items: start; }
.col { margin: 0; }
.col > figcaption { font-size: var(--text-2xs); text-transform: uppercase; letter-spacing: var(--tracking-caps); color: var(--fg-subtle); margin-bottom: var(--space-2); }
.col img { width: 100%; height: auto; display: block; border: var(--border-width) solid var(--border-default); border-radius: var(--radius-md); background: var(--bg-raised); }
.absent { border: var(--border-width) dashed var(--border-default); border-radius: var(--radius-md); padding: var(--space-6); color: var(--fg-muted); text-align: center; }
`;

function writeSheetHtml(rows, left, right, absent) {
  const cell = (side, shot) =>
    `<figure class="col"><figcaption>${esc(SIDES[side].label)}</figcaption>${
      shot
        ? `<img src="${esc(relative(shots, shot.file))}" alt="${esc(SIDES[side].label)}" width="${shot.width}" height="${shot.height}">`
        : `<div class="absent">${esc(absent[side])}</div>`
    }</figure>`;

  const body = rows
    .map(
      (r) => `<section class="pair" id="${esc(r.state)}">
  <div class="head">
    <h2>${esc(r.state)}</h2>
    ${r.kind === "paired" ? "" : `<span class="tag" data-only>${esc(r.kind)}</span>`}
    ${r.flagged ? `<span class="tag" data-flag>heights differ by ${Math.round(r.fraction * 100)}%</span>` : ""}
    <span class="tag">${r.left ? `${r.left.width}×${r.left.height}` : "—"} ${esc(left)}</span>
    <span class="tag">${r.right ? `${r.right.width}×${r.right.height}` : "—"} ${esc(right)}</span>
  </div>
  <div class="cols">
    ${cell(left, r.left)}
    ${cell(right, r.right)}
  </div>
</section>`,
    )
    .join("\n");

  const counts = ["paired", `${left}-only`, `${right}-only`]
    .map((k) => `${rows.filter((r) => r.kind === k).length} ${k}`)
    .join(" · ");

  writeFileSync(
    join(shots, "sheet.html"),
    `<!doctype html><meta charset="utf-8"><title>${esc(SIDES[left].label)} beside ${esc(SIDES[right].label)}</title>
<style>${tokens()}${SHEET_CSS}</style>
<h1>${esc(SIDES[left].label)} beside ${esc(SIDES[right].label)}</h1>
<p class="lede">Left is the ${esc(SIDES[left].label.toLowerCase())}, right is the ${esc(SIDES[right].label.toLowerCase())}, both at the same width. ${esc(counts)}. A state on one side only is the finding, not an omission from this page.</p>
${body}
`,
    "utf8",
  );
}

/* The sheet is for a person. This is for an agent: one PNG per state with both
 * halves in it, so the comparison can be held in a context window and
 * described. Composed by the same browser that took the shots — two <img> in a
 * grid, captured — rather than by an image library nothing else needs. */
async function writePairs(paired, left, right) {
  const into = join(shots, "pairs");
  rmSync(into, { recursive: true, force: true });
  mkdirSync(into, { recursive: true });

  const page = join(shots, ".run/pairs.html");
  mkdirSync(dirname(page), { recursive: true });
  writeFileSync(
    page,
    `<!doctype html><meta charset="utf-8">
<style>${tokens()}
body { margin: 0; background: var(--bg-base); font-family: var(--font-sans); }
.sheet { width: 1600px; padding: var(--space-4); box-sizing: border-box; }
.title { font-size: var(--text-sm); font-weight: var(--weight-heading); color: var(--fg-default); margin-bottom: var(--space-3); }
.cols { display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-4); align-items: start; }
figure { margin: 0; }
figcaption { font-size: var(--text-2xs); text-transform: uppercase; letter-spacing: var(--tracking-caps); color: var(--fg-subtle); margin-bottom: var(--space-2); }
img { width: 100%; height: auto; display: block; border: var(--border-width) solid var(--border-default); border-radius: var(--radius-sm); }
</style>
${paired
  .map(
    (r) => `<div class="sheet" data-shot="${esc(r.state)}">
  <div class="title">${esc(r.state)}</div>
  <div class="cols">
    <figure><figcaption>${esc(SIDES[left].label)}</figcaption><img src="${esc(r.left.file)}"></figure>
    <figure><figcaption>${esc(SIDES[right].label)}</figcaption><img src="${esc(r.right.file)}"></figure>
  </div>
</div>`,
  )
  .join("\n")}
`,
    "utf8",
  );

  await browse({ page, capture: true, into, width: 1640, height: 1200 });
}

function report(rows, left, right, manifestFile, pair) {
  console.log("\n.shots/sheet.html — one page, both halves, for a person");
  console.log(".shots/pairs/ — one PNG per paired state, for an agent");
  console.log(`${relative(root, manifestFile)} — the same comparison, for a caller\n`);

  const pad = Math.max(8, ...rows.map((r) => r.state.length));
  for (const r of rows) {
    const size = (s) => (s ? `${s.width}×${s.height}` : "—");
    const note = r.flagged
      ? `heights differ by ${Math.round(r.fraction * 100)}%`
      : r.kind === `${left}-only`
        ? pair.only.left
        : r.kind === `${right}-only`
          ? pair.only.right
          : "";
    console.log(
      `  ${r.kind === "paired" && !r.flagged ? " " : "!"} ${r.state.padEnd(pad)}  ` +
        `${size(r.left).padStart(10)} ${left}   ${size(r.right).padStart(10)} ${right}   ${note}`,
    );
  }

  const count = (k) => rows.filter((r) => r.kind === k).length;
  console.log(
    `\n${count("paired")} paired, ${count(`${left}-only`)} ${left}-only, ` +
      `${count(`${right}-only`)} ${right}-only, ${rows.filter((r) => r.flagged).length} flagged on height`,
  );
  console.log(
    `A height gap is flagged over ${HEIGHT_TOLERANCE * 100}% of the taller shot, and never under ${HEIGHT_FLOOR}px. Sizes are CSS pixels.`,
  );
  // The pairs are the point, so they are the last thing said and they are
  // openable. One line each rather than a directory to go rummage in.
  const paired = rows.filter((r) => r.kind === "paired");
  if (paired.length > 0) {
    console.log(`\n${plural(paired.length, "pair", "pairs")} to open:`);
    const w = Math.max(8, ...paired.map((r) => r.state.length));
    for (const r of paired) {
      console.log(`  ${r.state.padEnd(w)}  ${resolve(shots, "pairs", `${r.state}.png`)}`);
    }
  }
  console.log(`\nAll of them at once   ${resolve(shots, "sheet.html")}`);

  // Only a pair with an authority on the left has a blocking side. `app:bridge`
  // has neither half in charge: what it finds is drift between two renderings
  // of one screen, which is read rather than refused.
  const blocking = pair.blocking === null ? [] : rows.filter((r) => r.kind === `${left}-only`);
  if (blocking.length)
    console.log(
      `\n${pair.blocking}:${blocking.map((r) => `\n  ${r.state}`).join("")}\n` +
        "That is what this exists to find.",
    );
}

// ---------------------------------------------------------------------- the door

const argv = process.argv.slice(2);
if (argv.includes("--help") || argv.includes("-h")) {
  console.log(USAGE);
  process.exit(0);
}

const designAt = argv.indexOf("--design");
const design = designAt === -1 ? null : argv[designAt + 1];
if (designAt !== -1 && (!design || design.startsWith("--"))) die("--design needs a file.", "", USAGE);

mkdirSync(shots, { recursive: true });

try {
  if (argv.includes("--sheet")) {
    const named = argv[argv.indexOf("--sheet") + 1];
    const pair = named === undefined || named.startsWith("--") ? DEFAULT_PAIR : named;
    if (AGAINST[pair] === undefined)
      die(
        `--sheet ${pair} is not a pair this tool holds.`,
        `Pairs: ${Object.keys(AGAINST).join(", ")}.`,
      );
    await sheet(pair);
  }
  else if (design) await shootDesign(design, { suggest: argv.includes("--suggest") });
  else if (argv.includes("--bridge")) await shootBridge();
  else await shootApp();
} finally {
  rmSync(join(shots, ".run"), { recursive: true, force: true });
}
