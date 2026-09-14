import type { ReactNode } from "react";

/**
 * Helm's own conversation, under the dock's questions — #944.
 *
 * **Two voices only, and neither is `ActivityLog`'s three.** A person and
 * Helm are the whole of this thread; a Job's `armada`/`drone`/`fleet` split
 * answers a different question, about a step nobody typed into.
 */
export type HelmRowActor = "you" | "helm";

/** One line of the thread, already worded — `packages/screens` folds the wire into this. */
export type HelmThreadRow = {
  /** Stable across re-renders. The thread's own order. */
  id: string;
  at: string;
  actor: HelmRowActor;
  message: ReactNode;
  /** Machine-derived — a tool call — rather than prose. */
  mono?: boolean;
  /** A trailing caption — the reply's own cost, `$0.02 · 3 turns`. */
  meta?: ReactNode;
};

export type HelmThreadProps = {
  rows: HelmThreadRow[];
  /** A reply is on its way. Draws the working line after the last row. */
  replying?: boolean;
  /** What an empty, unstarted thread says. Never a blank. */
  emptyNote?: ReactNode;
  /** Fleet unreachable, or the socket failed. Drawn above the rows, never in place of them. */
  notice?: ReactNode;
};

export function HelmThread({
  rows,
  replying = false,
  emptyNote = "Ask Helm about this repository.",
  notice,
}: HelmThreadProps) {
  return (
    <div className="armada-helm-thread">
      {notice === undefined ? null : (
        <p className="armada-helm-thread__notice" role="alert">
          {notice}
        </p>
      )}
      {rows.length === 0 && !replying ? (
        <p className="armada-helm-thread__empty" role="note">
          {emptyNote}
        </p>
      ) : (
        <ol className="armada-helm-thread__rows">
          {rows.map((row) => (
            <li className="armada-helm-thread__row" key={row.id} data-actor={row.actor}>
              <div className="armada-helm-thread__head">
                <span className="armada-helm-thread__who">{row.actor === "you" ? "You" : "Helm"}</span>
                <span className="armada-helm-thread__at">{row.at}</span>
              </div>
              <div className="armada-helm-thread__message" data-mono={row.mono || undefined}>
                {typeof row.message === "string"
                  ? row.message.split("\n\n").map((paragraph, index) => <p key={index}>{paragraph}</p>)
                  : row.message}
              </div>
              {row.meta === undefined ? null : <p className="armada-helm-thread__meta">{row.meta}</p>}
            </li>
          ))}
        </ol>
      )}
      {!replying ? null : (
        <p className="armada-helm-thread__replying" role="status">
          <span className="armada-helm-thread__working" aria-hidden />
          Helm is replying…
        </p>
      )}
    </div>
  );
}
