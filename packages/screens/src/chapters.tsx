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
  briefSections,
  Button,
  ChangedFiles,
  Clamped,
  DroneBrief,
  FramesPaired,
  FramesShown,
  ShownAgain,
  Kbd,
  changedFilesSummary,
  keyFor,
  type ShownFrame,
  type StepChapter,
} from "@armada/components";

import type { Criterion, Diff, Footprint, KeptFrame, Turn } from "@armada/protocol";
import type { JobSummary, StepDetail } from "@armada/protocol";
import type { JobFootprint } from "@armada/protocol";
import { briefChecksOf, briefStepsOf } from "./brief";
import type { Calls } from "./calls";
import {
  DIFF_CHAPTER,
  FRAMES_CHAPTER,
  LOG_CHAPTER,
  namesChapter,
  type DetailKeys,
} from "./detail-keys";
import { againSummary, type Again } from "./again";
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
import { WorkGrouped } from "./grouped";
import { Log } from "./Log";
import type { Following, Outputs } from "./outputs";
import { keptOf, type KeptRead, type Opens } from "./phases";
import { noteUnder, producedIn } from "./produced";
import type { OpenSheet } from "./Sheets";
import { entriesOf, NOTHING_YET_ON_THIS_STEP, hideUnread } from "./story";

/** The chapters every step's story has, which the panel names before the step is read. */
export const EVERY_STORY_TELLS = {
  instructions: "Drone instructions",
  log: "Activity log",
  produced: "Produced",
} as const;

/**
 * The story: Drone instructions, then Activity log, then Produced. **The same
 * three chapters in the same order at every state** — what changes is which one
 * is the reason you are here.
 */
export function chaptersOf({
  job,
  step,
  steps,
  criteria,
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
  again,
  sheet,
  opens,
  onOpenSheet,
  now,
  following,
  undecided,
  asking,
  onRunHere,
}: {
  /** Now, injected, so a running Check's elapsed time moves with the clock. */
  now: number;
  /**
   * The running Check's log this window is following, and how to follow one —
   * `outputs.ts`. **Held for the Job, like `outputs`**, and streamed only while
   * somebody has the Checks chapter open on a running gate.
   */
  following: Following;
  job: JobSummary;
  step: StepDetail;
  /**
   * Every one of the Job's steps, in the frozen workflow's own order, for the
   * Drone brief's `steps` section — `part 2 of 3`, and which of the others are
   * done. **`JobDetail.steps`, unfiltered.** `step` above is the one open in
   * this panel; this is the whole order it sits in, which chapter one's brief
   * reads instead of `step` alone because "where you are" is a question about
   * every step, not the current one.
   */
  steps: readonly StepDetail[];
  /**
   * The Job's frozen acceptance criteria, for chapter five.
   *
   * **The Job's and not the step's**, which is what makes the join necessary:
   * `Judged.criterion_id` names a row of `JobDetail.acceptance_criteria`, and
   * without it a verdict grid draws an id where the criterion's own words go.
   */
  criteria: readonly Criterion[];
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
   * Asking the Job to show its work again, and the sets its presses kept on
   * this step — `again.tsx`. **Absent draws the Shown chapter as it was**, and
   * present builds it on a step that captured nothing of its own, because the
   * control and the sets are reasons to have the chapter too.
   */
  again?: Again;
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
  /**
   * Fleet's own reason the gate could not decide on this step's current
   * attempt, for the Verdicts chapter and the Checks chapter's Judge row.
   * `stuck.undecided`, scoped by the caller to the step `stuck.step_id` names
   * — a step open for any other reason gets none.
   */
  undecided?: string;
  /** The criterion a live judge question holds open on this step, where one is. */
  asking?: string;
  /** **Run it here** on a refused Check's row, from the run sheet — Journey 9. */
  onRunHere?: (checkId: string) => void;
}): StepChapter[] {
  const { rows, unread } = hideUnread(watching === null ? [] : entriesOf(watching.rows, step.step_id));
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
      title: EVERY_STORY_TELLS.instructions,
      // The turn the step opened with, in the words the Drone was given.
      // Armada's own turns are on the transcript beside the Drone's, so this
      // is the same stream chapter two draws, filtered to the rows Fleet
      // authored.
      summary: opened === undefined ? undefined : opened.at,
      preview:
        opened === undefined ? (
          <p className="text-2xs text-fg-muted">{transcript ?? NOT_OPENED_YET}</p>
        ) : (
          // **The payload, not its text** — each line carries what it is; block
          // headings are line numbers Fleet wrote as it wrote blocks. Mapped to
          // `line.text` the marking is dropped and the brief draws with the gap
          // above a heading as the only marker, #318 — nothing downstream can
          // recover it, since deciding by position or capitals is the guess the
          // marker replaces.
          //
          // **The outer clamp only where `DroneBrief` cannot bound itself** — a
          // step's opening turn was held to twelve lines since brief+standing+branch
          // note is a screen and a half. Since protocol 9.7 that is sections
          // instead, every heading carrying a kind: standing folds shut, the one
          // unbounded section (`about_this_job`) clamps itself — see
          // `DroneBrief.tsx`. A turn with no kinds (pre-9.7, or no headed blocks)
          // still draws flat, so this keeps the clamp for that case.
          briefSections(opened.payload) === undefined ? (
            <Clamped lines={INSTRUCTIONS_LINES} moreLabel="Read the whole instruction">
              <DroneBrief
                lines={opened.payload}
                steps={briefStepsOf(steps, step.step_id)}
                checks={briefChecksOf(step)}
              />
            </Clamped>
          ) : (
            <DroneBrief
              lines={opened.payload}
              steps={briefStepsOf(steps, step.step_id)}
              checks={briefChecksOf(step)}
            />
          )
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
      title: EVERY_STORY_TELLS.log,
      // The dot, not the word. `StepStory` composes `Chapter` now, so the
      // running mark has its own channel and the summary carries only counts.
      live,
      // Counts, and whether its sheet is open. What a row does is on the row.
      summary: [
        `${rows.length} ${rows.length === 1 ? "entry" : "entries"}`,
        ...(unread === 0 ? [] : [`${unread} hidden`]),
        ...(sheet === "log" ? ["open"] : []),
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
          {/* Folded, and the rows inside a group are the same `Log` chapter
              one draws — so the region and payload names the keyboard reads
              are the ones that were always there. The unread count is not
              passed: the header summary above already carries it, and one
              fact twice on one chapter is two places to disagree. */}
          <WorkGrouped
            rows={rows}
            turns={watching === null ? [] : watching.rows}
            stepId={step.step_id}
            most={PREVIEWED}
            emptyNote={transcript ?? NOTHING_YET_ON_THIS_STEP}
            calls={calls}
            log={log("log")}
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
    // shown evidence would be on almost every step of almost every Job,
    // reporting the absence of a thing nobody asked for.
    ...(shown.length === 0 && again === undefined
      ? []
      : [framesChapter(shown, step.frames ?? [], pairs, again)]),
    {
      id: DIFF_CHAPTER,
      title: EVERY_STORY_TELLS.produced,
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
              note={noteUnder(produced)}
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
    ...evidenceChaptersOf({ step, criteria, opens, outputs, now, following, undecided, asking, onRunHere }),
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
function framesChapter(
  shown: ShownFrame[],
  rows: KeptFrame[],
  pairs: Paired[],
  again?: Again,
): Unnumbered {
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
    summary:
      [compared ? pairedSummary(pairs) : framesSummary(rows), againSummary(again)]
        .filter((part) => part !== undefined)
        .join(" · ") || undefined,
    says: compared
      ? "Click to see what the change did to the screen"
      : "Click to see what the step produced",
    // The step's own frames first, then what presses kept. **The step's are
    // left out rather than drawn empty** where it captured nothing and only a
    // press or the control built the chapter — an empty gallery above the
    // control would read as a run that failed to load.
    preview: (
      <>
        {shown.length === 0 ? null : compared ? (
          <FramesPaired pairs={pairs} emptyNote={NOTHING_SHOWN} />
        ) : (
          <FramesShown frames={shown} emptyNote={NOTHING_SHOWN} />
        )}
        {again === undefined ? null : (
          <ShownAgain
            {...(again.offer === undefined ? {} : { offer: again.offer })}
            sets={again.sets}
            {...(again.said === undefined ? {} : { said: again.said })}
            onShow={again.onShow}
          />
        )}
      </>
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

/**
 * How many groups the log's collapsed preview shows.
 *
 * **Counted in groups now, not in entries.** It was the drawing's own five
 * rows, and five rows of a real step is five consecutive `Read` calls — the
 * same height in groups is eight different things the Drone did.
 */
const PREVIEWED = 8;

/** What chapter one says before Armada has opened the step. */

/**
 * How much of a step's opening turn is drawn before it is held back, on a
 * turn `DroneBrief` cannot section.
 *
 * **Twelve — the brief, not the standing instructions.** A step opens
 * with the whole of what a Drone is told — how to report, what it may
 * write, what its branch stands on — and the Job's brief is a few lines
 * inside that; a person opening chapter one wants the brief, the rest one
 * press away.
 *
 * **Since protocol 9.7, only where every heading's kind is absent** — a
 * turn whose headings carry `BlockKind` sections itself, the standing
 * block already folded shut, so the clamp would be redundant; it stays
 * for the one case still needing it, a turn with no kinds to pair.
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
  if (documents === 0) return "No changes yet";
  return "No repository changes";
}
