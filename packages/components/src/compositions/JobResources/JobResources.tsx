// Pulse — what one Job is costing the machine right now: what is running, what
// it is spending, the processes, the checkouts and the logs.
//
// Not a debug panel. The first thing on it is a sentence answering *is this
// working*, and the figures come after. `docs/contracts/design-system.md` →
// Fleet panel, for the figure rows; #1538 for the board.

import { Search } from "lucide-react";
import type { ReactNode } from "react";

import type { Finding, JobExamined, Look } from "@armada/protocol";
import { Button } from "../../primitives/Button/Button";
import { FigureList, type Figure } from "../FigureList/FigureList";
import { GuideMark } from "../GuideMark/GuideMark";
import { GUIDE_LOOK, GUIDE_PROCESSES, GUIDE_WORKTREE_SIZE } from "../../guides";
import type { Guide } from "../../guides/guide";
import { Logs, Processes, Worktrees } from "./JobResources.lists";
import type { PulseReading } from "./JobResources.lists";

export * from "./JobResources.lists";

/**
 * Why the panel offers nothing to press.
 *
 * `no_answer` — Fleet is not on the other end. Bridge holds no connection, or
 * the request itself could not be sent.
 *
 * `unreadable` — Fleet answered and Bridge could not read what came back. Fleet
 * is demonstrably up, which is exactly why the act still goes: the two do not
 * agree about the route, and the same request down the same route meets the
 * same disagreement.
 */
export type NothingToAsk = "no_answer" | "unreadable";

export type JobResourcesProps = {
  /**
   * The reading, or `null` where none has arrived.
   *
   * **`null` is not empty.** A read that has not answered and a job that holds
   * nothing are different things, and `note` is what says which.
   */
  reading: PulseReading | null;
  /**
   * What is running and what it is spending, as label and figure rows.
   *
   * **Above the reading and not inside it.** These come off the Job Fleet
   * answered with, so they are there whether or not anybody has looked at the
   * machine, and the instant under the tables does not qualify them.
   */
  figures?: Figure[];
  /** Why there is no reading, where there is none. */
  note?: string;
  /** How old the reading is, as a phrase — `4s`. Formatted by the caller. */
  age?: string;
  /**
   * What keeps the reading current, in the caller's words. **Beside the age**,
   * because "4s ago" answers nothing alone: a figure taken again and one as
   * fresh as it will ever get read the same.
   */
  refreshed?: string;
  /** What the last look found, or `null` where nobody has pressed. */
  examined: JobExamined | null;
  /** A look already out. A second press does not send a second act. */
  looking?: boolean;
  /** Why the last look failed, where it did. Drawn instead of a finding. */
  lookFailed?: string;
  /**
   * There is nothing to ask, and which of the two reasons it is. **The act is
   * not drawn at all**, and the panel says why rather than leaving a dead
   * control on screen.
   *
   * **Two readings and not one**, because the fixes are opposite: `no_answer`
   * is a Fleet that may only need starting, `unreadable` one that would come
   * back the same. The caller decides it from what the failure was, never from
   * the fact that something failed.
   */
  nothingToAsk?: NothingToAsk;
  onExamine: () => void;
};

/**
 * **Nothing running is drawn as loudly as a failure.** A job that reads
 * running and holds no process is the state that took a terminal to
 * establish, and an empty list under a heading is how it went unnoticed — so
 * each absence is a sentence in its own words rather than a table with no rows.
 */
export function JobResources({
  reading,
  figures = [],
  note,
  age,
  refreshed,
  examined,
  looking = false,
  lookFailed,
  nothingToAsk,
  onExamine,
}: JobResourcesProps) {
  return (
    <section className="armada-holds">
      <div className="armada-holds__head">
        <Headline
          examined={examined}
          looking={looking}
          lookFailed={lookFailed}
          nothingToAsk={nothingToAsk}
        />
        {/* No act where there is nothing to ask. **Absent rather than
            disabled**: a greyed control still says an act exists here and puts
            the reason on a person to work out. */}
        {nothingToAsk !== undefined ? null : (
          <span className="armada-holds__act">
            <Button size="sm" onClick={onExamine} disabled={looking}>
              <Search size={12} strokeWidth={2} aria-hidden="true" />
              {looking ? "Looking" : "Look now"}
            </Button>
            {/* What a look is, what it costs and what its three answers mean.
                Beside the act rather than beside the verdict: the verdict is
                this job's reading, and the act is the Armada word. */}
            <GuideMark guide={GUIDE_LOOK} />
          </span>
        )}
      </div>

      {figures.length === 0 ? null : <FigureList figures={figures} column="strip" />}

      {examined === null ? null : <Looks looks={examined.looks} />}

      {reading === null ? (
        <p className="armada-holds__note">
          {nothingToAsk === undefined
            ? (note ?? "Nothing has been read yet.")
            : INSTEAD[nothingToAsk]}
        </p>
      ) : (
        <>
          <Region label="Processes" guide={GUIDE_PROCESSES}>
            <Processes reading={reading} examined={examined} />
          </Region>
          <Region label="Worktrees" guide={GUIDE_WORKTREE_SIZE}>
            <Worktrees worktrees={reading.worktrees} />
          </Region>
          <Region label="Logs">
            <Logs logs={reading.logs} />
          </Region>
          {/* Last rather than a caption, because it qualifies everything over it. */}
          <p className="armada-holds__read-at">{readAt(reading, age, refreshed)}</p>
        </>
      )}
    </section>
  );
}

/**
 * When the figures above were true, and what keeps them true.
 *
 * **The second half is the caller's sentence.** How often a reading is taken
 * again is a fact about the app around this panel, and a panel that asserted
 * one would be claiming a schedule it cannot see.
 *
 * **Both halves are readings and neither explains one.** *A process can exit
 * between the reading and this screen* used to ride here; it is true of a job
 * that never ran, so it is the look's guide (#1602).
 */
function readAt(reading: PulseReading, age?: string, refreshed?: string): string {
  const said = age === undefined ? `Read at ${reading.readAt}.` : `Read ${age} ago.`;
  return refreshed === undefined ? said : `${said} ${refreshed}`;
}

/**
 * One band of the board and what is under it.
 *
 * **A label on every list, including an empty one.** A region that disappeared
 * when it held nothing would make "no worktree on disk" and "this build does
 * not draw worktrees" the same screen.
 */
function Region({ label, guide, children }: { label: string; guide?: Guide; children: ReactNode }) {
  return (
    <section className="armada-holds__region" aria-label={label}>
      {/* The `?` on the band, where the region's own word is. Two of the three
          bands carry one: a log is a log, and a mark on it would be the noise
          the rule about where a mark goes exists to refuse. */}
      <div className="armada-holds__band-row">
        <h3 className="armada-holds__band">{label}</h3>
        {guide === undefined ? null : <GuideMark guide={guide} />}
      </div>
      {children}
    </section>
  );
}

/**
 * The sentence a person came for.
 *
 * **A finding and never a status.** These three words are Fleet's answer to
 * *is this working*, and `working` here means *as it should be* rather than *a
 * processor is busy* — a job waiting for a person is working by that reading,
 * and under the other one every finished job would read as broken.
 */
function Headline({
  examined,
  looking,
  lookFailed,
  nothingToAsk,
}: {
  examined: JobExamined | null;
  looking: boolean;
  lookFailed?: string;
  nothingToAsk?: NothingToAsk;
}) {
  // Ahead of every other arm, including a look still in flight and a finding
  // from before. Both of those are claims about the job; this is the reason
  // there can be no claim, and it outranks a stale one.
  if (nothingToAsk !== undefined) {
    return (
      <p className="armada-holds__verdict" data-degraded>
        <span className="armada-holds__dot" aria-hidden="true" />
        {SILENT[nothingToAsk]}
      </p>
    );
  }
  if (lookFailed !== undefined) {
    return <p className="armada-holds__verdict" data-found="not_working">{lookFailed}</p>;
  }
  if (looking) {
    return <p className="armada-holds__verdict">Looking at this job now.</p>;
  }
  if (examined === null) {
    return (
      <p className="armada-holds__verdict">Nobody has asked whether this job is working.</p>
    );
  }
  return (
    <p className="armada-holds__verdict" data-found={examined.found}>
      {SAID[examined.found]}
    </p>
  );
}

/**
 * The headline, when there is nothing to ask.
 *
 * **Both name Fleet as the subject**: a job that holds nothing is a real
 * answer, and a sentence that did not say which would report a silent seam as
 * a silent job. **They must not read as the same message twice**, because the
 * fixes point in opposite directions.
 */
const SILENT: Record<NothingToAsk, string> = {
  no_answer: "Fleet is not answering, so there is nothing to ask.",
  unreadable: "Fleet answered, and Bridge could not read the answer.",
};

/**
 * What stands in for the reading, and neither of them is a reading.
 *
 * **`no_answer` points at the status bar and `unreadable` must not** — the bar
 * says "Fleet running" on the second, which would read as the two disagreeing.
 * So the sentence carries its own next step, and names the wrong move, because
 * a restart is the one somebody reaches for.
 */
const INSTEAD: Record<NothingToAsk, string> = {
  no_answer:
    "Nothing here is a reading of this job. The status bar names which Fleet state this is and what to do.",
  unreadable:
    "Nothing here is a reading of this job. This build of Bridge and this Fleet do not agree about the route, so restarting Fleet changes nothing. They ship as a pair, and rebuilding both is what settles it.",
};

/**
 * The three findings, said once.
 *
 * **`cannot_tell` names itself rather than hedging.** A person who pressed this
 * because they suspect a hang needs to know the checks came back short, not to
 * read a softened pass.
 */
const SAID: Record<Finding, string> = {
  working: "This job is doing what it should be.",
  not_working: "This job is not doing what it should be.",
  cannot_tell: "Some of these checks could not tell working from not.",
};

/** What each look asked, in the order a person reads them. */
const ASKED: Record<Look["asked"], string> = {
  process: "The process",
  worktree: "The worktree",
  writing: "What was written",
  span: "Where the job is",
  silence: "The liveness watch",
  repeating: "The last two attempts",
  scope_drift: "Work outside the declared scope",
};

/** Every look, with the ones that could not tell marked as such. */
function Looks({ looks }: { looks: Look[] }) {
  return (
    <ul className="armada-holds__looks">
      {looks.map((look) => (
        <li key={look.asked} className="armada-holds__look" data-found={look.found}>
          <span className="armada-holds__asked">{ASKED[look.asked] ?? look.asked}</span>
          <span className="armada-holds__said">{look.said}</span>
          {look.fields === undefined || look.fields.length === 0 ? null : (
            <span className="armada-holds__fields">
              {look.fields.map((field) => (
                <span key={field.name} className="armada-holds__field">
                  <span className="armada-holds__field-name">{field.name}</span>
                  <span className="armada-holds__mono">{field.value}</span>
                </span>
              ))}
            </span>
          )}
        </li>
      ))}
    </ul>
  );
}

