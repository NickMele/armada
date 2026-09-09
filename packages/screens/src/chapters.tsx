// The step's story, built. Drone instructions, then Activity log, then
// Produced, in the order they happened.
//
// **The same three chapters in the same order at every state.** What changes is
// which one is the reason you are here and what the panel offers you to do
// about it — never where a chapter sits, and never how many there are.
//
// **Two of them leave the panel.** The log holds 1676 entries on a real Job and
// the diff is the Job's whole patch, and neither is a longer version of a
// preview: opened in place they push everything under them off the screen. So
// chapters two and three carry an act on their header line rather than a body,
// and what the act opens is a trailing sheet — #286, and `Sheets.tsx`.
//
// **Chapters four and five are the evidence, and they are in `evidence.tsx`.**
// What the Checks found and what the panel made of it are the same story in the
// same order — the Drone was instructed, it worked, it produced, the Checks ran,
// the panel read them — so they are chapters here rather than a region
// somewhere else. They are conditional where these three are not: a step that
// declares no Check and asks no Judge draws neither, because an empty labelled
// region reads as a value that failed to load.
//
// **A deliverable sits beside the diff and never inside it.** The Produced
// chapter counts files in the patch, and `.armada/` is ignored by this
// repository's own deliberate choice, so a step whose whole product is a
// document under it reads back zero files. Counting the document there would
// make `3 files · +94 −31` a number that means two things; drawn beside it, the
// chapter can say a document was written without lying about the patch. #307.
//
// Split out of `JobDetail.tsx` at the 900-line line.

import {
  Button,
  ChangedFiles,
  DroneBrief,
  Clamped,
  FramesPaired,
  FramesShown,
  Kbd,
  changedFilesSummary,
  keyFor,
  type ShownFrame,
  type StepChapter,
} from "@armada/components";

import type { Criterion, Diff, Footprint, KeptFrame, Turn } from "@armada/protocol";
import type { JobSummary, StepDetail } from "@armada/protocol";
import type { JobFootprint } from "@armada/protocol";
import type { Calls } from "./calls";
import { DecidedDiff } from "./Decide";
import {
  DIFF_CHAPTER,
  FRAMES_CHAPTER,
  LOG_CHAPTER,
  namesChapter,
  type DetailKeys,
} from "./detail-keys";
import { evidenceChaptersOf, type Unnumbered } from "./evidence";
import {
  framesSummary,
  pairedFrames,
  pairedSummary,
  shownFrames,
  type Frames,
  type Paired,
} from "./frames";
import { readingFor, whyNoFootprint } from "./files";
import { Log } from "./Log";
import type { Outputs } from "./outputs";
import { keptOf, type KeptRead, type Opens } from "./phases";
import { producedIn } from "./produced";
import type { OpenSheet } from "./Sheets";
import { entriesOf, NOTHING_YET_ON_THIS_STEP } from "./story";

/**
 * The story: Drone instructions, then Activity log, then Produced. **The same
 * three chapters in the same order at every state** — what changes is which one
 * is the reason you are here.
 */
export function chaptersOf({
  job,
  step,
  criteria,
  render,
  watching,
  footprint,
  kept,
  diff,
  live,
  transcript,
  log,
  calls,
  outputs,
  frames,
  sheet,
  opens,
  onOpenSheet,
}: {
  job: JobSummary;
  step: StepDetail;
  /**
   * The Job's frozen acceptance criteria, for chapter five.
   *
   * **The Job's and not the step's**, which is what makes the join necessary:
   * `Judged.criterion_id` names a row of `JobDetail.acceptance_criteria`, and
   * without it a verdict grid draws an id where the criterion's own words go.
   */
  criteria: readonly Criterion[];
  render: string;
  watching: { rows: readonly Turn[]; skipped: number } | null;
  footprint: Footprint;
  /**
   * What this Job's own detail says it touched, where it has stopped. **Fleet
   * serves it on a terminal Job and on no other**, so its presence is what
   * chooses between the record and the live reading — see `produced.ts`.
   */
  kept: JobFootprint | undefined;
  diff: Diff;
  /** Whether the socket is still carrying rows, for the chapter's live mark. */
  live: boolean;
  /**
   * What the socket says about its own reading, from `whyNotWatching`, or
   * `undefined` while it is reading.
   *
   * **The third answer, and it replaces the other two where it is present.** A
   * chapter with no rows says either that nothing has happened on the step or
   * that Armada has not opened it — both of which are true only of a step that
   * has not started. A socket that failed or closed says so instead, and a
   * chapter that already has rows says it above them. #324.
   */
  transcript: string | undefined;
  /**
   * What one log takes, by name. Held by `detail-keys` for the reason the
   * chapters and the strip are: `h`/`l` open a row, and a keyboard and a
   * pointer disagreeing about which row is open is two answers to one question.
   * **By name, because a story draws two logs over one stream** — chapter one's
   * turns are also chapter two's rows, so a row is named with its log.
   */
  log: DetailKeys["inLog"];
  /**
   * The arguments this Job's cut rows have been opened to, and how to ask for
   * one. **Passed to every log rather than to the one that streams**, because
   * chapter one draws Armada's turns out of the same rows and a cut call can
   * land in either.
   */
  calls: Calls;
  /**
   * What each Check on this Job has printed, where somebody opened one, and how
   * to ask for the rest. **Held for the Job, like `calls`**, and for its reason:
   * a recorded output never moves, so it is one reader's gesture rather than
   * state the window redraws on.
   */
  outputs: Outputs;
  /**
   * The frames this Job's steps have produced, as this window has them, and how
   * to ask for a step's worth.
   *
   * **Asked for without anybody pressing**, which is the one place this departs
   * from `outputs`. The whole claim of #209 is that a change whose point is not
   * the code is reviewed by looking at it, and a panel that leads with a button
   * saying there are pictures has not led with anything. What bounds it is the
   * shape of the thing: one step is open at a time and a spec captures a
   * handful of screens.
   */
  frames: Frames;
  /**
   * Which sheet is open, so the chapter behind it says so and stops offering.
   *
   * **`holds` is one of them and no chapter answers it.** It opens from the run
   * column rather than from the story, so every chapter here is behind it and
   * none of them is the one it came from.
   */
  sheet: OpenSheet;
  /**
   * How the step's deliverable is opened, and where a refusal is said.
   *
   * **The same handler the phase strip takes, and required for the same
   * reason.** A path on screen that opens nothing is the defect `phases.ts`
   * names, and an optional handler is how a surface goes quietly back to it.
   */
  opens: Opens;
  onOpenSheet: (which: "log" | "diff") => void;
}): StepChapter[] {
  const rows = watching === null ? [] : entriesOf(watching.rows, step.step_id);
  // **The turns Fleet sent, and not everything in Armada's voice.** The two
  // were the same set until a turn the harness replays onto the Drone's stream
  // started arriving attributed (#110) — since then each of Armada's turns is
  // on the transcript twice, once as what Fleet wrote and once as the session
  // echoing it back. This chapter is the first of those.
  const told = rows.filter((row) => row.kind === "instructed");
  const opened = told[0];
  const produced = producedIn(kept, readingFor(footprint, job.id));
  // Read once, here, and drawn twice: the same call builds the Submitted
  // tier's rows on the phase strip. Two readings is how the two surfaces
  // came to state one ordering separately. #321.
  const documents = keptOf(step, opens);
  const shown = shownFrames(step.frames ?? [], frames);
  const pairs = pairedFrames(step.frames ?? [], frames);
  // **Numbered over what is drawn.** Every chapter below is built without an
  // ordinal and counted once, at the end — because which chapters exist
  // depends on the step. One that gates on nothing has no evidence chapter,
  // and one that shows its work has a chapter before Produced. Fixed numbers
  // were right until the second of those existed.
  const story: Unnumbered[] = [
    {
      id: "instructions",
      title: "Drone instructions",
      // The turn the step opened with, in the words the Drone was given.
      // Armada's own turns are on the transcript beside the Drone's, so this
      // is the same stream chapter two draws, filtered to the rows Fleet
      // authored.
      summary: opened === undefined ? undefined : opened.at,
      preview:
        opened === undefined ? (
          <p className="text-2xs text-fg-muted">{transcript ?? NOT_OPENED_YET}</p>
        ) : (
          // **The payload and not its text.** Each line carries what it is,
          // and the block headings are on the wire as line numbers Fleet wrote
          // down as it wrote the blocks. Mapped to `line.text` the marking is
          // dropped here and the brief draws with the gap above a heading as
          // the only thing marking it, which is #318 — nothing downstream can
          // recover it, because deciding by position or by capitals is the
          // guess the marker exists to replace.
          // **Held to a few lines, like every other long passage on this
          // screen.** A step's opening turn is the whole brief plus the
          // standing instructions plus the branch note — on a real Job that is
          // a screen and a half, and it is chapter one, so it pushed the
          // Checks, the Verdicts and everything a person opened the panel for
          // off the bottom. The Job's own brief above it has been clamped
          // since it was drawn; this is the same passage in the same column.
          <Clamped lines={INSTRUCTIONS_LINES} moreLabel="Read the whole instruction">
            <DroneBrief lines={opened.payload} />
          </Clamped>
        ),
      ...(told.length <= 1
        ? {}
        : {
            content: (
              <Log rows={told} emptyNote={NOT_OPENED_YET} calls={calls} {...log("instructions")} />
            ),
            openLabel: `Everything Armada told it — ${told.length} turns`,
          }),
    },
    {
      id: "log",
      title: "Activity log",
      // The dot, not the word. `StepStory` composes `Chapter` now, so the
      // running mark has its own channel and the summary carries only counts.
      live,
      summary: [
        `${rows.length} ${rows.length === 1 ? "entry" : "entries"}`,
        sheet === "log" ? "open" : "every line opens",
      ].join(" · "),
      // The affordance is on the header line and it names where it goes. A
      // chapter that opens a layer has no body for a foot control to sit under.
      ...(sheet === "log"
        ? {}
        : {
            act: (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => onOpenSheet("log")}
                {...namesChapter(LOG_CHAPTER)}
              >
                Open the log
                <Kbd>{keyFor("open_log")}</Kbd>
              </Button>
            ),
          }),
      // Always drawn, and never behind a control. The log is what says what is
      // happening right now, so it is on the page while the Job runs rather
      // than a thing to go and open.
      //
      // The note sits above the rows rather than replacing them: a socket that
      // stopped is a fact about the reading, and the rows already in hand are
      // still the step's record. With no rows the same sentence is the empty
      // note, so it is said once either way.
      preview: (
        <>
          {transcript === undefined || rows.length === 0 ? null : (
            <p className="text-2xs text-fg-muted">{transcript}</p>
          )}
          <Log
            rows={rows.slice(-PREVIEWED)}
            emptyNote={transcript ?? NOTHING_YET_ON_THIS_STEP}
            calls={calls}
            {...log("log")}
          />
        </>
      ),
    },
    // **What it looks like, above what changed** — the inverse of every other
    // step, and the whole of #209. On a change whose point is not the code the
    // diff is the least useful thing on the screen, and it was the only thing
    // offered: reviewing meant reading a patch to infer an outcome you could
    // have been shown. So a step that captured frames leads with them, and
    // Produced is the chapter underneath.
    //
    // **Only where there are frames.** A chapter saying a step declared no
    // visual evidence would be on almost every step of almost every Job,
    // reporting the absence of a thing nobody asked for.
    ...(shown.length === 0 ? [] : [framesChapter(shown, step.frames ?? [], pairs)]),
    {
      id: DIFF_CHAPTER,
      title: "Produced",
      // The header carries the summary, so a collapsed chapter still says what
      // the step produced. `changedFilesSummary` is the one reading of it —
      // the body draws the same files from the same answer. On a finished Job
      // that summary carries `+94 −31`, because the record it draws from is the
      // one reading anybody counted.
      //
      // **The documents are their own segment.** Folding them into the file
      // count would put a path that is deliberately outside the patch inside
      // the number that measures the patch; a segment of its own is what lets a
      // step read `0 files · 1 document` rather than as a step that produced
      // nothing. #307.
      summary: summaryOf(produced, documents.length, sheet === "diff"),
      preview: (
        <>
          {produced === undefined ? (
            <p className="text-2xs text-fg-muted">
              {whyNoFootprint(job.assigned_drone !== undefined)}
            </p>
          ) : (
            <ChangedFiles
              files={produced.files}
              emptyNote={nothingTouched(documents.length)}
              note={produced.note}
            />
          )}
          {documents.length === 0 ? null : <Documents kept={documents} />}
        </>
      ),
      // The patch opens on the layer that can hold it. It is the Job's whole
      // patch and the expensive read, and a 602px column was never going to
      // hold either — #286, frame 4j.
      //
      // **Not offered where there is no patch behind it.** This chapter reads
      // the footprint, which Fleet takes once and keeps; the diff reads the
      // worktree, which is given back when a Job is done with it. So a
      // finished Job lists what it wrote above a control that opens a layer
      // saying the worktree is gone — the count reads as the truth and the
      // patch reads as broken, and it is neither. #381. A control that goes
      // nowhere is worse than one that is not there.
      ...(sheet === "diff" || !readable(diff, job.id)
        ? {}
        : {
            act: (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => onOpenSheet("diff")}
                {...namesChapter(DIFF_CHAPTER)}
              >
                Open the diff
                <Kbd>{keyFor("open_diff")}</Kbd>
              </Button>
            ),
          }),
    },
    // The last chapters, where the step has them. `evidence.tsx` decides
    // whether either is drawn — a step that gates on nothing has neither, and
    // one that gates on a Judge alone has one.
    ...evidenceChaptersOf({ step, criteria, opens, outputs }),
  ];
  return story.map((chapter, at) => ({ ...chapter, ordinal: at + 1 }));
}

/**
 * What the step's harness produced, drawn.
 *
 * **A preview and no content, and that is not an oversight.** Every other long
 * chapter here has a body a person opens; this one has the frames in its
 * preview, because opening a chapter to find out that there are pictures is the
 * gesture #209 exists to remove. What a person opens is one frame, not the
 * chapter — and that is the sheet, which is the next slice.
 *
 * **No `act` either.** There is nowhere to send a reader that this does not
 * already show, and a control that opens what is already on screen is a second
 * answer to a question nobody asked.
 */
function framesChapter(shown: ShownFrame[], rows: KeptFrame[], pairs: Paired[]): Unnumbered {
  // **Two sides is a different question from two frames**, so it is a different
  // drawing. With a base to compare against, what a reader wants is what the
  // change did to the screen — the pairs that moved, and the ones that did not
  // folded out of the way. With only the branch there is nothing to fold and
  // nothing to flip, and the flat gallery is the honest answer.
  const compared = pairs.some((pair) => pair.before !== undefined);
  return {
    id: FRAMES_CHAPTER,
    title: "Shown",
    // What moved, where there are two sides; how many frames, where there is
    // one. A collapsed chapter reading `20 frames` over ten screens
    // photographed twice would be counting the work rather than the answer.
    summary: compared ? pairedSummary(pairs) : framesSummary(rows),
    says: compared
      ? "Click to see what the change did to the screen"
      : "Click to see what the step produced",
    preview: compared ? (
      <FramesPaired pairs={pairs} emptyNote={NOTHING_SHOWN} />
    ) : (
      <FramesShown
        frames={shown}
        // Unreachable — the chapter is not built where the list is empty — and
        // stated anyway, because a component that silently draws nothing is
        // how a surface goes quiet when a caller's condition drifts.
        emptyNote={NOTHING_SHOWN}
      />
    ),
  };
}

/** A step whose harness ran and captured nothing. */
const NOTHING_SHOWN = "This step's harness ran and captured nothing to look at.";

/**
 * The documents this step wrote, each one a control that opens it.
 *
 * **Reachable from the chapter a person came to, which is the whole of #307.**
 * The copy under `.armada/deliverables/` was already on the wire and already
 * drawn — on the Submitted tier of the phase strip, an affordance `opening.ts`
 * scopes to the records a person reads because a verdict went against them. A
 * step that passed sent nobody there, so a 7,605-byte plan was unreachable from
 * the chapter that exists to say what the step produced.
 *
 * **The ordering is not decided here.** `keptOf` decides it once for both
 * surfaces and carries the argument for it; a second reversal in this file
 * would be a second answer to one question, which is what #321 found.
 */
function Documents({ kept }: { kept: readonly KeptRead[] }) {
  return (
    <div>
      <span className="text-2xs text-fg-muted">{DOCUMENTS}</span>
      {kept.map((one) => (
        <span key={one.path}>
          {one.opening}
          <span className="text-2xs text-fg-subtle">{one.attempt}</span>
        </span>
      ))}
      <p className="text-2xs text-fg-subtle">{DOCUMENTS_NOTE}</p>
    </div>
  );
}

/**
 * The Produced header's trailing half — `3 files · +94 −31 · 1 document`.
 *
 * `undefined` where there is nothing to count and no sheet open, which is what
 * leaves the header line carrying only its name.
 */
function summaryOf(
  produced: ReturnType<typeof producedIn>,
  documents: number,
  open: boolean,
): string | undefined {
  const said = [
    ...(produced === undefined ? [] : [changedFilesSummary(produced.files, produced.planDeclared)]),
    ...(documents === 0 ? [] : [documents === 1 ? "1 document" : `${documents} documents`]),
    ...(open ? ["open"] : []),
  ];
  return said.length === 0 ? undefined : said.join(" · ");
}

/**
 * Whether there is a patch to open at all.
 *
 * **`work` absent is the worktree being gone**, which is the line the wire
 * already draws: a Drone that changed nothing answers `work` present with no
 * files, and that is a reading worth opening. Anything not yet read is
 * offered, because it is about to be one.
 */
function readable(diff: Diff, jobId: string): boolean {
  if (diff.state !== "read" || diff.jobId !== jobId) return true;
  return diff.work !== undefined;
}

/** How many entries the log's collapsed preview shows. The drawing's own five. */
const PREVIEWED = 5;

/** What chapter one says before Armada has opened the step. */
/**
 * How much of a step's opening turn is drawn before it is held back.
 *
 * **Twelve, which is the brief and not the standing instructions.** A step is
 * opened with the whole of what a Drone is told — how to report, what it may
 * write, what its branch is standing on — and the Job's brief is a few lines
 * inside that. A person opening chapter one wants the brief; the rest is the
 * same on every step of every Job and is one press away.
 */
const INSTRUCTIONS_LINES = 12;

const NOT_OPENED_YET = "Armada has not opened this step yet.";

/** The sub-label over the documents, so the block says what it holds. */
const DOCUMENTS = "Documents this step wrote";

/**
 * Why the documents are not in the count above them. **The disagreement is the
 * whole reason this line exists** — a reader who has just read `0 files` and is
 * looking at a document needs the two facts reconciled where they sit.
 */
const DOCUMENTS_NOTE =
  "Kept outside the diff, so the count above does not include them. One per attempt.";

/**
 * A reading that found nothing in the patch.
 *
 * **Ordinary, and never an error** — a Drone that has just started has changed
 * nothing yet. It no longer says the drone changed nothing: on a step whose
 * whole product is a document under `.armada/`, that sentence was false while a
 * 7,605-byte plan sat on disk, and the subject of the sentence is the
 * repository rather than the drone besides. #307.
 */
function nothingTouched(documents: number): string {
  if (documents === 0) return "Nothing in the repository has changed yet.";
  return documents === 1
    ? "Nothing in the repository changed. This step's product is the document below."
    : "Nothing in the repository changed. This step's product is the documents below.";
}
