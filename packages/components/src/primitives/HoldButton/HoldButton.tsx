import { useEffect, useId, useRef, useState, useSyncExternalStore } from "react";
import { Button, type ButtonProps } from "../Button/Button";

/**
 * A destructive act that confirms in place: it fills while held and commits
 * once held for `--duration-hold`. The rule is the design system contract's,
 * under Keyboard and command palette, "A hold confirms in place".
 *
 * Built on `Button`'s destructive variant, so outline, focus, disabled and
 * pending are that file's.
 */
export type HoldButtonProps = Omit<
  ButtonProps,
  | "variant"
  | "iconOnly"
  | "ref"
  | "children"
  | "onClick"
  | "onPointerDown"
  | "onPointerUp"
  | "onPointerLeave"
  | "onPointerCancel"
  | "onKeyDown"
  | "onKeyUp"
  | "onBlur"
  | "aria-describedby"
> & {
  /** The label while the hold is offered, naming the hold: `Hold to kill job`. */
  children: string;
  /** The label where the hold is not offered and a press asks: `Kill job`. */
  askLabel: string;
  /** What holding does, read to a person who cannot see the fill. */
  description: string;
  /** Held for the whole duration. Called once per hold. */
  onCommit: () => void;
  /** Pressed where the hold is not offered. Opens the confirmation. */
  onAsk: () => void;
};

const REDUCED = "(prefers-reduced-motion: reduce)";

function subscribeToMotion(onChange: () => void): () => void {
  const query = window.matchMedia(REDUCED);
  query.addEventListener("change", onChange);
  return () => query.removeEventListener("change", onChange);
}

function motionReduced(): boolean {
  return window.matchMedia(REDUCED).matches;
}

/** Whether a person asked for less motion, following the preference as it changes. */
export function usePrefersReducedMotion(): boolean {
  return useSyncExternalStore(subscribeToMotion, motionReduced, () => false);
}

/**
 * `--duration-hold` on `element`, in milliseconds. `null` where it is absent,
 * unparseable or zero: a hold with no length is not one, so the press asks.
 */
export function holdDurationOf(element: Element): number | null {
  const raw = getComputedStyle(element).getPropertyValue("--duration-hold").trim();
  const read = /^(\d+(?:\.\d+)?)(ms|s)$/.exec(raw);
  if (read === null) return null;
  const ms = Number(read[1]) * (read[2] === "s" ? 1000 : 1);
  return ms > 0 ? ms : null;
}

/** `rest` has no transition, `arming` fills across the hold, `releasing` empties fast. */
type Phase = "rest" | "arming" | "releasing";

function isHoldKey(key: string): boolean {
  return key === " " || key === "Enter";
}

export function HoldButton({
  children,
  askLabel,
  description,
  onCommit,
  onAsk,
  disabled,
  pending,
  ...rest
}: HoldButtonProps) {
  // Under reduced motion the fill is invisible, and it is the only thing that
  // says how long is left, so the press asks instead.
  const reduced = usePrefersReducedMotion();
  const element = useRef<HTMLButtonElement>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [phase, setPhase] = useState<Phase>("rest");
  const describedBy = useId();
  const inert = disabled === true || pending === true;

  function start(): void {
    if (inert || timer.current !== null || element.current === null) return;
    const duration = holdDurationOf(element.current);
    if (duration === null) {
      onAsk();
      return;
    }
    setPhase("arming");
    timer.current = setTimeout(() => {
      timer.current = null;
      setPhase("rest");
      onCommit();
    }, duration);
  }

  function cancel(): void {
    if (timer.current === null) return;
    clearTimeout(timer.current);
    timer.current = null;
    setPhase("releasing");
  }

  // A hold does not survive the control going off or the hold stopping being offered.
  useEffect(() => {
    if (inert || reduced) cancel();
  }, [inert, reduced]);
  useEffect(
    () => () => {
      if (timer.current !== null) clearTimeout(timer.current);
    },
    [],
  );

  if (reduced) {
    return (
      <Button {...rest} variant="destructive" disabled={disabled} pending={pending} onClick={onAsk}>
        {askLabel}
      </Button>
    );
  }

  // No `onClick`: a click is a press and release, and a press alone never commits.
  return (
    <Button
      {...rest}
      ref={element}
      variant="destructive"
      disabled={disabled}
      pending={pending}
      data-hold=""
      aria-describedby={describedBy}
      onPointerDown={(event) => {
        if (event.button !== 0) return;
        start();
      }}
      onPointerUp={cancel}
      onPointerLeave={cancel}
      onPointerCancel={cancel}
      onKeyDown={(event) => {
        if (!isHoldKey(event.key)) return;
        // Space would scroll and Enter would click. Auto-repeat must not restart the hold.
        event.preventDefault();
        if (event.repeat) return;
        start();
      }}
      onKeyUp={(event) => {
        if (!isHoldKey(event.key)) return;
        event.preventDefault();
        cancel();
      }}
      onBlur={cancel}
    >
      <span className="armada-hold__fill" data-phase={phase} aria-hidden="true" />
      <span className="armada-hold__label">{children}</span>
      {/* Hidden and still read: a description follows its reference into hidden content. */}
      <span id={describedBy} hidden>
        {description}
      </span>
    </Button>
  );
}
