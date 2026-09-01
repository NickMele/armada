/* Builds screens.html: every Bridge screen, rendered once, with the stylesheet
   Bridge itself ships. Self-contained, like the gallery, and for the same
   reason — it has to open with no server and no network.

   **The stylesheet is the app's compiled one, read out of its build.** Not a
   list assembled here: the renderer imports `tokens.css`, then `tailwindcss`,
   then the theme, then `base.css`, then the components — and only the first and
   the last of those can be read off disk. Following the components alone
   produced a page with no `box-sizing: border-box`, because that arrives with
   Tailwind's preflight; every screen then measured its own padding on top of
   `width: 100%` and overflowed by exactly the padding and the border. A shot of
   a layout the app does not have is worse than no shot, and it read as a defect
   in the app for the better part of an afternoon.

   So this builds the renderer and inlines what it emitted. The build is the
   cost of being right about what Bridge looks like. */
import { build } from "vite";
import { readFileSync, readdirSync, writeFileSync, mkdirSync, rmSync } from "node:fs";
import { spawn } from "node:child_process";
import { join, dirname, relative } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const app = join(here, "..");
const components = join(app, "../../packages/components");

await build({
  root: app,
  configFile: false,
  esbuild: { jsx: "automatic" },
  logLevel: "warn",
  build: {
    ssr: join(here, "render.tsx"),
    outDir: join(here, ".out"),
    emptyOutDir: true,
    rollupOptions: { output: { format: "es", entryFileNames: "render.mjs" } },
  },
});

const { collect } = await import(join(here, ".out/render.mjs"));
const groups = collect();

if (!groups.length) {
  console.error("No *.screens.tsx exported a title and a screens array. Nothing to build.");
  process.exit(1);
}

/* The renderer, built, so its stylesheet exists to be read. Spawned rather than
   imported: `electron-vite` owns the config that knows about Tailwind, and a
   second config here would be a second answer to what Bridge's CSS is. */
const built = await new Promise((done) =>
  spawn("pnpm", ["--filter", "@armada/desktop", "build"], { stdio: "inherit", cwd: app }).on(
    "exit",
    done,
  ),
);
if (built !== 0) {
  console.error("The renderer did not build, so there is no stylesheet to read.");
  process.exit(1);
}

const assets = join(app, "out/renderer/assets");
const sheets = readdirSync(assets).filter((f) => f.endsWith(".css"));
if (sheets.length !== 1) {
  console.error(`Expected one stylesheet in ${assets}, found ${sheets.length}.`);
  process.exit(1);
}
const css = [readFileSync(join(assets, sheets[0]), "utf8")];

const esc = (s) => s.replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]);

/* One mark, one shot. Two screens claiming the same mark would overwrite one
   another's PNG silently, which is the failure the gallery qualifies its marks
   to avoid — here the marks are written by hand, so the collision is a mistake
   to report rather than a case to handle. */
const seen = new Map();
for (const g of groups)
  for (const s of g.shots) {
    const first = seen.get(s.mark);
    if (first !== undefined) {
      console.error(`Two screens carry data-shot="${s.mark}" — ${first} and ${g.title}.`);
      process.exit(1);
    }
    seen.set(s.mark, g.title);
  }

const body = groups
  .map(
    (g) => `<section id="${encodeURIComponent(g.title)}">
  <h2>${esc(g.title)}</h2>
  ${g.shots
    .map(
      (s) =>
        `<figure data-shot="${s.mark}"><figcaption>${esc(s.name)}</figcaption><div class="stage"${
          s.width === undefined ? "" : ` style="width:${s.width}px"`
        }>${s.html}</div></figure>`,
    )
    .join("\n  ")}
</section>`,
  )
  .join("\n");

mkdirSync(join(here, "dist"), { recursive: true });
writeFileSync(
  join(here, "dist/screens.html"),
  `<title>Bridge Screens</title>
<style>
${css.join("\n")}
/* The app's stylesheet ends with window rules — html, body and #root at
   height 100% with overflow hidden, because Bridge is a window and does not
   scroll. This page is a document of many screens, and under those rules every
   figure past the first viewport was laid out, clipped, and captured blank.
   Restated here rather than filtered out of the sheet: the app's rules are
   right for the app, and this is the one place they do not apply. */
html, body { height: auto; overflow: visible; }
body { background: var(--bg-base); color: var(--fg-default); font-family: var(--font-sans); font-size: var(--text-sm); line-height: var(--leading-sm); margin: 0; padding: var(--space-6) var(--space-4) var(--space-12); }
section { margin-bottom: var(--space-12); }
h2 { font-size: var(--text-lg); font-weight: var(--weight-heading); margin: 0 0 var(--space-4); padding-bottom: var(--space-2); border-bottom: var(--border-width) solid var(--border-subtle); }
figure { margin: 0 0 var(--space-6); }
figcaption { color: var(--fg-muted); font-size: var(--text-xs); margin-bottom: var(--space-2); }
/* The stage is the captured box, and it is the size of Bridge's own window:
   main opens 1280x800, and a screen shot at some other width is a screen whose
   layout nobody is looking at. The height is a floor rather than a fix — a
   screen taller than the window is captured whole, which is the part a person
   would have to scroll to.
   **A clipped right edge is worth measuring before it is reported.** The first
   version of this page cut every screen at 1322px against a 1280px stage, and
   that was this file's missing preflight rather than anything about job detail.
   The rule the episode leaves: the stage is the window, so a cut edge is a
   claim about the app — check the page loads what the app loads before making
   it.
   No backticks in here: this block is inside the page's template literal. */
.stage { width: 1280px; min-height: 800px; overflow: hidden; background: var(--bg-base); }
</style>
${body}
`,
  "utf8",
);

/* The snapshots, which are the part that survives the run.
 *
 * **`.shots/` is ignored, so the PNGs are not a baseline.** Nothing in a diff
 * says a screen changed; you have to have chosen to look, which is the same
 * problem the tool was built to fix one level down. The markup is the cheap
 * half of a shot and it is text, so it is checked in and read in review: a
 * header rebuilt shows up as a changed snapshot on the PR whether or not
 * anybody ran this.
 *
 * It is not a substitute for looking. Markup says what is there; only the
 * image says whether it is drawn right. */
const snaps = join(here, "snapshots");
mkdirSync(snaps, { recursive: true });

const written = new Map();
for (const g of groups)
  for (const s of g.shots)
    written.set(s.mark, `<!-- ${g.title} — ${s.name} — render: ${s.render} -->\n${s.html}\n`);

const checking = process.argv.includes("--check");
const stale = [];
const gone = readdirSync(snaps).filter((f) => f.endsWith(".html") && !written.has(f.slice(0, -5)));

for (const [mark, body] of written) {
  const file = join(snaps, `${mark}.html`);
  let held = null;
  try {
    held = readFileSync(file, "utf8");
  } catch {
    held = null;
  }
  if (held === body) continue;
  if (checking) stale.push(held === null ? `${mark} — no snapshot` : `${mark} — changed`);
  else writeFileSync(file, body, "utf8");
}

if (!checking) for (const f of gone) rmSync(join(snaps, f));

if (checking && (stale.length || gone.length)) {
  console.error("The screens do not match their snapshots:");
  for (const line of stale) console.error(`  ${line}`);
  for (const f of gone) console.error(`  ${f.slice(0, -5)} — a snapshot for a screen that is gone`);
  console.error("\nRun `pnpm shoot --bridge` to bring them up to date, then look at the images.");
  process.exit(1);
}

const total = groups.reduce((n, g) => n + g.shots.length, 0);
console.log(
  `screens.html — ${total} screen${total === 1 ? "" : "s"} from ${groups.length} file(s)` +
    (checking ? ", snapshots current" : `, snapshots in ${relative(app, snaps)}`),
);
