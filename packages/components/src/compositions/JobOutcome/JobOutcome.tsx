import type { LucideIcon } from "lucide-react";
import type { MouseEvent, ReactNode } from "react";
import { useCallback } from "react";

import { FigureList, type Figure } from "../FigureList/FigureList";

/**
 * Job outcome — what a finished Job produced, one row per part of it.
 *
 * **The Land board is this region grown** (#1542): a Job that has finished is
 * read for how it was answered, what it left on the machine and what it cost,
 * and those were three surfaces before. The parts list is unchanged and every
 * addition is optional, so the region a step draws is the one it always was.
 *
 * **The region a finished Job is opened for.** A Job that stopped is read once,
 * to decide whether to take the work, and the parts of "produced" are the whole
 * of that decision: the branch, the commit, the pull request, the files that
 * changed, the evidence that was submitted.
 *
 * **A part nothing serves keeps its row and says so.** Four of the five are not
 * on the wire yet, and dropping them would draw a finished-looking outcome that
 * is a fifth of one. Each names the operation that would have to serve it, so
 * the hole is a finding rather than a silence — the same reason the screens
 * render a named absence instead of closing up.
 *
 * **Not `Absent` per row.** That treatment is a dashed `--status-completed-failed`
 * frame and it is right for a whole region with nothing in it; four of them
 * inside one region would make a Job that completed read as a Job that broke.
 * The row stays, the value is replaced by the sentence, and the weight drops to
 * `--fg-subtle` — which is what says "not here" without saying "wrong".
 *
 * **Marks run at 12px**, like every other row mark, and the registry sizes
 * `git-branch`, `git-commit-horizontal` and `git-pull-request` at 12 as well. A
 * part the registry has no glyph for keeps its mark column and renders it
 * empty, rather than borrowing a silhouette that means something else.
 */
export type JobOutcomePart = {
  /** What this part is: `Branch`, `Commit`, `Pull request`. Sentence case. */
  name: ReactNode;
  /** The glyph, where the registry has one for this part. */
  icon?: LucideIcon;
  /** The accessible name for the glyph, since the value beside it is machine text. */
  iconLabel?: string;
  /**
   * The value, where it is served. Mono, and it copies on click — a branch
   * name, a commit, a pull request reference are all things that get pasted
   * into a shell.
   */
  value?: string;
  /** What is known beside the value. Mono and neutral, never a count nothing measures. */
  meta?: ReactNode;
  /** Why there is no value. Said in words, never left as a blank. */
  absent?: ReactNode;
  /** A control that opens it. Secondary and unfilled: there is no decision here. */
  action?: ReactNode;
};

/**
 * One thing the Job was held to, and what answered it.
 *
 * **A criterion nothing answered reads "not covered", never green** (#1542).
 * The word is the caller's, out of the draft vocabulary — a verdict written at
 * a call site is the second vocabulary `lib/job-states.js` was deleted for.
 */
export type JobOutcomeCriterion = {
  /** The requester's own words, as the Job froze them. */
  text: ReactNode;
  /** The verb: `no objection`, `refused`, `not covered`. */
  verdict: ReactNode;
  /**
   * The status token stem the verdict takes — `completed-success`,
   * `completed-failed`, `not-started`. `Badge`'s own field, and typed the same
   * way for the same reason: the roster is the state machine's.
   */
  status?: string;
};

/**
 * The line a finished Job is read by: how many of the things it was held to
 * were met, and what completing it means.
 *
 * **The count is the headline and the criteria are under it.** A landed Job
 * that met three of four is not the same Job as one that met four, and a
 * region that opens with a branch name says neither.
 */
export type JobOutcomeHeadline = {
  /** `Landed`, `Delivered`. Where the work got to, in one word. */
  verb: ReactNode;
  /** `2 of 2 met`. Mono, because it is a count and not a claim. */
  count?: ReactNode;
  /** The sentence under the count. */
  says?: ReactNode;
  /** Every criterion, in the order the Job froze them. */
  criteria?: JobOutcomeCriterion[];
  /**
   * What finishing this Job means — "completes when its pull request lands",
   * or "when every member has landed" where a Job has members (#1530).
   */
  completes?: ReactNode;
};

/** Parts under a heading: what was produced, and what was left behind. */
export type JobOutcomeSection = {
  name: ReactNode;
  /** A fact about the section, beside its name. Mono. */
  meta?: ReactNode;
  parts: JobOutcomePart[];
  /** A sentence under the parts. Never a count nothing measured. */
  note?: ReactNode;
};

/**
 * One run of one test case.
 *
 * **Never an Evidence row** (#1530, 21 Sep): it carries who ran it, what became
 * of the run and how many frames it kept, and no verdict about the work.
 */
export type JobOutcomeRun = {
  /** The spec, as a repository path. Mono. */
  spec: string;
  /** Who ran it: `Fleet`, `you`, a contributor's name, `CI`. */
  who: ReactNode;
  /** What became of the run — `ran`, `not covered`, `the run failed`. */
  outcome: ReactNode;
  /** The status stem that verb takes. `JobOutcomeCriterion.status`'s field. */
  status?: string;
  /** What is known beside it: `6 frames`, why it did not run. */
  meta?: ReactNode;
  /** When it ran. Mono and quiet. */
  when?: ReactNode;
};

/** A set of runs under a heading — the handoff run, and what came after it. */
export type JobOutcomeRuns = {
  name: ReactNode;
  meta?: ReactNode;
  runs: JobOutcomeRun[];
  /** What a set with no runs in it says. Never an empty table. */
  absent?: ReactNode;
  /** A sentence under the set: that there is no before-run, and who may post one. */
  note?: ReactNode;
};

export type JobOutcomeProps = {
  /**
   * Every part of what was produced, in the order a reader asks for them.
   * **`sections` is the same rows under headings**, for a region drawing more
   * than one set of them; a caller passes one or the other.
   */
  parts?: JobOutcomePart[];
  sections?: JobOutcomeSection[];
  /** How the Job was answered, above everything else it left. */
  headline?: JobOutcomeHeadline;
  /** What it cost, as labelled readings. `FigureList`'s own rows. */
  cost?: { name: ReactNode; figures: Figure[]; note?: ReactNode };
  /** The test runs, one set per heading. */
  runs?: JobOutcomeRuns[];
  /**
   * What the person still owes. **Armada pushes and opens a review, and does
   * not merge**, so this region says what is left rather than implying the work
   * landed — and rather than denying the two things it does.
   */
  note?: ReactNode;
  /** What to do next about it — dispatching a follow-up, and nothing else. */
  act?: ReactNode;
  /** A clipboard write is silent, so the surface confirms it with a toast. */
  onCopied?: (value: string) => void;
};

/** Row marks are 12px at strokeWidth 2, like every mark below Job level. */
const ROW_ICON = 12;
const ROW_STROKE = 2;

export function JobOutcome({
  parts,
  sections,
  headline,
  cost,
  runs,
  note,
  act,
  onCopied,
}: JobOutcomeProps) {
  const copy = useCallback(
    (event: MouseEvent<HTMLElement>, value: string) => {
      event.stopPropagation();
      void navigator.clipboard.writeText(value).then(() => onCopied?.(value));
    },
    [onCopied],
  );

  return (
    <div className="armada-outcome">
      {headline === undefined ? null : <Headline {...headline} />}
      {parts === undefined ? null : <Parts parts={parts} onCopy={copy} />}
      {sections?.map((section, at) => (
        <section className="armada-outcome__section" key={at}>
          <p className="armada-outcome__section-head">
            <span className="armada-outcome__section-name">{section.name}</span>
            {section.meta === undefined ? null : (
              <span className="armada-outcome__section-meta">{section.meta}</span>
            )}
          </p>
          <Parts parts={section.parts} onCopy={copy} />
          {section.note === undefined ? null : (
            <p className="armada-outcome__aside">{section.note}</p>
          )}
        </section>
      ))}
      {cost === undefined ? null : (
        <section className="armada-outcome__section">
          <p className="armada-outcome__section-head">
            <span className="armada-outcome__section-name">{cost.name}</span>
          </p>
          <FigureList figures={cost.figures} column="fit" />
          {cost.note === undefined ? null : <p className="armada-outcome__aside">{cost.note}</p>}
        </section>
      )}
      {runs?.map((set, at) => (
        <Runs key={at} {...set} />
      ))}
      {note === undefined ? null : <p className="armada-outcome__note">{note}</p>}
      {act === undefined ? null : <div className="armada-outcome__act">{act}</div>}
    </div>
  );
}

/** The parts list. One `subgrid` per list, so each section aligns on its own. */
function Parts({
  parts,
  onCopy,
}: {
  parts: JobOutcomePart[];
  onCopy: (event: MouseEvent<HTMLElement>, value: string) => void;
}) {
  return (
    <ol className="armada-outcome__parts">
      {parts.map((part, i) => (
        <li className="armada-outcome__part" key={i}>
          <span className="armada-outcome__mark">
            {part.icon ? <part.icon size={ROW_ICON} strokeWidth={ROW_STROKE} aria-hidden /> : null}
            {part.iconLabel ? <span className="armada-outcome__sr">{part.iconLabel}</span> : null}
          </span>
          <span className="armada-outcome__name">{part.name}</span>
          {part.value === undefined ? (
            <span className="armada-outcome__absent">{part.absent}</span>
          ) : (
            /* The title carries the whole value however narrow the row gets,
               and so does the clipboard: a copy that truncated with the
               display would be worse than the overflow it was fixing. */
            <span
              className="armada-outcome__value"
              title={part.value}
              onClick={(event) => onCopy(event, part.value as string)}
            >
              {part.value}
            </span>
          )}
          <span className="armada-outcome__meta">{part.meta}</span>
          <span className="armada-outcome__action">{part.action}</span>
        </li>
      ))}
    </ol>
  );
}

/**
 * The headline: the word for where the work got to, the count of what was met,
 * and every criterion under it with what answered it.
 *
 * **The hue is the token's, named rather than picked** — `Badge` sets its own
 * two custom properties the same way, and for the same reason: the roster is
 * the state machine's and a stylesheet cannot enumerate it.
 */
function Headline({ verb, count, says, criteria, completes }: JobOutcomeHeadline) {
  return (
    <div className="armada-outcome__headline">
      <p className="armada-outcome__verdict">
        <span className="armada-outcome__verb">{verb}</span>
        {count === undefined ? null : <span className="armada-outcome__count">{count}</span>}
      </p>
      {says === undefined ? null : <p className="armada-outcome__says">{says}</p>}
      {criteria === undefined || criteria.length === 0 ? null : (
        <ol className="armada-outcome__criteria">
          {criteria.map((criterion, at) => (
            <li className="armada-outcome__criterion" key={at}>
              <span className="armada-outcome__criterion-text">{criterion.text}</span>
              <span
                className="armada-outcome__criterion-verdict"
                {...(criterion.status === undefined
                  ? {}
                  : { style: { color: `var(--status-${criterion.status})` } })}
              >
                {criterion.verdict}
              </span>
            </li>
          ))}
        </ol>
      )}
      {completes === undefined ? null : <p className="armada-outcome__completes">{completes}</p>}
    </div>
  );
}

/** One set of case runs, or the sentence saying there are none. */
function Runs({ name, meta, runs, absent, note }: JobOutcomeRuns) {
  return (
    <section className="armada-outcome__section">
      <p className="armada-outcome__section-head">
        <span className="armada-outcome__section-name">{name}</span>
        {meta === undefined ? null : <span className="armada-outcome__section-meta">{meta}</span>}
      </p>
      {runs.length === 0 ? (
        <p className="armada-outcome__absent" role="note">
          {absent}
        </p>
      ) : (
        <ol className="armada-outcome__runs">
          {runs.map((run, at) => (
            <li className="armada-outcome__run" key={at}>
              <span className="armada-outcome__run-spec" title={run.spec}>
                {run.spec}
              </span>
              <span
                className="armada-outcome__run-outcome"
                {...(run.status === undefined
                  ? {}
                  : { style: { color: `var(--status-${run.status})` } })}
              >
                {run.outcome}
              </span>
              <span className="armada-outcome__run-who">{run.who}</span>
              <span className="armada-outcome__run-meta">{run.meta}</span>
              <span className="armada-outcome__run-when">{run.when}</span>
            </li>
          ))}
        </ol>
      )}
      {note === undefined ? null : <p className="armada-outcome__aside">{note}</p>}
    </section>
  );
}
