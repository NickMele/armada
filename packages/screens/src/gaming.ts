// A step's gaming check, read once for the rail, the step panel and the card
// that holds a step it stopped. #1079.
//
// **Its own reading, never a tier's.** The gaming check is counted in neither
// Checks nor Judge on purpose — a flag is not a verdict, and a recount would
// make a green tier read red — so nothing here touches `gates.ts`' counts.
//
// **A flag a second reading cleared is still a flag**, and it never holds a
// step: it is drawn in the panel as cleared, with why, and nowhere else.

import type { DiffLine } from "@armada/components";
import type { Diff, Flagged, StepDetail } from "@armada/protocol";

import { onlyCurrentAttempt } from "./facts";
import { didNotPass, NOT_REACHED } from "./gates";
import { drawnOf } from "./review";

/**
 * `Flagged` as Fleet serves it once #1080 lands: a flag a second reading
 * disagreed with carries `cleared`, with why. **Replace with `Flagged` from
 * `@armada/protocol` once the generated type has the field** — this is that
 * shape exactly, and nothing more.
 */
export type FlaggedRead = Flagged & { cleared?: { why: string; brief_path?: string } };

/** One attempt's flags, split into those that hold the step and those cleared. */
export type FlagsRead = { held: FlaggedRead[]; cleared: FlaggedRead[] };

/** The newest attempt's flags, or the rows a caller already narrowed. */
export function flagsOf(step: StepDetail, rows?: readonly FlaggedRead[]): FlagsRead {
  const flags: readonly FlaggedRead[] = rows ?? onlyCurrentAttempt(step.flagged);
  return {
    held: flags.filter((flag) => flag.cleared === undefined),
    cleared: flags.filter((flag) => flag.cleared !== undefined),
  };
}

/** Whether the step declares a gaming check at all. */
export function declaresGaming(step: StepDetail): boolean {
  return (step.judge_checks ?? []).some((judge) => judge.gaming_check);
}

/**
 * Whether the gaming check ran on the step's newest attempt.
 *
 * **Read off the record, never assumed.** It ran where it flagged anything,
 * where the step advanced, or where the gate ruled with every Check passing —
 * a Check that failed stops the gate before the gaming check is asked.
 */
export function gamingReached(step: StepDetail): boolean {
  const attempt = step.attempts.at(-1)?.attempt;
  const mine = <T extends { attempt: number }>(rows: readonly T[]): T[] =>
    attempt === undefined ? [...rows] : rows.filter((row) => row.attempt === attempt);
  if (mine(step.flagged).length > 0 || step.state === "advanced") return true;
  return mine(step.verdicts).length > 0 && !mine(step.check_runs).some(didNotPass);
}

/** `1 flag`, `2 flags`. */
function flagsSaid(count: number): string {
  return `${count} ${count === 1 ? "flag" : "flags"}`;
}

/**
 * The rail's `Gaming check` fact — `1 flag · stopped here` — or `undefined`
 * where nothing was flagged. **Replaces `Verdict · evidence disputed`**, which
 * named the trigger and not the check that pulled it.
 *
 * `stopped` is whether these flags are what holds the step now; a step a
 * person carried on past still carries them, and says only how many.
 */
export function gamingStands({ held, cleared }: FlagsRead, stopped: boolean): string | undefined {
  if (held.length > 0) return stopped ? `${flagsSaid(held.length)} · stopped here` : flagsSaid(held.length);
  if (cleared.length > 0) return `${flagsSaid(cleared.length)} cleared`;
  return undefined;
}

/**
 * What the step panel's Gaming check row says folded — `1 flagged · stopped
 * the step`.
 *
 * **No `of 6`.** The owner's drawing counts against every pattern the step
 * declares, and the wire carries only whether a gaming check is declared,
 * never which patterns — so the denominator would be invented.
 */
export function gamingSummary({ held, cleared }: FlagsRead, reached: boolean, stopped: boolean): string {
  const parts = [
    ...(held.length > 0 ? [`${held.length} flagged`] : []),
    ...(cleared.length > 0 ? [`${cleared.length} cleared`] : []),
  ];
  if (held.length > 0 && stopped) return [...parts, "stopped the step"].join(" · ");
  if (parts.length > 0) return parts.join(" · ");
  return reached ? NOTHING_FLAGGED : NOT_REACHED;
}

/** What a gaming check that ran and found nothing says. */
export const NOTHING_FLAGGED = "nothing flagged";

/** One flag's lines, located in the patch: the file and the hunk that holds them. */
export type Located = { file: string; lines: DiffLine[] };

/**
 * The hunk a flag is about, out of this Job's patch, or `undefined`.
 *
 * **Located, never guessed.** A flag with a post-image line is found by that
 * line, counted off git's own hunk header. A flag cited on a removed line has
 * a file and no line, so it is found by the words it quotes — and where no line
 * in the file carries them, nothing is drawn rather than a nearby hunk.
 */
export function hunkFor(flag: Flagged, diff: Diff, jobId: string): Located | undefined {
  const at = flag.at;
  if (at === undefined) return undefined;
  const patch = diff.state === "read" && diff.jobId === jobId ? diff.work?.patch : undefined;
  if (patch === undefined) return undefined;
  const file = drawnOf(patch, () => "").files.find((one) => one.path === at.file);
  if (file === undefined) return undefined;
  const hunks = hunksIn(file.lines);
  const line = at.line;
  const found =
    line === undefined
      ? (hunks.find((hunk) => quotes(hunk, flag.cited, "removed")) ??
        hunks.find((hunk) => quotes(hunk, flag.cited, undefined)))
      : hunks.find((hunk) => holdsLine(hunk, line));
  return found === undefined ? undefined : { file: at.file, lines: found };
}

/** A file's lines, one array per `@@` hunk, each led by its header. */
function hunksIn(lines: readonly DiffLine[]): DiffLine[][] {
  const hunks: DiffLine[][] = [];
  for (const line of lines) {
    if (line.kind === "hunk") hunks.push([line]);
    else hunks.at(-1)?.push(line);
  }
  return hunks;
}

/** Whether a hunk draws the file's post-image line `line`, counted off its header. */
function holdsLine(hunk: readonly DiffLine[], line: number): boolean {
  const header = /^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(hunk[0]?.text ?? "");
  if (header === null) return false;
  let at = Number(header[1]);
  for (const row of hunk.slice(1)) {
    if (row.kind === "removed" || row.text.startsWith("\\")) continue;
    if (row.kind === "added" && at === line) return true;
    if (row.kind === "context" && at === line) return true;
    at += 1;
  }
  return false;
}

/** Shorter than this, a line matches too much to be the one quoted. */
const QUOTED_AT_LEAST = 8;

/**
 * Whether a line of the hunk, of `kind` where one is named, is quoted by the
 * citation — the whole line inside it, or a backticked quotation inside the
 * line. Whitespace is collapsed on both sides, since a model re-wraps what it
 * quotes.
 */
function quotes(hunk: readonly DiffLine[], cited: string, kind: DiffLine["kind"] | undefined): boolean {
  const said = collapsed(cited);
  const quoted = [...cited.matchAll(/`([^`\n]+)`/g)]
    .map((match) => collapsed(match[1] ?? ""))
    .filter((one) => one.length >= QUOTED_AT_LEAST);
  return hunk.slice(1).some((row) => {
    if (kind !== undefined && row.kind !== kind) return false;
    const content = collapsed(row.text.slice(1));
    if (content.length < QUOTED_AT_LEAST) return false;
    return said.includes(content) || quoted.some((one) => content.includes(one));
  });
}

function collapsed(text: string): string {
  return text.replace(/\s+/g, " ").trim();
}
