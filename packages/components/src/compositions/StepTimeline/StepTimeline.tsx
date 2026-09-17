import { useEffect, useRef, useState, type MutableRefObject, type ReactNode } from "react";

import { ChevronDown, ChevronRight } from "lucide-react";

import { Skeleton } from "../../primitives/Skeleton/Skeleton";
import { Chapter } from "../Chapter/Chapter";
import {
  StepActivityMark,
  stepActivitySaid,
  type StepActivity,
} from "../StepActivityMark/StepActivityMark";

/**
 * A step as one timeline: the phases in the order they happened, repeated for
 * each attempt the step was worked.
 *
 * **It replaces a graph and a list that said the same thing twice.** The strip
 * drew where the step stood and the story below it drew what happened there, so
 * a failed Check was a red node in one place and a printed exit code in
 * another. Here the phase *is* the row, and what happened in it opens beneath.
 *
 * **An attempt is a section, and the earlier ones fold.** A step handed back
 * twice has three sections; the one being worked is open and the rest carry
 * their outcome on a header a press opens. The strip could only ever say
 * "handed back twice" on a single edge, which is the count without the record.
 *
 * **A single attempt draws no section header at all.** Most steps are worked
 * once, and a header reading `Attempt 1` above every step on every Job is a
 * word that never varies.
 */
/**
 * What the Drone is doing, in parts rather than as one node, so the verb takes
 * the running hue here and no caller reaches for this stylesheet's class.
 */
export type StepTimelineNow = {
  /** `Editing`. Absent between calls, where a sentence is the whole of it. */
  verb?: ReactNode;
  /** The path, the command, or the Drone's own sentence. */
  detail?: ReactNode;
  /** How long the call has been open — `3s`. */
  took?: ReactNode;
  /** Whether `detail` is machine-derived. A path is; a sentence is not. */
  mono?: boolean;
};

export type StepTimelineRow = {
  id: string;
  /** `Instructed`, `Working`, `Checks`, `Judge`. */
  name: ReactNode;
  /** Where the phase stands, for the mark and its hue. */
  activity: StepActivity;
  /** What the row says while it is closed — `1756 turns · 43m 37s · 36 files`. */
  meta?: ReactNode;
  /**
   * The Drone is in this row now: the phase takes the running treatment and
   * the row opens by default.
   *
   * **The running mark leaves the header here**, and the bar sweeping the
   * card's top edge carries live-ness instead. A mark says *which* phase, and
   * on the one card that is already edged, washed and swept in running there
   * is no which left to say; what it cost was a second loop in a header that
   * already had one. Marks stay where they still answer something — the task
   * rows and the live narration row inside this body, where one row among
   * forty is the live one.
   */
  live?: boolean;
  /**
   * What the Drone is doing right now, under the phase's name — `Editing
   * crates/fleet/src/settling.rs · 3s`.
   *
   * **The call in flight, not the last row of the log.** A person watching a
   * running Job should not have to read a stream to find out whether anything
   * is happening, which is the whole of #1196. Drawn only on a live row: on a
   * phase that has finished there is no now.
   */
  now?: StepTimelineNow;
  /**
   * What the row opens to. **Absent draws a row that does not open**, which is
   * a phase with nothing recorded in it rather than a control over an empty box.
   */
  body?: ReactNode;
  /**
   * Hold the body to a height of its own, scrolling past it.
   *
   * **For a phase whose body is somebody else's passage.** The opening brief
   * is a screen and a half, and unfolded inside a row it pushes every phase
   * below it off the panel. A line clamp cannot help: it counts line boxes and
   * a sectioned brief is one box carrying none, which `Clamped.css` says in
   * those words.
   */
  bounded?: boolean;
  /**
   * Attributes naming this row for whatever finds it — the marker a keyboard
   * map queries rather than reaching for the class this component ships.
   *
   * **Written by the caller, because the name is the caller's.** A row is one
   * phase of one attempt and only the surface assembling it knows what to call
   * that; a name minted here would be a second vocabulary for the same row.
   */
  marker?: Record<string, string>;
  /**
   * The control on the row's line — `Open the log`, `Open the diff`.
   *
   * **A sibling of the opening control, never inside it**, which is the rule
   * `Chapter` already keeps: a button within a button is one target carrying
   * two meanings, and the outer one wins the press.
   */
  act?: ReactNode;
};

export type StepTimelineAttempt = {
  id: string;
  /** `Attempt 2`. Drawn only where the step was worked more than once. */
  name: ReactNode;
  /** What became of it, in the caller's words — `handed back · 2m 22s`. */
  said?: ReactNode;
  /** The attempt being read. Open on mount; the earlier ones fold. */
  current?: boolean;
  rows: StepTimelineRow[];
};

export type StepTimelineProps = {
  attempts: StepTimelineAttempt[];
  /** The label over the timeline. Absent draws none. */
  label?: ReactNode;
  /**
   * Which row is open, held by the caller. **Present makes the timeline
   * controlled**, so a keyboard map can open a phase by id rather than reaching
   * for the class this component happens to ship. `null` is every row closed.
   */
  openRow?: string | null;
  onOpenRow?: (rowId: string | null) => void;
  /**
   * Whether the timeline starts with every row closed. `VerdictSheet`'s own
   * `folded`, for the same reason: at the review gate the record above already
   * reads this step, and every row open under it read the same thing twice.
   */
  folded?: boolean;
};

export function StepTimeline({ attempts, label, openRow, onOpenRow, folded = false }: StepTimelineProps) {
  const [held, setHeld] = useState<string | null>(() => (folded ? null : whereItIs(attempts)));
  const many = attempts.length > 1;

  // Whether `held` is following the live phase rather than a row the reader
  // opened themselves. Starts `true` — nothing has been pressed yet, so the
  // seed above and the live phase are the same row — and only a press that
  // lands somewhere else turns it off; a press on the live row turns it back
  // on, the same rule `JobDetail.tsx` applies one level up. #1152.
  const following = useRef(true);
  const openRef = useRef<HTMLDivElement | null>(null);

  // **Synced, not controlled, and never synced to nothing.** A caller that
  // names a row — the keyboard — sets this rather than owning it. It starts at
  // `null` and mounts before anyone has pressed anything, so honouring that
  // null shut every row on arrival; closing is a press, which goes through
  // `toggle` below.
  useEffect(() => {
    if (openRow !== undefined && openRow !== null) {
      following.current = openRow === whereItIs(attempts);
      setHeld(openRow);
    }
  }, [openRow]);

  // `folded` transitioning true — arriving at the gate on a panel already
  // mounted — closes every row the same way the initial seed above does.
  useEffect(() => {
    if (folded) setHeld(null);
  }, [folded]);

  // Follow the live phase. **Re-read on every change of where it is, not only
  // at mount** — `whereItIs` already finds it, so a step moving from Working
  // to Checks to Judge is this running again, not a second function. A row
  // the reader opened by hand leaves `following` false, so this does nothing
  // until a press on the live row itself turns it back on.
  const live = folded ? null : whereItIs(attempts);
  useEffect(() => {
    if (following.current && live !== null) setHeld(live);
  }, [live]);

  // The followed row scrolls into view as it changes. `nearest`, not
  // `center` — `CommandPalette.tsx`'s own reasoning: centring would move a
  // row that was already visible. Left alone on a press elsewhere, which put
  // its own row in view already.
  useEffect(() => {
    if (following.current) openRef.current?.scrollIntoView({ block: "nearest" });
  }, [held]);

  function toggle(rowId: string): void {
    const next = held === rowId ? null : rowId;
    setHeld(next);
    // A press on the live row resumes following, whichever way it toggles;
    // any other press holds there until the live row is pressed again.
    following.current = rowId === whereItIs(attempts);
    onOpenRow?.(next);
  }

  return (
    <div className="armada-steps">
      {label === undefined ? null : <span className="armada-steps__label">{label}</span>}
      {attempts.map((attempt) => (
        <Attempt key={attempt.id} attempt={attempt} named={many} open={held} onToggle={toggle} openRef={openRef} />
      ))}
    </div>
  );
}

/** One attempt: its own header where there is more than one, and its phases. */
function Attempt({
  attempt,
  named,
  open,
  onToggle,
  openRef,
}: {
  attempt: StepTimelineAttempt;
  named: boolean;
  open: string | null;
  onToggle: (rowId: string) => void;
  openRef: MutableRefObject<HTMLDivElement | null>;
}) {
  const [shown, setShown] = useState(attempt.current === true);
  const rows = attempt.rows.map((row) => (
    <Row
      key={row.id}
      row={row}
      open={open === row.id}
      onToggle={() => onToggle(row.id)}
      openRef={open === row.id ? openRef : undefined}
    />
  ));
  if (!named) return <div className="armada-steps__rows">{rows}</div>;
  return (
    <section className="armada-steps__attempt">
      <button
        type="button"
        className="armada-steps__attempt-head"
        aria-expanded={shown}
        onClick={() => setShown((was) => !was)}
      >
        {shown ? (
          <ChevronDown className="armada-steps__fold" size={13} strokeWidth={2} aria-hidden />
        ) : (
          <ChevronRight className="armada-steps__fold" size={13} strokeWidth={2} aria-hidden />
        )}
        <span className="armada-steps__attempt-name">{attempt.name}</span>
        {attempt.said === undefined ? null : (
          <span className="armada-steps__said">{attempt.said}</span>
        )}
      </button>
      {/* Hidden rather than unmounted, so a row opened inside an attempt is
          still open when the attempt is folded and opened again. */}
      <div className="armada-steps__rows" hidden={!shown}>
        {rows}
      </div>
    </section>
  );
}

/**
 * One phase of one attempt.
 *
 * **A `Chapter`, which is what the draft drew.** The card, the right-aligned
 * meta, the running dot, the act on the header line and the body in its own
 * well are all already built and already agreed — drawing a bare line here
 * instead was a second answer to a question this package had settled.
 */
function Row({
  row,
  open,
  onToggle,
  openRef,
}: {
  row: StepTimelineRow;
  open: boolean;
  onToggle: () => void;
  /** Set only on the followed row, so the timeline can scroll it into view. */
  openRef?: MutableRefObject<HTMLDivElement | null>;
}) {
  // The running mark leaves the live phase's header — the card's own sweep is
  // what says it is working — and every other phase keeps the mark it always
  // had. A live row is a running row: every caller that sets one sets the
  // other, so the two conditions are one fact read twice.
  const swept = row.live === true;
  const name = (
    <span className="armada-steps__mark" data-live={swept || undefined} {...row.marker}>
      {swept ? (
        <>
          {/* The mark's slot, kept empty. A column of four phases reads down
              one left edge, and the live row's name jumping into the gap is
              the one row a person is looking for moving. */}
          <span className="armada-steps__slot" aria-hidden />
          {/* Hue, the wash and the sweep are the visible channels, and a
              reader hearing none of them would be told the phase's name and
              nothing about it. The word is the registry's own, through
              `stepActivitySaid`, not a second spelling written here. */}
          <span className="armada-steps__state">{`${rowLabel(row)}, ${stepActivitySaid(row.activity) ?? ""}`}</span>
        </>
      ) : (
        <StepActivityMark activity={row.activity} label={rowLabel(row)} />
      )}
      {row.now === undefined ? (
        row.name
      ) : (
        <span className="armada-steps__title">
          <span className="armada-steps__name">{row.name}</span>
          <Now now={row.now} />
        </span>
      )}
    </span>
  );
  return (
    <div className="armada-steps__row" ref={openRef}>
      <Chapter
        name={name}
        {...(row.meta === undefined ? {} : { meta: row.meta })}
        {...(swept ? { live: true, tone: "running" as const } : {})}
        {...(row.act === undefined ? {} : { act: row.act })}
        // A phase with nothing recorded draws its header as a label: `Chapter`
        // takes no `onToggle` there, so no control opens an empty box.
        {...(row.body === undefined ? {} : { open, onToggle })}
      >
        {row.bounded === true ? (
          <div className="armada-steps__bounded">{row.body}</div>
        ) : (
          row.body
        )}
      </Chapter>
    </div>
  );
}

/**
 * What the Drone is doing, on one clipped line under the phase's name.
 *
 * **The verb is the only hued part.** The path beside it changes every few
 * seconds and hue there would make the whole line flicker; the verb is the
 * word that says the thing is happening, and it takes the same running the
 * sweep above it carries.
 */
function Now({ now }: { now: StepTimelineNow }) {
  return (
    <span className="armada-steps__now" data-mono={now.mono || undefined}>
      {now.verb === undefined ? null : (
        <span className="armada-steps__now-verb">{now.verb} </span>
      )}
      {now.detail}
      {now.took === undefined ? null : (
        <span className="armada-steps__now-took"> · {now.took}</span>
      )}
    </span>
  );
}

/** Instructed, Working, Checks, Judge: the phases a step is read against. */
const SKELETON_ROWS = 4;

/**
 * The timeline, before the step it draws has come back.
 *
 * **Closed rows, and named.** A phase's name is the workflow's and is known
 * before the step is read, so the skeleton says which four are coming; what it
 * cannot say is where the step stands, which is the mark and the meta — and
 * those are the bars. It replaced a strip skeleton drawing four unnamed nodes
 * above a story skeleton drawing three named chapters, which between them
 * promised a shape this panel no longer has.
 */
export function StepTimelineSkeleton({ phases }: { phases?: readonly ReactNode[] }) {
  const named = phases ?? Array.from({ length: SKELETON_ROWS }, () => undefined);
  return (
    <div className="armada-steps" role="status" aria-label="Reading the step" aria-busy>
      <div className="armada-steps__rows">
        {named.map((name, at) => (
          <Chapter key={at} name={name ?? <Skeleton width="var(--space-12)" />} open={false} />
        ))}
      </div>
    </div>
  );
}

/**
 * The row a timeline opens on.
 *
 * **Where the step is, which is what the strip it replaces said.** The phase a
 * Drone is in wins it; on a step that has finished it is the last phase that
 * did something, because the reason a person opens a finished step is to read
 * what the gates found rather than what it was told at the start.
 *
 * Nothing opens where no row carries a body — four closed lines is a panel
 * reporting that it has nothing, and that is the honest drawing of a step that
 * has not begun.
 */
function whereItIs(attempts: readonly StepTimelineAttempt[]): string | null {
  const rows = (attempts.find((attempt) => attempt.current) ?? attempts[attempts.length - 1])?.rows;
  if (rows === undefined) return null;
  const live = rows.find((row) => row.live === true && row.body !== undefined);
  if (live !== undefined) return live.id;
  const reached = rows.filter((row) => row.body !== undefined && row.activity !== "not_started");
  return reached[reached.length - 1]?.id ?? null;
}

/**
 * The mark's accessible name.
 *
 * **The phase and its state, not the state alone.** Hue and silhouette are the
 * visible channels; a screen reader hearing only `failed` four times down a
 * timeline is told which things went wrong and not which phases they were.
 */
function rowLabel(row: StepTimelineRow): string {
  return typeof row.name === "string" ? row.name : "phase";
}
