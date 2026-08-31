import type { ReactNode } from "react";
import { useCallback, useState } from "react";

import { Button } from "../../primitives/Button/Button";
import type { ErrorClass } from "../ErrorCode/ErrorCode";
import { ErrorCode } from "../ErrorCode/ErrorCode";
import { debugInfo } from "./payload";
import type { DebugPayload } from "./payload";

// The payload module is re-exported through the notice it belongs to. It is
// not a component and never appears on its own: every error carries the
// payload, and the placement decides only whether it is shown, offered or
// expandable.
export type { DebugField, DebugPayload } from "./payload";
export { debugInfo } from "./payload";

/**
 * An error, in one of the four places an error may appear.
 *
 * **One treatment, four placements, and the placement is picked by blast
 * radius rather than by severity.** A critical failure with a narrow radius
 * stays inline: approve-refused is red-serious and affects one row, so it
 * renders in that row and nowhere else. Escalating placement for severity is
 * the specific mistake this component exists to stop, which is why
 * `placement` has no default — there is nothing safe to fall back to.
 *
 * **Two classes, and only one is red.** `fault` is Armada unable to do the
 * thing. `degraded` is Armada unable to refresh what it is showing, and takes
 * no hue at all beyond an amber dot. Unreachable Fleet and dropped events are
 * degraded, not faults. The fixes are opposite — restarting Fleet is the wrong
 * move when the process is alive and only the stream stopped — so `kind` has
 * no default either.
 *
 * **One value, not a ladder.** Placement already carries blast radius, so
 * severity picks nothing but the edge. There is no second red, no `critical`
 * variant, and the copy at every placement is the same size.
 *
 * **Every placement names the failure and the act**, with one exception the
 * type enforces: a toast may carry no act, because it is the only placement
 * that reports something already over.
 *
 * **No glyph.** `triangle-alert` is Doctor's, `octagon-alert` is `stalled`'s,
 * and there is no generic alarm mark. The code and the sentence do the work.
 *
 * **Every error carries the debug payload, and the four placements differ only
 * in whether it is shown, offered or expandable.** Inline copies directly
 * because a row has no room for an expanded view; a toast copies and dismisses
 * in one press because it is often the only sighting; a banner offers both,
 * because a standing condition gets read rather than only quoted; and a
 * full-surface error shows it, because nothing else is on the screen. The
 * expanded view renders the exact string the control copies — see `payload.ts`
 * for why there is one producer and not two.
 *
 * Where the thing sits is the caller's, and two classes are supplied for it:
 * `armada-error-toast-region` pins the bottom trailing corner inset
 * `--space-6` above the status bar, and `armada-error-surface-region` centres
 * a full-surface error in the region it replaces. Regions rather than props,
 * because a region holds several toasts and a component cannot own one.
 */
export type ErrorPlacement = "inline" | "toast" | "banner" | "surface";

export type ErrorField = {
  /** What the value is. Sentence case, no trailing colon. */
  label: string;
  /** The value. Mono, because the facts an error carries are machine-derived. */
  value: string;
};

type Common = {
  /** Fault or degraded, and never inferred from the message. */
  kind: ErrorClass;
  /**
   * The `code` off the wire. Required, because every error carries one and it
   * is what a person reads back to someone else.
   */
  code: string;
  /**
   * What failed, in one sentence. Event-first: the subject is the job or the
   * step, never the Drone. Never what threw, and never an apology.
   */
  message: ReactNode;
  /**
   * The facts needed to decide, on screen and without a click. The status
   * grammar's labelled field run, one step down from the sentence.
   */
  fields?: ErrorField[];
  /** Controls, where there is something to press. Never a second decision. */
  actions?: ReactNode;
  /**
   * The machine-derived facts, as one quotable artifact.
   *
   * **Every error carries it, and the placement decides only whether it is
   * shown, offered or expandable.** Optional here for one reason: a caller
   * that has not been given a payload to pass is a gap worth seeing as a
   * missing control rather than as an empty block, and the boundary fallbacks
   * in Bridge reached this component before they had one.
   */
  payload?: DebugPayload;
  /**
   * What the surface is told after a clipboard write, so it can raise a toast.
   *
   * A clipboard write is silent by nature and a failed one is
   * indistinguishable from a dead control, so this is called either way. It is
   * given the name of the thing copied, not the thing itself — the payload is
   * fifteen lines and a toast is one.
   */
  onCopied?: (what: string) => void;
};

/** What the toast says was copied. A noun, because the artifact is a block. */
const COPIED = "The debug info";

/** What the control is called, everywhere it appears. */
const COPY_LABEL = "Copy debug info";

/**
 * Why the payload is safe to copy, in the expanded view only.
 *
 * **Two sentences, because one cannot be written that is true.** The issue
 * this came from put a single sentence on screen — *structured fields cannot
 * hold a credential, the type has no variant that carries one* — and the first
 * half of it is exactly right about `fields`: `ipc::WireValue` is five
 * primitive variants, `Secret<T>` implements no `Display` and no `Serialize`,
 * so formatting a credential into a field does not compile. Getting one in
 * needs an explicit `expose()`, which is a deliberate act and greppable in one
 * search.
 *
 * It says nothing about the rest of the payload, and the payload is not only
 * `fields`. `message` and `chain` are prose written by whatever error `Display`
 * impl raised them, and no type bounds what an author put there. Rendering the
 * bounded claim over the whole artifact would promise an outcome the mechanism
 * does not reach — and the mechanism is what the reader was owed.
 *
 * It also makes no claim about the wider context: a credential sitting in a
 * repository file was never a `Secret<T>` and the type system was never
 * involved.
 */
const SAFETY =
  "Structured fields carry primitives only, and a credential does not compile into one. " +
  "Nothing bounds the message or the chain, which are prose an error wrote — read them before you send this.";

/**
 * The act is required everywhere but a toast, and the union is what enforces
 * it. A rule written only in prose is one a call site can miss.
 */
export type ErrorNoticeProps =
  | (Common & {
      placement: "toast";
      /** A toast may report something already over and ask for nothing. */
      act?: ReactNode;
      /**
       * How the toast goes away, where the region does not clear it on a
       * timer. **Copying dismisses in the same press**, because a toast is
       * often the only sighting an error gets and reaching for a second
       * control after the first is how it is missed.
       */
      onDismiss?: () => void;
    })
  | (Common & {
      placement: "inline" | "banner" | "surface";
      /** What to do. A failure with nothing to do is drawn as the dead end it is. */
      act: ReactNode;
    });

/**
 * Whether the payload is on screen without asking, offered behind a control,
 * or reachable through a disclosure. **One rule per placement, and it is the
 * placement's blast radius that decides it** — the same rule that picks the
 * placement in the first place.
 *
 * | Placement | Form |
 * | --- | --- |
 * | Inline | Copies directly. A row has no room for an expanded view |
 * | Toast | Its one action. Copies and dismisses in one press |
 * | Banner | Copy, plus Details. A standing condition gets read, not just copied |
 * | Full-surface | Shown, not offered. Nothing else is on screen |
 */
const EXPANDABLE: Record<ErrorPlacement, "never" | "disclosed" | "always"> = {
  inline: "never",
  toast: "never",
  banner: "disclosed",
  surface: "always",
};

export function ErrorNotice(props: ErrorNoticeProps) {
  const { kind, code, message, fields, actions, placement, payload, onCopied } = props;
  const act = props.act;
  const dismiss = props.placement === "toast" ? props.onDismiss : undefined;

  /** Open only where the placement discloses. Surface and inline never toggle. */
  const [open, setOpen] = useState(false);
  const disclosure = EXPANDABLE[placement];
  const expanded = payload !== undefined && (disclosure === "always" || open);

  const text = payload === undefined ? null : debugInfo(payload);

  const copy = useCallback(() => {
    if (text === null) return;
    void navigator.clipboard.writeText(text).then(
      () => onCopied?.(COPIED),
      // A failed clipboard write is otherwise indistinguishable from a dead
      // control, so the surface is told either way.
      () => onCopied?.(COPIED),
    );
    // The toast is often the only sighting, so its one action finishes the
    // job rather than leaving a dismissed-or-not toast behind the copy.
    dismiss?.();
  }, [text, onCopied, dismiss]);

  return (
    <section
      className={`armada-error armada-error--${kind} armada-error--${placement}`}
      data-error-class={kind}
      data-placement={placement}
      // A fault has stopped something and interrupts; degraded means what is
      // on screen is stale, which is worth saying and not worth cutting a
      // screen reader off for.
      role={kind === "fault" ? "alert" : "status"}
    >
      <div className="armada-error__head">
        {kind === "degraded" ? <span className="armada-error__dot" aria-hidden="true" /> : null}
        <span className="armada-error__message">{message}</span>
      </div>

      {act !== undefined ? <span className="armada-error__act">{act}</span> : null}

      {/* The code leads the run of machine-derived facts rather than sitting
          beside the sentence. Trailing the headline it competes with the copy
          for the line, and at the toast's measure a long code takes half of it
          and reflows the sentence into a column — the exact failure this app
          was built to escape. Here it wraps with everything else, and it is
          with the values it is quoted alongside. */}
      <div className="armada-error__facts">
        <ErrorCode kind={kind} code={code} />
        {fields !== undefined && fields.length > 0 ? (
          <dl className="armada-error__fields">
            {/* Keyed by position: a `fields` key arriving off the wire is not
                guaranteed unique against the rows beside it, and the run is
                rebuilt whole on every render. */}
            {fields.map((field, at) => (
              <div className="armada-error__field" key={at}>
                <dt className="armada-error__label">{field.label}</dt>
                <dd className="armada-error__value">{field.value}</dd>
              </div>
            ))}
          </dl>
        ) : null}
      </div>

      {/* Shown, not offered, wherever the placement says so — and it is the
          exact string the control puts on the clipboard, not a second
          rendering of the same fields. What is read on screen is what arrives
          in the issue body. */}
      {expanded && text !== null ? (
        <div className="armada-error__payload">
          <pre className="armada-error__debug">{text}</pre>
          <p className="armada-error__safety">{SAFETY}</p>
        </div>
      ) : null}

      {payload !== undefined || actions !== undefined ? (
        <div className="armada-error__actions">
          {payload === undefined ? null : (
            <Button
              variant="ghost"
              size="sm"
              ground={placement === "inline" ? "card" : "sunken"}
              onClick={copy}
            >
              {COPY_LABEL}
            </Button>
          )}
          {/* Only where the payload is disclosed. Inline has no room for an
              expanded view and a toast is gone before it would be read, so
              neither offers a control that would open one. */}
          {payload !== undefined && disclosure === "disclosed" ? (
            <Button
              variant="ghost"
              size="sm"
              ground="sunken"
              aria-expanded={open}
              onClick={() => setOpen((was) => !was)}
            >
              Details
            </Button>
          ) : null}
          {actions}
        </div>
      ) : null}
    </section>
  );
}
