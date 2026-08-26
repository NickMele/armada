import type { LucideIcon } from "lucide-react";
import type { MouseEvent, ReactNode } from "react";
import { useCallback } from "react";
import { Badge } from "../../primitives/Badge/Badge";

/**
 * Job detail header actions — what this Job is, and the one thing you can do
 * to it.
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
 * **Hedge by source.** Elapsed is measured and speaks flatly; spend is
 * estimated and is marked approximate. Rendering the two alike would destroy
 * trust in both.
 *
 * **The only action is `Kill`, and it stays outlined.** A running job offers no
 * primary: there is nothing to approve and nothing to merge while it works, so
 * the accent is spent elsewhere. Destructive is an outline because a solid red
 * button reads as an error state rather than an act — killing a job is
 * deliberate, not alarming.
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
   * The words after the value, where the fact reads as a sentence around it —
   * `All 4 of 4 steps advanced`. Sans, and never part of the mono run.
   */
  suffix?: ReactNode;
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
  /** The controls at the trailing edge. One, on a running job. */
  actions?: ReactNode;
  /** A clipboard write is silent, so the surface confirms it with a toast. */
  onCopied?: (value: string) => void;
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
}: JobDetailHeaderActionsProps) {
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
          {fields.map((field, i) => (
            <span className="armada-job-head__fact" key={i}>
              {field.label ? (
                <>
                  {field.label}
                  {field.value !== undefined ? " " : null}
                </>
              ) : null}
              {field.value !== undefined ? (
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
              ) : null}
              {field.suffix ? <> {field.suffix}</> : null}
            </span>
          ))}
        </div>
      </div>
      {actions ? <div className="armada-job-head__actions">{actions}</div> : null}
    </div>
  );
}
