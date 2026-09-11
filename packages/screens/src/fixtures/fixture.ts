// One Job, at one moment, in the shape Fleet sends it.
//
// **This is `JobDetailProps`' wire half, and nothing else.** The app never
// hands `JobDetail` a derived prop — it hands it `JobSummary`, `Watched` and
// four more reads, and derives the panel itself through `chapters.tsx`,
// `phases.tsx`, `heading.tsx` and `run.ts`. A story that built `InsideAJob`'s
// props directly skipped that derivation, and a bug in it — the block-heading
// wiring `chapters.test.ts` now pins — shipped because nothing rendered it.
// `JobFixture` is the input a story gives `JobDetail` instead, so the same
// derivation the app runs is what a story exercises.
//
// **Every value typed against `@armada/protocol`.** `build/*` composes these
// from a base Job rather than inventing a shape, because the point of this
// file is that drift from the wire is a compile error and not a story that
// quietly stops matching what Fleet sends.

import type {
  CallRead,
  CheckOutputRead,
  FrameRead,
  Holds,
  JobSummary,
  Journalled,
  ManifestSummary,
  Observed,
  Watched,
  WorkflowSummary,
} from "@armada/protocol";
import type { FoldedReads } from "../JobDetail";

/** One Job at one moment, in the shape Fleet sends it — what `JobDetail` is given. */
export type JobFixture = {
  /** The state, as a sentence — it becomes the story's name. */
  name: string;
  job: JobSummary;
  /** `GET /jobs/:job_id`, in state `read`. */
  watched: Watched;
  workflows: WorkflowSummary[];
  manifests: ManifestSummary[];
  observed: Observed;
  journalled: Journalled;
  resources: Holds;
  recorded: FoldedReads;
  /**
   * Answers to `onReadCall`, keyed by the call id a transcript row carries —
   * `Saw.called.call` and `Saw.answered.call`, the same id `CallArguments.call`
   * echoes back. `Log.tsx` reads through this key (`cut.id`) when a person
   * presses a cut argument open.
   */
  calls: Record<string, CallRead>;
  /**
   * Answers to `onReadCheckOutput`, keyed by the **basename** of
   * `CheckRun.output_path` — `checks.tsx`'s `ChecksOutput` calls
   * `outputs.fetch(basename(reading.output_path))`, never the whole path, so a
   * fixture keeping the full path here would never be found.
   */
  checkOutputs: Record<string, CheckOutputRead>;
  /**
   * Answers to `onReadFrame`, keyed by `KeptFrame.kept` — the run's directory
   * and file name, joined, exactly as Fleet sent it on the step's `frames[]`.
   */
  frames: Record<string, FrameRead>;
  /** The clock the fixture was taken at, so an elapsed reads the same every time. */
  now: number;
};
