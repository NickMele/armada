// What a step's gates answered, as TypeScript sees it. `crates/ipc/src/judged.rs`.
//
// **Split out of `protocol.ts`, and the cut is one the Rust side already made.**
// That file reached the 900 lines the gate refuses for the fourth time, and
// `judged.rs` was itself split off `detail.rs` on the same line for the same
// reason: these are what the gates *said* about a step, and a Job's shape is a
// different sentence. Splitting anywhere else would have invented a seam;
// `protocol.ts` re-exports every name here, so nothing that imported one had to
// change.
//
// The header rules there hold here: these are hand-written, they drift the day
// a field moves, and every closed set is left as `string`.

/**
 * One criterion the Judge answered.
 *
 * **A refusal is not a failed Check and does not read as one.** A Check says
 * the work is broken; a refusal says the work runs and is not what was asked
 * for, which is why one ends the Job and the other escalates it. The three
 * optional fields are what a refusal owes and a no-objection does not: there is
 * nothing to cite where nothing was refused, and `""` would read as a citation
 * somebody lost.
 */
export type Judged = {
  /** Which run of the step this was answered on, counted from one. Since
   * 7.0. Joins to `StepDetail.attempts`. */
  attempt: number;
  /** Which criterion was asked. Joins to `JobDetail.acceptance_criteria`. */
  criterion_id: string;
  /** `criterion_verdict_judge`: `met` or `not_met`. */
  verdict: string;
  /** What should be seen if the work were right. */
  expected?: string;
  /** What is seen instead. */
  produced?: string;
  /** What that difference does to whoever consumes it. The triage line. */
  consequence?: string;
  /**
   * Where the whole brief this verdict answers was written, relative to the
   * repository root. **The path, never the question** — a brief is the request,
   * the deliverable and the whole branch diff, and no answer on this seam
   * carries one. Absent where Fleet kept no brief, which is a verdict nobody
   * can re-read against its input. Opened the way `CheckRun.output_path` is.
   */
  brief_path?: string;
};

/**
 * One copy of a step's deliverable, as Fleet kept it.
 *
 * **A reference, never the document.** A deliverable is up to 16 KiB of text
 * and a detail is re-read every time an event names the open Job; the path is
 * what `main/open.ts` hands to the OS.
 *
 * **The attempt is on the row rather than implied by its position.** A step
 * worked three times keeps three copies and they are three different documents,
 * so a list a reader had to count through would make *the one the Judge read* a
 * guess. It is the same ordinal `StepAttempt.attempt` carries.
 */
export type KeptDeliverable = {
  /** Which run of the step wrote it, counted from one. Joins to `attempts`. */
  attempt: number;
  /**
   * Where the copy is, relative to the repository root.
   *
   * **Fleet checked it was there when it answered**, which is the one thing the
   * renderer cannot check for itself. It can still be gone by the time somebody
   * clicks it, and main says so.
   */
  path: string;
};

/**
 * One gaming pattern found, and what it was found in. **Never a verdict** — a
 * flag says the evidence is suspect, not that the step failed. `pattern` is a
 * string because no registry declares the set: it comes from what a workflow's
 * `flag_if` names.
 */
export type Flagged = {
  /** The pattern, spelled as `flag_if` spells it. */
  pattern: string;
  /** The file, line or assertion the flag is about. An uncited flag is unactionable. */
  cited: string;
};
