// The Manifest surface's forms as edits — Journey 9's *Editing*. What a
// declared file draws as, what a draft sends, what keeps Save back, and where a
// cap sits below what this repository's past Jobs cost. No React.
//
// **A draft is text as typed**, and an edit is sent only for a key whose value
// moved, so an untouched form sends nothing and changes no byte.

import type {
  ManifestFormCheck,
  ManifestFormCommand,
  ManifestFormDraft,
  ManifestFormEvidence,
  ManifestFormNarrow,
} from "@armada/components";
import type {
  CheckDraft,
  CommandDraft,
  EvidenceDraft,
  LinkDraft,
  ManifestDeclared,
  ManifestEdit,
  ManifestSpend,
  NarrowingDraft,
} from "@armada/protocol";

import { money } from "./facts";

const MICROS = 1_000_000;
/** Both caps are `u32` in the file. */
const MOST_A_CAP_HOLDS = 4_294_967_295;

/** What a declared file draws as. */
export function draftOf(declared: ManifestDeclared): ManifestFormDraft {
  return {
    checks: declared.checks.map(({ name, check }) => ({
      name,
      run: check.run,
      expectExitCode: numberText(check.expect_exit_code === 0 ? undefined : check.expect_exit_code),
      requires: check.requires ?? [],
      when: linesOf(check.when),
      narrow:
        check.narrow === undefined
          ? null
          : {
              run: check.narrow.run,
              each: check.narrow.each,
              from: linesOf(check.narrow.from),
              under: check.narrow.under ?? "",
              except: linesOf(check.narrow.except),
            },
    })),
    commands: declared.commands.map(({ name, command }) => ({
      name,
      run: command.run ?? "",
      destructive: command.destructive ?? false,
      serve: command.serve ?? "",
      ready: command.ready ?? "",
      links: (command.links ?? []).map((link) => ({ url: link.url, name: link.name ?? "" })),
    })),
    ports: declared.ports.map(({ name, port }) => ({
      name,
      container: port.container === undefined ? "" : String(port.container),
      env: port.env ?? "",
    })),
    autoMerge: declared.auto_merge.written,
    reviewGate: declared.review_gate.written,
    costCap:
      declared.cost_cap_micros_per_job === undefined
        ? ""
        : String(declared.cost_cap_micros_per_job / MICROS),
    turnCap: numberText(declared.turn_cap_per_job),
    base: declared.base ?? "",
    evidence:
      declared.evidence === undefined
        ? null
        : {
            serve: declared.evidence.serve ?? "",
            ready: declared.evidence.ready ?? "",
            run: declared.evidence.run,
            frames: declared.evidence.frames,
            never: linesOf(declared.evidence.never),
          },
    afterMerge: declared.after_merge_checks ?? [],
    setup: declared.setup_requires ?? [],
    quietAfter: numberText(declared.quiet_after_seconds),
    pokeLimit: numberText(declared.poke_limit),
    excludePaths: linesOf(declared.exclude_paths),
  };
}

/**
 * The edits that take the file from `declared` to `draft`, removals first.
 * **Entries match by name**, so one removed and declared again is edited in
 * place and keeps the comment above it.
 */
export function editsOf(declared: ManifestDeclared, draft: ManifestFormDraft): ManifestEdit[] {
  const removes: ManifestEdit[] = [];
  const adds: ManifestEdit[] = [];
  const sets: ManifestEdit[] = [];

  const checks = new Map(declared.checks.map(({ name, check }) => [name, check]));
  const keptChecks = new Set(draft.checks.map((row) => row.name));
  for (const name of checks.keys()) {
    if (!keptChecks.has(name)) removes.push({ edit: "remove_check", name });
  }
  for (const row of draft.checks) {
    const name = row.name;
    const next = checkOf(row);
    const was = checks.get(name);
    if (was === undefined) {
      adds.push({ edit: "add_check", name, check: next });
      continue;
    }
    if (next.run !== was.run) sets.push({ edit: "set_check_run", name, run: next.run });
    const code = next.expect_exit_code ?? 0;
    if (code !== (was.expect_exit_code ?? 0)) {
      sets.push({ edit: "set_check_expect_exit_code", name, expect_exit_code: code });
    }
    const requires = next.requires ?? [];
    if (!same(requires, was.requires ?? [])) sets.push({ edit: "set_check_requires", name, requires });
    const when = next.when ?? [];
    if (!same(when, was.when ?? [])) sets.push({ edit: "set_check_when", name, when });
    if (narrowKey(next.narrow) !== narrowKey(was.narrow)) {
      sets.push({ edit: "set_check_narrow", name, narrow: next.narrow ?? null });
    }
  }

  const commands = new Map(declared.commands.map(({ name, command }) => [name, command]));
  const keptCommands = new Set(draft.commands.map((row) => row.name));
  for (const name of commands.keys()) {
    if (!keptCommands.has(name)) removes.push({ edit: "remove_command", name });
  }
  for (const row of draft.commands) {
    const name = row.name;
    const next = commandOf(row);
    const was = commands.get(name);
    if (was === undefined) {
      adds.push({ edit: "add_command", name, command: next });
      continue;
    }
    const run = next.run ?? null;
    if (run !== (was.run ?? null)) sets.push({ edit: "set_command_run", name, run });
    const destructive = next.destructive ?? false;
    if (destructive !== (was.destructive ?? false)) {
      sets.push({ edit: "set_command_destructive", name, destructive });
    }
    const serve = next.serve ?? null;
    if (serve !== (was.serve ?? null)) sets.push({ edit: "set_command_serve", name, serve });
    const ready = next.ready ?? null;
    if (ready !== (was.ready ?? null)) sets.push({ edit: "set_command_ready", name, ready });
    const links = next.links ?? [];
    if (linksKey(links) !== linksKey(was.links ?? [])) sets.push({ edit: "set_command_links", name, links });
  }

  const ports = new Map(declared.ports.map(({ name, port }) => [name, port]));
  const keptPorts = new Set(draft.ports.map((row) => row.name));
  for (const name of ports.keys()) {
    if (!keptPorts.has(name)) removes.push({ edit: "remove_port", name });
  }
  for (const row of draft.ports) {
    const name = row.name;
    const container = wholeOf(row.container);
    const env = textOf(row.env);
    const was = ports.get(name);
    if (was === undefined) {
      adds.push({
        edit: "add_port",
        name,
        port: {
          ...(container === null ? {} : { container }),
          ...(env === null ? {} : { env }),
        },
      });
      continue;
    }
    if (container !== (was.container ?? null)) sets.push({ edit: "set_port_container", name, container });
    if (env !== (was.env ?? null)) sets.push({ edit: "set_port_env", name, env });
  }

  const base = textOf(draft.base);
  if (base !== (declared.base ?? null)) sets.push({ edit: "set_base", base });
  const was = declared.evidence;
  const evidence = draft.evidence === null ? null : evidenceOf(draft.evidence);
  if (evidence === null) {
    if (was !== undefined) removes.push({ edit: "remove_evidence" });
  } else if (was === undefined) {
    adds.push({ edit: "add_evidence", evidence });
  } else {
    const serve = evidence.serve ?? null;
    if (serve !== (was.serve ?? null)) sets.push({ edit: "set_evidence_serve", serve });
    const ready = evidence.ready ?? null;
    if (ready !== (was.ready ?? null)) sets.push({ edit: "set_evidence_ready", ready });
    if (evidence.run !== was.run) sets.push({ edit: "set_evidence_run", run: evidence.run });
    if (evidence.frames !== was.frames) sets.push({ edit: "set_evidence_frames", frames: evidence.frames });
    const never = evidence.never ?? [];
    if (!same(never, was.never ?? [])) sets.push({ edit: "set_evidence_never", never });
  }
  if (!same(draft.afterMerge, declared.after_merge_checks ?? [])) {
    sets.push({ edit: "set_after_merge_checks", checks: draft.afterMerge });
  }
  if (!same(draft.setup, declared.setup_requires ?? [])) {
    sets.push({ edit: "set_setup_requires", requires: draft.setup });
  }
  const quiet = wholeOf(draft.quietAfter);
  if (quiet !== (declared.quiet_after_seconds ?? null)) {
    sets.push({ edit: "set_quiet_after_seconds", quiet_after_seconds: quiet });
  }
  const pokes = wholeOf(draft.pokeLimit);
  if (pokes !== (declared.poke_limit ?? null)) sets.push({ edit: "set_poke_limit", poke_limit: pokes });
  const excluded = listOf(draft.excludePaths);
  if (!same(excluded, declared.exclude_paths ?? [])) {
    sets.push({ edit: "set_exclude_paths", exclude_paths: excluded });
  }

  if (draft.autoMerge !== declared.auto_merge.written) {
    sets.push({ edit: "set_auto_merge", auto_merge: draft.autoMerge });
  }
  if (draft.reviewGate !== declared.review_gate.written) {
    sets.push({ edit: "set_review_gate", review_gate: draft.reviewGate });
  }
  const cost = microsOf(draft.costCap);
  if (cost !== (declared.cost_cap_micros_per_job ?? null)) {
    sets.push({ edit: "set_cost_cap_micros_per_job", cost_cap_micros_per_job: cost });
  }
  const turns = wholeOf(draft.turnCap);
  if (turns !== (declared.turn_cap_per_job ?? null)) {
    sets.push({ edit: "set_turn_cap_per_job", turn_cap_per_job: turns });
  }

  return [...removes, ...adds, ...sets];
}

/**
 * What keeps Save back, keyed as the file spells where — `checks.lint.run`.
 * **Only what the form can tell on its own**; whether the result loads is
 * Fleet's, and a refusal says so key by key.
 */
export function problemsOf(draft: ManifestFormDraft): Record<string, string> {
  const problems: Record<string, string> = {};
  for (const check of draft.checks) {
    if (check.run.trim() === "") problems[`checks.${check.name}.run`] = "A Check needs a command.";
    if (check.expectExitCode.trim() !== "" && integerOf(check.expectExitCode) === null) {
      problems[`checks.${check.name}.expect_exit_code`] = "An exit code is a whole number.";
    }
    const narrow = check.narrow;
    if (narrow !== null && (narrow.run.trim() === "" || narrow.each.trim() === "")) {
      problems[`checks.${check.name}.narrow`] = "Narrowing needs its command and what each path becomes.";
    }
  }
  for (const command of draft.commands) {
    if (command.run.trim() === "" && command.serve.trim() === "") {
      problems[`commands.${command.name}.run`] = "A Command needs a command to run, or one to serve.";
    }
    if (command.links.some((link) => link.url.trim() === "" && link.name.trim() !== "")) {
      problems[`commands.${command.name}.links`] = "A link needs its address.";
    }
  }
  for (const port of draft.ports) {
    const container = wholeOf(port.container);
    if (port.container.trim() !== "" && (container === null || container < 1 || container > 65_535)) {
      problems[`ports.${port.name}.container`] = "A container port is a whole number from 1 to 65535.";
    }
  }
  const evidence = draft.evidence;
  if (evidence !== null) {
    if (!evidence.run.includes("{}")) problems["evidence.run"] = "It needs {} where the spec's path goes.";
    if (evidence.frames.trim() === "") problems["evidence.frames"] = "Frames need a directory to land in.";
    if ((evidence.serve.trim() === "") !== (evidence.ready.trim() === "")) {
      problems["evidence.ready"] = "Serves and Ready when go together, or neither is set.";
    }
  }
  const quiet = wholeOf(draft.quietAfter);
  if (draft.quietAfter.trim() !== "" && (quiet === null || quiet < 1 || quiet > MOST_A_CAP_HOLDS)) {
    problems["drone.quiet_after_seconds"] = "A silence threshold is a whole number of seconds, 1 or more.";
  }
  const pokes = wholeOf(draft.pokeLimit);
  if (draft.pokeLimit.trim() !== "" && (pokes === null || pokes > MOST_A_CAP_HOLDS)) {
    problems["drone.poke_limit"] = "A poke limit is a whole number, 0 or more.";
  }
  const cost = microsOf(draft.costCap);
  if (draft.costCap.trim() !== "" && (cost === null || cost > MOST_A_CAP_HOLDS)) {
    problems["budget.cost"] = `A cap is a number of dollars, from 0 to ${String(MOST_A_CAP_HOLDS / MICROS)}.`;
  }
  const turns = wholeOf(draft.turnCap);
  if (draft.turnCap.trim() !== "" && (turns === null || turns > MOST_A_CAP_HOLDS)) {
    problems["budget.turns"] = "A turn cap is a whole number of turns, zero or more.";
  }
  return problems;
}

/**
 * Where a cap in the draft is below what one of this repository's past Jobs
 * cost. **The costliest Job, not a total** — a cap is set per Job, so one below
 * it is one a Job like it stops at.
 */
export function budgetWarningsOf(draft: ManifestFormDraft, spend: ManifestSpend | null): string[] {
  if (spend === null || spend.jobs === 0) return [];
  const jobs = spend.jobs === 1 ? "past Job" : `${spend.jobs} past Jobs`;
  const warnings: string[] = [];
  const cost = microsOf(draft.costCap);
  if (cost !== null && cost < spend.most_cost_micros) {
    warnings.push(
      `The costliest of this repository's ${jobs} cost ${money(spend.most_cost_micros)}, more than this cap. A Job like it would stop before it finished.`,
    );
  }
  const turns = wholeOf(draft.turnCap);
  if (turns !== null && turns < spend.most_turns) {
    warnings.push(
      `The longest of this repository's ${jobs} took ${spend.most_turns} turns, more than this cap. A Job like it would stop before it finished.`,
    );
  }
  return warnings;
}

function checkOf(row: ManifestFormCheck): CheckDraft {
  const when = listOf(row.when);
  const requires = row.requires.filter((name) => name.trim() !== "");
  const code = integerOf(row.expectExitCode) ?? 0;
  return {
    run: row.run.trim(),
    ...(code === 0 ? {} : { expect_exit_code: code }),
    ...(requires.length === 0 ? {} : { requires }),
    ...(when.length === 0 ? {} : { when }),
    ...(row.narrow === null ? {} : { narrow: narrowingOf(row.narrow) }),
  };
}

function narrowingOf(row: ManifestFormNarrow): NarrowingDraft {
  const from = listOf(row.from);
  const except = listOf(row.except);
  const under = textOf(row.under);
  return {
    run: row.run.trim(),
    each: row.each.trim(),
    ...(from.length === 0 ? {} : { from }),
    ...(under === null ? {} : { under }),
    ...(except.length === 0 ? {} : { except }),
  };
}

function commandOf(row: ManifestFormCommand): CommandDraft {
  const run = textOf(row.run);
  const serve = textOf(row.serve);
  const ready = textOf(row.ready);
  const links: LinkDraft[] = row.links
    .filter((link) => link.url.trim() !== "")
    .map((link) => {
      const name = textOf(link.name);
      return { url: link.url.trim(), ...(name === null ? {} : { name }) };
    });
  return {
    ...(run === null ? {} : { run }),
    ...(row.destructive ? { destructive: true } : {}),
    ...(serve === null ? {} : { serve }),
    ...(ready === null ? {} : { ready }),
    ...(links.length === 0 ? {} : { links }),
  };
}

function evidenceOf(row: ManifestFormEvidence): EvidenceDraft {
  const serve = textOf(row.serve);
  const ready = textOf(row.ready);
  const never = listOf(row.never);
  return {
    ...(serve === null ? {} : { serve }),
    ...(ready === null ? {} : { ready }),
    run: row.run.trim(),
    frames: row.frames.trim(),
    ...(never.length === 0 ? {} : { never }),
  };
}

function narrowKey(narrow: NarrowingDraft | undefined): string {
  if (narrow === undefined) return "";
  return JSON.stringify([narrow.run, narrow.each, narrow.from ?? [], narrow.under ?? null, narrow.except ?? []]);
}

function linksKey(links: readonly LinkDraft[]): string {
  return JSON.stringify(links.map((link) => [link.url, link.name ?? null]));
}

function same(one: readonly string[], other: readonly string[]): boolean {
  return one.length === other.length && one.every((value, at) => value === other[at]);
}

/** One entry a line, as a person types a list. */
function linesOf(list: readonly string[] | undefined): string {
  return (list ?? []).join("\n");
}

function listOf(typed: string): string[] {
  return typed
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "");
}

function textOf(typed: string): string | null {
  const trimmed = typed.trim();
  return trimmed === "" ? null : trimmed;
}

function numberText(value: number | undefined): string {
  return value === undefined ? "" : String(value);
}

/** A whole number that may be negative, as an exit code is. */
function integerOf(typed: string): number | null {
  const trimmed = typed.trim();
  return /^-?\d+$/.test(trimmed) ? Number(trimmed) : null;
}

/** A whole number of zero or more, `null` where empty, `NaN`-free. */
function wholeOf(typed: string): number | null {
  const trimmed = typed.trim();
  if (!/^\d+$/.test(trimmed)) return null;
  return Number(trimmed);
}

/** Dollars as typed, in micros. `null` where empty or not an amount. */
function microsOf(typed: string): number | null {
  const trimmed = typed.trim();
  if (!/^\d+(\.\d*)?$|^\.\d+$/.test(trimmed)) return null;
  return Math.round(Number(trimmed) * MICROS);
}
