import { Cpu, HardDrive, Search } from "lucide-react";

import type { Finding, JobExamined, JobProcess, JobResources as Held, Look } from "@armada/protocol";
import { Button } from "../../primitives/Button/Button";

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

/**
 * What this job holds on the machine, and the act that goes and looks.
 *
 * **Not a debug panel.** The moment it is for is a person worried about a job,
 * not an engineer instrumenting one, so the first thing on it is a sentence
 * answering *is this working* and the figures come after. A dump of numbers
 * makes a person do the reading; this does it and shows its working.
 *
 * **Nothing running is drawn as loudly as a failure.** A job that reads running
 * and holds no process is the state that took a terminal to establish, and an
 * empty list under a heading is exactly how it went unnoticed — so the absence
 * is a sentence in the failed treatment rather than a table with no rows.
 *
 * **Every figure carries when it was read.** A process can exit between the
 * sample and the render. The instant is the last line rather than a caption,
 * because it qualifies everything above it.
 *
 * **What could not be told is on screen, never rounded up.** An examination
 * that answers *everything looks fine* on a plainly hung job spends a person's
 * suspicion and returns nothing, so a look that cannot separate working from
 * not says so in its own row and keeps the headline off working.
 *
 * **No byte count is drawn without its unit resolved.** Disk is the figure with
 * a second reason to exist — seventy-four worktrees once took 220 GB and three
 * agents died at zero bytes free — and it is the one number here a person acts
 * on directly.
 *
 * **The act is not offered where pressing it could not work.** Every reading
 * here is Fleet's and the button asks Fleet for one, so the panel has to know
 * when there is nothing on the other end to ask — `nothingToAsk`. Drawn as an
 * unpressed panel it read "Nobody has asked whether this job is working" over
 * a live control, which is the opposite of what was true, and a disabled
 * control with no sentence beside it is the same dead end drawn quieter.
 */
export type JobResourcesProps = {
  /**
   * The reading, or `null` where none has arrived.
   *
   * **`null` is not empty.** A read that has not answered and a job that holds
   * nothing are different things, and `note` is what says which.
   */
  reading: Held | null;
  /** Why there is no reading, where there is none. */
  note?: string;
  /** How old the reading is, as a phrase — `4s`. Formatted by the caller. */
  age?: string;
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
   * **Not `lookFailed`, and the difference is what to do next.** A failed look
   * is one attempt that did not come back, which invites another; both of
   * these say attempts are not the shape of the problem. The caller decides it
   * from what the failure was, never from the fact that something failed — a
   * read Fleet refused or answered late is a read worth sending again.
   *
   * **Two readings and not one**, because the fixes are opposite. `no_answer`
   * is a Fleet that may only need starting; `unreadable` is a Fleet that is
   * running and would come back the same, so restarting it is the wrong move
   * and the pair has to be rebuilt. One sentence covering both would send
   * somebody to do the wrong one half the time.
   *
   * **A reading rather than a sentence**, because the copy is fixed and
   * belongs to one producer.
   */
  nothingToAsk?: NothingToAsk;
  onExamine: () => void;
};

export function JobResources({
  reading,
  note,
  age,
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
            the reason on a person to work out, and both of these have a reason
            worth reading. */}
        {nothingToAsk !== undefined ? null : (
          <Button size="sm" onClick={onExamine} disabled={looking}>
            <Search size={12} strokeWidth={2} aria-hidden="true" />
            {looking ? "Looking" : "Look now"}
          </Button>
        )}
      </div>

      {examined === null ? null : <Looks looks={examined.looks} />}

      {reading === null ? (
        <p className="armada-holds__note">
          {nothingToAsk === undefined
            ? (note ?? "Nothing has been read yet.")
            : INSTEAD[nothingToAsk]}
        </p>
      ) : (
        <>
          <Processes reading={reading} examined={examined} />
          <Disk reading={reading} />
          <p className="armada-holds__read-at">
            {age === undefined
              ? `Read at ${reading.read_at}.`
              : `Read ${age} ago. A process can exit between the reading and this screen.`}
          </p>
        </>
      )}
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
 * **Both name Fleet as the subject, because the alternative reading is the one
 * this panel exists to make loud.** A job that holds nothing is a real answer
 * and reads as one; these are the panel having nothing to report, and a
 * sentence that did not say which would report a silent seam as a silent job.
 *
 * **They must not read as the same message twice**, because they are not the
 * same condition and the fixes point in opposite directions. One is a Fleet
 * that is not there; the other is a Fleet that is up and answering, which is
 * why "not answering" would be false on it.
 */
const SILENT: Record<NothingToAsk, string> = {
  no_answer: "Fleet is not answering, so there is nothing to ask.",
  unreadable: "Fleet answered, and Bridge could not read the answer.",
};

/**
 * What stands in for the reading, and neither of them is a reading.
 *
 * **Unknown rather than absent**, said out loud in both, because everything
 * else on this panel is a figure and an empty panel reads as a job holding
 * nothing.
 *
 * **`no_answer` points at the status bar and `unreadable` must not.** The bar
 * names which Fleet state a silent one is and what to do about it, and on the
 * second of these it says "Fleet running" — which is true, and would read as
 * this panel and the bar disagreeing about the same moment. So the sentence
 * carries its own next step.
 *
 * **The wrong move is named, because it is the one somebody reaches for.** A
 * Fleet that is up and unreadable comes back the same after a restart: the two
 * were built apart, and building them together is the fix. No version and no
 * word for the disagreement — a number here is machinery, and the person
 * reading this needs the act.
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
 * The processes, or the sentence that says there are none.
 *
 * **The absence is the loud case and it is prose.** Four arms rather than one,
 * because *no drone was expected*, *fleet believes one is running and it is
 * gone*, *the pid came round as something else* and *the probe would not run*
 * are four different things to do next.
 */
function Processes({ reading, examined }: { reading: Held; examined: JobExamined | null }) {
  if (reading.processes.length === 0) {
    return (
      <p className="armada-holds__nothing" data-loud={loud(reading, examined) || undefined}>
        {NO_PROCESS[reading.held]}
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
 * no status. Without one, `gone` and `replaced` are loud on their own — fleet
 * believing something is running that is not is a fault at any status — and
 * `none` is left quiet, since a job at its approval gate holds nothing and is
 * right to.
 */
function loud(reading: Held, examined: JobExamined | null): boolean {
  const look = examined?.looks.find((one) => one.asked === "process");
  if (look !== undefined) return look.found === "not_working";
  return reading.held === "gone" || reading.held === "replaced";
}

const NO_PROCESS: Record<Held["held"], string> = {
  none: "Fleet holds no process for this job. Nothing has been dispatched, or the drone that was here has gone.",
  running:
    "Fleet's process is alive and the process table would not read, so what it is running is unknown.",
  gone: "Fleet believes a process is running here and nothing holds that pid. Nothing is running.",
  replaced:
    "The pid fleet recorded is held by a different process. The drone that was here has gone.",
  unreadable: "The process check would not run, so nothing here can say what this job holds.",
};

function Row({ process }: { process: JobProcess }) {
  return (
    <tr data-recorded={process.recorded || undefined}>
      <td>
        <span className="armada-holds__mono">{process.command}</span>
        <span className="armada-holds__pid">{process.pid}</span>
      </td>
      <td className="armada-holds__mono">{process.cpu_percent.toFixed(1)}%</td>
      <td className="armada-holds__mono">{sized(process.memory_bytes)}</td>
      <td className="armada-holds__mono">{process.running_for}</td>
    </tr>
  );
}

/**
 * The disk the worktree has taken.
 *
 * **Absent and unmeasured are two sentences.** No worktree is a job at its
 * approval gate or one already reclaimed; a size that did not arrive is a walk
 * that ran past its bound, which is what a very large checkout does — and that
 * is itself worth knowing.
 */
function Disk({ reading }: { reading: Held }) {
  if (reading.worktree === undefined) {
    return <p className="armada-holds__note">No worktree on disk.</p>;
  }
  const { path, branch, bytes } = reading.worktree;
  return (
    <p className="armada-holds__disk">
      <HardDrive size={12} strokeWidth={2} aria-hidden="true" />
      <span className="armada-holds__size">
        {bytes === undefined ? "Not measured in time" : sized(bytes)}
      </span>
      <span className="armada-holds__mono" title={path}>
        {path}
      </span>
      <span className="armada-holds__mono">{branch}</span>
    </p>
  );
}

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
