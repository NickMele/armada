import { ChevronDown, ChevronRight } from "lucide-react";
import type { ReactNode } from "react";
import { useCallback, useState } from "react";

/**
 * The activity log — one stream carrying the Drone's turns, Armada's injected
 * turns and Fleet's own events.
 *
 * **Every entry names who.** That is what keeps one stream honest: three
 * sources in one column with no attribution reads as one narrator, and the
 * whole point of folding Fleet's events in beside the Drone's turns is that a
 * Check failing and a Drone claiming it passed are visibly two different
 * voices.
 *
 * **Every entry opens to its payload.** A command opens to its full text, its
 * output, its exit code and where it ran — so the log answers "what did it
 * actually run" without a transcript being opened somewhere else.
 *
 * **Not `DroneTurns`.** That is one Drone's transcript, read while it is being
 * written, on a screen of its own; it carries one voice and no Fleet events,
 * and its rows do not open to an exit code. This is a chapter of a step's
 * story, bounded to the step, and it is the surface the design means by "one
 * stream".
 *
 * **An opened payload is bounded and says so when it cut** — the same rule the
 * diff follows. A cut names the file on disk, because a payload a reader cannot
 * finish reading here has to be finishable somewhere.
 */

/**
 * Who wrote the entry. **Three, and the set is closed by what can write into a
 * Job's log**: the Drone takes turns, Armada injects turns, and Fleet records
 * what it did. No registry in the repository spells these — `enum-verbs.toml`
 * covers Job states, step states and verdicts — so the words are copy, written
 * once here rather than in every screen that draws a log.
 */
export type ActivityActor = "drone" | "armada" | "fleet";

/** What each actor is called. One place, so two surfaces cannot disagree. */
const NAMED: Record<ActivityActor, string> = {
  drone: "Drone",
  armada: "Armada",
  fleet: "Fleet",
};

export type ActivityEntry = {
  /** Stable across re-renders. Entries arrive in order and nothing reorders. */
  id: string;
  /** When it happened, as the log recorded it. Mono: a machine wrote it. */
  at: string;
  actor: ActivityActor;
  /**
   * What happened, in one line — `Edit packages/settings/src/selectors.ts`,
   * `Heartbeat — the Drone has been quiet for 48 seconds`.
   */
  summary: ReactNode;
  /**
   * The tool, the Check id or the command, in mono beside the summary. Absent
   * on an entry that is only prose.
   */
  subject?: string;
  /**
   * Which of `passed`, `failed` or `refused` the entry is, for the hue on its
   * subject. Absent on nearly all of them: an entry is a record of something
   * happening, and most of what happens is not a verdict.
   */
  named?: string;
  /**
   * The machine text the entry opens to — the command and its output, the
   * Check's failures. Rendered as a block, in mono, cut at `maxLines`.
   */
  output?: string;
  /**
   * Where the whole of `output` is on disk, named when the block is cut. **A
   * cut that names nothing leaves a reader with no way to finish reading**,
   * which is the failure the rule exists to prevent.
   */
  outputAt?: string;
  /**
   * What the run came to — `exit 0 · 47.61s · in .armada/worktrees/job_2d90bb`.
   * Beneath the output, because it is read after it.
   */
  ran?: ReactNode;
  /** Anything the fields above cannot carry. Drawn under the output. */
  payload?: ReactNode;
};

export type ActivityLogProps = {
  entries: ActivityEntry[];
  /**
   * How many lines of an opened payload are drawn before it is cut. The default
   * is the diff's own bound, for the diff's own reason: a payload long enough
   * to need virtualizing is one nobody is reading in a panel.
   */
  maxLines?: number;
  /**
   * What the stream itself left out, where it is bounded — the same sentence
   * shape as a cut payload, one level up. Absent where every entry is here.
   */
  cut?: ReactNode;
  /** What an empty log says. Never a blank: a blank reads as a failed render. */
  emptyNote?: ReactNode;
  /** Which entry is open on mount. After that the log holds its own. */
  openId?: string;
};

/** Chevrons are 16px — disclosure is chrome, and chrome runs at 16. */
const CHEVRON = 16;
const STROKE = 2;

/**
 * The line bound on an opened payload, and the answer to the second question
 * #186 left open.
 *
 * **The same rule as the diff, and the same number: a cut says so and names
 * where the rest is.** A log payload and a patch are the same kind of thing —
 * machine text read inside a panel — and the bound exists for the same reason,
 * which is that a 14,000-line block puts 14,000 nodes in the document and that
 * is the freeze the v1 failure log recorded nine times. Choosing a second,
 * smaller number for a log entry would need a measurement nobody has made.
 *
 * **Bridge holds the authoritative copy**, as `DRAWN_LINES` in the renderer's
 * `review.ts`, and passes it in. This is the fallback for a caller that does
 * not — a component cannot import from the app, so the two numbers are two
 * statements of one value until something generates it. Reported.
 */
const MAX_LINES = 2000;

export function ActivityLog({
  entries,
  maxLines = MAX_LINES,
  cut,
  emptyNote = "Nothing has been recorded against this step yet.",
  openId,
}: ActivityLogProps) {
  const [open, setOpen] = useState<ReadonlySet<string>>(() =>
    openId === undefined ? new Set<string>() : new Set([openId]),
  );

  const toggle = useCallback((entryId: string) => {
    setOpen((held) => {
      const next = new Set(held);
      if (next.has(entryId)) next.delete(entryId);
      else next.add(entryId);
      return next;
    });
  }, []);

  if (entries.length === 0) {
    return (
      <p className="armada-activity__empty" role="note">
        {emptyNote}
      </p>
    );
  }

  return (
    <div className="armada-activity">
      <ol className="armada-activity__entries">
        {entries.map((entry) => {
          const shown = open.has(entry.id);
          const opens = entry.output !== undefined || entry.payload !== undefined || entry.ran !== undefined;
          const bounded = boundedTo(entry.output, maxLines);
          return (
            <li className="armada-activity__entry" key={entry.id}>
              <button
                type="button"
                className="armada-activity__row"
                data-open={shown || undefined}
                data-opens={opens || undefined}
                aria-expanded={shown}
                onClick={() => toggle(entry.id)}
              >
                <span className="armada-activity__chevron">
                  {/* An entry with nothing behind it keeps the column and
                      renders no chevron: the rows stay aligned, and nothing
                      offers to open something that is not there. */}
                  {!opens ? null : shown ? (
                    <ChevronDown size={CHEVRON} strokeWidth={STROKE} aria-hidden />
                  ) : (
                    <ChevronRight size={CHEVRON} strokeWidth={STROKE} aria-hidden />
                  )}
                </span>
                <span className="armada-activity__at">{entry.at}</span>
                {/* Named on every entry, never on some of them. Three voices in
                    one column with attribution on two is worse than none. */}
                <span className="armada-activity__who" data-actor={entry.actor}>
                  {NAMED[entry.actor]}
                </span>
                <span className="armada-activity__summary">{entry.summary}</span>
                {entry.subject === undefined ? null : (
                  <span className="armada-activity__subject" data-named={entry.named}>
                    {entry.subject}
                  </span>
                )}
              </button>

              {!shown || !opens ? null : (
                <div className="armada-activity__payload">
                  {bounded === null ? null : (
                    <pre className="armada-activity__output">{bounded.text}</pre>
                  )}
                  {bounded === null || !bounded.cut ? null : (
                    <p className="armada-activity__cut" role="note">
                      {`Cut at ${maxLines} of ${bounded.lines} lines. `}
                      {entry.outputAt === undefined
                        ? "Nothing names where the whole of it was written, so the rest is not reachable from here."
                        : `The whole of it is in ${entry.outputAt}.`}
                    </p>
                  )}
                  {entry.ran === undefined ? null : (
                    <p className="armada-activity__ran">{entry.ran}</p>
                  )}
                  {entry.payload === undefined ? null : (
                    <div className="armada-activity__extra">{entry.payload}</div>
                  )}
                </div>
              )}
            </li>
          );
        })}
      </ol>

      {cut === undefined ? null : (
        <p className="armada-activity__cut" role="note">
          {cut}
        </p>
      )}
    </div>
  );
}

/**
 * The payload, cut to `maxLines`, with what it was cut from.
 *
 * Returns the line count as well as the text, because a cut that says only
 * "cut" tells a reader nothing about how much they are not seeing — and the
 * difference between 201 lines and 40,000 decides whether they open the file.
 */
function boundedTo(
  output: string | undefined,
  maxLines: number,
): { text: string; lines: number; cut: boolean } | null {
  if (output === undefined) return null;
  const lines = output.split("\n");
  return lines.length <= maxLines
    ? { text: output, lines: lines.length, cut: false }
    : { text: lines.slice(0, maxLines).join("\n"), lines: lines.length, cut: true };
}
