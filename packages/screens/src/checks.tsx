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
// **The reading is `gates.ts`'s** — which attempt's runs count. This file
// decides only what a row looks like; a press opens its output on a sheet.

import {
  AssertionSet,
  Button,
  CheckRuns,
  Kbd,
  keyFor,
  type CheckRun as CheckRunRow,
} from "@armada/components";
import type { StepChapter } from "@armada/components";
import { CHECK_OUTCOME, CRITERION_VERDICT_CHECK, CRITERION_VERDICT_JUDGE } from "@armada/components";
import type { CheckRun, StepDetail } from "@armada/protocol";

import { assertedIn, WHAT_THE_SUITE_ASSERTED } from "./asserted";
import { judgeOf } from "./declared";
import { namesChapter } from "./detail-keys";
import { span } from "./duration";
import {
  checksFromAttempt,
  checksOf,
  checksStand,
  didNotPass,
  droneRunOf,
  DRONES_RUN,
  isRunning,
  isWaiting,
  notedFrom,
  outputRunOf,
  runEnded,
  sentenceOf,
  stoppedUndecided,
  waitingBehind,
  type CheckRead,
  type Panel,
} from "./gates";
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
  now: number,
  /** Fleet's own reason the gate could not decide. `verdictsChapter`'s own. */
  undecided?: string,
  /**
   * **Run it here** on a refused Check's row — Journey 9. Opens the run sheet
   * with that Check selected and narrowed. Absent where the caller has no run
   * sheet to send the row to.
   */
  onRunHere?: (checkId: string) => void,
  /** Which Check's output sheet is open, so its row says so. `undefined` for none. */
  openCheckId?: string,
  /**
   * Opens the Check output sheet on this Check — live where the gate is still
   * running it, kept once it has ruled. **The sheet is what decides which**,
   * from `checkSheetOf`; a press here only names the Check.
   */
  onOpenCheck?: (checkId: string) => void,
): Omit<StepChapter, "ordinal"> | undefined {
  // **A Drone's own run draws here until the gate takes the step**, marked as
  // the Drone's in the summary so it never reads as a ruling. #1062.
  const drone = droneRunOf(step);
  const reads = drone ?? checksOf(step);
  if (reads.length === 0) return undefined;

  const rows = reads.map((read) => checkRow(read, now, onRunHere));
  const judge = judgeRow(step, panels, undecided);
  if (judge !== undefined) rows.push(judge);

  // The header act, `o` and the sheet all open the same file, because all
  // three come through one reading. Absent where no Check on this attempt
  // kept an output — a control that opens nothing is the defect `#246` was
  // about with a click added to it.
  const reading = outputRunOf(step);
  const output = reading?.output_path;
  const asserted = assertedIn(step);

  return {
    id: CHECKS_CHAPTER,
    title: "Checks",
    // The same sentence the phase strip's tier stands at, from the same call
    // — with which attempt it is from, where a rerun gate has left it behind
    // the step's own current one.
    summary:
      drone === undefined
        ? notedFrom(checksStand(reads), checksFromAttempt(step))
        : `${DRONES_RUN} · ${checksStand(reads)}`,
    // The preview is the whole list — four rows is not a reading. What has no
    // end is what is behind each row, and pressing one opens the sheet rather
    // than filling this chapter with it. `CheckRuns` only draws the control on
    // a row that has an `output` field, so a queued Check offers nothing to
    // press.
    preview: (
      <CheckRuns
        rows={rows}
        openSaid={OPENS_THE_OUTPUT}
        openId={openCheckId ?? null}
        onOpen={onOpenCheck}
      />
    ),
    // **`AssertionSet` and nothing else.** It is one row per Check the gate
    // decided and it has an end, which is what lets it stay inline — the rule
    // `StepChapter.act` states. A Check's own output has no end and left for
    // the sheet; drawing it here is #1021.
    ...(asserted.length === 0
      ? {}
      : {
          content: <AssertionSet rows={asserted} label={WHAT_THE_SUITE_ASSERTED} />,
          openLabel: OPENS_THE_READING,
        }),
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
 * What one Check's output sheet should read — live where the gate is still
 * writing it, kept once it has ruled, or `undefined` where the Check named has
 * written nothing yet. **The sheet's own question, asked of the same reading
 * `checkRow` draws from**, so a row that offers a press and the sheet the
 * press opens can never disagree about which file is behind it.
 */
export type CheckSheetRead = { kind: "live" | "kept"; kept: string };

export function checkSheetOf(step: StepDetail, checkId: string): CheckSheetRead | undefined {
  const read = checksOf(step).find((one) => one.name === checkId);
  if (read === undefined) return undefined;
  if (read.live?.output_path !== undefined) return { kind: "live", kept: basename(read.live.output_path) };
  if (read.run?.output_path !== undefined) return { kind: "kept", kept: basename(read.run.output_path) };
  return undefined;
}

/**
 * One Check's row.
 *
 * **`says` is the finding and `result` is the measurement.** The wire draws
 * that line itself: `produced` is *"the exit code, the signal, the budget it
 * outran"* and it is **absent on a pass, because a pass measured nothing** — so
 * a passed row is the outcome verb and nothing else, which is the whole
 * sentence there is to say about it.
 *
 * **While the gate runs it, a Check waits or runs, and says which.** The
 * elapsed time is counted here from when it started, against `now`: the wire
 * sends one message when it starts and one when it finishes, never a tick.
 */
export function checkRow(
  read: CheckRead,
  now: number,
  /** **Run it here**, drawn only on a Check that did not pass — Journey 9. */
  onRunHere?: (checkId: string) => void,
): CheckRunRow {
  const { name, run, live } = read;
  if (isWaiting(read)) {
    const behind = waitingBehind(read);
    return {
      id: name,
      says: behind === undefined ? WAITING_TO_START : waitingForRoom(behind),
      identifier: name,
      named: "queued",
      icon: iconOf(undefined),
      result: "waiting",
    };
  }
  if (isRunning(read)) {
    const since = live?.started_at;
    const elapsed = since === undefined ? null : span(since, now);
    return {
      id: name,
      says: elapsed === null ? RUNNING_NOW : `Running for ${elapsed}.`,
      identifier: name,
      named: "running",
      result: "running",
      ...(live?.output_path === undefined ? {} : { output: basename(live.output_path) }),
    };
  }
  // Stopped when another Check failed first, on a Drone's run. It finished
  // nothing, so it takes no hue and says what stopped it. #1062.
  const stoppedBy = live?.stopped_by;
  if (stoppedBy !== undefined) {
    return { id: name, says: `Stopped when ${stoppedBy} did not pass.`, identifier: name, result: STOPPED };
  }
  const failed = run !== undefined && didNotPass(run);
  return {
    id: name,
    says: saidOf(run),
    // The Check's declared name, which is what a citation resolves against. The
    // command it ran is on the phase strip's row and is not repeated here.
    identifier: name,
    named: run === undefined ? "queued" : failed ? "failed" : "passed",
    icon: iconOf(run),
    ...(run?.produced === undefined ? {} : { result: run.produced }),
    ...(run?.output_path === undefined ? {} : { output: basename(run.output_path) }),
    ...(failed && onRunHere !== undefined ? { onRunHere: () => onRunHere(name) } : {}),
  };
}

/**
 * What one Check came to, in a sentence.
 *
 * The registry's verb, and what the Check was measured against where it
 * declared one — `Failed, expected exit 0`, with what it produced instead in
 * the column beside it. Nothing here invents a word: `check-outcomes.toml` owns
 * all six.
 */
export function saidOf(run: CheckRun | undefined): string {
  if (run === undefined) return NOTHING_HAS_RUN_IT;
  const verb = CHECK_OUTCOME[run.outcome]?.verb ?? run.outcome;
  const said =
    run.expected === undefined ? asSentence(verb) : `${asSentence(verb)}, expected ${run.expected}`;
  // **The gate ran nothing for this row.** `run.reused_from_dry_run` is only
  // ever present on a pass — `#1014` — so a person reading a green row can
  // tell one the gate measured itself from one it trusted off the Drone's own
  // ask.
  return run.reused_from_dry_run === undefined ? said : `${said} — ${REUSED_FROM_THE_DRONE}`;
}

/**
 * The glyph for a Check's row — the `shield-*` family, which is what means
 * gates and Checks. From the registry, never chosen here: `enum-verbs.toml`
 * carries one per outcome and `criterion_verdict_check` carries the one for a
 * tier nothing has reached.
 */
export function iconOf(run: CheckRun | undefined): CheckRunRow["icon"] {
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
 *
 * **`reason` is drawn only where the step this row is about was overruled.**
 * `provesItOf` is the one caller that ever has one — it reads a person's own
 * words off the Job's log, scoped to this step — and passing it on a step
 * nobody overruled would be a row claiming an act that did not happen.
 */
export function judgeRow(
  step: StepDetail,
  panels: Panel[],
  undecided?: string,
  /** The reason a person gave for overruling this step, where the log kept one. */
  reason?: string,
): CheckRunRow | undefined {
  const declared = step.judge_checks;
  if (declared === undefined || declared.length === 0) return undefined;
  const identifier = declared.map(judgeOf).join(" · ");
  if (step.overridden && panels.length > 0) {
    const refused = panels.filter((panel) => panel.verdict === "not_met");
    const finding = refused[0]?.refused[0];
    const grounds = finding?.expected ?? finding?.produced ?? finding?.consequence;
    return {
      id: JUDGE_ROW,
      says: countedIn(refused.length, panels.length),
      identifier: `${identifier} · overruled by you`,
      identifierIsAName: true,
      named: "overruled",
      icon: CRITERION_VERDICT_JUDGE.not_met?.icon ?? undefined,
      detail:
        grounds === undefined
          ? undefined
          : reason === undefined
            ? `Not met: ${grounds}`
            : `Not met: ${grounds} You: “${reason}”`,
    };
  }
  if (panels.length === 0) {
    // **Asked and silent outranks queued and running.** `gate_undecided` is a
    // call Fleet already made and could not read back, which is a different
    // fact from a call still out (`judging` present) or one still ahead of the
    // step (`judging` absent) — both of those are `stoppedUndecided`'s false.
    if (stoppedUndecided(step)) {
      return {
        id: JUDGE_ROW,
        says: undecided === undefined ? ASKED_AND_SILENT : sentenceOf(undecided),
        identifier,
        identifierIsAName: true,
      };
    }
    // Three facts and they are never one row: a call out right now, a gate
    // still ahead of its Judge, and a run that ended without ever reaching it.
    // The third used to read as the second, so an attempt handed back a day
    // ago said its Judge was waiting on Checks that had long since failed.
    if (step.judging !== undefined) {
      return { id: JUDGE_ROW, says: ASKING_NOW, identifier, identifierIsAName: true, named: "running" };
    }
    if (runEnded(step)) {
      return { id: JUDGE_ROW, says: NEVER_ASKED, identifier, identifierIsAName: true };
    }
    return {
      id: JUDGE_ROW,
      says: NOT_ASKED_YET,
      identifier,
      identifierIsAName: true,
      named: "queued",
    };
  }
  const refused = panels.filter((panel) => panel.verdict === "not_met").length;
  const stillAsking = panels.filter((panel) => panel.verdict === "asking").length;
  return {
    id: JUDGE_ROW,
    says: countedIn(refused, panels.length, stillAsking),
    identifier,
    identifierIsAName: true,
    // No `named` for an open question — `CheckRunNamed` has no word for it,
    // and a row that is neither passed nor refused takes no hue rather than
    // borrowing one that claims more than is known.
    named: refused > 0 ? "refused" : stillAsking > 0 ? undefined : "passed",
    icon:
      refused > 0
        ? (CRITERION_VERDICT_JUDGE.not_met?.icon ?? undefined)
        : stillAsking > 0
          ? undefined
          : (CRITERION_VERDICT_JUDGE.met?.icon ?? undefined),
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
const NOTHING_HAS_RUN_IT = "Not run yet";

/** What a Check the gate trusted off the Drone's own dry run says. */
const REUSED_FROM_THE_DRONE = "reused from the drone's run";

/** What the Judge's row says before a call has gone out. */
const NOT_ASKED_YET = "Waiting on the Checks";

/** What it says while a call is out. `judging` is what makes the two different. */
const ASKING_NOW = "The panel is answering now.";

/** What it says on a run that ended before its gate ever got to the Judge. */
const NEVER_ASKED = "The gate did not reach the panel.";

/**
 * What the Judge's row says where the gate asked and never read an answer
 * back, and `undecided` carried nothing to say instead. **Never both**: where
 * Fleet's own sentence is present, it stands alone — saying the panel did not
 * answer and then saying why is the same fact twice.
 */
const ASKED_AND_SILENT = "The panel was asked and did not answer.";

/**
 * What a row's own control says. **One sentence for live and kept alike** —
 * the row cannot tell which the sheet will open (`checkSheetOf` decides that
 * from the same reading), and a caption that guessed would be wrong half the
 * time.
 */
const OPENS_THE_OUTPUT = "Click to open this Check's output";

/** What a Check the gate has reached and not started says. */
const WAITING_TO_START = "Waiting to start.";

/** What a Check waiting for room other work holds on the machine says. #1063. */
function waitingForRoom(behind: number): string {
  return `Waiting for room behind ${behind} other ${behind === 1 ? "Check" : "Checks"} on this machine.`;
}

/** What a running Check says where its start will not parse. */
const RUNNING_NOW = "Running now.";

/** What a Check a Drone's run stopped reads as, in the result column. */
const STOPPED = "stopped";

/**
 * What opening the chapter offers.
 *
 * **It names the reading, not the file.** The act on the header line opens the
 * output in an editor and says so; this opens the assertion set, and a control
 * that said "open the output" twice on one line would be two words for two
 * different places.
 */
const OPENS_THE_READING = "Read what the Checks asserted";
