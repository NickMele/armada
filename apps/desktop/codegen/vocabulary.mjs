// The vocabulary Bridge renders with, emitted from the files that own it.
//
// `crates/core-model/domain/enum-verbs.toml` is the authority on how a variant
// reads, and `protocol-version.toml` is the one number both sides check. Both
// are read here and written into one TypeScript module, because the alternative
// — a verb list, a glyph map or a version literal typed into a component — is
// the second vocabulary that drifted three times before it was deleted.
//
// This is a stopgap in the honest sense: `crates/ipc/src/lib.rs` says a codegen
// step emits TypeScript from the Rust types, and when that lands it should
// absorb this script rather than sit beside it. The precedent for a Node
// generator in this repository is `packages/components/gallery/build.mjs`.
//
// Run it with `pnpm --filter @armada/desktop codegen`. The output is checked in.

import { readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..", "..");
const VERBS = join(repo, "crates", "core-model", "domain", "enum-verbs.toml");
const FIELDS = join(repo, "crates", "core-model", "domain", "job-fields.toml");
const VERSION = join(repo, "protocol-version.toml");
const OUT = join(here, "..", "src", "shared", "generated", "vocabulary.ts");
// Its own file, and not a second export from the one above: the preload and the
// main process both read the version, and neither may pull a glyph — a React
// component — across the process boundary to get at a number.
const OUT_VERSION = join(here, "..", "src", "shared", "generated", "protocol-version.ts");

// The three vocabularies a Job list renders. The four verdict vocabularies in
// the same file are a step's and a criterion's, and neither is on this screen —
// emitting them would import glyphs no surface draws.
const WANTED = ["job_status", "queued_reason", "escalation_reason"];

// A glyph the registry names and this lucide-react version does not export.
// Carried as data rather than fixed by a rename, because the rename is the
// registry's to make: `packages/icons/icons.toml` decides what a glyph means,
// and a generator quietly substituting a different export would be deciding it
// here instead. The variant renders without a glyph and is counted in `GAPS`.
const NOT_EXPORTED = new Set(["file-question"]);

/// The one shape these files have: table headers, `key = value`, `#` comments,
/// no multi-line strings. The same line parser `xtask/src/rules_icons.rs`
/// justifies for the same reason — not general TOML, and it does not pretend to
/// be. A line it cannot read stops the build.
function tables(text, path) {
  const found = new Map();
  let current = null;
  text.split("\n").forEach((raw, i) => {
    const line = raw.trim();
    if (line === "" || line.startsWith("#")) return;
    if (line.startsWith("[")) {
      if (!line.endsWith("]")) throw new Error(`${path}:${i + 1} — unreadable table header`);
      current = line.slice(1, -1);
      found.set(current, {});
      return;
    }
    const split = line.indexOf("=");
    if (split < 0) throw new Error(`${path}:${i + 1} — neither a header nor a key`);
    const key = line.slice(0, split).trim();
    const value = line.slice(split + 1).trim();
    const table = current === null ? null : found.get(current);
    if (table === null) throw new Error(`${path}:${i + 1} — a key outside every table`);
    table[key] = value.startsWith('"') && value.endsWith('"') ? value.slice(1, -1) : value;
  });
  return found;
}

/** `user-check` becomes `UserCheck`, which is what lucide-react exports it as. */
function pascal(kebab) {
  return kebab
    .split("-")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join("");
}

/** A TypeScript string literal, or `null`. */
function quoted(value) {
  return value === null || value === "" ? "null" : JSON.stringify(value);
}

const verbs = tables(readFileSync(VERBS, "utf8"), "enum-verbs.toml");

// `protocol-version.toml` carries one key under no table, so it is read off the
// raw text rather than through a parser that would have to learn that shape.
const versionMatch = /^version\s*=\s*(\d+)\s*$/m.exec(readFileSync(VERSION, "utf8"));

// `job-fields.toml` carries multi-line strings the parser above deliberately
// does not read, and one value set the create form needs: the urgency row types
// itself `enum{normal, incident}`. Read exactly that, and stop if the row stops
// spelling its values — a picker that silently loses a variant is worse than a
// build that fails.
const urgencyRow = /\[fields\.urgency\][\s\S]*?type = "enum\{([^}]*)\}"/.exec(
  readFileSync(FIELDS, "utf8"),
);
if (urgencyRow === null) throw new Error("job-fields.toml — [fields.urgency] names no value set");
const urgencies = urgencyRow[1].split(",").map((value) => value.trim()).filter(Boolean);
if (!versionMatch) throw new Error("protocol-version.toml — no `version = <n>` line");

const vocabularies = new Map(WANTED.map((name) => [name, []]));
const gaps = [];
const glyphs = new Set();

for (const [header, table] of verbs) {
  const parts = header.split(".");
  if (parts[0] !== "verbs" || parts.length !== 3) continue;
  const [, vocabulary, variant] = parts;
  const rows = vocabularies.get(vocabulary);
  if (rows === undefined) continue;

  const verb = table.verb ?? "";
  const icon = table.icon ?? "";
  const token = table.status_token ?? "";
  const missing = [];
  if (verb === "") missing.push("verb");
  if (icon === "") missing.push("icon");
  else if (NOT_EXPORTED.has(icon)) missing.push("icon");
  if (token === "") missing.push("token");
  if (missing.length > 0) gaps.push({ vocabulary, variant, missing, icon });

  const usable = icon !== "" && !NOT_EXPORTED.has(icon);
  if (usable) glyphs.add(icon);
  // Badge takes the token stem and spells `--status-{stem}` and
  // `--status-{stem}-bg` itself, so the stem is derived from the token rather
  // than from the variant name — `queued` renders `--status-not-started`.
  const stem = token.startsWith("--status-") ? token.slice("--status-".length) : "";
  rows.push({ variant, verb, icon: usable ? icon : "", token, stem });
}

const imported = [...glyphs].sort().map(pascal);

const lines = [];
lines.push("// GENERATED by `pnpm --filter @armada/desktop codegen`. Do not hand-edit.");
lines.push("//");
lines.push("// The verb, the glyph and the status token each variant renders as, from");
lines.push("// `crates/core-model/domain/enum-verbs.toml`, and the protocol version from");
lines.push("// `protocol-version.toml`. Nothing here is written by hand, which is the point:");
lines.push("// a status label typed into a component is a second vocabulary.");
lines.push("//");
lines.push("// A `null` verb, glyph or token is a gap in the registry, not a default. Every");
lines.push("// one is listed in `GAPS` so a surface can say what it could not render instead");
lines.push("// of inventing copy for it.");
lines.push("");
lines.push(`import { ${imported.join(", ")} } from "lucide-react";`);
lines.push('import type { LucideIcon } from "lucide-react";');
lines.push("");
lines.push("/** How one variant reads. `null` where the registry carries no answer. */");
lines.push("export type Rendering = {");
lines.push("  readonly verb: string | null;");
lines.push("  readonly icon: LucideIcon | null;");
lines.push("  /** Badge's `status` prop: the token stem, without its `--status-` prefix. */");
lines.push("  readonly badgeStatus: string | null;");
lines.push("  readonly statusToken: string | null;");
lines.push("};");
lines.push("");

for (const [vocabulary, rows] of vocabularies) {
  const constant = vocabulary.toUpperCase();
  lines.push(`/** \`${vocabulary}\`, keyed by the wire value. */`);
    // `| undefined` is spelled out rather than left to `noUncheckedIndexedAccess`,
  // because a wire value this build has never heard of is a real answer and a
  // reader of this file should see that without checking a compiler flag.
lines.push(`export const ${constant}: Readonly<Record<string, Rendering | undefined>> = {`);
  for (const row of rows) {
    const icon = row.icon === "" ? "null" : pascal(row.icon);
    lines.push(
      `  ${JSON.stringify(row.variant)}: { verb: ${quoted(row.verb)}, icon: ${icon}, ` +
        `badgeStatus: ${quoted(row.stem)}, statusToken: ${quoted(row.token)} },`,
    );
  }
  lines.push("};");
  lines.push("");
}

lines.push("/** What a Job's urgency may be, from `job-fields.toml`. Not a scale. */");
lines.push(
  `export const URGENCIES: readonly string[] = [${urgencies.map((u) => JSON.stringify(u)).join(", ")}];`,
);
lines.push("");
lines.push("/** A variant the registry has no sanctioned copy, glyph or hue for. */");
lines.push("export type Gap = {");
lines.push("  readonly vocabulary: string;");
lines.push("  readonly variant: string;");
lines.push("  readonly missing: readonly string[];");
lines.push("};");
lines.push("");
lines.push("export const GAPS: readonly Gap[] = [");
for (const gap of gaps) {
  lines.push(
    `  { vocabulary: ${JSON.stringify(gap.vocabulary)}, variant: ${JSON.stringify(gap.variant)}, ` +
      `missing: [${gap.missing.map((m) => JSON.stringify(m)).join(", ")}] },`,
  );
}
lines.push("];");
lines.push("");

mkdirSync(dirname(OUT), { recursive: true });
writeFileSync(OUT, lines.join("\n"));
writeFileSync(
  OUT_VERSION,
  [
    "// GENERATED by `pnpm --filter @armada/desktop codegen`. Do not hand-edit.",
    "//",
    "// `protocol-version.toml` at the repository root is the one number both sides",
    "// check. `crates/ipc/build.rs` emits the Rust constant from it; this is the",
    "// TypeScript one. A hand-typed literal in Bridge is a second source of truth the",
    "// day the file changes.",
    "",
    `export const PROTOCOL_VERSION = ${versionMatch[1]};`,
    "",
  ].join("\n"),
);

const counted = gaps.map((gap) => `${gap.vocabulary}.${gap.variant} (${gap.missing.join(", ")})`);
process.stdout.write(
  `vocabulary.ts: ${[...vocabularies.values()].reduce((n, rows) => n + rows.length, 0)} variants, ` +
    `${gaps.length} with a gap\n`,
);
for (const line of counted) process.stdout.write(`  gap: ${line}\n`);
