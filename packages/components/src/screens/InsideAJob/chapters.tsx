import { ActivityLog, type ActivityEntry } from "../../compositions/ActivityLog/ActivityLog";
import {
  ChangedFiles,
  changedFilesSummary,
  type ChangedFile,
} from "../../compositions/ChangedFiles/ChangedFiles";
import { DroneBrief, type BriefLine, type BriefStep } from "../../compositions/DroneBrief/DroneBrief";
import { Button } from "../../primitives/Button/Button";
import { Kbd } from "../../primitives/Kbd/Kbd";
import type { StepChapter } from "../../compositions/StepStory/StepStory";
import { WORKTREE } from "./fixtures";

/**
 * The three chapters `InsideAJobOneArrangementAtEveryState` draws, and the
 * data behind them — split out of `fixtures.tsx` so that file could stay
 * under the line limit once the sectioned brief (#611) joined it. `WORKTREE`
 * is the one value this file needs from there; everything else it builds is
 * its own.
 */

const PREVIEW: ActivityEntry[] = [
  { id: "1", at: "14:22:07", actor: "armada", summary: "Go on to Implement." },
  {
    id: "2",
    at: "14:26:31",
    actor: "drone",
    summary: "Edit",
    subject: "packages/settings/src/selectors.ts",
    output: [
      "@@ -14,6 +14,9 @@",
      "+import { selectColumnOrder } from './selectors/columns'",
      "+",
      " export const selectSettings = (s: RootState) => s.settings",
    ].join("\n"),
    ran: `+3 −0 · in ${WORKTREE}`,
  },
  {
    id: "3",
    at: "14:29:40",
    actor: "drone",
    summary: "Bash",
    subject: "cargo build --workspace --locked",
    output: [
      "$ cargo build --workspace --locked",
      "   Compiling armada-settings v0.1.0 (packages/settings)",
      "   Compiling armada-fleet v0.1.0 (crates/fleet)",
      "    Finished `dev` profile [unoptimized] in 47.61s",
    ].join("\n"),
    ran: `exit 0 · 47.61s · in ${WORKTREE}`,
  },
  {
    id: "4",
    at: "14:30:28",
    actor: "fleet",
    summary: "Heartbeat — the Drone has been quiet for 48 seconds",
  },
  { id: "5", at: "14:31:58", actor: "drone", summary: "thinking" },
];

/** Every entry the step carried, which is what the log sheet draws. */
export const WHOLE: ActivityEntry[] = [
  PREVIEW[0]!,
  {
    id: "1b",
    at: "14:22:44",
    actor: "drone",
    summary:
      "Splitting the selector block into its own module so the tests can import it without the store.",
  },
  {
    id: "1c",
    at: "14:23:11",
    actor: "drone",
    summary: "Read",
    subject: "packages/settings/src/reducer.ts",
    output: [
      "import { createSlice } from '@reduxjs/toolkit'",
      "import type { SettingsState } from './types'",
      "",
      "const initialState: SettingsState = { columns: {}, density: 'comfortable' }",
    ].join("\n"),
    ran: `214 lines · in ${WORKTREE}`,
  },
  ...PREVIEW.slice(1),
];

/**
 * The three files the step produced, each with its own count — the drawing
 * lists them that way, and the chapter's header is the same reading summed.
 *
 * **Nothing on the wire fills `added` and `deleted`.** The seam carries the
 * names and never the bytes, by its own stated rule. Drawn here because the
 * drawing draws it; the surface cannot reach it yet, and that is reported
 * rather than papered over with a shorter fixture.
 */
export const PRODUCED_FILES: ChangedFile[] = [
  { path: "packages/settings/src/selectors.ts", change: "modified", added: 61, deleted: 4 },
  { path: "packages/settings/src/reducer.ts", change: "modified", added: 12, deleted: 27 },
  { path: "packages/settings/src/index.ts", change: "added", added: 21 },
];

const PRODUCED = (
  <ChangedFiles emptyNote="This drone has not changed anything yet." files={PRODUCED_FILES} />
);

/**
 * The story, in the order it happened. **Same three chapters at every state** —
 * what changes is which one is the reason you are here.
 */
/**
 * The affordance on a chapter whose content has no end.
 *
 * **It names its own destination and it is on the header line.** The log and
 * the diff open as a trailing sheet rather than in place — 1676 entries and a
 * whole patch are not longer versions of a preview — so there is no body for
 * the control to sit under, and the word is never *more*. Its binding is drawn
 * inline, which is Journey 4's stated departure from the contract: a missing
 * one is then visible, which is how the `open_log` gap was found.
 */
export function chapterAct(label: string, binding: string, onClick?: () => void) {
  return (
    <Button variant="ghost" size="sm" onClick={onClick}>
      {label}
      <Kbd>{binding}</Kbd>
    </Button>
  );
}

/**
 * The brief Fleet opened `fix` with, one heading per fact, since protocol
 * 9.7. **The steps and Checks it names are `BEHIND`, `RUN_RUNNING` and
 * `CHECKS_NOT_RUN` read back**, not a second, disagreeing count typed here —
 * this gallery's whole point is one Job drawn consistently across its
 * regions.
 */
const FIX_BRIEF: BriefLine[] = [
  { text: "JOB BRIEF", named: "heading", kind: "about_this_job" },
  { text: "" },
  {
    text:
      "Move the selector block into its own module so the tests can import it without " +
      "constructing the store. Do not change reducer behaviour. The columns selector is " +
      "the one that matters: it is memoised against the whole settings slice today, so any " +
      "write to any setting invalidates it, and the board re-sorts on a change to something " +
      "it does not read.",
  },
  { text: "" },
  { text: "HOW TO HAND WORK IN", named: "heading", kind: "standing" },
  { text: "" },
  { text: "Report progress every step. Keep the worktree clean between attempts." },
  { text: "" },
  { text: "WHERE YOU ARE", named: "heading", kind: "steps" },
  { text: "" },
  { text: "This task runs in 6 parts. You are on part 3." },
  { text: "" },
  { text: "WHAT THIS PART HAS TO PASS", named: "heading", kind: "checks" },
  { text: "" },
  { text: "Checks: build, test" },
];

/** `fix`'s position against the six steps `BEHIND`, `fix` and `AHEAD` make. */
const FIX_STEPS: BriefStep[] = [
  { id: "repro", label: "Reproduction", position: "done" },
  { id: "root_cause", label: "Root cause", position: "done" },
  { id: "fix", label: "Fix", position: "current" },
  { id: "regression_verify", label: "Regression check", position: "not_yours" },
  { id: "consumers", label: "Check the consumers still compile", position: "not_yours" },
  { id: "land", label: "Land", position: "not_yours" },
];

export const CHAPTERS: StepChapter[] = [
  {
    id: "instructions",
    ordinal: 1,
    title: "Drone instructions",
    // The instant the step opened, and nothing else. `chapters.tsx` puts
    // `opened.at` here bare; the criteria are what the Judge stage of the
    // strip opens to, which is one place rather than two.
    summary: "14:22:07",
    // **A real sectioned brief, not a Clamped paragraph.** `about_this_job`
    // clamps its own prose now, `steps` and `checks` draw the structured
    // reading `chapters.tsx` builds, and `standing` folds shut by default —
    // see `DroneBrief.tsx`. This is the state the mock this gallery answers
    // to was drawn against.
    preview: <DroneBrief lines={FIX_BRIEF} steps={FIX_STEPS} checks={["build", "test"]} />,
  },
  {
    id: "log",
    ordinal: 2,
    title: "Activity log",
    // `live` is the running dot, not the word. A count says how many entries
    // there are and only the dot says they are still arriving.
    live: true,
    // Counted from the list, not typed beside it — the same rule
    // `changedFilesSummary` keeps below. It read `47 entries` over a stream of
    // 7, which the log sheet then contradicted in its own subtitle.
    //
    // **`every line opens` is the second segment and is the product's.** It is
    // the only thing that says the rows under it are pressable, and this sheet
    // was dropping it.
    summary: `${WHOLE.length} entries · every line opens`,
    preview: <ActivityLog entries={PREVIEW} />,
    act: chapterAct("Open the log", "L"),
  },
  {
    id: "produced",
    ordinal: 3,
    title: "Produced",
    // The act, not the concept: what a reader wants from this header is the
    // files, and the sentence naming it is written at the control because
    // that is where an act's copy belongs.
    says: "Click to view the files",
    // Built from the list rather than typed beside it, so the header and the
    // rows cannot disagree about what the reading found.
    summary: changedFilesSummary(PRODUCED_FILES),
    preview: PRODUCED,
    act: chapterAct("Open the diff", "f"),
  },
];

/** The stream on the step whose Check failed, with Fleet's own hand-back in it. */
export const REPAIR_CHAPTERS: StepChapter[] = [
  {
    id: "instructions",
    ordinal: 1,
    title: "Drone instructions",
    summary: "14:44:20",
    preview: "Run the regression suite and fix anything it turns up.",
  },
  {
    id: "log",
    ordinal: 2,
    title: "Activity log",
    summary: "88 entries · every line opens",
    act: chapterAct("Open the log", "L"),
    preview: (
      <ActivityLog
        entries={[
          {
            id: "r1",
            at: "14:46:02",
            actor: "drone",
            summary: "Bash",
            subject: "cargo nextest run --workspace",
          },
          {
            id: "r2",
            at: "14:47:09",
            actor: "fleet",
            summary: "Check failed — 3 of 2034 tests. Handed back to the Drone, attempt 2 of 3.",
            subject: "test",
            named: "failed",
            output: [
              "FAIL settings::selectors::visible_manifests_memoises",
              "  expected the same reference on repeat calls, got a new object",
              "and 2 more",
            ].join("\n"),
            ran: `exit 101 · 1m 22s · in ${WORKTREE}`,
          },
        ]}
        openId="r2"
      />
    ),
  },
  {
    id: "produced",
    ordinal: 3,
    title: "Produced",
    says: "Click to view the files",
    summary: "4 files · being repaired",
    preview:
      "The work is on fix/settings-split-selectors and the Drone is editing it now. Nothing was " +
      "thrown away and nothing was rolled back.",
    act: chapterAct("Open the diff", "f"),
  },
];
