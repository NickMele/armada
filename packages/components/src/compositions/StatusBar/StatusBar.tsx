import type { ReactNode } from "react";

/**
 * Status bar — fixed to the bottom, full window width, spanning **beneath**
 * the sidebar rather than inset to the content area.
 *
 * Fixed because a healthy state has to say "Fleet running" out loud, and that
 * guarantee fails the moment the bar can scroll away — it is a liveness
 * indicator for a daemon that outlives the window. Full width because the bar
 * is app-level rather than Bridge-level: it appears on Helm too. Inset it and
 * it reads as belonging to whatever surface is open.
 *
 * **Fleet's state is one of three, and the bar names which** — plus `unknown`,
 * for the readings that are neither: still connecting, and a runtime file or a
 * protocol version Bridge refused. Those keep the neutral dot rather than
 * taking a fourth hue. A leading 6px dot carries the hue, the one exception to
 * everything else in the bar staying `--fg-muted`, on the same grounds
 * Doctor's pass, warn and fail reuse the Job values. It is not a glyph, so
 * "the status bar carries no icons" is unaffected.
 *
 * The two failure states differ on the runtime file: a missing file is a Fleet
 * that is not there, a live pid with no answer is a Fleet that is wedged. Two
 * different things to do about it, so two sentences rather than one timeout
 * message.
 *
 * **Five items is the ceiling.** Collapsed when healthy, expanded when not.
 */
export type FleetState = "running" | "not-running" | "unreachable" | "unknown";

export type StatusBarProps = {
  fleet: FleetState;
  /**
   * The sentence naming Fleet's state — "Fleet running", "Fleet is not
   * running", "Fleet unreachable". Fixed copy from the contract, identical
   * every time, because uniformity is scannability.
   */
  fleetLabel: ReactNode;
  /**
   * The machine facts beside it, in mono: pid, port and drone count when
   * running; the runtime file's absence when not; the live pid and how long it
   * has been silent when unreachable.
   */
  detail?: ReactNode;
  /**
   * What to do, or how stale the reading is. Sans, because it is a sentence
   * rather than a value the system reported.
   */
  advice?: ReactNode;
  /**
   * Further segments, `--fg-muted`, in the order they are read. The job count,
   * and the spend figure in the active billing mode — the visible number is
   * always the number that gates dispatch.
   */
  items?: ReactNode[];
  /**
   * The spend figure, pushed to the trailing edge. Sized for both billing
   * modes: `68% quota left` and `~$2.40 of $20` are very different strings and
   * neither mode is the default.
   */
  spend?: ReactNode;
  /**
   * Escalations waiting. Shown only when non-zero, in `--status-escalated`.
   * Escalations interrupt; approvals queue, and styling the two alike would
   * collapse that distinction and turn the bar into a second alert surface.
   * How much louder than its token this should render is open —
   * `[status-bar-loudness]`.
   */
  escalations?: number;
  /** Approvals waiting. Shown only when non-zero, in `--status-awaiting-review`. */
  approvals?: number;
};

/**
 * The count segments' copy. Not from the enum→verb map: that map carries Job
 * statuses, and "3 escalations" is a count of them rather than one of their
 * verbs. No vocabulary in the repository sanctions the plural noun. Reported.
 */
function count(n: number, one: string, many: string) {
  return `${n} ${n === 1 ? one : many}`;
}

export function StatusBar({
  fleet,
  fleetLabel,
  detail,
  advice,
  items,
  spend,
  escalations,
  approvals,
}: StatusBarProps) {
  return (
    <div className="armada-status-bar" role="status" data-fleet={fleet}>
      <span className="armada-status-bar__fleet">
        <span className="armada-status-bar__dot" aria-hidden />
        {fleetLabel}
      </span>

      {detail ? <span className="armada-status-bar__mono">{detail}</span> : null}
      {advice ? <span className="armada-status-bar__advice">{advice}</span> : null}

      {items?.map((item, i) => (
        <span className="armada-status-bar__item" key={i}>
          {item}
        </span>
      ))}

      {/* Shown only when non-zero. A zero count is not news, and the bar
          expands in proportion to what is wrong. */}
      {escalations ? (
        <span className="armada-status-bar__count" data-kind="escalated">
          {count(escalations, "escalation", "escalations")}
        </span>
      ) : null}
      {approvals ? (
        <span className="armada-status-bar__count" data-kind="awaiting-review">
          {count(approvals, "approval", "approvals")}
        </span>
      ) : null}

      {spend ? <span className="armada-status-bar__spend">{spend}</span> : null}
    </div>
  );
}
