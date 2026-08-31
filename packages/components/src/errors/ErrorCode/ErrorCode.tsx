/**
 * The code an error carries, in the chip a status never gets.
 *
 * **This is where an error is told apart from a failed Job.** Both are red, so
 * hue decides nothing. Two things do, and this component is both of them: the
 * chip is a SOLID fill where a status badge is a 12% tint, and it holds a
 * CODE, which a status has none of. The geometry is the badge's exactly —
 * `--h-badge`, `--space-2` of horizontal padding, `--radius-sm`,
 * `--text-2xs`, weight 500 — so the two are the same object at the same size,
 * differing only in the channels that mean something.
 *
 * **The code is always shown.** It is what a person reads back to someone
 * else, and the error contract guarantees every error carries one, so it is a
 * required prop rather than an optional flourish. Mono, because it is
 * machine-derived, and because that is the third channel separating it from a
 * badge's sans verb.
 *
 * **No glyph, here or anywhere in the error treatment.** `triangle-alert` is
 * Doctor's and `octagon-alert` is `stalled`'s, and the icon registry carries
 * no mark for a Bridge failure. An error carries the code and the sentence
 * instead.
 */
export type ErrorClass = "fault" | "degraded";

export type ErrorCodeProps = {
  /**
   * Which of the two an error is, and never inferred. A **fault** is Armada
   * unable to do the thing. **Degraded** is Armada unable to refresh what it
   * is showing. The fixes are opposite — restarting Fleet is the wrong move
   * when the process is alive and only the stream stopped — so the caller
   * states it.
   */
  kind: ErrorClass;
  /**
   * The `code` off the wire, opaque and never parsed. `ipc::WireError`
   * carries it always, so there is no case with nothing to render here.
   */
  code: string;
};

export function ErrorCode({ kind, code }: ErrorCodeProps) {
  return (
    <span className={`armada-error-code armada-error-code--${kind}`} data-error-class={kind}>
      {code}
    </span>
  );
}
