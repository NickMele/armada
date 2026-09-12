import type { ReactNode } from "react";
import { Fragment } from "react";
import {
  JUDGE_FINDING,
  JUDGE_FINDING_LABEL,
  JUDGE_FINDING_SAID,
  type JudgeFindingField,
} from "../../judge-record";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * A refusal, whole — why the work was refused, the lines that rest on,
 * and whether the panel disagreed with itself.
 *
 * The finding is the contract's three fields, not prose: it was one free
 * paragraph called `grounds` until 2026-09-08, a second vocabulary for
 * something `docs/contracts/agent-copy.md` already specifies — a refusal
 * is `expected`, `produced` and `consequence`, each one line, because "a
 * structural rule gets satisfied identically forever and produces twenty
 * interchangeable paragraphs." `CriterionVerdicts` had been drawing the
 * three correctly the whole time, one level down; this component invented
 * the paragraph.
 */

/**
 * The three carry rules a reader can check: `expected` and `produced`
 * must be the same class of artifact, a suite state against a suite
 * state; `expected` names the action and its result together, since it's
 * the half a Drone gets on a retry and `consequence` never reaches it;
 * and each must cite something that could not appear in another Job's
 * summary — a field that will not fit on a line is a finding nobody has
 * made yet.
 *
 * The machinery is in tooltips, not standing sections: stacking six
 * labelled prose blocks to explain a split, a met judge's silence and
 * the citations buried the two that matter. The explanation is on hover
 * now, against the thing it explains.
 */

/**
 * The quoted lines are text this record carries, not a pointer into a
 * file: cleanup takes a Job's run log with it, so a verdict citing
 * `test_suite.log:2007` would stop resolving the day the worktree is
 * reclaimed. The refusal quotes what it read; the log is where you go to
 * check it, not where the finding lives.
 *
 * The overlap is the component's reason to exist, and it is not the
 * inputs: the inputs view says what Fleet handed the panel, identical for
 * all three judges; the overlap says what each judge cited, a subset that
 * differs between them — which tells apart a judge catching something the
 * others missed from an ambiguous criterion read differently, with no
 * extra model call. It was labelled *What each judge read* and a reviewer
 * took that for the inputs, fairly; it names the disagreement now.
 */

/**
 * The overlap and the reading are one block: what differs and what that
 * means for the next act is one thought, and splitting it left a generic
 * *What this means* stranded under the block it concluded.
 *
 * Silence is a state and it is stated: a judge that met the criterion
 * produced no text, and an empty region under two refusals would read as
 * a judge whose answer failed to load.
 */

/**
 * What each part of a refusal is, for the reader who has not met one before.
 *
 * **Standing copy, written once.** Every one of these is true of every refusal
 * on every Job — a screen that retyped them would be the second place the
 * veto-only contract is explained, and the two would drift.
 */
const EXPLAINS = {
  split:
    "How many of the panel refused. One veto is a refusal whatever the size — the count never changes the verdict, it says how much of the panel agreed.",
  otherwise:
    "A Judge may only refuse, never grant. A judge with no objection writes nothing, so silence here is a pass and not a missing answer.",
  quoted:
    "The lines the refusal rests on, copied into the verdict. The run log is cleaned up with the Job, so a citation that only pointed at it would stop resolving.",
  cited:
    "What the refusal points into. Each one opens that artifact at the region the Judge named.",
  overlap:
    "What each judge cited — not what it was given. Every judge received the same inputs; whether they pointed at the same lines is what tells a judge that caught something from a criterion people read differently.",
} as const;

export type JudgeRefusalCitation = {
  /** What a selection names — the artifact this citation points into. */
  id: string;
  /** What it is, in the reader's words — `the missing assertion`. */
  says: ReactNode;
  /** Where exactly, in mono — `check:test_suite · 2007–2008`. */
  where: ReactNode;
  onOpen?: (citedId: string) => void;
};

export type JudgeRefusalProps = {
  /**
   * Which criterion was refused — `Refused — 02 Behaviour unchanged`.
   *
   * **Absent inside a `JudgeVerdicts` row**, where the row above is the
   * criterion and restating it is the screen naming one thing twice. It is
   * here for the standalone use, where nothing else says what was refused.
   */
  heading?: ReactNode;
  /**
   * How large the refusal is — `2 of 3 judges`, `grounds shared by j1 and j3`.
   * **Mono, and it never changes the verdict**: it is a confidence signal.
   */
  split?: ReactNode;
  /** What the rest of the panel did — `j2 met it`. */
  otherwise?: ReactNode;
  /**
   * The finding, as the contract's three fields. Each holds one line.
   *
   * **Not prose.** `agent-copy.md` specifies a refusal this way and
   * `CriterionVerdicts` has always drawn it so; a paragraph here was this
   * component inventing a second shape for one record.
   */
  finding: Partial<Record<JudgeFindingField, ReactNode>>;
  /** What the quote is of — `The case that stopped existing: …`. */
  quoteLead?: ReactNode;
  /**
   * The lines the refusal rests on, verbatim. Mono and never glossed — this is
   * quoted output, and the whole point of carrying it here is that it survives
   * the log.
   */
  quoted?: ReactNode;
  /** What was cited, as selectors into the viewer. */
  cited?: JudgeRefusalCitation[];
  /**
   * The line over them. **A word is not enough.** `Cited` alone left a reader
   * asking cited by whom, of what, and whether it was something to press — so
   * the default is a sentence that answers all three, and a caller overrides it
   * only where the sentence would be wrong.
   */
  citedLabel?: ReactNode;
  /**
   * Whether the judges read the same evidence, and what follows — `All three
   * judges cited the same lines. j1 and j3 read “database round trip” as the
   * SQL store and met it; j2 read it as any backing store and refused.`
   */
  overlap?: ReactNode;
  /** What the split and the overlap say to do. A sentence, never a verdict. */
  reading?: ReactNode;
  /**
   * The line over the block.
   *
   * **It says what the block is for, not what it contains.** It read *Where
   * the judges differ* and *What the split means*, which describe the
   * observation — and a reader who has just seen a refusal is not looking for
   * a description of the disagreement, they are looking for what to do with
   * it. The block ends in a recommendation, so the label names that.
   */
  readingLabel?: ReactNode;
  /**
   * The line over the finding. It says this is the reason, which is the one
   * thing an unlabelled block could not.
   */
  findingLabel?: ReactNode;
  /**
   * What a judge that met the criterion produced. Absent where every judge
   * refused, and then there is no silence to explain.
   *
   * **One clause, and only where nothing else has said it.** A grid above this
   * already draws that judge's green mark and `otherwise` already names it, so
   * a third statement is the surface explaining its own contract to somebody
   * who did not ask. It earns its place in the standalone use, where neither of
   * those is on screen.
   */
  silence?: ReactNode;
};

/**
 * **There are no acts on a refusal, deliberately.** Overrule, retry and
 * redispatch act on the step rather than on a criterion — one Job is killed
 * once however many criteria were refused — so they belong to the decision
 * after the story, where one set serves every refusal. A block that carried
 * its own would have offered to kill the same Job three times on a Job with
 * three refusals. What the refusal owes instead is `reading`: what the split
 * and the overlap mean for what to do, in a sentence.
 */

export function JudgeRefusal({
  heading,
  split,
  otherwise,
  finding,
  quoteLead,
  quoted,
  cited,
  citedLabel = "What the refusal points at",
  overlap,
  // Not "What each judge read": a reviewer took that for the inputs, fairly,
  // since "read" is what you do to something you were handed. This names the
  // disagreement, which is what the line is actually about.
  reading,
  readingLabel = "What to do next",
  findingLabel = "Why the Judge refused it",
  silence,
}: JudgeRefusalProps) {
  return (
    <div className="armada-judge-refusal">
      <div className="armada-judge-refusal__block">
        <div className="armada-judge-refusal__head">
          {heading === undefined ? null : (
            <span className="armada-judge-refusal__heading">{heading}</span>
          )}
          {split === undefined ? null : (
            <Tooltip label={EXPLAINS.split}>
              <span className="armada-judge-refusal__split">{split}</span>
            </Tooltip>
          )}
          {otherwise === undefined ? null : (
            <>
              {/* An element rather than the `::before` this used to be. Both
                  facts are wrapped in tooltips now, so they are no longer
                  adjacent siblings and the `+` selector that drew the
                  separator stopped matching — silently, which is how the two
                  sentences ran back into one. */}
              {split === undefined ? null : (
                <span className="armada-judge-refusal__between" aria-hidden>
                  ·
                </span>
              )}
              <Tooltip label={EXPLAINS.otherwise}>
                <span className="armada-judge-refusal__otherwise">{otherwise}</span>
              </Tooltip>
            </>
          )}
        </div>
        {/* The finding, in the contract's order. Each label is hoverable
            because what the field holds is a rule the Judge was held to, and a
            reader checking whether it obeyed needs the rule to hand. */}
        <div className="armada-judge-refusal__finding">
          <span className="armada-judge-refusal__finding-label">{findingLabel}</span>
          <dl className="armada-judge-refusal__fields">
            {JUDGE_FINDING.filter((field) => finding[field] !== undefined).map((field) => (
              // Both halves are children of one grid, so the three values share
              // a column edge. A wrapper per pair would give each its own grid
              // and align nothing — the same reason `CriterionVerdicts` draws
              // its three this way.
              <Fragment key={field}>
                <Tooltip asChild label={JUDGE_FINDING_SAID[field]}>
                  <dt className="armada-judge-refusal__field">{JUDGE_FINDING_LABEL[field]}</dt>
                </Tooltip>
                <dd className="armada-judge-refusal__value" data-field={field}>
                  {finding[field]}
                </dd>
              </Fragment>
            ))}
          </dl>
        </div>
        {quoteLead === undefined ? null : (
          <Tooltip label={EXPLAINS.quoted}>
            <span className="armada-judge-refusal__quote-lead">{quoteLead}</span>
          </Tooltip>
        )}
        {quoted === undefined ? null : (
          <pre className="armada-judge-refusal__quoted">{quoted}</pre>
        )}
        {cited === undefined || cited.length === 0 ? null : (
          <div className="armada-judge-refusal__cited">
            <Tooltip label={EXPLAINS.cited}>
              <span className="armada-judge-refusal__cited-label">{citedLabel}</span>
            </Tooltip>
            <ul className="armada-judge-refusal__citations">
              {cited.map((one) => (
                // A row, not a chip. Boxed chips in a column read as filled-in
                // form fields, which is what a border around a value on a
                // raised ground says to anyone who has used a form. The
                // affordance is the underline on the name, which is what every
                // other selector on this screen carries.
                <li key={`${one.id}-${one.where}`}>
                  <button
                    type="button"
                    className="armada-judge-refusal__citation"
                    onClick={() => one.onOpen?.(one.id)}
                  >
                    <span className="armada-judge-refusal__citation-says">{one.says}</span>
                    <span className="armada-judge-refusal__citation-where">{one.where}</span>
                  </button>
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
      {/* What differs and what that means are one thought, so they are one
          block. Splitting them left a generic *What this means* stranded under
          the block it was the conclusion of. */}
      {overlap === undefined && reading === undefined ? null : (
        <div className="armada-judge-refusal__differ">
          <Tooltip label={overlap === undefined ? EXPLAINS.split : EXPLAINS.overlap}>
            <span className="armada-judge-refusal__differ-label">{readingLabel}</span>
          </Tooltip>
          {overlap === undefined ? null : (
            <p className="armada-judge-refusal__overlap-says">{overlap}</p>
          )}
          {reading === undefined ? null : (
            <p className="armada-judge-refusal__reading-says">{reading}</p>
          )}
        </div>
      )}
      {silence === undefined ? null : (
        <p className="armada-judge-refusal__silence">{silence}</p>
      )}
    </div>
  );
}
