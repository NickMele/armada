/* Builds screens.html: every Bridge screen, rendered once, with the token set
   and every component stylesheet inlined. Self-contained, like the gallery, and
   for the same reason — it has to open with no server and no network.

   **The stylesheet list is the app's own.** `packages/components/src/index.css`
   is the one list of component stylesheets and the app imports it, so reading
   it here makes this page show what Bridge shows. A second list is how a
   stylesheet goes unimported with every gate green; the gallery's build says so
   at length and this is the same rule, not a second one. */
import { build } from "vite";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { join, dirname } from "node:path";
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

/* An `@import` naming a file that is not there throws, rather than rendering a
   component with its class names and no rules behind them — which reads as a
   component drawn wrong instead of one never registered. */
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
  readFileSync(join(components, "../tokens/tokens.css"), "utf8"),
  flatten(join(components, "src/index.css")),
];

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
        `<figure data-shot="${s.mark}"><figcaption>${esc(s.name)}</figcaption><div class="stage">${s.html}</div></figure>`,
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
   **A clipped right edge is a finding, not an artifact.** The stage is the
   window, so a screen whose content will not shrink below the window is cut
   here exactly as it is cut there. Measured with width:fit-content, job
   detail's stopped render wants 1408px and its terminal ones 1322px; only the
   running and at-review renders come in at 1280. That is job detail's own
   minimum width and it is nothing to do with the header's acts — the running
   render carries two buttons and fits.
   No backticks in here: this block is inside the page's template literal. */
.stage { width: 1280px; min-height: 800px; overflow: hidden; background: var(--bg-base); }
</style>
${body}
`,
  "utf8",
);

const total = groups.reduce((n, g) => n + g.shots.length, 0);
console.log(`screens.html — ${total} screen${total === 1 ? "" : "s"} from ${groups.length} file(s)`);
