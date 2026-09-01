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
 * A manifest per side, because a side is captured on its own and is worth
 * reading on its own.
 *
 * Nothing here knows what a browser is. That lives behind `browser.mjs`.
 *
 * Documented in `docs/practices/comparing-to-the-drawing.md`. */
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
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
  app: {
    dir: join(shots, "app"),
    label: "App",
    absent: "Not built. The drawing has a state the build does not.",
  },
};

const USAGE = `shoot — screenshot a screen and its drawing, and pair them

  pnpm shoot                        the app: build the gallery, capture every
                                    marked screen story to .shots/app/
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

// ---------------------------------------------------------------------- the door

const argv = process.argv.slice(2);
if (argv.includes("--help") || argv.includes("-h")) {
  console.log(USAGE);
  process.exit(0);
}

mkdirSync(shots, { recursive: true });

try {
  await shootApp();
} finally {
  rmSync(join(shots, ".run"), { recursive: true, force: true });
}
