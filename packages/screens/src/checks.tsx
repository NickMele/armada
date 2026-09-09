// Chapter four — every Check this step declares, and what each came to.
//
// **Declared rather than run.** A Check the gate has not reached keeps its row,
// because the shape of what is coming is part of reading a running Job, and a
// list built from the runs would make a Job look like it has fewer gates than
// it has.
//
// **The Judge is a row here as well as a chapter of its own.** It is one of the
// things that gates the step, so leaving it off would make the list shorter
// than the gate. It says what the grid below says, from the same reading of the
// same rows, because it is the same panel.
//
// **The reading is `gates.ts`'s** — which attempt's runs count, and which
// output a press opens. This file decides only what a row looks like.

import { Button, CheckRuns, Kbd, keyFor, type CheckRun as CheckRunRow } from "@armada/components";
import type { StepChapter } from "@armada/components";
import { CHECK_OUTCOME, CRITERION_VERDICT_CHECK, CRITERION_VERDICT_JUDGE } from "@armada/components";
import type { CheckRun, StepDetail } from "@armada/protocol";

import { judgeOf } from "./declared";
import { namesChapter } from "./detail-keys";
import { checksOf, checksStand, didNotPass, outputOf, type CheckRead, type Panel } from "./gates";
import { basename, openKept, type Opens } from "./phases";
import { countedIn } from "./verdicts";

/** Which chapter the Checks are, so the keyboard can name it. */
export const CHECKS_CHAPTER = "checks";

/**
 * Chapter four, or none.
 *
 * **A step that declares no Check draws nothing.** So does one whose workflow
 * Fleet does not hold — the phase strip's note is what tells those two apart,
 * and an empty labelled list here would read as a reading that failed rather
 * than as a step that gates on nothing.
 */
export function checksChapter(
  step: StepDetail,
  panels: Panel[],
  opens: Opens,
): Omit<StepChapter, "ordinal"> | undefined {
  const reads = checksOf(step);
  if (reads.length === 0) return undefined;

  const rows = reads.map(checkRow);
  const judge = judgeRow(step, panels);
  if (judge !== undefined) rows.push(judge);

  // What each Check wrote, by the row that wrote it, so a press opens that
  // Check's own file rather than the one the header act opens.
  const outputs = new Map<string, string>();
  for (const read of reads) {
    if (read.run?.output_path !== undefined) outputs.set(read.name, read.run.output_path);
  }
  // The header act and `o` open the same file, because both come through one
  // reading. Absent where no Check on this attempt kept an output — a control
  // that opens nothing is the defect `#246` was about with a click added to it.
  const output = outputOf(step);

  return {
    id: CHECKS_CHAPTER,
    title: "Checks",
    // The same sentence the phase strip's tier stands at, from the same call.
    summary: checksStand(reads),
    // The preview is the whole list — four rows is not a reading. What has no
    // end is what is behind each row, and that is what a press opens.
    preview: (
      <CheckRuns
        rows={rows}
        // Not "in the viewer". Bridge has no evidence viewer; `main/open.ts`
        // hands the path to the OS, and a tooltip promising a panel nobody
        // built is the surface describing a screen that does not exist.
        openSaid={OPENS_WHERE}
        onOpen={(checkId) => {
          const kept = outputs.get(checkId);
          if (kept !== undefined) openKept(opens, { kept, what: "check" });
        }}
      />
    ),
    ...(output === undefined
      ? {}
      : {
          act: (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => openKept(opens, { kept: output, what: "check" })}
              {...namesChapter(CHECKS_CHAPTER)}
            >
              Open the output
              <Kbd>{keyFor("open_output")}</Kbd>
            </Button>
          ),
        }),
  };
}

/**
 * One Check's row.
 *
 * **`says` is the finding and `result` is the measurement.** The wire draws
 * that line itself: `produced` is *"the exit code, the signal, the budget it
 * outran"* and it is **absent on a pass, because a pass measured nothing** — so
 * a passed row is the outcome verb and nothing else, which is the whole
 * sentence there is to say about it.
 */
function checkRow({ name, run }: CheckRead): CheckRunRow {
  return {
    id: name,
    says: saidOf(run),
    // The Check's declared name, which is what a citation resolves against. The
    // command it ran is on the phase strip's row and is not repeated here.
    identifier: name,
    named: run === undefined ? "queued" : didNotPass(run) ? "failed" : "passed",
    icon: iconOf(run),
    ...(run?.produced === undefined ? {} : { result: run.produced }),
    ...(run?.output_path === undefined ? {} : { output: basename(run.output_path) }),
  };
}

/**
 * What one Check came to, in a sentence.
 *
 * The registry's verb, and what the Check was measured against where it
 * declared one — `Failed — expected exit 0`, with what it produced instead in
 * the column beside it. Nothing here invents a word: `check-outcomes.toml` owns
 * all six.
 */
function saidOf(run: CheckRun | undefined): string {
  if (run === undefined) return NOTHING_HAS_RUN_IT;
  const verb = CHECK_OUTCOME[run.outcome]?.verb ?? run.outcome;
  return run.expected === undefined
    ? asSentence(verb)
    : `${asSentence(verb)} — expected ${run.expected}`;
}

/**
 * The glyph for a Check's row — the `shield-*` family, which is what means
 * gates and Checks. From the registry, never chosen here: `enum-verbs.toml`
 * carries one per outcome and `criterion_verdict_check` carries the one for a
 * tier nothing has reached.
 */
function iconOf(run: CheckRun | undefined): CheckRunRow["icon"] {
  const held =
    run === undefined ? CRITERION_VERDICT_CHECK.not_reached?.icon : CHECK_OUTCOME[run.outcome]?.icon;
  return held ?? undefined;
}

/**
 * The Judge's own row on this list.
 *
 * **`identifierIsAName` is why this row can exist.** A Check's second line is a
 * declared id a citation resolves against; a panel has none — it is one
 * `judge_checks[]` entry on a step — so the row says what was declared, in
 * words, and claims nothing about being joinable.
 */
function judgeRow(step: StepDetail, panels: Panel[]): CheckRunRow | undefined {
  const declared = step.judge_checks;
  if (declared === undefined || declared.length === 0) return undefined;
  const identifier = declared.map(judgeOf).join(" · ");
  if (panels.length === 0) {
    // A call that is out right now is not the same fact as a panel nothing has
    // asked yet, and `judging` is the only thing on the wire that says so.
    return {
      id: JUDGE_ROW,
      says: step.judging === undefined ? NOT_ASKED_YET : ASKING_NOW,
      identifier,
      identifierIsAName: true,
      named: step.judging === undefined ? "queued" : "running",
    };
  }
  const refused = panels.filter((panel) => panel.verdict === "not_met").length;
  return {
    id: JUDGE_ROW,
    says: countedIn(refused, panels.length),
    identifier,
    identifierIsAName: true,
    named: refused === 0 ? "passed" : "refused",
    icon: CRITERION_VERDICT_JUDGE[refused === 0 ? "met" : "not_met"]?.icon ?? undefined,
  };
}

/**
 * A registry verb as the first word of a sentence.
 *
 * **Case, never a word.** The verbs are `enum-verbs.toml`'s and are written for
 * a mono column; `says` is a sentence in sans, where a lower-case first letter
 * reads as a value rather than as a finding. Nothing else about it changes.
 */
function asSentence(verb: string): string {
  return verb.charAt(0).toUpperCase() + verb.slice(1);
}

/** What the Judge's row is called. Not a Check's name, and never joined to one. */
const JUDGE_ROW = "judge";

/** What a declared Check with no run on this attempt says. */
const NOTHING_HAS_RUN_IT = "Nothing has run this Check yet.";

/** What the Judge's row says before a call has gone out. */
const NOT_ASKED_YET = "Waiting for every Check to finish.";

/** What it says while a call is out. `judging` is what makes the two different. */
const ASKING_NOW = "The panel is answering now.";

/** Where an output goes when it is pressed. Bridge has no viewer of its own. */
const OPENS_WHERE = "Click to open this output in your editor";
