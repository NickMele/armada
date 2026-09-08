import {
  cloneElement,
  isValidElement,
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
  type HTMLAttributes,
  type ReactElement,
  type ReactNode,
} from "react";

/**
 * A tooltip carries what a reader cannot get from the thing itself — the full
 * value behind an abbreviated one, what pressing a control does, or what a word
 * naming an Armada concept means. It never restates the text it sits on.
 *
 * Where the action has a binding it gains a trailing kbd. The 400ms delay
 * stands, and it is a token: `--tooltip-delay`.
 *
 * **The bubble stays in the document while closed.** `aria-describedby` has to
 * name an element that exists, and a description referenced explicitly is read
 * from a hidden node — so the association is set once rather than appearing
 * 400ms after focus, by which time nothing re-announces it. Same rule `Chapter`
 * and `StepRow` keep for `aria-controls`.
 */
export type TooltipProps = {
  /**
   * The full value, what the act does, or what the concept is. Sentence case.
   *
   * **An act names the control it is on and never the one beside it.** A reader
   * who presses on the strength of one describing a neighbour has learnt that
   * the surface lies, which is worse than no tooltip rather than less good. The
   * rule and the near-miss it was written from are in
   * `docs/contracts/design-system.md`, under Tooltip.
   */
  label: ReactNode;
  /** The binding, where the action has one. Rendered as a trailing kbd. */
  shortcut?: string;
  children: ReactNode;
  /** Render open without hovering. Storybook draws resting states. */
  defaultOpen?: boolean;
  /**
   * Attach to the child element instead of wrapping it in a `span`.
   *
   * **This is what lets a `td`, a `th` or a `tr` be annotated.** A `span` is
   * not a thing a table row may contain, so the wrapper is not an option
   * there — `JudgeVerdicts` is a table and so is every check list. The child
   * takes the handlers, the anchor class and the description, and the bubble is
   * appended inside it rather than beside it, because a sibling of a `td` is as
   * invalid as a parent of one.
   *
   * The cost is that the child must forward `className`, `onMouseEnter`,
   * `onFocus` and a ref to a DOM element. Every primitive in this package does;
   * one that does not drops the tooltip silently, which is why the wrapper
   * stays the default.
   */
  asChild?: boolean;
  /**
   * Whether `label` is a card rather than a sentence.
   *
   * **The card is `PhaseCard`, and there is no second one.** A value like a
   * step's `not run` is answered by the commands that will run, which is a
   * reading and not a line of prose — so the bubble drops its own surface and
   * its prose width cap, and the card inside draws the floating surface it
   * already draws off the phase strip. A hover card primitive of its own would
   * be a second answer to a question `PhaseCard` settles.
   */
  card?: boolean;
};

/**
 * When a tooltip anywhere on screen last closed.
 *
 * **Module-level, and deliberately not context.** The group is whatever the
 * pointer is crossing — every tooltip on the screen at once, not a subtree
 * somebody remembered to wrap — and a provider would still be holding this one
 * number. Before it existed, crossing eight annotated chips meant eight
 * independent 400ms waits.
 */
let groupClosedAt = 0;

/**
 * What is already reachable by tab. A wrapper around one of these must not take
 * a stop of its own: every annotated button would become two stops, and the
 * second would do nothing.
 */
const FOCUSABLE = "a[href],button,input,select,textarea,summary,[tabindex]:not([tabindex='-1'])";

function readMs(name: string, fallback: number): number {
  const raw = getComputedStyle(document.documentElement).getPropertyValue(name);
  const parsed = Number.parseInt(raw, 10);
  return Number.isNaN(parsed) ? fallback : parsed;
}

export function Tooltip({
  label,
  shortcut,
  children,
  defaultOpen = false,
  asChild = false,
  card = false,
}: TooltipProps) {
  const [open, setOpen] = useState(defaultOpen);
  const timer = useRef<number | undefined>(undefined);
  /**
   * Whether this one actually showed. Only a tooltip a reader read warms the
   * group — a pointer that crossed a chip in 40ms and opened nothing has not
   * earned the next one its instant reveal.
   */
  const shown = useRef(defaultOpen);
  const frame = useRef<HTMLElement | null>(null);
  const bubbleId = useId();

  useEffect(() => () => window.clearTimeout(timer.current), []);

  // The description belongs on whatever a keyboard lands on. On an ancestor it
  // is announced by nothing, so the control inside is found here rather than
  // guessed from the element type — and where there is no control, the frame
  // takes the stop, because a tooltip only a pointer can open reaches nobody.
  useEffect(() => {
    const held = frame.current;
    if (held === null) return;
    const on = held.matches(FOCUSABLE) ? held : held.querySelector<HTMLElement>(FOCUSABLE);
    if (on !== null) {
      on.setAttribute("aria-describedby", bubbleId);
      return;
    }
    held.setAttribute("aria-describedby", bubbleId);
    // A stop of its own only where it is not already inside a control. A
    // focusable span within a button is not somewhere a keyboard can go, and a
    // concept annotated on part of a control is read by hovering it — the
    // control's own description is what a keyboard gets.
    if (held.closest(FOCUSABLE) === null) held.tabIndex = 0;
  }, [bubbleId, children]);

  const show = useCallback(() => {
    window.clearTimeout(timer.current);
    // Instant while the group is warm. The first one in a run waits; a reader
    // already reading tooltips is not asking to wait again for the next chip.
    if (Date.now() - groupClosedAt < readMs("--tooltip-grace", 300)) {
      shown.current = true;
      setOpen(true);
      return;
    }
    timer.current = window.setTimeout(() => {
      shown.current = true;
      setOpen(true);
    }, readMs("--tooltip-delay", 400));
  }, []);

  const hide = useCallback(() => {
    window.clearTimeout(timer.current);
    if (shown.current) {
      groupClosedAt = Date.now();
      shown.current = false;
    }
    setOpen(false);
  }, []);

  // `aria-hidden`, and no `role="tooltip"`, because the bubble is drawn inside
  // the thing it describes. A button takes its name from its contents, so a
  // bubble in the accessibility tree there would be read twice — once as part
  // of the control's own name and again as its description — and the name is
  // the one that has to stay the control's own words. The description still
  // reaches a reader: a node named by `aria-describedby` is read whether it is
  // hidden or not, which is the same rule that lets the bubble stay in the
  // document while closed.
  const bubble = (
    <span
      className="armada-tooltip__bubble"
      aria-hidden
      id={bubbleId}
      data-card={card || undefined}
      hidden={!open}
    >
      <span className="armada-tooltip__label">{label}</span>
      {shortcut ? <kbd className="armada-tooltip__kbd">{shortcut}</kbd> : null}
    </span>
  );

  const handlers = {
    onMouseEnter: show,
    onMouseLeave: hide,
    onFocus: show,
    onBlur: hide,
  };

  const hold = (node: HTMLElement | null) => {
    frame.current = node;
  };

  if (!asChild) {
    return (
      <span className="armada-tooltip" ref={hold} {...handlers}>
        {children}
        {bubble}
      </span>
    );
  }

  // One element, or there is nothing to attach to. A caller passing text or a
  // fragment here has asked for a wrapper without saying so, and drawing one
  // silently is what would put a `span` back inside the table row.
  if (!isValidElement(children)) {
    throw new Error("Tooltip asChild takes exactly one element");
  }

  const child = children as ReactElement<HTMLAttributes<HTMLElement>>;
  return cloneElement(child, {
    ...handlers,
    className: [child.props.className, "armada-tooltip-on"].filter(Boolean).join(" "),
    ref: hold,
    children: (
      <>
        {child.props.children}
        {bubble}
      </>
    ),
  } as HTMLAttributes<HTMLElement>);
}
