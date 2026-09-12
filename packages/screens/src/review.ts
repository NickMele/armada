// The two reads a review is made of, shaped for the surface that draws
// them, and the sentences that say what is not there.
// **Nothing here reads a worktree and nothing here parses a transcript** —
// the patch arrives as a served string, claims as served fields; this
// splits the first into files and lines as git wrote it, hands the
// second on unchanged.
//
// **The patch is bounded, loudly** — `docs/practices/bridge.md` names
// the v1 failure this escapes; no virtualization chosen
// (`[list-virtualization]`, open) means a hard bound with a sentence
// naming the worktree the screen's foot links to.
//
// **No line numbers** — git states position in the `@@` header and
// nowhere else; numbering each line would derive a value and set it
// beside machine output, reading as something the repository said.

import { FileCheck } from "lucide-react";
import { EVIDENCE_TYPE } from "@armada/components";
import type { DiffFile, DiffLine, EvidenceTrailEntry } from "@armada/components";

import type { Diff, Evidence, Remarks } from "@armada/protocol";
import type { JobDetail as JobWhole } from "@armada/protocol";
import type { Submitted, Work } from "@armada/protocol";
import { hostLabel } from "./facts";

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
 * **The word for an evidence type is the registry's.** `enum-verbs.toml`
 * carries six `evidence_type` rows and each was authored for this line — the
 * `diff` row says so outright, that it renders in mono on the provenance line
 * beside the time, which is a label and not a badge. A type this build's
 * registry does not carry falls back to the wire spelling, which is
 * recoverable; nothing here invents a word.
 */
export function claimsOf(steps: Submitted[], whole: JobWhole | null): EvidenceTrailEntry[] {
  return steps.map((step) => ({
    step: labelOf(step.step_id, whole),
    provenance: EVIDENCE_TYPE[step.evidence_type]?.verb ?? step.evidence_type,
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
    return "Fleet did not answer";
  }
  return "Reading this job's diff.";
}

/** What an empty reading says. **Ordinary, and never an error.** */
export const CHANGED_NOTHING =
  "This job's worktree opened and holds no change against the branch it was cut from. " +
  "That is what a diff_nonempty check refuses, and it is not the same as a job that never " +
  "had a worktree.";

/**
 * The same fact as `NO_WORKTREE`, short enough for a header line.
 *
 * **A header says which silence this is, or it says nothing useful.** It read
 * `no reading`, which is true of a Job Fleet never answered for, a Job being
 * read right now, and a Job whose worktree was given back — three different
 * things wearing one phrase, above a body that names them apart. The Produced
 * chapter beside it still lists what the Job wrote, so a person reading both
 * had a count against a blank and no way to tell which was lying.
 */
export const WORKTREE_GIVEN_BACK = "the worktree was given back";

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
 * `get_diff` reads the plan declaration from the slot this job's own drone
 * holds, which a stopped drone no longer has — so `plan_declared` is false
 * there whatever the step declared. Fleet now keeps a roster of slots and
 * reads keyed by job id, so a second job beside this one changes nothing.
 * The old sentence read "This step declared no plan, so no file is
 * marked," **a claim about what the drone did** rather than a report of
 * what can be read — four shipped workflows declare a plan at step start,
 * so a reader checking scope was told it was never scoped.
 *
 * **The last clause is not padding** — dropping the sentence leaves the
 * same wrong reading available: unmarked rows read as rows inside a plan.
 * So the silence is named and the inference closed off in the same breath.
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
    return "Fleet did not answer";
  }
  return "Reading what this job's drones claimed.";
}

/**
 * What a Job with no submissions says. **Ordinary, and a real finding** — a Job
 * that reached a human gate having claimed nothing is exactly the case the
 * submission schema exists to make visible.
 */
export const CLAIMED_NOTHING = "No evidence submitted";

/**
 * Why there is no conversation on screen, which is never the same sentence
 * twice — and the failure here says more than the other two.
 *
 * **A forge that would not answer is not a pull request nobody commented on.**
 * Every other read on this surface reaches the machine Fleet is on; this one
 * reaches a network and an account, and a person shown a silence as an empty
 * review would conclude their comments had vanished. So the sentence says
 * outright which of the two this is.
 */
export function whyNoRemarks(remarks: Remarks, jobId: string): string {
  if (remarks.state === "failed" && remarks.jobId === jobId) {
    return (
      "Fleet could not read this pull request, so what anybody wrote on it is unknown. " +
      "That is not the same as a pull request with no comments on it."
    );
  }
  return "Reading what people wrote on this pull request.";
}

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

/**
 * What the confirmation for a merge says.
 *
 * **The one act in Armada that writes into a repository Fleet did not make.**
 * `crates/fleet/src/merging.rs` opens on that sentence, and it is why this
 * confirms while approving does not: everything else a press here does is
 * confined to a worktree Fleet cut, and this changes what everybody else
 * builds on.
 *
 * **It states the mechanism and promises no outcome**, which is the design
 * contract's rule about a confirmation. So it names where the commits land,
 * says the after-merge checks run against what landed — the whole reason Fleet
 * performs the merge rather than a person doing it on the code host themselves —
 * and says plainly that taking it back is a revert somebody makes in the
 * repository, because Bridge has no undo to offer and a dialog implying one
 * would be worse than the reading.
 *
 * **It does not ask "merge this pull request?" a second time.** The button
 * already said that; a body that restates the title costs a keystroke and
 * carries nothing.
 *
 * **A function, not a constant, because the host is a fact about this pull
 * request.** `hostLabel` reads it off the address the caller already holds —
 * see `Decide.tsx`'s own `pullRequest` prop — rather than this file assuming
 * every pull request Armada ever opens is on the same one.
 */
export function confirmMerge(pullRequest: string): { title: string; body: string } {
  const host = hostLabel(pullRequest);
  return {
    title: "Merge this job's pull request?",
    body:
      `The pull request merges on ${host}, so the job's commits land on the base branch and ` +
      "everybody working from it gets them on their next pull. Armada then runs the " +
      `repository's after-merge checks against what landed — merging it on ${host} yourself ` +
      "instead skips them, and running them is the reason this button exists. The job is " +
      "approved with it. Bridge cannot take a merge back: undoing one is a revert made in the " +
      "repository.",
  };
}
