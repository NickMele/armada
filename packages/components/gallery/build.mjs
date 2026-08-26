/* Builds gallery.html: every story, rendered once, with the token set and every
   component stylesheet inlined. Self-contained on purpose — it has to open on a
   phone with no server and no network. */
import { build } from "vite";
import { readFileSync, writeFileSync, readdirSync, mkdirSync } from "node:fs";
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

const css = [readFileSync(join(root, "../tokens/tokens.css"), "utf8")];
for (const dir of readdirSync(join(root, "src/primitives"))) {
  const f = join(root, "src/primitives", dir, `${dir}.css`);
  try { css.push(readFileSync(f, "utf8")); } catch {}
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
    .map((s) => `<figure><figcaption>${esc(s.name)}</figcaption><div class="stage">${s.html}</div></figure>`)
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
.stage { background: var(--bg-raised); border: var(--border-width) solid var(--border-default); border-radius: var(--radius-md); padding: var(--space-4); overflow: auto; position: relative; }
/* A dialog, sheet, toast or palette is position:fixed, which ignores every
   ancestor and covers the page. \`contain\` makes the stage a containing block
   for them, so each overlay renders inside its own box. Without it the command
   palette opens over everything and the page is unusable. */
.stage { contain: layout paint; }
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

console.log(`${groups.length} components, ${groups.reduce((n, g) => n + g.stories.length, 0)} stories`);
