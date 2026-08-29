// The two reads a review is made of, shaped for the surface that draws them,
// and the sentences that say what is not there.
//
// **Nothing here reads a worktree and nothing here parses a transcript.** The
// patch arrives as a served string and the claims arrive as served fields; this
// splits the first into files and lines the way git already wrote it, and hands
// the second on unchanged.
//
// # The patch is bounded, and the bound is loud
//
// `docs/practices/bridge.md` names the v1 failure this app exists to escape and
// says a diff of more than a couple of hundred lines is virtualized. **No
// virtualization library is chosen** — that is `[list-virtualization]`, still
// open — so the honest thing is a hard bound and a sentence saying it was hit.
// A silently truncated diff is a decision taken on part of the work, which is
// the one failure this surface exists to prevent, so the sentence names the
// worktree the region at the foot of the screen gives the path to.
//
// # No line numbers
//
// git states position in the `@@` header and nowhere else. Numbering each line
// would derive a value here and set it beside machine output, where it would
// read as something the repository said.

import { FileCheck } from "lucide-react";
import type { DiffFile, DiffLine, EvidenceTrailEntry } from "@armada/components";

import type { Diff, Evidence } from "../../shared/bridge";
import type { JobDetail as JobWhole } from "../../shared/protocol";
import type { Submitted, Work } from "../../shared/work";

/**
 * How many lines of a patch are drawn before it is cut.
 *
 * A ceiling and not a guess at a page: it is what keeps a 14,000-line patch
 * from putting 14,000 nodes in the document, which is the freeze the v1 failure
 * log recorded nine times in different words. It goes when a virtualization
 * approach is chosen.
 */
export const DRAWN_LINES = 2000;

/** What the drawn files are, and what was left off the end of them. */
export type Drawn = { files: DiffFile[]; cut?: string };

/**
 * The patch, split into the files and lines git wrote, with the drift mark
 * carried over from the file list beside it.
 *
 * **One vocabulary for one fact.** `Work.files` is the same `ChangedFile` a
 * `job.files_changed` event carries and it is what says which paths the step's
 * declared plan does not cover; joining it here means a file that reads drifted
 * in a footprint does not read clean in the diff.
 */
export function drawn(work: Work): Drawn {
  const patch = work.patch;
  if (patch === undefined) return { files: [] };
  const outside = new Set(
    work.files.filter((file) => file.outside_plan === true).map((file) => file.path),
  );
  const parsed = split(patch);
  const files = parsed.files.map((file) => ({
    ...file,
    outsidePlan: outside.has(file.path) || undefined,
  }));
  return parsed.cutAt === undefined
    ? { files }
    : { files, cut: cutSentence(parsed.cutAt, parsed.total) };
}

/**
 * What a cut patch says. **It names where the rest is** rather than trailing
 * off, because the next thing the reader does has to be reading the whole diff
 * and not pressing one of the three buttons under it.
 */
function cutSentence(drawnLines: number, total: number): string {
  return (
    `This is the first ${drawnLines.toLocaleString()} lines of a ${total.toLocaleString()}-line ` +
    "patch. The rest is not on screen. Read the whole diff in the worktree named under " +
    "Where the work is before deciding."
  );
}

/** One file being built as the patch is walked. */
type Building = { path: string; meta?: string; lines: DiffLine[] };

/** The prefix git writes before every file's block. */
const FILE_HEADER = "diff --git ";

/**
 * The header lines that say something about the file rather than about a line
 * of it. `index` is left out: a pair of abbreviated object ids tells a reader
 * nothing they can act on, and it is the noisiest line in every block.
 */
const META = [
  "new file mode ",
  "deleted file mode ",
  "old mode ",
  "new mode ",
  "rename from ",
  "rename to ",
  "copy from ",
  "copy to ",
  "Binary files ",
];

/**
 * A patch that names no file. Fleet renders the patch with `git diff`, which
 * always writes a header, so this is the shape of a reading that broke rather
 * than a case to expect — and it says so rather than drawing a blank row.
 */
const UNNAMED = "(the patch names no file)";

/**
 * The files, the number of lines drawn, and the number the patch held.
 *
 * **`inHunk` is what makes this correct rather than nearly correct.** A content
 * line reading `+++ something` is an added line whose text happens to start
 * with three plusses, and a parser that matched header prefixes anywhere would
 * open a new file in the middle of one — which is the state a diff viewer is
 * least allowed to be in, since the reader would be deciding on a file
 * attributed to the wrong path.
 */
function split(patch: string): { files: DiffFile[]; cutAt?: number; total: number } {
  const rows = patch.split("\n");
  const files: Building[] = [];
  let open: Building | null = null;
  let inHunk = false;
  let drawnSoFar = 0;
  let total = 0;
  let cut = false;

  for (const row of rows) {
    if (row.startsWith(FILE_HEADER)) {
      open = { path: pathOf(row) ?? UNNAMED, lines: [] };
      files.push(open);
      inHunk = false;
      continue;
    }
    if (!inHunk) {
      // The `+++`/`---` pair names the file more reliably than the header
      // does: git escapes both the same way, and the header carries an `a/ b/`
      // pair that a path containing " b/" makes ambiguous. A deletion's `+++`
      // is `/dev/null`, so only a real path replaces what is held.
      if (row.startsWith("+++ ") || row.startsWith("--- ")) {
        const named = stripped(row.slice(4));
        if (open !== null && named !== null) open.path = named;
        continue;
      }
      if (open !== null && META.some((prefix) => row.startsWith(prefix))) {
        open.meta = open.meta === undefined ? row : `${open.meta} \u00b7 ${row}`;
        continue;
      }
    }
    const line = lineOf(row);
    if (line === undefined) continue;
    if (line.kind === "hunk") inHunk = true;
    total += 1;
    if (drawnSoFar >= DRAWN_LINES) {
      cut = true;
      continue;
    }
    if (open === null) {
      open = { path: UNNAMED, lines: [] };
      files.push(open);
    }
    open.lines.push(line);
    drawnSoFar += 1;
  }

  return {
    // A file whose every line fell past the bound is dropped rather than drawn
    // as an empty block: a header with nothing under it reads as a file that
    // changed in no way, which is a claim the patch does not make.
    files: files.filter((file) => file.lines.length > 0),
    ...(cut ? { cutAt: drawnSoFar } : {}),
    total,
  };
}

/**
 * One line of a hunk, or `undefined` where the row is not one.
 *
 * `\ No newline at end of file` is git's own note about the line above it and
 * stays in the block: dropping it would leave a reader thinking a file ends the
 * way every other one does.
 */
function lineOf(row: string): DiffLine | undefined {
  if (row.startsWith("@@")) return { kind: "hunk", text: row };
  if (row.startsWith("+")) return { kind: "added", text: row };
  if (row.startsWith("-")) return { kind: "removed", text: row };
  if (row.startsWith(" ") || row.startsWith("\\")) return { kind: "context", text: row };
  return undefined;
}

/** The path out of a `diff --git a/x b/x` header, or nothing. */
function pathOf(header: string): string | null {
  const rest = header.slice(FILE_HEADER.length);
  const half = Math.floor(rest.length / 2);
  // `a/x b/x` is the same path twice with one space between, so the midpoint
  // is the space on every path that does not itself contain " b/".
  if (rest[half] === " ") return stripped(rest.slice(half + 1));
  const cut = rest.indexOf(" b/");
  return cut === -1 ? null : stripped(rest.slice(cut + 1));
}

/** `a/`, `b/` and `/dev/null` off a header path. `null` where nothing is left. */
function stripped(value: string): string | null {
  const path = value.trim();
  if (path === "" || path === "/dev/null") return null;
  return path.startsWith("a/") || path.startsWith("b/") ? path.slice(2) : path;
}

/**
 * Every submission, step by step, as the trail draws them.
 *
 * **The step's own label where the detail carries one**, so the trail and the
 * rail name the same step the same way. Where it does not — a step id the
 * frozen workflow no longer names — the id renders as itself rather than being
 * replaced by a word chosen here.
 *
 * `evidence_type` renders as the wire spells it. `enum-verbs.toml` carries no
 * rows for it, so there is no verb, glyph or hue and none is invented.
 * Reported.
 */
export function claimsOf(steps: Submitted[], whole: JobWhole | null): EvidenceTrailEntry[] {
  return steps.map((step) => ({
    step: labelOf(step.step_id, whole),
    provenance: step.evidence_type,
    // `file-check` is reserved to a submission that landed, which is what every
    // row here is.
    icon: FileCheck,
    iconLabel: "Evidence",
    claimed: step.claimed,
    shownBy: step.shown_by,
    // Absent is a submission that drew no boundary, and the trail renders
    // "Nothing" for it — which is the reading the field exists to produce.
    notClaimed: step.not_claimed,
  }));
}

/** What a person reads for a step, or the id where the workflow names none. */
function labelOf(stepId: string, whole: JobWhole | null): string {
  const named = whole?.steps.find((step) => step.step_id === stepId);
  return named === undefined ? stepId : named.label;
}

/**
 * Why there is no diff on screen, which is never the same sentence twice.
 *
 * **Four silences, four sentences.** Nobody asked, still reading, the read
 * failed, and a Job with no worktree are four different facts, and one sentence
 * for all of them would tell somebody a Drone wrote nothing when what is true
 * is that nothing was read.
 */
export function whyNoDiff(diff: Diff, jobId: string): string {
  if (diff.state === "failed" && diff.jobId === jobId) {
    return "Fleet did not answer for this job's diff, so what it changed is unknown.";
  }
  return "Reading this job's diff. It is the expensive read, and it is only made here.";
}

/** What an empty reading says. **Ordinary, and never an error.** */
export const CHANGED_NOTHING =
  "This job's worktree opened and holds no change against the branch it was cut from. " +
  "That is what a diff_nonempty check refuses, and it is not the same as a job that never " +
  "had a worktree.";

/** What a reading with no worktree behind it says. A different fact entirely. */
export const NO_WORKTREE =
  "This job has no worktree, so there is nothing to read. Absent is not empty — a drone that " +
  "changed nothing is a different answer, and this is not it.";

/**
 * Where the reading came from. **True of every reading**, which is why it is
 * separable from the plan sentence beside it — see `diffNote`.
 */
export const DIFF_READ_FROM = "Read from this job's worktree against the branch it was cut from.";

/**
 * What an unreadable declaration says, and it is the whole of #157.
 *
 * `get_diff` reads the plan declaration out of the live working slot, which a
 * job whose drone has stopped no longer holds — so `plan_declared` is false
 * there whatever the step declared. The old sentence read "This step declared
 * no plan, so no file is marked," which is **a claim about what the drone did**
 * rather than a report of what can be read, and four of the shipped workflows
 * declare a plan at step start. A reader deciding whether work stayed in scope
 * was being told it was never scoped.
 *
 * **The last clause is not padding.** Dropping the sentence altogether was the
 * other option and it leaves the same wrong reading available: unmarked rows
 * with nothing beside them read as rows inside a plan. So the silence is named
 * and the inference is closed off in the same breath.
 */
export const PLAN_NOT_READABLE =
  "The plan this step declared is not readable once its drone has stopped, so no file is " +
  "marked — an unmarked file here is not a file that was inside the plan.";

/**
 * The same silence, where the record beside it is not silent.
 *
 * **Two tabs of one record must not answer one question two ways.** `Files
 * changed` draws the footprint Fleet kept, marked against every plan the steps
 * declared; this tab is the same paths one level deeper and cannot read a
 * declaration at all. Saying only that the plan is unreadable, a tab away from
 * a list that names the drift, is not false and is not coherent either — so
 * where the record carries a declaration, the sentence sends the reader to it
 * rather than leaving two answers side by side.
 *
 * It names the tab and not a direction: the record is a strip, so `above` would
 * point at nothing.
 */
export const PLAN_IS_IN_THE_RECORD =
  "The plan this step declared is not readable once its drone has stopped, so no file is " +
  "marked here. Files changed is the record kept when the job stopped, and it marks every path " +
  "that fell outside the plans the steps declared.";

/**
 * Where the reading came from, and what the drift mark does or does not mean
 * on it.
 *
 * `planReadable` is the caller saying whether a drone is still holding the pen
 * on this job. **It is not derivable here**: `work` carries `plan_declared`,
 * and false on it is "no plan was declared" and "no plan can be read" at once —
 * which is exactly the conflation #157 was.
 *
 * `markedInRecord` is the caller saying whether the footprint on this Job's
 * `JobDetail` carries a declaration. Also not derivable here: `work` is the
 * live read of a worktree and knows nothing about the record served beside it.
 */
export function diffNote(work: Work, planReadable: boolean, markedInRecord = false): string {
  const read = DIFF_READ_FROM;
  if (!planReadable) {
    return `${read} ${markedInRecord ? PLAN_IS_IN_THE_RECORD : PLAN_NOT_READABLE}`;
  }
  if (!work.plan_declared) {
    return `${read} This step declared no plan, so no file is marked.`;
  }
  const outside = work.files.filter((file) => file.outside_plan === true).length;
  const total = work.files.length;
  return outside === 0
    ? `${read} Every path is inside the plan this step declared.`
    : `${read} ${outside} of ${total} paths are outside the plan this step declared.`;
}

/** Why there are no claims on screen, which is never the same sentence twice. */
export function whyNoClaims(evidence: Evidence, jobId: string): string {
  if (evidence.state === "failed" && evidence.jobId === jobId) {
    return "Fleet did not answer for this job's evidence, so what its drones claimed is unknown.";
  }
  return "Reading what this job's drones claimed.";
}

/**
 * What a Job with no submissions says. **Ordinary, and a real finding** — a Job
 * that reached a human gate having claimed nothing is exactly the case the
 * submission schema exists to make visible.
 */
export const CLAIMED_NOTHING =
  "No step on this job has submitted evidence. The work is here to read, and nothing states " +
  "what it was meant to do.";

/**
 * What the confirmation for a rejection says. **What happens and what
 * survives**, the two halves the copy contract asks for — and it must not read
 * as a heavier request for changes, so it names the drone.
 */
export const CONFIRM_REJECT = {
  title: "Reject this job's work?",
  body:
    "The job ends at rejected, which is terminal and carries a verdict — this is a decision " +
    "about the work rather than a kill. The drone is stopped and nothing resumes it, and the " +
    "branch stays where it left it. To send the work back instead, close this and request " +
    "changes: that keeps the drone, the worktree and the step.",
} as const;
