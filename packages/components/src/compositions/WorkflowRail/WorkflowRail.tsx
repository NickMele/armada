import type { LucideIcon } from "lucide-react";
import type { MouseEvent, ReactNode } from "react";
import { useCallback } from "react";
import { CriterionVerdicts, type CriterionVerdict } from "../CriterionVerdicts/CriterionVerdicts";
import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";

/**
 * Workflow rail — what ran, one row per step, on job detail.
 *
 * A rail answers *why is this stuck*, which is job detail's question. A list
 * row answers *which step, how far through, and is it moving*, which is the
 * step bar's. They are not two renderings of one thing.
 *
 * **Sans names work, mono names machinery.** A step is a unit of work with a
 * name, so it reads as one — nouns naming the artifact, in sans. The gate rows
 * beneath it keep mono identifiers, because a Check is a command.
 *
 * **In a rail, background states what the row is and the accent left edge
 * states which row you are on.** The surface is constant, selection adds the
 * edge. Two activity values carry a surface — `stopped` and `failed` — because
 * a glyph only holds while its row is selected, and the row that ended the Job
 * has to stay findable while you read the Check output beside it.
 *
 * **The rail carries the pulse.** Job detail has a rail, so it holds the most
 * specific running mark on the screen and the header's Running badge stays
 * static.
 */

/**
 * One Check beneath a step. A step carries `mechanical_checks[]` and every
 * entry must pass, so a step can have several of these rows.
 */
export type WorkflowRailGate = {
  /**
   * The Check, in mono — its id and its command. A Check is a command, so it
   * is machine-derived and renders as one.
   */
  command: string;
  /**
   * The exit code or the Check's state, in mono. Measured facts speak flatly,
   * so this stays neutral even under a hued step: the step's state is hued,
   * the Check's exit code is measured.
   */
  result?: string;
  /**
   * The glyph, from the `shield-*` family that means gates and checks
   * throughout. An evidence row takes the `file-*` family instead.
   */
  icon?: LucideIcon;
  /** The accessible name for the glyph. */
  iconLabel?: string;
  /**
   * Where the Check's stdout and stderr were written, relative to the
   * repository root. **The path, never the contents** — Bridge does not read
   * the filesystem, and a Check that failed is unreadable without this.
   *
   * Machine-derived, so it is mono and copies on click with no `copy` glyph:
   * the affordance token is the affordance, and a glyph repeated down a rail
   * of gate rows is noise. Absent where the Check wrote no file.
   */
  outputPath?: string;
};

/**
 * One thing that will look at a step beyond its Checks: the semantic tier the
 * workflow declares, and what it takes to advance past the step.
 *
 * **A declaration, never a result.** Both are knowable from the frozen
 * workflow before anything runs, which is the whole reason the row exists — a
 * step that will halt for a person has to say so before it halts, and a rail
 * that drew only the mechanical tier rendered a step with three gates as a
 * step with two. What the Judge then answered arrives as `verdicts`.
 *
 * **Label-only, by rule 1 of the iconography contract.** `circle-*` is
 * reserved to Judge verdicts and `shield-*` to Checks; a declaration is
 * neither, and the label alone is unambiguous, so no glyph is spent. The mark
 * column stays empty so the row still aligns with the gate rows above it.
 */
export type WorkflowRailDeclaration = {
  /**
   * What is declared, in mono beside the gate rows — counts and an enum
   * spelling, which is machinery rather than a sentence about the workflow.
   */
  label: string;
  /** Whether it has been reached yet. Absent once there is more to read. */
  result?: string;
};

export type WorkflowRailStep = {
  id: string;
  /**
   * The step's name, in sans. Nouns naming the artifact: Reproduction, Root
   * cause, Fix, Regression check. Where a workflow supplies no label the
   * `step_id` renders instead, which is honest and useless to scan — see
   * `[workflow-step-human-label]`.
   */
  label: ReactNode;
  /** Whether `label` is a `step_id` rather than a name, so it renders in mono. */
  labelIsAnIdentifier?: boolean;
  activity: StepActivity;
  /**
   * The activity, in words, at the row's trailing edge. Not from the enum→verb
   * map: that map covers Job states, and no vocabulary in the repository
   * carries a verb per step-activity value. Written by the caller until one
   * does. Reported.
   */
  status?: string;
  /**
   * How long the step took — `entered_at` to `updated_at` on the served step.
   * Measured, so it is mono and speaks flatly whatever the row's state is.
   */
  elapsed?: ReactNode;
  /**
   * The last ruling against the step, from `last_verdict`. **Not the same axis
   * as `activity`** — a step retrying after a refusal is `running` in activity
   * and `failed` in verdict at the same moment, which is why one row shows
   * both. Absent until a gate has ruled.
   */
  verdict?: ReactNode;
  /** Which of `passed`, `failed` or `not_reached` the verdict is, for its hue. */
  verdictNamed?: string;
  /**
   * That a person overruled the gate and the step advanced anyway, in words.
   *
   * **A third axis, and it sits beside the verdict rather than replacing it.**
   * `StepDetail.overridden` is served as a field precisely so no surface has to
   * read `advanced` beside `failed` and work the pair out; a rail that drew only
   * one of the two would render a Judge that was overruled as a Judge that
   * cleared the work, which is how a verifier quietly becomes decorative.
   *
   * Absent on every ordinary advance, which writes `passed` and needs nothing
   * said about it.
   */
  overridden?: ReactNode;
  /**
   * The Checks beneath. An empty array is a step with no Check, and it says so
   * in words rather than leaving a gap — an empty slot where a gate row would
   * sit reads as a gate that failed to render.
   */
  gates?: WorkflowRailGate[];
  /**
   * What will look at this step beyond its Checks — the Judge the workflow
   * declares, and the gate the step advances through. Drawn beneath the gate
   * rows and counted with them: a step carrying one of these is not ungated,
   * and must not say it is.
   */
  declarations?: WorkflowRailDeclaration[];
  /**
   * What a step with no Check and no declaration says instead. Defaults to the
   * contract's own sentence.
   */
  ungatedLabel?: ReactNode;
  /**
   * What the Judge answered on this step, beneath the gate rows.
   *
   * **A refusal sits under the step it judged and never on it.** Verdict hue
   * is per criterion and never sums onto the step or the Job, so a red cross
   * can sit under a running step beneath an escalated badge without any of the
   * three contradicting the others. Empty on the steps that ask nothing, which
   * is most of them.
   */
  verdicts?: CriterionVerdict[];
  /**
   * The evidence submission on an ungated step, in mono, sitting where a Check
   * would. A step with no gate still produced something, and the row that says
   * "no check on this step" is where that fact belongs.
   */
  evidence?: { icon?: LucideIcon; iconLabel?: string; label: string };
  /**
   * Whichever row you are on. `--accent-muted` tint and a 2px `--accent` left
   * edge — emphasis, not status. One per rail.
   */
  current?: boolean;
  /**
   * Anything at the row's trailing edge beyond the status word — the hard
   * prerequisite lock, which is its own registry row and is not built here.
   */
  trailing?: ReactNode;
};

export type WorkflowRailProps = {
  steps: WorkflowRailStep[];
  /**
   * The running mark on the current step animates. One per screen, on the
   * thing being read: true on the Job detail open in front of you, false on a
   * rail rendered anywhere a more specific mark is present.
   */
  pulsing?: boolean;
  /** A clipboard write is silent, so the surface confirms every one with a toast. */
  onCopied?: (value: string) => void;
};

/** Gate glyphs are 12px at strokeWidth 2, like every mark below Job level. */
const GATE_ICON = 12;
const GATE_STROKE = 2;

/** Whether the ungated row has an evidence submission to name beside it. */
function named(step: WorkflowRailStep): boolean {
  return (step.evidence?.label ?? "") !== "";
}

export function WorkflowRail({ steps, pulsing = false, onCopied }: WorkflowRailProps) {
  const copy = useCallback(
    (event: MouseEvent<HTMLSpanElement>, value: string) => {
      event.stopPropagation();
      void navigator.clipboard.writeText(value).then(
        // A failed clipboard write is otherwise indistinguishable from a dead
        // element, so the surface is told either way.
        () => onCopied?.(value),
        () => onCopied?.(value),
      );
    },
    [onCopied],
  );

  return (
    <ol className="armada-rail">
      {steps.map((step, i) => {
        const gates = step.gates ?? [];
        // A declaration is a row beneath the step exactly as a Check is, so it
        // counts towards whether the step has anything beneath it at all. The
        // step that halts the Job read "no check on this step" while a person
        // was what it was waiting for, and this is the sum that was missing.
        const declarations = step.declarations ?? [];
        return (
          <li className="armada-rail__step" key={step.id}>
            <div
              className="armada-rail__row"
              data-activity={step.activity}
              data-current={step.current || undefined}
            >
              <StepActivityMark
                activity={step.activity}
                label={step.status ?? step.activity}
                ordinal={i + 1}
                pulsing={pulsing && step.current}
              />
              <span
                className="armada-rail__name"
                data-identifier={step.labelIsAnIdentifier || undefined}
              >
                {step.label}
              </span>
              {step.trailing}
              {step.verdict ? (
                <span className="armada-rail__verdict" data-verdict={step.verdictNamed}>
                  {step.verdict}
                </span>
              ) : null}
              {/* After the verdict and never instead of it. The refusal is
                  still what the gate said; this is only the fact that it did
                  not stand. */}
              {step.overridden ? (
                <span className="armada-rail__overridden">{step.overridden}</span>
              ) : null}
              {step.elapsed ? <span className="armada-rail__elapsed">{step.elapsed}</span> : null}
              {step.status ? <span className="armada-rail__status">{step.status}</span> : null}
            </div>
            {gates.length + declarations.length > 0 ? (
              <ul className="armada-rail__gates">
                {gates.map((gate, g) => (
                  <li className="armada-rail__gate" key={g}>
                    <span className="armada-rail__gate-mark">
                      {gate.icon ? (
                        <gate.icon size={GATE_ICON} strokeWidth={GATE_STROKE} aria-hidden />
                      ) : null}
                      {gate.iconLabel ? (
                        <span className="armada-rail__sr">{gate.iconLabel}</span>
                      ) : null}
                    </span>
                    <span className="armada-rail__gate-command">{gate.command}</span>
                    {gate.result ? (
                      <span className="armada-rail__gate-result">{gate.result}</span>
                    ) : null}
                    {gate.outputPath === undefined ? null : (
                      // The whole path is on the clipboard and in the title
                      // however narrow the row gets: a copy truncated with the
                      // display would be worse than the overflow it fixed.
                      // `data-copies` carries a value rather than standing bare,
                      // the way `Job row (stacked)` writes it.
                      <span
                        className="armada-rail__gate-output"
                        title={gate.outputPath}
                        data-copies="true"
                        onClick={(e) => copy(e, gate.outputPath as string)}
                      >
                        {gate.outputPath}
                      </span>
                    )}
                  </li>
                ))}
                {declarations.map((declared, d) => (
                  // No glyph, and the mark column held open anyway: the row
                  // reads as one of the list it sits in rather than starting a
                  // second one at a different indent.
                  <li className="armada-rail__gate" key={`declared-${d}`}>
                    <span className="armada-rail__gate-mark" />
                    <span className="armada-rail__gate-command">{declared.label}</span>
                    {declared.result ? (
                      <span className="armada-rail__gate-result">{declared.result}</span>
                    ) : null}
                  </li>
                ))}
              </ul>
            ) : (
              // An ungated step says so in words. A step carrying no Check is
              // ordinary rather than exceptional, and a blank would read as a
              // gate that failed to render.
              <ul className="armada-rail__gates">
                <li className="armada-rail__gate" data-ungated>
                  <span className="armada-rail__gate-mark">
                    {step.evidence?.icon ? (
                      <step.evidence.icon size={GATE_ICON} strokeWidth={GATE_STROKE} aria-hidden />
                    ) : null}
                    {step.evidence?.iconLabel ? (
                      <span className="armada-rail__sr">{step.evidence.iconLabel}</span>
                    ) : null}
                  </span>
                  {named(step) ? (
                    <span className="armada-rail__gate-command">{step.evidence?.label}</span>
                  ) : null}
                  {/* With no evidence to name, the sentence takes the command's
                      slot instead of the trailing edge: a phrase alone at the
                      far right reads as unattached to the step above it. */}
                  <span className="armada-rail__gate-ungated" data-alone={named(step) ? undefined : "true"}>
                    {step.ungatedLabel ?? "no check on this step"}
                  </span>
                </li>
              </ul>
            )}
            {step.verdicts === undefined || step.verdicts.length === 0 ? null : (
              <div className="armada-rail__verdicts">
                <CriterionVerdicts rows={step.verdicts} />
              </div>
            )}
          </li>
        );
      })}
    </ol>
  );
}
