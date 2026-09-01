/* Builds gallery.html: every story, rendered once, with the token set and every
   component stylesheet inlined. Self-contained on purpose — it has to open on a
   phone with no server and no network. */
import { build } from "vite";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");

await build({
  root,
  configFile: false,
  // esbuild handles TSX directly; the react plugin exists for fast refresh,
  // which a one-shot static render has no use for.
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

// `dist` is on the gate's skip list. This file inlines the whole token set, so
// every hex in it reads as an off-contract literal to a rule that cannot tell
// generated output from source — the same way `storybook-static` did.
mkdirSync(join(here, "dist"), { recursive: true });

/* Exactly what the app loads, flattened. `src/index.css` is the app's one list
   of component stylesheets, and Storybook loads that same file rather than a
   rule per story, so reading it here makes the gallery show what the app shows
   and nothing else.

   It used to `readdirSync` three named trees instead, and its own comment
   admitted the flaw: a tree left off that list did not fail the build, it
   rendered unstyled and the gallery said nothing. That was a second answer to
   "which stylesheets are there", and a second answer is how a stylesheet goes
   unimported with every gate green. There is one answer now — a line missing
   from `index.css` is missing from the app too, where a gate rule can see it.

   An `@import` naming a file that is not there throws. In the app that is a
   build error; here it would otherwise be a component rendered with its class
   names and no rules behind them, which reads as a component drawn wrong
   rather than one never registered. */
function flatten(file, seen = new Set()) {
  if (seen.has(file)) return "";
  seen.add(file);
  const dir = dirname(file);
  return readFileSync(file, "utf8").replace(/@import\s+["']([^"']+)["']\s*;/g, (_, spec) => {
    const target = join(dir, spec);
    try {
      readFileSync(target);
    } catch {
      throw new Error(`${file} imports ${spec}, which does not exist`);
    }
    return flatten(target, seen);
  });
}

const css = [
  readFileSync(join(root, "../tokens/tokens.css"), "utf8"),
  flatten(join(root, "src/index.css")),
];

/* The mark `shoot` captures by, stamped on a screen's story and on nothing
   else. A component is not a screen and has no frame in a drawing to pair
   with, so stamping one would invent a state no drawing can answer.

   `WaitingOnYou` becomes `waiting-on-you`, and a drawing whose frame carries
   `data-shot="waiting-on-you"` is drawing the same state. */
const kebab = (s) =>
  s
    .replace(/([a-z0-9])([A-Z])/g, "$1-$2")
    .replace(/([A-Z]+)([A-Z][a-z])/g, "$1-$2")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");

/* A screen's own mark, used only to break a tie. A screen title is a headline
   and a subtitle joined by an em dash — "The list — six states, one row
   shape" — and the headline alone is what a person calls the screen. */
const screenMark = (title) => kebab(title.split("/").pop().split(/\s+—\s+/)[0]);

const screens = groups.filter((g) => g.title.startsWith("Screens/"));

/* Two screens each export a story called `Running`, and one flat mark cannot
   hold both — one PNG would overwrite the other and the overwrite would be
   silent. A mark claimed once stays bare; a mark claimed twice is qualified by
   its screen on both sides, so neither wins by accident and the pairing can
   name the two screens that collided. */
const claimed = new Map();
for (const g of screens)
  for (const s of g.stories) claimed.set(kebab(s.key), (claimed.get(kebab(s.key)) ?? 0) + 1);

const marks = new Map();
for (const g of screens)
  for (const s of g.stories) {
    const bare = kebab(s.key);
    marks.set(s, claimed.get(bare) > 1 ? `${screenMark(g.title)}-${bare}` : bare);
  }

const esc = (s) => s.replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]);

const nav = groups
  .map((g) => `<a href="#${encodeURIComponent(g.title)}">${esc(g.title.split("/").pop())}</a>`)
  .join("");

const body = groups
  .map(
    (g) => `<section id="${encodeURIComponent(g.title)}">
  <h2>${esc(g.title.split("/").pop())}</h2>
  ${g.stories
    .map(
      (s) =>
        `<figure${marks.has(s) ? ` data-shot="${marks.get(s)}"` : ""}><figcaption>${esc(s.name)}</figcaption><div class="stage">${s.html}</div></figure>`,
    )
    .join("\n  ")}
</section>`,
  )
  .join("\n");

writeFileSync(
  join(here, "dist/gallery.html"),
  `<title>Armada Components</title>
<style>
${css.join("\n")}
body { background: var(--bg-base); color: var(--fg-default); font-family: var(--font-sans); font-size: var(--text-sm); line-height: var(--leading-sm); margin: 0; padding: var(--space-6) var(--space-4) var(--space-12); }
h1 { font-size: var(--text-xl); line-height: var(--leading-xl); font-weight: var(--weight-heading); margin: 0 0 var(--space-2); }
.lede { color: var(--fg-muted); margin: 0 0 var(--space-6); max-width: 60ch; }
nav { display: flex; flex-wrap: wrap; gap: var(--space-2); margin-bottom: var(--space-8); }
nav a { color: var(--fg-muted); text-decoration: none; font-size: var(--text-xs); border: var(--border-width) solid var(--border-subtle); border-radius: var(--radius-sm); padding: var(--space-1) var(--space-2); }
nav a:hover { color: var(--accent); border-color: var(--accent); }
section { margin-bottom: var(--space-12); }
h2 { font-size: var(--text-lg); font-weight: var(--weight-heading); margin: 0 0 var(--space-4); padding-bottom: var(--space-2); border-bottom: var(--border-width) solid var(--border-subtle); }
figure { margin: 0 0 var(--space-4); }
figcaption { font-size: var(--text-2xs); text-transform: uppercase; letter-spacing: var(--tracking-caps); color: var(--fg-subtle); margin-bottom: var(--space-2); }
.stage { background: var(--bg-raised); border: var(--border-width) solid var(--border-default); border-radius: var(--radius-md); padding: var(--space-4); position: relative; }
/* No overflow on the stage. A menu, a popover and a tooltip all resolve inside
   it and reach past its edge on purpose, and any overflow value but visible
   clips them — including \`overflow-x\` alone, which forces the other axis to
   auto. Wide content scrolls with the page instead. */
.stage { overflow: visible; }
/* A dialog, sheet, toast or palette is position:fixed, which ignores every
   ancestor and covers the page. Layout containment makes the stage a containing
   block for them, so each overlay renders inside its own box.
   Not \`paint\`: paint containment also clips to the padding box, which cut the
   dropdowns off inside their card and squashed the dialogs. Containing where an
   overlay resolves against and clipping what escapes are two different things,
   and only the first is wanted here. */
.stage { contain: layout; }
/* An overlay resolving against the stage needs the stage to have room. */
.stage:has([class*="scrim"]), .stage:has([class*="-layer"]), .stage:has([class*="-region"]) { min-height: 320px; }
.stage > * { max-width: 100%; }
/* The scrim is a full-viewport wash; inside a stage it should tint the stage. */
.stage [style*="position:fixed"], .stage [class*="scrim"], .stage [class*="backdrop"] { position: absolute; }
</style>
<h1>Armada components</h1>
<p class="lede">Every story, rendered from the same modules the app imports. Resting states only &mdash; hover and focus need a pointer.</p>
<nav>${nav}</nav>
${body}
`,
  "utf8",
);

console.log(
  `${groups.length} components, ${groups.reduce((n, g) => n + g.stories.length, 0)} stories, ` +
    `${marks.size} marked for shoot`,
);
