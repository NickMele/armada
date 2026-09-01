import { ChevronDown, ChevronRight } from "lucide-react";
import type { ReactNode } from "react";

/**
 * One line of the activity log, and the payload it opens to.
 *
 * **Every line opens.** That is the promise the header makes, so a line that
 * opens to nothing breaks it — and a line that opens to a sentence explaining
 * that the payload was cut breaks it worse, because the entire purpose of the
 * gesture is seeing the payload. Where an argument is genuinely too large to
 * send whole, the payload shows what was sent with its real size and offers
 * the rest. It never reports that Bridge was given nothing.
 *
 * **Every entry names who.** Armada, Drone, Fleet — three actors, in one
 * stream, in the order things happened. A stream carrying only the Drone's
 * turns is a transcript, and the reason the log is a log is that Armada's
 * injected turns and Fleet's own Check results and heartbeats sit between them
 * in time.
 *
 * **The payload keeps its own newlines and scrolls sideways.** A build log
 * reflowed is not a build log; wrapping a column of compiler output turns one
 * long line into three and loses which was which.
 *
 * **The row is four columns and the message is the only one that flexes.** A
 * timestamp column that moved when a message got longer is the "columns
 * flip-flopped between renders" line in the v1 failure log, and this is a
 * stream where a new row arrives every second.
 */

/**
 * Who the entry is. Not a style choice — the three are the three things that
 * can put a line into this stream, and the vocabulary is the wire's.
 *
 * `armada` is a turn Armada injected: *Go on to Implement.*
 * `drone` is the Drone's own turn — a tool call, a thought, a message.
 * `fleet` is Fleet reporting on itself: a Check result, a heartbeat.
 */
export type LogActor = "armada" | "drone" | "fleet";

/** What each actor is called on the row. The wire's spelling, capitalised. */
const NAMED: Record<LogActor, string> = {
  armada: "Armada",
  drone: "Drone",
  fleet: "Fleet",
};

export type LogEntryProps = {
  /** `14:29:40`. Mono, fixed width, and the column never moves. */
  at: string;
  actor: LogActor;
  /**
   * The line. A tool call is mono because it is a command; a message and a
   * heartbeat are sans because they are sentences. `mono` says which, and the
   * caller knows because the wire told it.
   */
  message: ReactNode;
  /** Whether `message` is machine-derived. Sans names work, mono names machinery. */
  mono?: boolean;
  /**
   * Whether the Drone is still producing this line — the thinking marker. The
   * running dot before the message, and nothing else: a spinner in a stream
   * where a row arrives every second is motion nobody can read.
   */
  working?: boolean;
  /** Whether the payload is shown. */
  open?: boolean;
  /** Open or close it. Absent draws a row that does not open, which is a defect. */
  onToggle?: () => void;
  /**
   * The payload. Mono, preserved newlines, its own scroll. Absent on an open
   * row draws the empty-state line rather than a paragraph about the wire.
   */
  payload?: ReactNode;
  /** What an open row with no payload says. One sentence, and no transport in it. */
  payloadAbsent?: ReactNode;
  /** For a caller that needs to point at the payload. */
  payloadId?: string;
};

/** Row glyphs are 12px at strokeWidth 2, as every mark on this screen is. */
const GLYPH = 12;
const STROKE = 2;

export function LogEntry({
  at,
  actor,
  message,
  mono,
  working,
  open = false,
  onToggle,
  payload,
  payloadAbsent,
  payloadId,
}: LogEntryProps) {
  const who = NAMED[actor];
  const row = (
    <>
      <span className="armada-entry__t">{at}</span>
      <span className="armada-entry__who">{who}</span>
      <span className="armada-entry__msg" data-mono={mono || undefined}>
        {working ? <span className="armada-entry__working" aria-hidden /> : null}
        {message}
      </span>
      {open ? (
        <ChevronDown size={GLYPH} strokeWidth={STROKE} className="armada-entry__mark" aria-hidden />
      ) : (
        <ChevronRight size={GLYPH} strokeWidth={STROKE} className="armada-entry__mark" aria-hidden />
      )}
    </>
  );

  return (
    <div className="armada-entry-group">
      {onToggle === undefined ? (
        <div className="armada-entry" data-actor={actor} data-open={open || undefined}>
          {row}
        </div>
      ) : (
        <button
          type="button"
          className="armada-entry"
          data-actor={actor}
          data-open={open || undefined}
          aria-expanded={open}
          aria-controls={payloadId}
          onClick={onToggle}
        >
          {row}
        </button>
      )}

      {/* Unmounted while closed, unlike the chapter's body and the step's
          facts. A log holds hundreds of rows and a payload is the largest
          thing on the screen; keeping every one of them in the document is
          the unbounded render the v1 log recorded as a freeze. Nothing points
          at it while it is shut, so there is no dangling reference. */}
      {open ? (
        <div className="armada-entry__payload" id={payloadId}>
          {payload ?? (
            <p className="armada-entry__absent">
              {payloadAbsent ?? "This line carried no payload."}
            </p>
          )}
        </div>
      ) : null}
    </div>
  );
}

/**
 * One line inside a payload. Mono, its own newlines kept, and a hue where the
 * line is a result rather than output.
 *
 * **`named` is not a severity.** `echo` is the command being echoed back,
 * `passed` and `failed` are what the run came to, and `meta` is the trailer
 * saying where and how long. A payload with a red line in the middle of its
 * output would be a component deciding what a compiler meant.
 */
export type PayloadLineNamed = "echo" | "passed" | "failed" | "meta";

export type PayloadLineProps = {
  children: ReactNode;
  named?: PayloadLineNamed;
};

export function PayloadLine({ children, named }: PayloadLineProps) {
  return (
    <span className="armada-entry__pl" data-named={named}>
      {children}
    </span>
  );
}
