import type { LucideIcon } from "lucide-react";
import type { MouseEvent, ReactNode } from "react";
import { Fragment, useCallback } from "react";
import { Badge } from "../../primitives/Badge/Badge";

/**
 * Job detail header — what this Job is, and whatever you can do to it from
 * here. One component for every state job detail reaches: running, failed and
 * finished all render through this, so the block cannot drift between them.
 *
 * **The badge is static here.** The pulse is one per screen, on the most
 * specific mark present, and job detail has a workflow rail — so the rail's
 * current step carries it and this badge does not. Two breathing marks on one
 * screen would be two answers to "what is still working".
 *
 * **The facts are a labelled run, not a table.** Step, branch, elapsed and
 * spend are what a person needs to decide whether to leave a job alone, and
 * they are on screen without a click. None is dropped at any width.
 *
 * **The run changes with the state, the block does not.** A running job reports
 * the step it is on and the branch it writes to; a stopped one reports where it
 * stopped and what it ran. The facts are the caller's; their type, spacing and
 * order are not.
 *
 * **Hedge by source.** Elapsed is measured and speaks flatly; spend is
 * estimated and is marked approximate. Rendering the two alike would destroy
 * trust in both.
 *
 * **A fact may point somewhere, and the run is where that belongs.** A pull
 * request is read in the same breath as the branch it was opened from, and a
 * control at the trailing edge would put going to read something among the
 * acts that end a Job. So `href` makes a fact a link in place, beside the
 * facts it is about, and nothing moves to the other end of the header.
 *
 * **What the header offers changes with the state; how it is arranged does
 * not.** The set is the caller's, and which state carries what is written where
 * that decision is made — `Acts` in Bridge's `JobDetail.tsx`. The arrangement
 * is this file's: ghost first, the acts that end something as one outlined
 * destructive group, and the one primary a Job detail ever carries last, at the
 * trailing edge where the shell head puts its own.
 */
export type JobDetailField = {
  /** The label, where the fact needs one. `Dispatched by you` does not. */
  label?: ReactNode;
  value?: ReactNode;
  /**
   * Machine-derived: a step count, a branch, a duration, a cost. Mono is the
   * signal that Armada reported this rather than wrote it.
   */
  mono?: boolean;
  /**
   * The exact string this fact puts on the clipboard. Setting it makes the
   * value copy on click and go to `--accent` on hover, with no `copy` glyph.
   */
  copyValue?: string;
  /**
   * Where this fact goes, for a fact that names something outside Armada.
   * Setting it draws the value in `--accent` — the token the design contract
   * gives links — and hands the address to `onFollowed` when it is clicked.
   *
   * **The address is never the value.** A forge address is long, a person
   * reads one number out of it, and a fact run is a line of short readings;
   * what this carries is where the fact points, and the caller decides what it
   * spells. The full address is on the element's `title`, which is the only
   * place a person can still get at the whole of it.
   *
   * **This component does not navigate, and no fact here ever will.** The
   * anchor's default is cancelled and the address goes back to the caller,
   * because a surface that let a click load a page would be a surface that
   * could be steered off the app by a value that arrived on a wire. Where a
   * caller sets `href` and no `onFollowed`, the fact draws as a link and the
   * click does nothing — which is a caller wiring only half of it, and is the
   * one shape to look for when a link goes dead.
   */
  href?: string;
  /**
   * The words after the value, where the fact reads as a sentence around it —
   * `All 4 of 4 steps advanced`. Sans, and never part of the mono run.
   */
  suffix?: ReactNode;
  /**
   * This fact continues the one before it, joined by a comma inside the same
   * span instead of separated by the gap between facts. `Stopped at Run tests,
   * step 3 of 4` is one fact — where the job stopped — carrying two values of
   * different type; the gap would read it as two unrelated ones and force
   * `step` to be re-cased as a label of its own.
   */
  continues?: boolean;
};

export type JobDetailHeaderActionsProps = {
  /** The status token stem, e.g. `running`. Drives the badge's hue and tint. */
  status: string;
  /** The glyph, from `packages/icons/icons.toml`, group `Job state`. */
  statusIcon: LucideIcon;
  /** The verb, from the enum→verb map. Never written by hand where it ships. */
  statusLabel: ReactNode;
  /** The Job's title, in the person's own words. Sans. */
  headline: ReactNode;
  /** The job id, in mono and set back. It identifies, it does not describe. */
  jobId?: ReactNode;
  /** The facts, in the order the drawing runs them. */
  fields: JobDetailField[];
  /** The controls at the trailing edge. None on a terminal Job. */
  actions?: ReactNode;
  /** A clipboard write is silent, so the surface confirms it with a toast. */
  onCopied?: (value: string) => void;
  /**
   * A fact with an `href` was clicked. **The host opens it, not this** — a
   * component that reached for a browser could not be rendered in Storybook,
   * and the process with the shell is the one that should decide what leaving
   * the app means.
   */
  onFollowed?: (href: string) => void;
};

export function JobDetailHeaderActions({
  status,
  statusIcon,
  statusLabel,
  headline,
  jobId,
  fields,
  actions,
  onCopied,
  onFollowed,
}: JobDetailHeaderActionsProps) {
  // The anchor is a real one so it reads as a link and carries the address on
  // hover, and its default is cancelled so nothing here can navigate. See
  // `href`.
  const follow = useCallback(
    (event: MouseEvent<HTMLAnchorElement>, href: string) => {
      event.preventDefault();
      event.stopPropagation();
      onFollowed?.(href);
    },
    [onFollowed],
  );

  const copy = useCallback(
    (event: MouseEvent<HTMLSpanElement>, value: string) => {
      event.stopPropagation();
      void navigator.clipboard.writeText(value).then(
        () => onCopied?.(value),
        // A failed clipboard write is otherwise indistinguishable from a dead
        // element, so the surface is told either way.
        () => onCopied?.(value),
      );
    },
    [onCopied],
  );

  // Facts that continue one another fold into a single span, so a comma-joined
  // fact wraps and truncates as the one thought it is.
  const runs: JobDetailField[][] = [];
  for (const field of fields) {
    const previous = runs[runs.length - 1];
    if (field.continues && previous) previous.push(field);
    else runs.push([field]);
  }

  return (
    <div className="armada-job-head">
      <div className="armada-job-head__ident">
        <div className="armada-job-head__line">
          {/* No `pulsing`. The rail beneath takes the one pulse on this
              screen, and this badge stays still. */}
          <Badge status={status} icon={statusIcon}>
            {statusLabel}
          </Badge>
          <span className="armada-job-head__title">{headline}</span>
          {jobId ? <span className="armada-job-head__id">{jobId}</span> : null}
        </div>
        <div className="armada-job-head__facts">
          {runs.map((run, i) => (
            <span className="armada-job-head__fact" key={i}>
              {run.map((field, j) => (
                <Fragment key={j}>
                  {j > 0 ? ", " : null}
                  {field.label ? (
                    <>
                      {field.label}
                      {field.value !== undefined ? " " : null}
                    </>
                  ) : null}
                  {field.value === undefined ? null : field.href !== undefined ? (
                    <a
                      className="armada-job-head__value"
                      data-mono={field.mono || undefined}
                      data-opens=""
                      href={field.href}
                      title={field.href}
                      onClick={(e) => follow(e, field.href as string)}
                    >
                      {field.value}
                    </a>
                  ) : (
                    <span
                      className="armada-job-head__value"
                      data-mono={field.mono || undefined}
                      data-copies={field.copyValue !== undefined || undefined}
                      onClick={
                        field.copyValue !== undefined
                          ? (e) => copy(e, field.copyValue as string)
                          : undefined
                      }
                    >
                      {field.value}
                    </span>
                  )}
                  {field.suffix ? <> {field.suffix}</> : null}
                </Fragment>
              ))}
            </span>
          ))}
        </div>
      </div>
      {actions ? <div className="armada-job-head__actions">{actions}</div> : null}
    </div>
  );
}
