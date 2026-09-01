import type { ReactNode } from "react";
import { useCallback, useState } from "react";
import { FactChip, type FactChipNamed } from "../FactChip/FactChip";
import { PathChip } from "../PathChip/PathChip";
import { StepRow } from "../StepRow/StepRow";
import type { StepActivity } from "../StepActivityMark/StepActivityMark";

/**
 * The run — the workflow as a tree, on the left of job detail.
 *
 * **The chevron opens a step's facts; the row selects it.** The two are
 * separate controls on one row because they answer different questions: the
 * tree holds the short facts a step produced, and the panel beside it holds
 * anything that is a sentence. Building either alone loses that division —
 * a tree that opened into prose is the panel, badly, and a panel with no tree
 * cannot say where in the run you are.
 *
 * **Not `WorkflowRail`.** The rail draws every step's gate rows inline, always,
 * because it was the only place a gate could be read. A step's gates are now
 * the phase strip's, in the panel, where each tier is a control that says what
 * it is waiting on. What is left here is what a step *produced*, *cleared* and
 * *tried* — short facts, closed by default.
 *
 * **Elapsed is a figure, never a chart.** A filled bar reads as progress and a
 * step has no percentage.
 *
 * **An attempt is a row, not a counter.** Attempts beside each other show
 * whether a Drone is trying different things or rephrasing one; a count shows
 * neither.
 *
 * **Waiting, stopped and failed must never look alike.** They are three kinds
 * of stopped: waiting on you is the workflow working, stopped is a Drone that
 * tried and cannot get further, failed is over. `StepActivityMark` owns the
 * glyphs and the stylesheet owns the surfaces; both are the same values the
 * rail uses, so a step renders the same way on either.
 *
 * **The tree does not draw a row.** `StepRow` does, `FactChip` and `PathChip`
 * draw what is under it, and this holds the well and the order. It drew all
 * three itself while those components were also drawing them, which is two
 * answers to one question — and it passed a list number to a step that had not
 * run, where the drawing gives a hollow ring. In a tree of six rows a column
 * of numbers says only that the tree has six rows.
 */

/**
 * A path a step produced. **The basename never truncates at any width**, and
 * the directory recedes and truncates ahead of it — a run of six paths in one
 * column all reading `…/index.ts` is the failure this split exists to prevent.
 */
export type RunTreePath = {
  /** Everything up to and including the final separator. May be empty. */
  directory?: string;
  /** The filename. Never truncated, never abbreviated. */
  basename: string;
  /** What it is, where the path alone does not say — `+61 −4`, `work product`. */
  note?: ReactNode;
};

/**
 * One short fact beneath a step: what it produced, what it cleared, how many
 * attempts it took, what its Checks and its Judge came to.
 *
 * **A fact is a value, never a sentence.** Anything that reads as prose belongs
 * in the panel, where there is room for it — that division is the whole reason
 * the tree and the panel are two regions rather than one.
 */
export type RunTreeFact = {
  /**
   * What the fact is, in sans: `Produced`, `Cleared`, `Attempt 2`, `Checks`,
   * `Judge`. A name, so it reads as one.
   */
  label: ReactNode;
  /**
   * The value, in mono — `2 of 2 passed`, `refused`, `not run`. Machine-derived
   * and reported flatly, so it stays neutral under a hued step unless `named`
   * says which verdict it is.
   */
  value?: ReactNode;
  /**
   * Which of `passed`, `failed`, `advanced` or `refused` the value is, for its
   * hue. Absent on every fact that is only a measurement — which is most of
   * them, and the default is neutral for exactly that reason.
   */
  named?: string;
  /**
   * The paths, where the fact is one. Drawn as their own rows beneath the
   * label rather than folded into `value`, because a path is the one value in
   * the tree that must keep its full basename.
   */
  paths?: RunTreePath[];
};

export type RunTreeStep = {
  id: string;
  /**
   * The step's name, in sans. Nouns naming the artifact — Reproduction, Root
   * cause, Fix, Regression check. Where a workflow supplies no label the
   * `step_id` renders instead, which is honest and useless to scan.
   */
  label: ReactNode;
  /** Whether `label` is a `step_id` rather than a name, so it renders in mono. */
  labelIsAnIdentifier?: boolean;
  activity: StepActivity;
  /**
   * How long the step took, or has been going. Mono, measured, and a figure —
   * see the header note. Absent on a step that has not started.
   */
  elapsed?: ReactNode;
  /**
   * The activity in words — `waiting on you`, `retries spent`. **The mark's
   * accessible name, and nothing visible**: hue and silhouette are the two
   * visible channels and this is the third, because a mark whose only label is
   * its colour says nothing to a screen reader. What a reader *sees* the step
   * is doing is a fact row, where the drawing puts it.
   *
   * Written by the caller: no registry in the repository carries a verb per
   * step-activity value, and inventing one here would be a second vocabulary.
   */
  status?: string;
  /**
   * Whether the step is a hard prerequisite of the workflow definition. Drawn
   * as a `lock` at the row's trailing edge in `--fg-muted`, label only, with no
   * action behind it — the way past a locked step is Pilot.
   */
  locked?: boolean;
  /** What the lock says on hover and to a screen reader. */
  lockedLabel?: string;
  /** The short facts the chevron opens. Empty is a step that produced none. */
  facts?: RunTreeFact[];
  /** What a step with no facts says instead of leaving a blank. */
  factsAbsent?: ReactNode;
  /** Whether this is the step the panel is showing. One per tree. */
  current?: boolean;
  /**
   * Whether the facts start open. **Read once, on mount.** After that the tree
   * holds what the reader opened — see `RunTreeProps.onOpen`.
   */
  factsOpen?: boolean;
};

export type RunTreeProps = {
  steps: RunTreeStep[];
  /**
   * The running mark on the current step animates. One per screen, on the most
   * specific mark present and on the thing being read.
   */
  pulsing?: boolean;
  /**
   * Select a step. The panel beside the tree draws whatever this sets; a tree
   * with no handler is a record being read rather than a surface being acted
   * on, and its rows are not controls.
   */
  onSelect?: (stepId: string) => void;
  /**
   * Told when a step's facts are opened or closed, for a caller that wants to
   * remember it across a remount. **The tree does not need it** — it holds its
   * own open set, because the alternative is every screen holding one.
   */
  onOpen?: (stepId: string, open: boolean) => void;
  /** A clipboard write is silent, so the surface confirms every one. */
  onCopied?: (value: string) => void;
};

/**
 * **Facts stay as the reader left them; selecting a step does not open them.**
 *
 * The alternative — auto-expanding the selected step — was the obvious default
 * and is wrong for the reason the tree exists: a workflow with every step
 * expanded fits no screen, and a reader walking a seven-step run with the arrow
 * keys would open seven. Re-collapsing what a reader opened is worse still. So
 * the chevron is the only thing that opens facts, and `factsOpen` seeds the set
 * once so a caller can open the step that stopped.
 *
 * This answers one of the two questions #186 left open. Recorded here rather
 * than only in a commit, because the next person to touch selection will reach
 * for the same default.
 */
export function RunTree({ steps, pulsing = false, onSelect, onOpen, onCopied }: RunTreeProps) {
  const [open, setOpen] = useState<ReadonlySet<string>>(
    () => new Set(steps.filter((step) => step.factsOpen).map((step) => step.id)),
  );

  const toggle = useCallback(
    (stepId: string) => {
      setOpen((held) => {
        const next = new Set(held);
        if (next.has(stepId)) next.delete(stepId);
        else next.add(stepId);
        onOpen?.(stepId, next.has(stepId));
        return next;
      });
    },
    [onOpen],
  );

  return (
    <ol className="armada-run">
      {steps.map((step) => (
        <li className="armada-run__step" key={step.id}>
          <StepRow
            label={step.label}
            labelIsAnIdentifier={step.labelIsAnIdentifier}
            activity={step.activity}
            status={step.status ?? step.activity}
            elapsed={step.elapsed}
            selected={step.current}
            open={open.has(step.id)}
            onToggle={() => toggle(step.id)}
            onSelect={onSelect === undefined ? undefined : () => onSelect(step.id)}
            locked={step.locked}
            lockedLabel={step.lockedLabel}
            pulsing={pulsing && (step.current ?? false)}
            factsId={`armada-run-facts-${step.id}`}
            factsAbsent={step.factsAbsent}
            facts={(step.facts ?? []).map((fact) => ({
              label: fact.label,
              value: (
                <>
                  {fact.value === undefined ? null : (
                    <FactChip named={fact.named as FactChipNamed | undefined}>{fact.value}</FactChip>
                  )}
                  {(fact.paths ?? []).map((path, p) => (
                    <PathChip
                      key={p}
                      directory={path.directory}
                      basename={path.basename}
                      note={path.note}
                      onCopy={onCopied}
                    />
                  ))}
                </>
              ),
            }))}
          />
        </li>
      ))}
    </ol>
  );
}
