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
// Split out of `JobDetail.tsx` at the 900-line line.

import {
  Button,
  ChangedFiles,
  Kbd,
  changedFilesSummary,
  type StepChapter,
} from "@armada/components";

import type { Diff, Footprint, Turn } from "@armada/protocol";
import type { JobSummary, StepDetail } from "@armada/protocol";
import type { JobFootprint } from "@armada/protocol";
import type { Calls } from "./calls";
import { DecidedDiff } from "./Decide";
import { DIFF_CHAPTER, LOG_CHAPTER, namesChapter, type DetailKeys } from "./detail-keys";
import { readingFor, whyNoFootprint } from "./files";
import { Log } from "./Log";
import { producedIn } from "./produced";
import { entriesOf, NOTHING_YET_ON_THIS_STEP } from "./story";

/**
 * The story: Drone instructions, then Activity log, then Produced. **The same
 * three chapters in the same order at every state** — what changes is which one
 * is the reason you are here.
 */
export function chaptersOf({
  job,
  step,
  render,
  watching,
  footprint,
  kept,
  diff,
  live,
  log,
  calls,
  sheet,
  onOpenSheet,
}: {
  job: JobSummary;
  step: StepDetail;
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
  /** Which sheet is open, so the chapter behind it says so and stops offering. */
  sheet: "log" | "diff" | null;
  onOpenSheet: (which: "log" | "diff") => void;
}): StepChapter[] {
  const rows = watching === null ? [] : entriesOf(watching.rows, step.step_id);
  const told = rows.filter((row) => row.actor === "armada");
  const opened = told[0];
  const produced = producedIn(kept, readingFor(footprint, job.id));
  return [
    {
      id: "instructions",
      ordinal: 1,
      title: "Drone instructions",
      // The turn the step opened with, in the words the Drone was given.
      // Armada's own turns are on the transcript beside the Drone's, so this
      // is the same stream chapter two draws, filtered to one voice.
      summary: opened === undefined ? undefined : opened.at,
      preview:
        opened === undefined ? (
          <p className="text-2xs text-fg-muted">{NOT_OPENED_YET}</p>
        ) : (
          <p className="text-fg-muted">{opened.payload.map((line) => line.text).join("\n")}</p>
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
      ordinal: 2,
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
                <Kbd>Enter</Kbd>
              </Button>
            ),
          }),
      // Always drawn, and never behind a control. The log is what says what is
      // happening right now, so it is on the page while the Job runs rather
      // than a thing to go and open.
      preview: (
        <Log
          rows={rows.slice(-PREVIEWED)}
          emptyNote={NOTHING_YET_ON_THIS_STEP}
          calls={calls}
          {...log("log")}
        />
      ),
    },
    {
      id: DIFF_CHAPTER,
      ordinal: 3,
      title: "Produced",
      // The header carries the summary, so a collapsed chapter still says what
      // the step produced. `changedFilesSummary` is the one reading of it —
      // the body draws the same files from the same answer. On a finished Job
      // that summary carries `+94 −31`, because the record it draws from is the
      // one reading anybody counted.
      summary:
        produced === undefined
          ? sheet === "diff"
            ? "open"
            : undefined
          : [changedFilesSummary(produced.files, produced.planDeclared)]
              .concat(sheet === "diff" ? ["open"] : [])
              .join(" · "),
      preview:
        produced === undefined ? (
          <p className="text-2xs text-fg-muted">{whyNoFootprint(job.assigned_drone !== undefined)}</p>
        ) : (
          <ChangedFiles files={produced.files} emptyNote={NOTHING_TOUCHED} note={produced.note} />
        ),
      // The patch opens on the layer that can hold it. It is the Job's whole
      // patch and the expensive read, and a 602px column was never going to
      // hold either — #286, frame 4j.
      ...(sheet === "diff"
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
                <Kbd>f</Kbd>
              </Button>
            ),
          }),
    },
  ];
}

/** How many entries the log's collapsed preview shows. The drawing's own five. */
const PREVIEWED = 5;

/** What chapter one says before Armada has opened the step. */
const NOT_OPENED_YET = "Armada has not opened this step yet.";

/**
 * A reading that found nothing. **Ordinary, and never an error** — a Drone that
 * has just started has changed nothing yet.
 */
const NOTHING_TOUCHED = "This drone has not changed anything yet.";

