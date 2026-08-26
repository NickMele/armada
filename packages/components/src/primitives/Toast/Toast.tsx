import type { ReactNode } from "react";

/**
 * A floating layer, so `--bg-overlay` and a shadow, and no blur.
 *
 * The contract names one use directly: a machine value copies on click, and a
 * toast confirms, because a clipboard write is silent by nature and a failed
 * one is otherwise indistinguishable from a dead element.
 *
 * The leading dot carries a Job state and is never chosen — pass the enum
 * variant's stem, or pass none. A clipboard write is not a Job state, so that
 * toast has no dot; see the report.
 */
export type ToastProps = {
  /** A Job state stem, e.g. "killed". Omitted where the event is not one. */
  status?: string;
  /** The verb, from the generated enum to verb map. An action keeps its name. */
  children: ReactNode;
  /** One trailing link at most. Never a second decision. */
  actionLabel?: string;
  onAction?: () => void;
};

export function Toast({ status, children, actionLabel, onAction }: ToastProps) {
  return (
    <div className="armada-toast" role="status">
      {status ? (
        <span
          className="armada-toast__dot"
          style={{ background: `var(--status-${status})` }}
          aria-hidden="true"
        />
      ) : null}
      <span className="armada-toast__text">{children}</span>
      {actionLabel ? (
        <button type="button" className="armada-toast__action" onClick={onAction}>
          {actionLabel}
        </button>
      ) : null}
    </div>
  );
}
