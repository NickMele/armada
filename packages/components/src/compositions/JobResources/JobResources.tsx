// Pulse — what one Job is costing the machine right now: what is running, what
// it is spending, the processes, the checkouts and the logs.
//
// Not a debug panel. The first thing on it is a sentence answering *is this
// working*, and the figures come after. `docs/contracts/design-system.md` →
// Fleet panel, for the figure rows; #1538 for the board.

import { Cpu, HardDrive, Search } from "lucide-react";
import type { ReactNode } from "react";

import type { Finding, JobExamined, Look } from "@armada/protocol";
import { Button } from "../../primitives/Button/Button";
import { FigureList, type Figure } from "../FigureList/FigureList";

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

/** One process this Job holds, and which checkout it belongs to. */
export type PulseProcessRow = {
  pid: number;
  /**
   * The executable's own name — `node`, `cargo`, `git`. **Never its argument
   * vector**, which carries absolute paths and whatever a Check was invoked with.
   */
  command: string;
  /**
   * The checkout it belongs to, by branch. **`null` is a process nothing could
   * place** — with several checkouts it must not read as the first one.
   */
  owner: string | null;
  /** Share of one core. **Not capped at 100**: a process across four is real. */
  cpuPercent: number;
  memoryBytes: number;
  /** `ps`'s own spelling of how long it has been up. Rendered, never parsed. */
  runningFor: string;
  /** The process Fleet wrote down. At most one row carries it. */
  recorded: boolean;
};

/**
 * One checkout this Job holds.
 *
 * **A list, not a field.** A Job running several Drones holds one per member
 * and a Job running one holds one — the same drawing at a different count,
 * rather than a screen that changes shape the day a Job gains a member.
 */
export type PulseWorktreeRow = {
  /** What names the row. Two checkouts of one Job differ by their branch. */
  branch: string;
  path: string;
  /** What it is doing, in the caller's words — a member's state, or the Job's. */
  state: string;
  /** What it takes on disk. **Absent is a walk that ran past its bound, never zero.** */
  bytes?: number;
  /** Whether the state is a fault. Draws it in `--error`. */
  wrong?: boolean;
};

/** One log file this Job is writing, or has written. */
export type PulseLogRow = {
  /** Which log it is — the Job's own, a member's, a Check's. */
  kind: string;
  /** The member it belongs to. `null` is the Job's own. */
  owner: string | null;
  /** What it weighs. **Absent is unmeasured, never zero.** */
  bytes?: number;
  /** Whether something is writing to it right now. */
  writing: boolean;
};

/**
 * What this Job is taking, at one instant.
 *
 * **The instant rides with the figures rather than beside them.** A process
 * can exit between the sample and the render, so a caller cannot hand over the
 * numbers without the time they were true.
 */
export type PulseReading = {
  /**
   * Fleet's reading of its recorded Drone: `running`, `gone`, `replaced`, …
   *
   * **`string`, not the wire's union.** A Fleet ahead of this build can send a
   * reading it has no sentence for, and `unknownHold` says so rather than
   * leaving the row blank.
   */
  held: string;
  /** When every figure here was read. */
  readAt: string;
  /** The recorded process and everything descended from it, that one first. */
  processes: PulseProcessRow[];
  /** One row per member where the Job has members, one otherwise. */
  worktrees: PulseWorktreeRow[];
  /** One row per log file. Empty is a Job that has written none. */
  logs: PulseLogRow[];
};

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
          <Button size="sm" onClick={onExamine} disabled={looking}>
            <Search size={12} strokeWidth={2} aria-hidden="true" />
            {looking ? "Looking" : "Look now"}
          </Button>
        )}
      </div>

      {figures.length === 0 ? null : <FigureList figures={figures} column="fit" />}

      {examined === null ? null : <Looks looks={examined.looks} />}

      {reading === null ? (
        <p className="armada-holds__note">
          {nothingToAsk === undefined
            ? (note ?? "Nothing has been read yet.")
            : INSTEAD[nothingToAsk]}
        </p>
      ) : (
        <>
          <Region label="Processes">
            <Processes reading={reading} examined={examined} />
          </Region>
          <Region label="Worktrees">
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
 */
function readAt(reading: PulseReading, age?: string, refreshed?: string): string {
  const said =
    age === undefined
      ? `Read at ${reading.readAt}.`
      : `Read ${age} ago. A process can exit between the reading and this screen.`;
  return refreshed === undefined ? said : `${said} ${refreshed}`;
}

/**
 * One band of the board and what is under it.
 *
 * **A label on every list, including an empty one.** A region that disappeared
 * when it held nothing would make "no worktree on disk" and "this build does
 * not draw worktrees" the same screen.
 */
function Region({ label, children }: { label: string; children: ReactNode }) {
  return (
    <section className="armada-holds__region" aria-label={label}>
      <h3 className="armada-holds__band">{label}</h3>
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
      <p className="armada-holds__verdict">
        Nobody has asked whether this job is working. Looking costs no model call.
      </p>
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

/**
 * The processes, or the sentence that says there are none — four of those,
 * because *no drone was expected*, *fleet believes one is running and it is
 * gone*, *the pid came round as something else* and *the probe would not run*
 * are four different things to do next.
 *
 * **The owner is a column and not a suffix.** *Which member is eating the
 * machine* is what a person opens this with, and a column is what reads down.
 */
function Processes({ reading, examined }: { reading: PulseReading; examined: JobExamined | null }) {
  if (reading.processes.length === 0) {
    return (
      <p
        className="armada-holds__nothing"
        data-loud={nothingRunningIsAFault(reading.held, examined) || undefined}
      >
        {NO_PROCESS[reading.held] ?? unknownHold(reading.held)}
      </p>
    );
  }
  return (
    <table className="armada-holds__table">
      <thead>
        <tr>
          <th scope="col">
            <Cpu size={12} strokeWidth={2} aria-hidden="true" /> Process
          </th>
          <th scope="col">Belongs to</th>
          <th scope="col">CPU</th>
          <th scope="col">Memory</th>
          <th scope="col">Running for</th>
        </tr>
      </thead>
      <tbody>
        {reading.processes.map((one) => (
          <Row key={one.pid} process={one} />
        ))}
      </tbody>
    </table>
  );
}

/**
 * Whether an absence is a fault or an ordinary state.
 *
 * **The examination decides where there has been one**, because whether a job
 * ought to hold a process is a question about its status and this component has
 * none. Without one, `gone` and `replaced` are loud on their own and `none` is
 * quiet — a job at its approval gate holds nothing and is right to.
 *
 * **Exported**, because `JobHoldsSummary` asks the same question and a second
 * copy would let the summary and the board disagree about one Job.
 */
export function nothingRunningIsAFault(held: string, examined: JobExamined | null): boolean {
  const look = examined?.looks.find((one) => one.asked === "process");
  if (look !== undefined) return look.found === "not_working";
  return held === "gone" || held === "replaced";
}

const NO_PROCESS: Record<string, string> = {
  none: "Fleet holds no process for this job. Nothing has been dispatched, or the drone that was here has gone.",
  running:
    "Fleet's process is alive and the process table would not read, so what it is running is unknown.",
  gone: "Fleet believes a process is running here and nothing holds that pid. Nothing is running.",
  replaced:
    "The pid fleet recorded is held by a different process. The drone that was here has gone.",
  unreadable: "The process check would not run, so nothing here can say what this job holds.",
};

function Row({ process }: { process: PulseProcessRow }) {
  return (
    <tr data-recorded={process.recorded || undefined}>
      <td>
        <span className="armada-holds__mono">{process.command}</span>
        <span className="armada-holds__pid">{process.pid}</span>
      </td>
      {/* A process nothing could place says so. **Never the first worktree**,
          which with several checkouts would name the wrong member. */}
      <td className="armada-holds__mono">{process.owner ?? NOT_PLACED}</td>
      <td className="armada-holds__mono">{process.cpuPercent.toFixed(1)}%</td>
      <td className="armada-holds__mono">{sized(process.memoryBytes)}</td>
      <td className="armada-holds__mono">{process.runningFor}</td>
    </tr>
  );
}

/** A process no checkout claims. A real answer, and never a guess at one. */
const NOT_PLACED = "not placed";

/**
 * A reading this build has no sentence for. **The wire's own spelling, said as
 * unreadable** — never a blank, and never rounded into one of the five.
 */
function unknownHold(held: string): string {
  return `Fleet reports this job as ${held}, which this build of Bridge has no reading for.`;
}

/**
 * The checkouts this Job holds, one row each.
 *
 * **Absent and unmeasured are two sentences.** No worktree is a job at its
 * approval gate or one already reclaimed; a size that did not arrive is a walk
 * that ran past its bound, which is what a very large checkout does — and that
 * is itself worth knowing.
 */
function Worktrees({ worktrees }: { worktrees: PulseWorktreeRow[] }) {
  if (worktrees.length === 0) {
    return <p className="armada-holds__note">No worktree on disk.</p>;
  }
  return (
    <ul className="armada-holds__disks">
      {worktrees.map((one) => (
        <li key={one.branch} className="armada-holds__disk">
          <HardDrive size={12} strokeWidth={2} aria-hidden="true" />
          <span className="armada-holds__size">
            {one.bytes === undefined ? NOT_WALKED : sized(one.bytes)}
          </span>
          <span className="armada-holds__mono">{one.branch}</span>
          <span className="armada-holds__state" data-wrong={one.wrong || undefined}>
            {one.state}
          </span>
          <span className="armada-holds__mono" title={one.path}>
            {one.path}
          </span>
        </li>
      ))}
    </ul>
  );
}

/**
 * The Job's logs, one row per file, marked where one is still being written.
 *
 * **Being written is the fact the list exists for.** A log that stopped
 * growing while its Job reads running is the shape of a hang, and a list of
 * file names with no such mark leaves a person opening each one to find out.
 */
function Logs({ logs }: { logs: PulseLogRow[] }) {
  if (logs.length === 0) {
    return <p className="armada-holds__note">{NO_LOGS}</p>;
  }
  return (
    <ul className="armada-holds__logs">
      {logs.map((one) => (
        <li key={`${one.owner ?? ""}/${one.kind}`} className="armada-holds__log">
          <span className="armada-holds__mono">{one.kind}</span>
          <span className="armada-holds__mono">{one.owner ?? THE_JOBS_OWN}</span>
          <span className="armada-holds__size">
            {one.bytes === undefined ? NOT_WEIGHED : sized(one.bytes)}
          </span>
          {one.writing ? (
            <span className="armada-holds__writing">still being written</span>
          ) : (
            <span className="armada-holds__mono">{NOT_WRITING}</span>
          )}
        </li>
      ))}
    </ul>
  );
}

/** A Job nothing has written a log for. Not a missing read — there are none. */
const NO_LOGS = "Nothing has been written to this job's logs.";

/** A log that belongs to the Job rather than to one of its members. */
const THE_JOBS_OWN = "this job";

/** A file nothing weighed. **Its own answer, and never a zero.** */
const NOT_WEIGHED = "not measured";

/** A walk that ran past its bound. The same fact on a directory. */
const NOT_WALKED = "Not measured in time";

/** Nothing has this file open. Said in words, so a row is never a blank. */
const NOT_WRITING = "not being written";

/**
 * Bytes as a person reads them. **Binary units and their own names**, because
 * `du` and `df` answer in them and a figure that disagreed with the shell a
 * person is about to open would be worse than no figure.
 */
export function sized(bytes: number): string {
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  let at = 0;
  let value = bytes;
  while (value >= 1024 && at < units.length - 1) {
    value = value / 1024;
    at += 1;
  }
  return `${at === 0 ? value : value.toFixed(1)} ${units[at]}`;
}
