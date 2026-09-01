// What a finished job touched, as TypeScript sees it.
// `crates/ipc/src/work.rs`.
//
// **Its own file, because `protocol.ts` reached the 900 lines the gate refuses
// at.** These four types are one subject and the only one on that file with a
// natural seam: they are read together or not at all, nothing outside a
// finished job's record uses any of them, and they name no other wire type — so
// the import runs one way, from `protocol.ts` to here.
//
// **The record, not the live reading.** `ChangedFile` on a `job.files_changed`
// event is what a drone is doing now and stays in `protocol.ts` beside the
// event that carries it. This is what was written down when the job stopped,
// which is the one reading that survives `armada clean` giving the worktree
// back — and the only one carrying line counts, because counting costs the
// walk that renders the patch.

/**
 * What one job's worktree held when the job stopped. On `JobDetail` rather than
 * a read of its own: it is a path and a word per file, and Fleet asks for it
 * only where a job has one, so an open of a running job pays nothing.
 */
export type JobFootprint = {
  /** Every file, in the order the reading found them. */
  files: TouchedFile[];
  /**
   * When the reading was taken. **The instant the job stopped**, not the
   * instant it was asked for — which is what makes this a record and lets a
   * surface say so.
   */
  recorded_at: string;
  /**
   * What each run of each step said its work would be, in declaration order.
   * Since protocol 4.17, and absent from a fleet older than that.
   *
   * **Empty is the whole of "there is nothing to be outside of."** Every
   * `TouchedFile.planned_by` is then absent rather than empty, so a surface
   * that never reads this list still cannot draw an unmeasured path as one
   * that stayed in scope.
   */
  plans?: DeclaredPlan[];
};

/**
 * What one run of one step promised its work would be. Since protocol 4.17.
 *
 * **The promise, beside the record of what was done.** A footprint is the job's
 * whole work and a plan belongs to one step, so the two arrive side by side
 * rather than folded into one mark. A step that never declared has no entry: it
 * is silent, not counted.
 */
export type DeclaredPlan = {
  step_id: string;
  /**
   * Which run of that step declared it, one-based. **A step may be worked twice
   * and then declares twice**, and without this the two entries would read as
   * one step contradicting itself.
   */
  attempt: number;
  /** When the declaration was taken, by fleet's clock. */
  declared_at: string;
  /**
   * The paths the drone named, each covering everything beneath it at a segment
   * boundary. **Empty is a declaration of nothing**, which every changed path is
   * outside of — not a step that never declared.
   */
  paths: string[];
};

/**
 * One file a finished job touched.
 *
 * **Not `ChangedFile`, and the drift mark is the reason.** A live reading
 * carries `outside_plan` as a boolean, because the step being watched declared
 * the plan it is measured against. A record is the job's whole work, and the
 * step holding the pen when a job stops is usually not the step that scoped it
 * — so one boolean here could only be right by accident. `planned_by` names
 * steps rather than asserting a verdict.
 */
export type TouchedFile = {
  /** Repository-relative, exactly as git spells it. */
  path: string;
  /** The same closed set `ChangedFile.change` carries, left as `string`. */
  change: string;
  /**
   * The steps whose declared plan covers this path, in `JobFootprint.plans`
   * order. Since protocol 4.17.
   *
   * **Three readings, and the absent one is why this is not a boolean.** Absent
   * is a job where no step declared anything, so nothing was measured. Present
   * and empty is a path outside every plan that was declared — the drift a
   * finished job could not state before. Present with steps in it is a path one
   * of those steps promised.
   */
  planned_by?: string[];
  /**
   * What the file gained and lost. Since protocol 6.3.
   *
   * **Absent where nothing counted it** — a binary file, a footprint recorded
   * by a fleet older than this, or a patch the repository would not build.
   * Absent is not zero: a file moved without being edited is present and `0`.
   *
   * **`ChangedFile` has no counterpart and is not getting one.** The live
   * reading is taken every two seconds inside fleet's 250ms turn, and counting
   * is the same walk that renders the patch — 25ms over a hundred files, 90ms
   * over four hundred. This is read once, when the job stops.
   */
  lines?: LineCount;
};

/**
 * How many lines one file gained and lost. Since protocol 6.3.
 *
 * **One object rather than two optional numbers**, because the repository
 * answers both from one walk of one file's patch or answers neither.
 */
export type LineCount = {
  added: number;
  deleted: number;
};
