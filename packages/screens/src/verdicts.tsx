// Chapter five — the panel's answer, as criteria against judges.
//
// **A panel's output is not a stream, so it is not drawn as one.** Every other
// Check writes a transcript; a panel writes verdicts, and under the veto-only
// contract a judge that meets a criterion writes nothing at all. The grid is
// the artifact, and a refusal opens under the row it refuses.
//
// **The reading is `gates.ts`'s.** Which attempt counts, how the members of a
// panel group onto one criterion and whether one veto refuses it are decided
// there, once, for this chapter and for the phase strip. What is decided here
// is only what the rows look like.
//
// **What the wire has not got is not drawn.** `JudgeVerdicts` takes a
// `measured` band for a criterion a Check settled, and nothing joins a
// criterion to a Check — so no measured row is built, and only criteria the
// panel actually answered get a row. `JudgeRefusal.overlap` takes the citation
// sets that say where two judges' readings met and where they parted; `cited`
// carries each member's list and nothing computes the intersection.
//
// **Two regions under the grid, and each is scoped where its record is.**
// `JudgeCitations` is one list for the step, because a row names both the judge
// and the criterion. `JudgeInputs` is one block per criterion inside that
// criterion's own disclosure, because a brief is per criterion — every member
// of one panel answers one brief and two criteria answer two, so a digest
// folded over the step would differ for a reason that says nothing.

import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import {
  JudgeCitations,
  JudgeInputs,
  JudgeRefusal,
  JudgeVerdicts,
  type JudgeMark,
  type JudgeVerdictRow,
  type StepChapter,
} from "@armada/components";
import { CRITERION_VERDICT_JUDGE } from "@armada/components";
import type { Judged, StepDetail } from "@armada/protocol";

import { citationsOf, givenTo } from "./cited";
import {
  askedOf,
  judgesSaid,
  NOT_REACHED,
  panelSizeOf,
  sentenceOf,
  stoppedUndecided,
  type Panel,
} from "./gates";
import { basename, howThePanelWent, openKept, type Opens } from "./phases";

/** Which chapter the Verdicts are, so the keyboard can name it. */
export const VERDICTS_CHAPTER = "verdicts";

/**
 * Chapter five, or none.
 *
 * **Drawn on a step that asks a Judge, whatever it has answered.** A panel that
 * has not been reached is not an empty region: the chapter says so and says how
 * many criteria are coming, which is a fact about a running Job. A step that
 * declares no criteria and has answered none draws nothing at all — a
 * `judge_checks[]` entry asking none only looks for gaming, and what that finds
 * arrives as `flagged` rather than as a verdict.
 */
export function verdictsChapter(
  step: StepDetail,
  panels: Panel[],
  opens: Opens,
  /**
   * Fleet's own sentence for why the gate could not decide, where this step's
   * current attempt is the one it is about. `stuck.undecided` — see its own
   * doc for why it is `undefined` on every other stop.
   */
  undecided?: string,
): Omit<StepChapter, "ordinal"> | undefined {
  const declared = step.judge_checks;
  if (declared === undefined || declared.length === 0) return undefined;
  const asked = askedOf(step);
  if (asked === 0 && panels.length === 0) return undefined;

  if (panels.length === 0) {
    // **Asked and silent is not the same absence as not asked yet.** Both
    // arrive with `judged` empty for this attempt, and only the trigger tells
    // them apart: `gate_undecided` is a call Fleet already made and could not
    // read back, not a call still ahead of the step.
    if (stoppedUndecided(step)) {
      return {
        id: VERDICTS_CHAPTER,
        title: "Verdicts",
        // The registry's own word for the same mark on a single criterion row
        // — `JudgeVerdicts`' `MEANS.gate_undecided` — kept short for the
        // column it sits in beside "not reached" and "1 of 1 criterion met".
        summary: COULD_NOT_READ,
        preview: askedAndSilent(undecided),
      };
    }
    return {
      id: VERDICTS_CHAPTER,
      title: "Verdicts",
      summary: NOT_REACHED,
      // A sentence rather than an empty grid. What is coming is knowable — the
      // criteria were frozen at dispatch — and saying so is what keeps this
      // from reading as a table that failed to load.
      preview: notAskedYet(asked),
    };
  }

  const refused = panels.filter((panel) => panel.verdict === "not_met").length;
  const size = panelSizeOf(step, panels);
  // Every pointer the panel made, across criteria. It is one list rather than
  // one per criterion because a row names both — `j2 · 02` — and because what
  // it is for is reading two criteria's citations against each other.
  const citations = citationsOf(panels, size, (path) =>
    openKept(opens, { kept: path, what: "brief" }),
  );
  return {
    id: VERDICTS_CHAPTER,
    title: "Verdicts",
    summary: countedIn(refused, panels.length),
    // The grid is its own disclosure, so the chapter has no second one.
    preview: (
      <>
        <JudgeVerdicts
          judges={judgeColumns(size)}
          glyphs={GLYPHS}
          rows={panels.map((panel) => verdictRow(panel, size, opens))}
        />
        {/* Under the grid, and on two conditions. **Only where something was
            refused**, because a panel that met every criterion wrote no prose
            to quote out of and a note saying so under every passing step is
            this screen explaining the veto-only contract to somebody who did
            not ask. **And only where the record was kept**: `citationsOf`
            answers `null` for a Fleet that recorded none, and the note below
            is a claim about how the refusals were worded — which is a claim
            nobody could make about rows nobody asked. */}
        {refused === 0 || citations === null ? null : (
          <JudgeCitations label={CITED_LABEL} rows={citations} emptyNote={QUOTED_NOTHING} />
        )}
      </>
    ),
  };
}

/**
 * The panel's shape, as the grid's header.
 *
 * **A count, not a roster.** `Judged.member` is a position and never a person,
 * so the columns are the positions. At one judge there is no position to name,
 * and the column says what the column is — the same silence `panel_size` keeps
 * by being absent at one.
 */
function judgeColumns(size: number): string[] {
  if (size < 2) return [ONE_JUDGE];
  return Array.from({ length: size }, (_, at) => `j${at + 1}`);
}

/** One criterion's row in the grid. */
function verdictRow(panel: Panel, size: number, opens: Opens): JudgeVerdictRow {
  const refusal = refusalOf(panel, size, opens);
  return {
    ...(panel.ordinal === undefined ? {} : { ordinal: panel.ordinal }),
    criterionId: panel.criterionId,
    // The criterion's own words where the Job carries them, and its id where it
    // does not. `text` is left empty: the wire carries one sentence per
    // criterion rather than a short name and a description, and cutting one
    // string in two would be this screen inventing the half it did not get.
    name: panel.criterion?.text ?? panel.criterionId,
    marks: panel.members.map(markOf),
    ...(panel.refused.length === 0
      ? {}
      : {
          // On the closed row too, so several refusals can be triaged without
          // opening one. Silent at a panel of one, where a split would be a
          // count nobody asked for. The same sentence the strip's row carries.
          ...(panel.members.length < 2
            ? {}
            : { split: howThePanelWent(panel.verdict, panel.refused.length, panel.members.length) }),
          ...(refusal === undefined ? {} : { refusal }),
        }),
  };
}

/**
 * One judge's mark. `met` and `not_met` are the two the wire spells; anything
 * else is a value this build has no word for, and `gate_undecided` is the mark
 * that says exactly that rather than a guess in either direction.
 */
function markOf(one: Judged): JudgeMark {
  if (one.verdict === "met") return "met";
  return one.verdict === "not_met" ? "not_met" : "gate_undecided";
}

/**
 * What opens under a refused criterion, or nothing.
 *
 * **One block per set of grounds, not one per judge.** Two judges that refused
 * for the same reason wrote the same three fields, and drawing that twice is
 * the screen saying one thing twice; two that refused for different reasons are
 * two findings, and a reader deciding whether to overrule needs both.
 *
 * **Nothing where the panel recorded a verdict and no grounds.** The component
 * says why: a chevron opening an empty region would promise a reading nobody
 * wrote. The marks stay, and they are the honest whole of what is known.
 */
function refusalOf(panel: Panel, size: number, opens: Opens): ReactNode {
  const grounds = groupedGrounds(panel.refused);
  const met = panel.members.filter((one) => one.verdict === "met");
  const panelled = panel.members.length > 1;
  const given = givenTo(panel);
  const drawn = grounds
    .map((group, at) => {
      const first = group[0] as Judged;
      const finding = {
        ...(first.expected === undefined ? {} : { expected: first.expected }),
        ...(first.produced === undefined ? {} : { produced: first.produced }),
        ...(first.consequence === undefined ? {} : { consequence: first.consequence }),
      };
      const briefs = [...new Set(group.map((one) => one.brief_path))].filter(
        (path): path is string => path !== undefined,
      );
      if (Object.keys(finding).length === 0 && briefs.length === 0) return null;
      return (
        <JudgeRefusal
          key={group.map((one) => one.member ?? 1).join("-")}
          {...(panelled ? { split: splitSaid(panel, group, grounds.length) } : {})}
          // Once, on the first block. A judge that met the criterion produced
          // no text, and an empty region under a refusal would read as an
          // answer that failed to load — but a third statement of it is the
          // surface explaining its own contract to somebody who did not ask.
          {...(at === 0 && panelled && met.length > 0
            ? { otherwise: `${judgesSaid(met)} had no objection` }
            : {})}
          finding={finding}
          // The brief, as a citation. It is the one artifact a verdict carries
          // a path to, and a reader deciding whether to overrule needs what the
          // Judge was asked as well as what it said — that pair is what
          // separates a bad Judge from a bad brief. The label is overridden
          // because the default says the refusal points at it, and it does not:
          // the brief is the input, not the evidence.
          {...(briefs.length === 0
            ? {}
            : {
                citedLabel: BRIEF_LABEL,
                cited: briefs.map((path) => ({
                  id: path,
                  says: BRIEF_SAYS,
                  where: basename(path),
                  onOpen: () => openKept(opens, { kept: path, what: "brief" }),
                })),
              })}
        />
      );
    })
    .filter((block) => block !== null);
  // What this criterion's panel was handed, under the grounds it produced.
  // **Per criterion, because a brief is per criterion** — every member of one
  // panel answers one brief and two criteria answer two, so a digest folded
  // over the step would differ for a reason that says nothing about whether
  // the judges agreed on their input.
  //
  // **No segmented control at one judge**, which is what `each` being absent
  // draws: a control offering one segment is a promise nobody kept. The
  // assurance goes with it — there is no panel for the objects to be identical
  // across, and saying so would be a claim about a comparison nobody made.
  const handed =
    given === null ? null : (
      <JudgeInputs
        key="given"
        label={size < 2 ? GIVEN_TO_ONE : GIVEN_TO_PANEL}
        rows={given.rows}
        {...(size < 2 ? {} : { each: given.each })}
        {...(size < 2
          ? {}
          : given.identical
            ? { identical: HANDED_ALIKE }
            : { differed: HANDED_APART })}
      />
    );
  if (drawn.length === 0 && handed === null) return undefined;
  return [...drawn, handed];
}

/**
 * The refusing members, grouped by the grounds they gave.
 *
 * Keyed on the three fields the contract names. `agent-copy.md` holds each of
 * them to one line, so the whole of a finding is comparable as a string.
 */
function groupedGrounds(refused: readonly Judged[]): Judged[][] {
  const held = new Map<string, Judged[]>();
  for (const one of refused) {
    const key = [one.expected, one.produced, one.consequence].join(" ");
    const already = held.get(key);
    if (already === undefined) held.set(key, [one]);
    else already.push(one);
  }
  return [...held.values()];
}

/**
 * How large this set of grounds is.
 *
 * **A confidence signal and never the verdict** — one veto is a refusal
 * whatever its size. With one set of grounds the sentence is the panel's; with
 * more than one it names which judges shared each, because *2 of 3 refused*
 * over two different findings would be the same line twice.
 */
function splitSaid(panel: Panel, group: readonly Judged[], sets: number): string {
  if (sets > 1) {
    return group.length > 1
      ? `grounds shared by ${judgesSaid(group)}`
      : `${judgesSaid(group)} refused`;
  }
  const said = `${panel.refused.length} of ${panel.members.length} judges refused`;
  return panel.refused.length > 1 ? `${said}, on the same grounds` : said;
}

/**
 * `2 of 2 criteria met`, `1 of 4 criteria refused`.
 *
 * **Criteria, never calls.** A panel of three answering two criteria sends six
 * rows, and counting those would report `1 of 6 refused` for a step where one
 * criterion of two was refused. Exported because the Judge's row on the Checks
 * list says the same thing, and two counts that could disagree is the drift
 * `gates.ts` exists to stop.
 */
export function countedIn(refused: number, criteria: number): string {
  const said = criteria === 1 ? "criterion" : "criteria";
  return refused === 0
    ? `${criteria} of ${criteria} ${said} met`
    : `${refused} of ${criteria} ${said} refused`;
}

/**
 * What the chapter says before the panel has answered anything.
 *
 * **It says what is coming rather than that there is nothing.** The criteria
 * are frozen at dispatch, so how many will be asked is knowable now — which is
 * what makes this a state and not a table that failed to load.
 */
function notAskedYet(asked: number): string {
  if (asked === 0) return "The panel has not been asked anything on this step yet.";
  const counted = asked === 1 ? "one criterion" : `${asked} criteria`;
  return (
    `The panel has not been asked anything on this step yet. The ${counted} it will be ` +
    "asked were frozen when the Job was dispatched, and are in the brief above."
  );
}

/**
 * The header over an asked-and-silent chapter. `JudgeVerdicts`' own word for
 * the same mark on a single criterion row — see `MEANS.gate_undecided` —
 * rather than a phrase invented for this column alone.
 */
const COULD_NOT_READ = "could not read the artifact";

/**
 * What the chapter says where the panel was asked and the gate never read an
 * answer back — never "not asked yet", which is a call still ahead of the
 * step rather than one already made.
 *
 * **Fleet's own reason stands alone where it kept one.** Saying the panel did
 * not answer and then saying why is the same fact twice; `undecided` already
 * names what the gate could not read, so it replaces the shorter sentence
 * rather than following it.
 */
function askedAndSilent(undecided: string | undefined): string {
  return undecided === undefined
    ? "The panel was asked and did not answer."
    : sentenceOf(undecided);
}

/**
 * The verdict glyphs, from the `circle-*` family the Judge owns. Read off the
 * registry rather than imported from lucide here — a glyph chosen in a screen
 * is the second place a verdict is drawn.
 */
const GLYPHS: Partial<Record<JudgeMark, LucideIcon>> = {
  ...(CRITERION_VERDICT_JUDGE.met?.icon ? { met: CRITERION_VERDICT_JUDGE.met.icon } : {}),
  ...(CRITERION_VERDICT_JUDGE.not_met?.icon
    ? { not_met: CRITERION_VERDICT_JUDGE.not_met.icon }
    : {}),
};

/** The header over a single judge's column. There is no position to name. */
const ONE_JUDGE = "Judge";

/** The line over the brief, in place of the default, which names evidence. */
const BRIEF_LABEL = "What this verdict answers";

/** What the brief is, in the reader's words. */
const BRIEF_SAYS = "The whole brief the Judge was given";

/** The line over the citation list. It is every pointer, not only a refusal's. */
const CITED_LABEL = "What the panel quoted";

/**
 * What a refused step whose refusals quoted nothing says.
 *
 * **A fact about the refusals, not an apology for the list.** A refusal may
 * argue in the Judge's own words, and that is a complete refusal under
 * `docs/concepts/judge.md` rule 4 — what it is not is one a reader can follow
 * back into the brief without reading the brief.
 */
const QUOTED_NOTHING =
  "Nothing was quoted. The refusals above describe the work rather than " +
  "quoting it, so there is nothing to point at in the brief.";

/** The line over one criterion's inputs, at a panel. */
const GIVEN_TO_PANEL = "What each judge was handed";

/** The same line where one judge answered and there is nothing to compare. */
const GIVEN_TO_ONE = "What the Judge was handed";

/** That the panel was a panel. */
const HANDED_ALIKE = "Every judge was handed the same brief";

/** That it was not, which is the answer this view exists to be able to give. */
const HANDED_APART = "The judges were not handed the same brief";
