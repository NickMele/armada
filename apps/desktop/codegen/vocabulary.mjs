// The vocabulary Bridge renders with, emitted from the files that own it.
//
// `crates/core-model/domain/enum-verbs.toml` is the authority on how a variant
// reads, `job-statuses.toml` on whether a Job is over, what it is doing and who
// it waits on, `actions.toml` on what every act is called and bound to, and
// `protocol-version.toml` on what each side of the wire speaks. All of them are
// read here and written into TypeScript, because the alternative — a verb list,
// a glyph map or a version literal typed into a component — is the second
// vocabulary that drifted three times before it was deleted.
//
// **`actions.toml` is here because it had been transcribed by hand three
// times** — into the contract's key map, into `packages/screens/src/keys.ts`,
// and into a 592-line `packages/components/src/actions.ts` that landed and was
// deleted the same week. A registry with three copies has three answers the
// day one of them is edited alone. The contract's copy stays and is held to the
// registry by `xtask/src/rules_actions.rs`; the two TypeScript copies are now
// this file's output.
//
// This is a stopgap in the honest sense: `crates/ipc/src/lib.rs` says a codegen
// step emits TypeScript from the Rust types, and when that lands it should
// absorb this script rather than sit beside it. The precedent for a Node
// generator in this repository is `packages/components/gallery/build.mjs`.
//
// Run it with `pnpm --filter @armada/desktop codegen`. The output is checked in,
// and `cargo xtask verify-foundations` compares it against a fresh run through
// `--emit` — see the bottom of this file, and `xtask/src/rules_vocabulary.rs`.

import { readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..", "..");
const VERBS = join(repo, "crates", "core-model", "domain", "enum-verbs.toml");
const FIELDS = join(repo, "crates", "core-model", "domain", "job-fields.toml");
const STATUSES = join(repo, "crates", "core-model", "domain", "job-statuses.toml");
const OUTCOMES = join(repo, "crates", "core-model", "domain", "check-outcomes.toml");
const ACTS = join(repo, "crates", "core-model", "domain", "actions.toml");
const VERSION = join(repo, "protocol-version.toml");
// The vocabulary is a rendering and not a wire type: it carries the glyph each
// variant draws as, so it imports `lucide-react`. It belongs with the
// components that render it, and `@armada/protocol` stays dependency-free.
const OUT = join(repo, "packages", "components", "src", "generated", "vocabulary.ts");
// Its own file, and not a second export from the one above: the preload and the
// main process both read the version, and neither may pull a glyph — a React
// component — across the process boundary to get at a number.
const OUT_VERSION = join(repo, "packages", "protocol", "src", "generated", "protocol-version.ts");
// Beside the vocabulary and not inside it: a variant is a state a Job is in and
// an act is something a person does to one, and the two are separate registries
// with separate gates. They share a destination for the same reason the
// vocabulary is there at all — an act draws a glyph, and a glyph is React.
const OUT_ACTIONS = join(repo, "packages", "components", "src", "generated", "actions.ts");

// The vocabularies a surface renders. `criterion_verdict_attested` is left out
// because nothing serves an attestation.
//
// `criterion_verdict_judge` is here because job detail draws a Judge's citation
// beneath the step it judged. Both its rows carry a verb and a glyph and
// neither carries a status token: the hue is `--verdict-met` and
// `--verdict-not-met`, which are their own axis in `tokens/status.css` and not
// a Job status. So both land in `GAPS` as missing a token, which is accurate —
// there is no `--status-` stem to give `Badge`, and a criterion row is not a
// badge.
//
// `criterion_verdict_check` is here for one variant. The rail says `not reached`
// beside a declared Check the gate has not run, and `check_outcome` has no such
// row — its five are what a Check that *ran* did. `icons.toml` already calls
// "not reached" a Check state and reserves `shield-minus` to it, so the word and
// the glyph are the registry's. Reported: the variant belongs under
// `check_outcome` too.
//
// `step_state` and `advance_gate` were wanted here before either had a row, on
// purpose: asking for a vocabulary is what puts the gap in this script's own
// output — `advance_gate (no rows in enum-verbs.toml)` — rather than leaving a
// renderer to write the missing words itself. Both have rows now, and the
// mechanism is why the omission was visible for long enough to fix.
//
// The five below them arrived the same way and are drawn on four surfaces
// nobody had a word for: the changed-files list, the evidence trail, the
// transition history's kind and subject columns, and the observe pane's note
// when the transcript socket closes. Each was rendering the wire spelling in
// mono at a person.
const WANTED = [
  "job_status",
  "queued_reason",
  "admission_hold",
  "resumption",
  "escalation_reason",
  "check_outcome",
  "criterion_verdict_check",
  "criterion_verdict_judge",
  "step_state",
  "advance_gate",
  "gaming_pattern",
  "evidence_type",
  "change_kind",
  "movement_kind",
  "drone_presence",
  "silence",
];

// `admission_hold` is here because the status bar says which of the four things
// is holding the next drone back, and every one of them folds to
// `queued_reason.waiting_on_resources` on a Board row. Its four rows carry a
// verb and a token and no glyph, so all four land in `GAPS` as missing one —
// which is accurate: the status bar carries no icons, and `cpu` is reserved to
// `queued_reason` in `packages/icons/icons.toml`. Nothing renders a glyph for
// these today and nothing should invent one.
//
// **It is the one vocabulary Fleet may widen without a protocol bump.** The map
// below is keyed by the wire value with an `undefined` answer for a key this
// build has never heard of, which is exactly what makes that safe — see
// `crates/ipc/src/capacity.rs`.

// `resumption` is here because a queued row says which act a person took to put
// it back, and that word is the registry's. Its three rows carry a verb and a
// token and no glyph, which is accurate rather than a gap left open: the verb
// renders as a suffix on the row's headline — where a recurrence count already
// goes, per `Job row (stacked)`'s own story — and a headline carries no glyphs.

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
    table[key] = unquoted(value);
  });
  return found;
}

/// A basic string, a literal string, or a bare token left as it is.
///
/// **The literal form is read because one binding needs it.** `⌘\` is Toggle
/// sidebar's, and a trailing backslash cannot be written in a TOML basic string
/// without escaping — so `actions.toml` spells that one row `'⌘\'`. Every other
/// value in every registry this script reads is double-quoted, which is why
/// adding this changed nothing already emitted.
function unquoted(value) {
  for (const quote of ['"', "'"]) {
    if (value.length >= 2 && value.startsWith(quote) && value.endsWith(quote)) {
      return value.slice(1, -1);
    }
  }
  return value;
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

// `protocol-version.toml` carries its two keys under no table, so they are read
// off the raw text rather than through a parser that would have to learn that
// shape. Both or neither: half a version cannot be compared.
const versionText = readFileSync(VERSION, "utf8");
const majorMatch = /^major\s*=\s*(\d+)\s*$/m.exec(versionText);
const minorMatch = /^minor\s*=\s*(\d+)\s*$/m.exec(versionText);

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
if (!majorMatch) throw new Error("protocol-version.toml — no `major = <n>` line");
if (!minorMatch) throw new Error("protocol-version.toml — no `minor = <n>` line");

// `terminal`, `mode` and `who_is_acting` per Job status. Read with a scan of its
// own rather than through `tables` above, because job-statuses.toml carries
// arrays and prose that parser deliberately does not read — and a surface
// choosing which detail screen a Job takes needs to know whether the Job is
// over, not how it reads.
//
// **`who_is_acting` is emitted because a surface groups statuses by it.** The
// Board's `Needs you` tab is defined as "the statuses that stop until a person
// reads them", which is this field and nothing else. Without it Bridge reached
// that set by subtracting the ones it could name — membership derived from the
// absence of a rule rather than from the rule, which drifts silently the first
// time a status is added.
const ACTORS = new Set(["Person", "Drone", "None"]);
const lifecycles = [];
{
  let current = null;
  for (const raw of readFileSync(STATUSES, "utf8").split("\n")) {
    const line = raw.trim();
    const header = /^\[statuses\.([a-z_]+)\]$/.exec(line);
    if (header !== null) {
      current = { status: header[1], terminal: null, mode: null, acting: null };
      lifecycles.push(current);
      continue;
    }
    if (current === null) continue;
    const terminal = /^terminal = (true|false)$/.exec(line);
    if (terminal !== null) current.terminal = terminal[1] === "true";
    const mode = /^mode = "([^"]*)"$/.exec(line);
    if (mode !== null) current.mode = mode[1];
    const acting = /^who_is_acting = "([^"]*)"$/.exec(line);
    if (acting !== null) current.acting = acting[1];
  }
}
for (const row of lifecycles) {
  if (row.terminal === null || row.mode === null || row.acting === null) {
    throw new Error(
      `job-statuses.toml — [statuses.${row.status}] names no terminal, no mode or no who_is_acting`,
    );
  }
  // The declared domain, from the field list at the head of job-statuses.toml.
  // A fourth value means the registry moved, and stopping here is better than
  // emitting a string every reader would fall through: the Board would put the
  // status under no tab at all and nobody would be told why.
  if (!ACTORS.has(row.acting)) {
    throw new Error(
      `job-statuses.toml — [statuses.${row.status}] has who_is_acting = "${row.acting}", ` +
        `and the declared domain is Person | Drone | None`,
    );
  }
}

// `advances` per Check outcome, from the registry that owns the set.
//
// Emitted because a surface has to tell a Check that stopped a step from one
// that did not, and the two are not the same question as passed. `skipped`
// advances and is not a pass; `never_ran` neither advances nor passes, and both
// carry --status-not-started — so a rule read off the status token gets one of
// them wrong. The registry already answers it; Bridge reads that answer rather
// than writing a second one.
const advances = new Map();
{
  let current = null;
  for (const raw of readFileSync(OUTCOMES, "utf8").split("\n")) {
    const line = raw.trim();
    const header = /^\[outcomes\.([a-z_]+)\]$/.exec(line);
    if (header !== null) {
      current = header[1];
      advances.set(current, null);
      continue;
    }
    if (current === null) continue;
    const flag = /^advances = (true|false)$/.exec(line);
    if (flag !== null) advances.set(current, flag[1] === "true");
  }
}
for (const [outcome, flag] of advances) {
  if (flag === null) throw new Error(`check-outcomes.toml — [outcomes.${outcome}] names no advances`);
}

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
lines.push("// `crates/core-model/domain/enum-verbs.toml`; whether a status is terminal,");
lines.push("// what it is doing and who it waits on, from `job-statuses.toml`; whether a");
lines.push("// Check outcome advances a step, from `check-outcomes.toml`; and the");
lines.push("// protocol version from `protocol-version.toml`. Nothing here is written by");
lines.push("// hand, which is the point: a status label typed into a component is a");
lines.push("// second vocabulary.");
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
  if (rows.length === 0) {
    lines.push(`/** \`${vocabulary}\` — **no rows in \`enum-verbs.toml\`.** Every variant is a gap. */`);
    lines.push(`export const ${constant}: Readonly<Record<string, Rendering | undefined>> = {};`);
    lines.push("");
    continue;
  }
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

// The two files have to name the same twelve statuses. They are separate
// registries and nothing else joins them, so a status added to one and not the
// other would render with a verb and no lifecycle, or the reverse.
const rendered = new Set((vocabularies.get("job_status") ?? []).map((row) => row.variant));
const lived = new Set(lifecycles.map((row) => row.status));
for (const status of rendered) {
  if (!lived.has(status)) throw new Error(`job-statuses.toml has no [statuses.${status}]`);
}
for (const status of lived) {
  if (!rendered.has(status)) throw new Error(`enum-verbs.toml has no [verbs.job_status.${status}]`);
}

// The same join `job_status` gets, for the same reason: an outcome in one
// registry and not the other renders with a verb and no rule, or the reverse.
const drawn = new Set((vocabularies.get("check_outcome") ?? []).map((row) => row.variant));
for (const outcome of drawn) {
  if (!advances.has(outcome)) throw new Error(`check-outcomes.toml has no [outcomes.${outcome}]`);
}
for (const outcome of advances.keys()) {
  if (!drawn.has(outcome)) {
    throw new Error(`enum-verbs.toml has no [verbs.check_outcome.${outcome}]`);
  }
}

lines.push("/** Where a Job is in its life, from `job-statuses.toml`. Not a rendering. */");
lines.push("export type Lifecycle = {");
lines.push("  /** Whether the Job is over here. */");
lines.push("  readonly terminal: boolean;");
lines.push("  /** `Working`, `Waited on` or `N/A` — what the Job is doing. */");
lines.push("  readonly mode: string;");
lines.push("  /**");
lines.push("   * `Person`, `Drone` or `None` — who the Job is waiting on.");
lines.push("   *");
lines.push("   * **Not the same question as `mode`.** `piloted` is `Working` and its");
lines.push("   * actor is a person; `queued` is `Waited on` and its actor is a drone. A");
lines.push("   * surface grouping the statuses that stop until somebody reads them wants");
lines.push("   * this field, and answers wrongly with either half of it alone.");
lines.push("   */");
lines.push("  readonly whoIsActing: string;");
lines.push("};");
lines.push("");
lines.push("/** `job_status`, keyed by the wire value. What the status *is*, not how it reads. */");
lines.push("export const JOB_LIFECYCLE: Readonly<Record<string, Lifecycle | undefined>> = {");
for (const row of lifecycles) {
  lines.push(
    `  ${JSON.stringify(row.status)}: { terminal: ${row.terminal}, ` +
      `mode: ${JSON.stringify(row.mode)}, whoIsActing: ${JSON.stringify(row.acting)} },`,
  );
}
lines.push("};");
lines.push("");
lines.push("/**");
lines.push(" * Whether a step may advance past one Check outcome, from");
lines.push(" * `check-outcomes.toml`. **Not the same question as passed** — `skipped`");
lines.push(" * advances and measured nothing, and a surface that read the status token");
lines.push(" * instead would draw it identically to `never_ran`, which is a failure.");
lines.push(" */");
lines.push("export const CHECK_ADVANCES: Readonly<Record<string, boolean | undefined>> = {");
for (const [outcome, flag] of advances) {
  lines.push(`  ${JSON.stringify(outcome)}: ${flag},`);
}
lines.push("};");
lines.push("");
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

const vocabularyModule = lines.join("\n");

// ------------------------------------------------------------------ acts
//
// Every act Bridge offers, from `crates/core-model/domain/actions.toml`.
//
// **This generator validates and does not decide.** `xtask/src/rules_actions.rs`
// is the gate on the registry itself — it holds the glyph column to
// `packages/icons/icons.toml`, the whole map to the contract's key blocks, and
// the safety rules to the QWERTY layout. What is checked here is narrower and
// is the emitter's own business: that a row says something this script can turn
// into TypeScript without guessing. A row it cannot read stops the build rather
// than reaching a surface as a blank, because a palette entry with no verb is a
// row a person presses and learns nothing from.

const ACTION_KINDS = new Set(["Action", "Motion"]);
const ACTION_TIERS = new Set(["Global", "Contextual"]);
// Why a glyph may be missing. The registry's two words, and there is no third:
// `undecided` is a gap the icon registry has to close, `by design` is a
// document having closed it with "none". A surface says which it is drawing.
const ABSENCES = new Set(["undecided", "by design"]);

const acts = [];
for (const [header, table] of tables(readFileSync(ACTS, "utf8"), "actions.toml")) {
  const parts = header.split(".");
  if (parts[0] !== "actions" || parts.length !== 2) {
    throw new Error(`actions.toml — [${header}] is not an [actions.<id>] table`);
  }
  const id = parts[1];
  const where = `actions.toml — [actions.${id}]`;
  const kind = table.kind ?? "";
  const tier = table.tier ?? "";
  const verb = table.verb ?? "";
  const icon = table.icon ?? "";
  const absent = table.icon_absent ?? "";
  const shortcut = table.shortcut ?? "";
  const scope = table.scope ?? "";
  const unbuilt = table.unbuilt ?? "";

  if (!ACTION_KINDS.has(kind)) throw new Error(`${where} — kind is "${kind}", not Action or Motion`);
  if (!ACTION_TIERS.has(tier)) throw new Error(`${where} — tier is "${tier}", not Global or Contextual`);
  if (verb === "") throw new Error(`${where} — no verb, and the palette draws the verb`);
  if (shortcut === "") throw new Error(`${where} — no shortcut, and the palette draws one per row`);
  if (scope === "") throw new Error(`${where} — no scope, so no surface can decide whether to offer it`);

  // A Motion appears in no palette, so it carries neither a glyph nor a reason
  // for having none. An Action carries exactly one of the two.
  if (kind === "Motion" && (icon !== "" || absent !== "")) {
    throw new Error(`${where} — a Motion appears in no palette and carries no glyph column`);
  }
  if (icon !== "" && absent !== "") {
    throw new Error(`${where} — a glyph and a reason for having none, and it is one or the other`);
  }
  if (kind === "Action" && icon === "" && !ABSENCES.has(absent)) {
    throw new Error(
      `${where} — no glyph and icon_absent = "${absent}". It is "undecided" or "by design"`,
    );
  }
  // The registry names a glyph this lucide-react cannot export. The vocabulary
  // counts that as a gap because it has a `GAPS` channel to count it in; an act
  // has none, and drawing the row with no glyph would silently contradict a
  // registry that says it has one.
  if (icon !== "" && NOT_EXPORTED.has(icon)) {
    throw new Error(`${where} — icon "${icon}" is in icons.toml and lucide-react does not export it`);
  }
  // `unbuilt` is an issue reference and nothing else — the same rule
  // `rules_actions.rs` applies, restated because what is emitted is a number.
  const built = unbuilt === "" ? null : /^#(\d+)$/.exec(unbuilt);
  if (unbuilt !== "" && built === null) {
    throw new Error(`${where} — unbuilt = "${unbuilt}", and it takes an issue reference`);
  }
  for (const [key, value] of [["destructive", table.destructive], ["confirms", table.confirms]]) {
    if (value !== "true" && value !== "false") {
      throw new Error(`${where} — ${key} is "${value ?? ""}", and it is true or false`);
    }
  }

  acts.push({
    id,
    kind,
    tier,
    verb,
    icon,
    absent,
    shortcut,
    scope,
    destructive: table.destructive === "true",
    confirms: table.confirms === "true",
    issue: built === null ? null : Number(built[1]),
  });
}

if (acts.length === 0) throw new Error("actions.toml — no [actions.*] table, and Bridge draws them all");

const actGlyphs = [...new Set(acts.map((act) => act.icon).filter((icon) => icon !== ""))].sort();
// The scopes are read off the rows rather than listed here. A scope is where a
// binding is offered and the registry decides that; a set written into this
// script would be a fourth copy of the thing this file exists to stop having
// four of. What holds a *new* scope to something is the emitted union: the
// palette maps every member of it to a context, so a scope nobody has placed
// fails `pnpm typecheck` with the scope's own name in the message.
const scopes = [...new Set(acts.map((act) => act.scope))].sort();

const actLines = [];
actLines.push("// GENERATED by `pnpm --filter @armada/desktop codegen`. Do not hand-edit.");
actLines.push("//");
actLines.push("// Every act Bridge offers: what it is called, what it is bound to, and the");
actLines.push("// glyph it draws — or why it draws none. From");
actLines.push("// `crates/core-model/domain/actions.toml`, which is the artifact");
actLines.push("// `docs/contracts/design-system.md` promises under \"One artifact, three");
actLines.push("// columns\" and the authority on all three.");
actLines.push("//");
actLines.push("// **A Motion is here and is not an act.** `move_focus`, `open_focused` and");
actLines.push("// `focus_chapter` move the cursor and act on nothing; the registry says they");
actLines.push("// appear in no palette and carry no glyph, so they are emitted for");
actLines.push("// completeness and filtered out by anything that draws a list of acts.");
actLines.push("//");
actLines.push("// **A blank glyph is a fact, not a default.** `iconAbsent` says which kind of");
actLines.push("// blank it is: `undecided` means no registered silhouette means the act and");
actLines.push("// assigning one is a decision for `packages/icons/icons.toml`; `by design`");
actLines.push("// means a document decided the act carries none. A surface says which it is");
actLines.push("// drawing rather than inventing a glyph to fill the column.");
actLines.push("//");
actLines.push("// **`unbuilt` names the issue that answers the key.** The registry is ahead");
actLines.push("// of the app deliberately, because the map was settled by drawing. A palette");
actLines.push("// that displays a binding beside every entry would otherwise offer a row a");
actLines.push("// person presses and gets nothing from, which is worse than one that is");
actLines.push("// absent.");
actLines.push("");
if (actGlyphs.length > 0) {
  actLines.push(`import { ${actGlyphs.map(pascal).join(", ")} } from "lucide-react";`);
}
actLines.push('import type { LucideIcon } from "lucide-react";');
actLines.push("");
actLines.push("/** Whether the row is an act or a movement of the cursor. */");
actLines.push('export type ActionKind = "Action" | "Motion";');
actLines.push("");
actLines.push("/** Modifier-based and working anywhere, or single-key and on what is focused. */");
actLines.push('export type ActionTier = "Global" | "Contextual";');
actLines.push("");
actLines.push("/**");
actLines.push(" * Where the binding is offered. The registry's own set, spelled the registry's");
actLines.push(" * way, and read off the rows rather than declared — so a scope that appears");
actLines.push(" * in `actions.toml` appears here and nowhere else has to be told.");
actLines.push(" */");
actLines.push(`export type ActionScope =${scopes.map((s) => `\n  | ${JSON.stringify(s)}`).join("")};`);
actLines.push("");
actLines.push("/** Why an act's glyph column is empty. `null` where it is not. */");
actLines.push('export type IconAbsence = "undecided" | "by design";');
actLines.push("");
actLines.push("export type Action = {");
actLines.push("  /** The id an implementation binds to. The registry's table key. */");
actLines.push("  readonly id: string;");
actLines.push("  readonly kind: ActionKind;");
actLines.push("  readonly tier: ActionTier;");
actLines.push("  /** What a person reads, in the lexicon's word. Never the id. */");
actLines.push("  readonly verb: string;");
actLines.push("  /** The glyph, or `null` — in which case `iconAbsent` says why. */");
actLines.push("  readonly icon: LucideIcon | null;");
actLines.push("  readonly iconAbsent: IconAbsence | null;");
actLines.push("  /** The binding, spelled as the contract's map spells it. */");
actLines.push("  readonly shortcut: string;");
actLines.push("  readonly scope: ActionScope;");
actLines.push("  readonly destructive: boolean;");
actLines.push("  readonly confirms: boolean;");
actLines.push("  /** The issue that gives the binding an act, on a row nothing answers yet. */");
actLines.push("  readonly unbuilt: string | null;");
actLines.push("};");
actLines.push("");
actLines.push("/**");
actLines.push(" * The map, in the registry's order: global first, then contextual.");
actLines.push(" *");
actLines.push(" * Order is load-bearing in one place only — the palette groups by section and");
actLines.push(" * keeps registry order inside each — so nothing here is sorted.");
actLines.push(" */");
actLines.push("export const ACTIONS: readonly Action[] = [");
for (const act of acts) {
  actLines.push("  {");
  actLines.push(`    id: ${JSON.stringify(act.id)},`);
  actLines.push(`    kind: ${JSON.stringify(act.kind)},`);
  actLines.push(`    tier: ${JSON.stringify(act.tier)},`);
  actLines.push(`    verb: ${JSON.stringify(act.verb)},`);
  actLines.push(`    icon: ${act.icon === "" ? "null" : pascal(act.icon)},`);
  actLines.push(`    iconAbsent: ${quoted(act.absent)},`);
  actLines.push(`    shortcut: ${JSON.stringify(act.shortcut)},`);
  actLines.push(`    scope: ${JSON.stringify(act.scope)},`);
  actLines.push(`    destructive: ${act.destructive},`);
  actLines.push(`    confirms: ${act.confirms},`);
  actLines.push(`    unbuilt: ${act.issue === null ? "null" : JSON.stringify(`#${act.issue}`)},`);
  actLines.push("  },");
}
actLines.push("];");
actLines.push("");
actLines.push("/**");
actLines.push(" * One act, by the id an implementation binds to.");
actLines.push(" *");
actLines.push(" * `| undefined` because a caller may ask for an id this build has never");
actLines.push(" * heard of, and a missing act is a real answer rather than a crash.");
actLines.push(" */");
actLines.push("export const ACTION: Readonly<Record<string, Action | undefined>> = Object.fromEntries(");
actLines.push("  ACTIONS.map((action) => [action.id, action]),");
actLines.push(");");
actLines.push("");

const actionsModule = actLines.join("\n");
const versionModule = [
  "// GENERATED by `pnpm --filter @armada/desktop codegen`. Do not hand-edit.",
  "//",
  "// `protocol-version.toml` at the repository root is what both sides read.",
  "// `crates/ipc/build.rs` emits the Rust constant from it; this is the TypeScript",
  "// one. A hand-typed literal in Bridge is a second source of truth the day the",
  "// file changes.",
  "//",
  "// Which number moved decides what a mismatch does — `skew` in",
  "// `version.ts` beside this file is the only comparison, and there is no bare",
  "// here to spell `!==` against.",
  "",
  `export const PROTOCOL_VERSION = { major: ${majorMatch[1]}, minor: ${minorMatch[1]} };`,
  "",
].join("\n");

// Everything this script writes, in one list rather than a `writeFileSync` per
// destination. The gate reads the same list back through `--emit` and compares
// each entry against what is checked in, so a fourth output added here is gated
// the moment it exists — the alternative is a rule naming three files by hand,
// which is the hole reopened one file over the first time somebody adds a
// fifth.
const EMITTED = [
  [OUT, vocabularyModule],
  [OUT_VERSION, versionModule],
  [OUT_ACTIONS, actionsModule],
];

// `--emit` writes nothing and prints `<repo-relative path>\0<text>\0` for each
// output instead. NUL rather than a line or a length: a generated file holds
// newlines, quotes and backslashes, and holds no NUL, so the gate splits on one
// byte and nothing has to be escaped or counted. The summary below goes to
// stderr in that mode, so stdout carries the payload and nothing else.
const emitting = process.argv.includes("--emit");
if (emitting) {
  for (const [path, text] of EMITTED) {
    process.stdout.write(`${relative(repo, path).split(sep).join("/")}\0${text}\0`);
  }
} else {
  for (const [path, text] of EMITTED) {
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, text);
  }
}

const summary = emitting ? process.stderr : process.stdout;
const counted = gaps.map((gap) => `${gap.vocabulary}.${gap.variant} (${gap.missing.join(", ")})`);
// A wanted vocabulary with no rows at all names itself, because it lands in no
// `GAPS` row — there is no variant to key one on.
for (const [vocabulary, rows] of vocabularies) {
  if (rows.length === 0) counted.push(`${vocabulary} (no rows in enum-verbs.toml)`);
}
summary.write(
  `vocabulary.ts: ${[...vocabularies.values()].reduce((n, rows) => n + rows.length, 0)} variants, ` +
    `${gaps.length} with a gap\n`,
);
for (const line of counted) summary.write(`  gap: ${line}\n`);

// The acts get their own line rather than a row in `GAPS`, because a missing
// glyph is not the same absence there. A vocabulary variant with no glyph is a
// registry that has not decided; an act with none has either that or a decision
// that it draws none, and the registry says which. Counting the undecided ones
// is the number that should fall.
const undecided = acts.filter((act) => act.absent === "undecided");
const awaiting = acts.filter((act) => act.issue !== null);
summary.write(
  `actions.ts: ${acts.length} acts, ${undecided.length} with an undecided glyph, ` +
    `${awaiting.length} not built\n`,
);
for (const act of undecided) summary.write(`  no glyph: ${act.id}\n`);
for (const act of awaiting) summary.write(`  not built: ${act.id} (#${act.issue})\n`);
