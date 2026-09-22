// Pulse's three lists: the processes a Job holds, the checkouts they are in,
// and the logs being written. Beside `JobResources.tsx` rather than inside it,
// because that file is the verdict and the act and this is the reading.

import { Cpu, HardDrive } from "lucide-react";

import type { JobExamined } from "@armada/protocol";

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
 * The processes, or the sentence that says there are none — four of those,
 * because *no drone was expected*, *fleet believes one is running and it is
 * gone*, *the pid came round as something else* and *the probe would not run*
 * are four different things to do next.
 *
 * **The owner is a column and not a suffix.** *Which member is eating the
 * machine* is what a person opens this with, and a column is what reads down.
 */
export function Processes({ reading, examined }: { reading: PulseReading; examined: JobExamined | null }) {
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
      <td className="armada-holds__owner">{process.owner ?? NOT_PLACED}</td>
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
export function Worktrees({ worktrees }: { worktrees: PulseWorktreeRow[] }) {
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
export function Logs({ logs }: { logs: PulseLogRow[] }) {
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