// Where a step is — its phases and its gate tiers, from the one `StepDetail`
// the panel is showing.
//
// **The phases are derived and the tiers are served.** `job_steps.state` says
// whether the step is instructed, working or submitted; `checks`, `check_runs`,
// `judge_checks`, `judged` and `advance_gate` say what looks at it afterwards.
// Nothing here invents a tier: a step declaring no Check and no Judge draws
// three stages and a sentence saying what does advance it, which is the whole
// of "an absent tier is not a failed tier".
//
// # The three records open here, each beside the thing it is evidence for
//
// A person reads a Check's output, the document the Judge read and the question
// it was asked **because a verdict went against them**, and until `#246` all
// three were a path on a screen that nothing opened — the owner hit it on a
// real Job and lost the thread on a race the log would have named.
//
// They are not a fourth region. What a Check printed belongs on the Check's own
// row, the brief belongs on the criterion it answers, and the deliverable
// belongs to Submitted, which is the phase where the Drone handed the work
// over. **Split up rather than listed together**, because the question being
// asked is never "what files does this step have" — it is "why did that one
// say that", and the answer is next to the thing that said it.
//
// # A `.tsx` because the rows carry a control now
//
// `PhaseCardRow.label`, `result` and `cited` are all `ReactNode`, so the strip
// takes an element without any change to the component. That is the whole
// reason this file grew an extension: the alternative was a path on a row and a
// second surface to open it from.

import type { ReactNode } from "react";
import { Button } from "@armada/components";
import type { PhaseStage, PhaseStageRow, PhaseStripProps } from "@armada/components";

import { ADVANCE_GATE, CHECK_ADVANCES, CHECK_OUTCOME, CRITERION_VERDICT_JUDGE } from "@armada/components";
import type { CheckRun, Criterion, Judged, StepDetail } from "@armada/protocol";
import type { Kept } from "@armada/protocol";
import { commandOf, nameOf } from "./declared";
import { openArtifact, type OpenArtifact } from "./opening";

/**
 * How a record is opened, and where a refusal is said.
 *
 * **Handed in rather than reached for, and required.** This file builds data
 * for a strip and holds no Job id; the Job is the panel's, and the sentence a
 * failed open writes has to reach a toast the panel owns. Required because the
 * paths were already on screen and unopenable, and an optional handler is how
 * a surface quietly goes back to that.
 */
export type Opens = {
  jobId: string;
  /**
   * Ask the host to open a file. **Carried here rather than reached for**: a
   * screen says what an unopenable file means and the process with a
   * filesystem does the opening, and the two sit on different layers.
   */
  open: OpenArtifact;
  /** Say a sentence to the person. Called only where an open did not happen. */
  onSaid: (sentence: string) => void;
};

/**
 * One record, as a control that opens it.
 *
 * **The basename is the label and the whole path is the title.** A column of
 * these clipped from the right would all read `.armada/checks/01JOB/…`, which
 * is a run of rows saying nothing; the basename is the half a person came to
 * read, and it is the half they were hunting for — `implement.3.3.log`.
 *
 * **Ghost, because opening a file decides nothing.** Nothing about the Job
 * moves and nothing is spent, so it carries the same weight as the chapter act
 * beside it rather than the weight of an act on the work.
 */
export function Opening({ path, what, opens }: { path: string; what: Kept["what"]; opens: Opens }) {
  const kept: Kept = { kept: path, what };
  return (
    <Button
      variant="ghost"
      size="sm"
      title={path}
      onClick={(event) => {
        // The row sits inside a card the strip pins on a click. Without this
        // the open would also be a press on the stage behind it.
        event.stopPropagation();
        void openArtifact(opens.open, opens.jobId, kept).then((because) => {
          if (because !== null) opens.onSaid(because);
        });
      }}
    >
      {basename(path)}
    </Button>
  );
}

/** The last segment of a repository-relative path. The informative half. */
function basename(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}

/** The gate value whose whole meaning is "the Checks are the whole gate". */
const AUTO = "auto";
/** The gate value that holds the Job for a person whatever the machines said. */
const HUMAN = "human_always";
/** The gate value where a model is the last thing to look, and no person is. */
const JUDGE_ONLY = "auto_if_judge_passes";

/**
 * The strip for one step.
 *
 * **The three phases are one progression with the tiers, not a marker beside
 * them.** A step that has been submitted and is waiting on a Check is not in
 * two places, and drawing them apart made a reader hold two readings of one
 * fact.
 */
export function phasesOf(
  step: StepDetail,
  criteria: Criterion[],
  opens: Opens,
): PhaseStripProps {
  const submitted = HAS_SUBMITTED.has(step.state);
  const working = step.state === "running" || step.state === "retrying";
  const kept = keptRows(keptOf(step, opens));
  const stages: PhaseStage[] = [
    {
      id: "instructed",
      label: "Instructed",
      state: step.state === "not_started" ? "ahead" : "cleared",
      stands: step.state === "not_started" ? "not reached" : undefined,
      detail: "Armada wrote the step's brief and the Drone opened with it.",
    },
    {
      id: "working",
      label: "Working",
      state: working ? "current" : submitted ? "cleared" : "ahead",
      detail: "The Drone has the step and is taking turns against it.",
    },
    {
      id: "submitted",
      label: "Submitted",
      state: submitted ? "cleared" : "ahead",
      stands: submitted ? undefined : "nothing submitted",
      // **The document the Judge read hangs off Submitted, not off Judge.** It
      // is what the Drone handed over, and it is the same document however many
      // criteria were asked about it — putting it on the Judge card would make
      // one artifact look like a property of the tier that read it. A step that
      // declares no deliverable keeps none and this stays a marker, which is
      // the rule the phases already follow: a card with nothing to say is not
      // drawn as a control that opens an empty box.
      opens: kept.length > 0 || undefined,
      rows: kept,
      cardNote:
        kept.length === 0
          ? undefined
          : "What the Judge was shown, copied out as it was read. It outlives the checkout, " +
            "so a step argued about after a clean can still be argued about.",
      detail:
        "The Drone reported the step complete through the Evidence tool. What it claimed is a " +
        "signal; everything that gates is computed on Fleet's side.",
    },
  ];

  const checks = checksStage(step, opens);
  if (checks !== undefined) stages.push(checks);

  const judge = judgeStage(step, criteria, opens);
  if (judge !== undefined) stages.push(judge);

  // **`You` closes the strip, always.** It is the last thing that can hold a
  // step, and a strip that stopped at the Judge said a step could only ever be
  // waiting on a machine. Where the workflow asks for no person it is still
  // drawn and still last, under its own label — an absent tier is not a failed
  // tier, and a tier that can never ask is not one that has not asked yet.
  stages.push(youStage(step));

  return { stages, note: noteOf(step, checks !== undefined, judge !== undefined) };
}

/** The step states that mean the Drone has handed the work over. */
const HAS_SUBMITTED: ReadonlySet<string> = new Set([
  "awaiting_human",
  "advanced",
  "stopped",
]);

/**
 * The Checks tier, or none.
 *
 * **The tier names its commands rather than counting them while two fit.** Past
 * three it counts, because six commands on one control is a paragraph in a
 * strip.
 */
function checksStage(step: StepDetail, opens: Opens): PhaseStage | undefined {
  const declared = step.checks;
  if (declared === undefined || declared.length === 0) return undefined;

  const rows: PhaseStageRow[] = declared.map((check) => {
    const run = step.check_runs.find((ran) => ran.name === nameOf(check));
    return {
      label: commandOf(check),
      mono: true,
      result: run === undefined ? "not run" : resultOf(run),
      named: run === undefined ? undefined : didNotPass(run) ? "failed" : "passed",
      // **The output opens from the row of the Check that wrote it.** An exit
      // code is the whole of what a failed Check said here, and the sentence
      // that says why is in the file — which was a path on this screen that
      // nothing opened.
      cited:
        run?.output_path === undefined ? undefined : (
          <Opening path={run.output_path} what="check" opens={opens} />
        ),
    };
  });

  const runs = step.check_runs;
  const failed = runs.filter(didNotPass);
  const label =
    declared.length > 2
      ? `${declared.length} Checks`
      : declared.map((check) => nameOf(check)).join(", ");

  return {
    id: "checks",
    label,
    kind: "checks",
    state: failed.length > 0 ? "failed" : runs.length === declared.length ? "cleared" : runs.length > 0 ? "current" : "ahead",
    stands:
      runs.length === 0
        ? "not run"
        : failed.length > 0
          ? `${failed.length} of ${declared.length} did not pass`
          : `${runs.length} of ${declared.length} passed`,
    rows,
  };
}

/**
 * The Judge tier, or none.
 *
 * **A cleared tier reports the criteria it met**, because that is the reason to
 * trust it, and a declared one says how many it will answer — which is what it
 * will report against.
 */
function judgeStage(
  step: StepDetail,
  criteria: Criterion[],
  opens: Opens,
): PhaseStage | undefined {
  const declared = step.judge_checks;
  if (declared === undefined || declared.length === 0) return undefined;

  const asked = declared.reduce((sum, judge) => sum + judge.criteria, 0);
  const rows: PhaseStageRow[] = step.judged.map((judged) => {
    const criterion = criteria.find((held) => held.criterion_id === judged.criterion_id);
    return {
      // The criterion's own text where the Job carries it, and its id where it
      // does not. A criterion is a sentence somebody wrote, so it is not mono;
      // an id that could not be joined is machine-derived, so it is.
      label: criterion?.text ?? judged.criterion_id,
      mono: criterion === undefined || undefined,
      // The registry's verb: `no objection`, `refused`. `not_met` is the wire's
      // key and reads as a field name rather than as a ruling.
      result: CRITERION_VERDICT_JUDGE[judged.verdict]?.verb ?? judged.verdict,
      named: judged.verdict,
      cited: citedOf(judged, opens),
    };
  });

  if (step.judged.length === 0) {
    return {
      id: "judge",
      label: asked === 0 ? "Judge" : `Judge · ${asked} ${asked === 1 ? "criterion" : "criteria"}`,
      kind: "judge",
      state: "ahead",
      stands: step.judging === undefined ? "not reached" : `asking · ${step.judging.look}`,
      rows,
    };
  }

  const met = step.judged.filter((judged) => judged.verdict === "met").length;
  const refused = step.judged.length - met;
  return {
    id: "judge",
    label:
      refused === 0
        ? `Judge · ${met} of ${step.judged.length} met`
        : `Judge · ${refused} of ${step.judged.length} refused`,
    kind: "judge",
    state: refused === 0 ? "cleared" : "failed",
    stands: refused === 0 ? `${met} of ${step.judged.length} met` : `${refused} refused`,
    rows,
  };
}

/**
 * What a refusal cites — what should be seen, what is seen instead, and what
 * that difference does to whoever consumes it.
 *
 * **The whole persuasive content of a refusal.** A criterion id and `not_met`
 * tell a person nothing about their own Job; the override dialog has read these
 * three in full since it was built, and the panel a person reaches that dialog
 * from should not be weaker than the dialog. Absent on a criterion nothing was
 * refused on: there is nothing to cite where nothing was disputed.
 */
function citationOf(judged: Judged): string | undefined {
  const said = [
    judged.expected === undefined ? undefined : `Expected ${judged.expected}`,
    judged.produced === undefined ? undefined : `Found ${judged.produced}`,
    judged.consequence,
  ].filter((part): part is string => part !== undefined);
  return said.length === 0 ? undefined : said.join(" · ");
}

/**
 * What the row cites, and the brief the verdict answers.
 *
 * **The brief is on every row, including the met ones**, which is the rule
 * `CriterionVerdicts` states and the reason it is worth carrying: a Judge that
 * refuses work it should have passed gets argued with the same day, and one
 * that *passes* work it should have refused is the quiet failure — visible only
 * against what it was shown.
 *
 * **Beside the citation rather than instead of it.** The citation is what the
 * Judge said and the brief is what it was asked, and a reader deciding whether
 * to overrule needs both: the pair is what separates a bad Judge from a bad
 * brief. A row with neither cites nothing and draws nothing.
 */
function citedOf(judged: Judged, opens: Opens): ReactNode {
  const said = citationOf(judged);
  if (judged.brief_path === undefined) return said;
  return (
    <>
      {said === undefined ? null : `${said} · `}
      <Opening path={judged.brief_path} what="brief" opens={opens} />
    </>
  );
}

/**
 * One kept document, read once for both surfaces that draw it.
 *
 * **The markup is not shared and should not be.** The strip wants a
 * `PhaseStageRow` with a mark and a hue; the Produced chapter wants two
 * elements in a flex row. What drifts is the reading — the ordering, the
 * `what`, and the *attempt N* label — so that is what this carries.
 */
export type KeptRead = {
  /** The path, which is also the row key: one document per attempt. */
  path: string;
  /** The control that opens it, in the words a person was hunting for. */
  opening: ReactNode;
  /** Which run wrote it — `attempt 3`. */
  attempt: string;
};

/**
 * The documents this step kept, one per run, newest run first.
 *
 * **Per run, because a re-run is a different document.** A step worked three
 * times was judged on three, and a single row would make *the one the Judge
 * read* a guess on the one screen where that question is being asked. Empty on
 * a step that declares no deliverable, and on one whose Judge was never asked —
 * the bytes are copied where the call is built and nowhere else.
 *
 * **Newest run first, on both surfaces.** The wire orders them oldest first,
 * which is what a history wants; this is a person looking at why the last run
 * went the way it did, and the run they are reading about is the one at the
 * top. The Produced chapter reversed for the same reason and said so
 * separately, and two statements of one ordering is how a step retried twice
 * ends up listing its documents two ways. #321.
 */
export function keptOf(step: StepDetail, opens: Opens): KeptRead[] {
  return [...(step.deliverables ?? [])].reverse().map((kept) => ({
    path: kept.path,
    opening: <Opening path={kept.path} what="deliverable" opens={opens} />,
    attempt: `attempt ${kept.attempt}`,
  }));
}

/** The kept documents as rows on the Submitted tier. A path is machine-derived. */
function keptRows(kept: KeptRead[]): PhaseStageRow[] {
  return kept.map((one) => ({
    label: one.opening,
    mono: true,
    result: one.attempt,
    state: "cleared" as const,
  }));
}

/**
 * The human tier. **Always drawn, and named for whether it can ever ask.**
 *
 * A step whose gate is `auto` will never stop for anybody. **That is not the
 * same fact as a tier not yet reached, and it used to draw as one** — both sat
 * `ahead` under the label `You`, so the one question a reader opens this tier
 * with, *can this step ever wait for me*, was answered only on a hover nobody
 * performs on a step that already passed. The distinction was in the data and
 * absent from the chip.
 *
 * **So the never-asks case is its own state and its own label.** `never` says
 * the tier is out of this step's progression rather than ahead of it, and `No
 * one` says who it is waiting for. Neither takes hue: amber is spent on
 * waiting on you, and the pair is told apart by label and glyph the way every
 * same-hue state is. An absent tier is not a failed tier and a missing tier is
 * not an absent one — this tier is still drawn, still last, and still counted.
 *
 * **Three cases, because an absent `advance_gate` is a fourth thing again.**
 * The key is missing where the Job named a workflow this Fleet does not hold,
 * so nothing here knows whether a person is asked, and answering `No one`
 * would be this file deciding a question the wire declined. It stays `You` and
 * unlit, and `noteOf` is what names the missing workflow on screen.
 */
function youStage(step: StepDetail): PhaseStage {
  const gate = step.advance_gate;
  if (gate === AUTO || gate === JUDGE_ONLY) {
    return {
      id: "you",
      label: "No one",
      kind: "human",
      state: "never",
      stands: "this step advances without a person",
      // No `said` and no `detail`. Both lines used to be written here, and
      // that was the defect: the card keyed its standing copy by kind alone,
      // so a caller who omitted them got *amber, not red — it is waiting on
      // you* on a tier that can never wait. `phaseSaid` and `phaseClosesWith`
      // now key by state, and the discipline is the component's rather than
      // every caller's. #320.
    };
  }

  if (gate === undefined) {
    return {
      id: "you",
      label: "You",
      kind: "human",
      state: "ahead",
      stands: "Fleet cannot say",
      // **The one `detail` this file still writes, and `null` is the point.**
      // The card's `ahead` line says this step's gate will ask for a person
      // and has not got that far, which is a claim about a workflow Fleet does
      // not hold. Whether anybody is ever asked is exactly what is not known
      // here, so the card closes with nothing and `noteOf` names the gap.
      detail: null,
    };
  }

  const waiting = step.state === "awaiting_human";
  const named = gate === HUMAN ? undefined : ADVANCE_GATE[gate]?.verb ?? gate;
  return {
    id: "you",
    label: named === undefined ? "You" : `You · ${named}`,
    kind: "human",
    state: waiting ? "waiting" : step.state === "advanced" ? "cleared" : "ahead",
    stands: waiting ? "waiting on you" : "not reached",
    // Where `advance_gate` is a manifest rule, the tier resolved at dispatch
    // from the Manifest's own policy — so two Jobs on one workflow can show
    // different gates. Naming the value is what says why.
    detail:
      gate === HUMAN
        ? undefined
        : `This step's gate is ${gate}, resolved when the Job was dispatched.`,
  };
}

/**
 * The sentence beneath the strip. **This is what an ungated step says instead
 * of an empty gate**, so it is not decoration — a greyed-out tier reads as a
 * gate that failed to render.
 */
function noteOf(step: StepDetail, checks: boolean, judge: boolean): string {
  if (!checks && !judge) {
    return step.checks === undefined
      ? "Fleet cannot say what gates this step, because it does not hold the workflow this Job named."
      : "This step declares no Check and asks no Judge. Its evidence advances it, and nothing else.";
  }
  if (step.state === "awaiting_human") {
    return "Everything mechanical has cleared. Nothing is wrong; the workflow asks for a person here.";
  }
  if (step.state === "running" || step.state === "retrying") {
    return step.check_runs.length === 0
      ? "The Drone is working. Nothing has been submitted, so no gate has been asked anything yet."
      : "The gate has run and the Drone has the step back. The tiers behind it are still ahead, not cancelled.";
  }
  if (step.state === "not_started") return "Nothing has reached this step yet.";
  return "";
}

/** What one Check did, in the registry's own verb. */
function resultOf(run: CheckRun): string {
  const outcome = CHECK_OUTCOME[run.outcome]?.verb ?? run.outcome;
  const measured = [run.expected, run.produced].filter((part) => part !== undefined);
  return measured.length === 0 ? outcome : `${outcome} · ${measured.join(" → ")}`;
}

/** A Check that did not pass, read off the registry's own `advances`. */
function didNotPass(run: CheckRun): boolean {
  return CHECK_ADVANCES[run.outcome] === false;
}
