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
 * A manifest per side, because a side is captured on its own and is worth
 * reading on its own.
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

/* A side is a directory of PNGs named by state, and where the sheet will look
 * for them. Nothing that captures one knows about the other. */
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
};

const USAGE = `shoot — screenshot a screen and its drawing, and pair them

  pnpm shoot                        the app: build the gallery, capture every
                                    marked screen story to .shots/app/
  pnpm shoot --design <file.dc.html>
                                    a drawing: capture every [data-shot] frame
                                    to .shots/design/, and cache the source
  pnpm shoot --design <file> --suggest
                                    propose a mark for each unmarked frame
                                    instead of refusing
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

const printShots = (rows, where) => {
  console.log(`\n${plural(rows.length, "state", "states")} captured to ${where}`);
  const pad = Math.max(8, ...rows.map((r) => r.state.length));
  for (const r of rows) console.log(`  ${r.state.padEnd(pad)}  ${r.css.width}×${r.css.height}`);
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
  if (design) await shootDesign(design, { suggest: argv.includes("--suggest") });
  else await shootApp();
} finally {
  rmSync(join(shots, ".run"), { recursive: true, force: true });
}
