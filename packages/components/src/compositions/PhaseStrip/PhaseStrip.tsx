import type { ReactNode } from "react";
import { useId, useState } from "react";

/**
 * Where this step is — a step's phases and its gate tiers, drawn as one
 * progression: Instructed, Working, Submitted, then its Checks, its Judge, and
 * you.
 *
 * **They are one strip because they are one progression.** A step that has been
 * submitted and is waiting on a Check is not in two places; drawing the phases
 * as a position marker and the gates as a separate row of chips made a reader
 * hold two readings of the same fact.
 *
 * **Every stage is a control.** Opening one states what that stage is, what it
 * is waiting on and where it stands — so the explanation lives where the
 * question is asked, instead of in a page somebody has to remember.
 *
 * **An absent tier is not a failed tier.** A step that declares no Check and no
 * Judge passes neither stage, and `note` is where it says what does advance it.
 * An empty gate drawn greyed out says the gate failed to render.
 *
 * **Checks and the Judge are different in kind, and this is where that shows.**
 * A Check is a command the repository declares and Fleet runs, judged by an
 * exit code, and it may pass or fail. The Judge is a model reading the work
 * against the step's acceptance criteria, and it may only refuse. Drawing them
 * as one row of chips risks reading as one kind of thing, so the standing
 * sentence for each is `SAID` below — written once, here, rather than retyped
 * on every screen that draws a strip.
 */

/**
 * What a stage is. The three phases share one kind because nothing standing is
 * true of one that is not true of the others; the three tiers each have their
 * own, because what each tier is is exactly what a reader opens it to learn.
 */
export type PhaseStageKind = "phase" | "checks" | "judge" | "human";

/**
 * Where a stage stands. **Not a step activity and not a Job status** — those
 * are the tree's and the badge's. Hue comes from the below-Job-level tokens
 * `tokens/status.css` declares and nothing else: cleared is `--step-advanced`,
 * current is `--step-running`, waiting is `--step-waiting`, failed is
 * `--step-failed`, and a stage still ahead takes no hue at all.
 */
export type PhaseStageState = "cleared" | "current" | "waiting" | "failed" | "ahead";

/**
 * One row inside an opened stage: a Check and its exit code, a criterion and
 * its verdict.
 */
export type PhaseStageRow = {
  /** The command or the criterion. */
  label: ReactNode;
  /** What it came to — `exit 0 · 47s`, `met`, `running · 1m 04s`. */
  result?: ReactNode;
  /** Which of `passed`, `failed` or `met`/`not_met` it is, for the hue. */
  named?: string;
  /**
   * Whether `label` is machine-derived. A Check is a command, so it is mono; a
   * criterion is a sentence somebody wrote, so it is not.
   */
  mono?: boolean;
};

export type PhaseStage = {
  id: string;
  /**
   * What the stage is called on the strip — `Instructed`, `build, test`,
   * `Judge · 2 criteria`, `You`. **The Checks tier names its commands rather
   * than counting them** while two fit; past three it counts, and the story's
   * Produced chapter lists them.
   */
  label: ReactNode;
  kind?: PhaseStageKind;
  state: PhaseStageState;
  /**
   * Where it stands, in the caller's words — what it is waiting on, or what it
   * came to. Drawn above the rows, beneath the standing sentence for its kind.
   */
  stands?: ReactNode;
  /** The Checks and their exit codes, or the criteria and their verdicts. */
  rows?: PhaseStageRow[];
  /** Anything the standing sentence and the rows cannot say. */
  detail?: ReactNode;
};

export type PhaseStripProps = {
  stages: PhaseStage[];
  /** The label over the strip. */
  label?: ReactNode;
  /**
   * The sentence beneath — where the step stands overall, and what advances it
   * where no tier does. **This is what an ungated step says instead of an empty
   * gate**, so it is not decoration.
   */
  note?: ReactNode;
  /** Which stage is open on mount. After that the strip holds its own. */
  openId?: string;
  /** Told when a stage is opened, for a caller that wants to record it. */
  onOpen?: (stageId: string | null) => void;
};

/**
 * What each tier *is*, in one sentence, written once.
 *
 * These are standing copy rather than values: they are true of every Job on
 * every workflow, and a screen that retyped them would be the second place the
 * difference between a Check and a Judge is stated. The difference is the whole
 * reason the two tiers are not one row of chips.
 */
const SAID: Record<PhaseStageKind, string | undefined> = {
  phase: undefined,
  checks:
    "Commands this repository declares in its own Manifest. Fleet runs them and the Drone never " +
    "does — a Drone reporting its own tests is a claim, not a result. A command and an exit code: " +
    "nothing to interpret, and the same answer every time it runs.",
  judge:
    "A model reading the work against this step's acceptance criteria, the ones written when the " +
    "Job was dispatched. It answers per criterion, and it never sees the Drone's transcript, so it " +
    "cannot be argued at by the thing it is judging. It can only refuse — a Judge never turns a " +
    "failed Check into a pass.",
  human:
    "The human gate, where the workflow asks for one. Everything mechanical has already cleared by " +
    "the time this tier is lit, so a step sitting here is stopped with nothing wrong. Approve, or " +
    "send it back with a reason. Both are recorded on the Job.",
};

export function PhaseStrip({
  stages,
  label = "Where this step is",
  note,
  openId,
  onOpen,
}: PhaseStripProps) {
  const [open, setOpen] = useState<string | null>(openId ?? null);
  // Two strips on one page is the gallery, every day. A fixed id would point
  // every stage on the second strip at the first strip's panel.
  const panelId = useId();
  const shown = stages.find((stage) => stage.id === open) ?? null;

  function toggle(stageId: string): void {
    const next = open === stageId ? null : stageId;
    setOpen(next);
    onOpen?.(next);
  }

  return (
    <section className="armada-phases">
      {label === undefined ? null : <span className="armada-phases__label">{label}</span>}

      <ol className="armada-phases__strip">
        {stages.map((stage) => (
          <li className="armada-phases__stage" key={stage.id}>
            <button
              type="button"
              className="armada-phases__control"
              data-state={stage.state}
              data-open={open === stage.id || undefined}
              aria-expanded={open === stage.id}
              aria-controls={panelId}
              onClick={() => toggle(stage.id)}
            >
              {stage.label}
            </button>
          </li>
        ))}
      </ol>

      {/* One open at a time, and it opens in place beneath the strip rather
          than as a popover: what a tier is is read alongside where the step is,
          and a layer over the strip would cover the thing being explained. */}
      <div className="armada-phases__open" id={panelId} hidden={shown === null}>
        {shown === null ? null : (
          <>
            <div className="armada-phases__open-head">
              <span className="armada-phases__open-name">{shown.label}</span>
              {shown.stands === undefined ? null : (
                <span className="armada-phases__open-stands" data-state={shown.state}>
                  {shown.stands}
                </span>
              )}
            </div>

            {SAID[shown.kind ?? "phase"] === undefined ? null : (
              <p className="armada-phases__said">{SAID[shown.kind ?? "phase"]}</p>
            )}

            {shown.rows === undefined || shown.rows.length === 0 ? null : (
              <ul className="armada-phases__rows">
                {shown.rows.map((row, r) => (
                  <li className="armada-phases__row" key={r}>
                    <span className="armada-phases__row-label" data-mono={row.mono || undefined}>
                      {row.label}
                    </span>
                    {row.result === undefined ? null : (
                      <span className="armada-phases__row-result" data-named={row.named}>
                        {row.result}
                      </span>
                    )}
                  </li>
                ))}
              </ul>
            )}

            {shown.detail === undefined ? null : (
              <div className="armada-phases__detail">{shown.detail}</div>
            )}
          </>
        )}
      </div>

      {note === undefined ? null : <p className="armada-phases__note">{note}</p>}
    </section>
  );
}
