import type { Meta, StoryObj } from "@storybook/react-vite";
import { DroneTurns, type DroneTurn, type TurnStep } from "./DroneTurns";

/**
 * One Drone's turns, read while it is still working.
 *
 * **Every row kind renders as the wire's own word.** `Saw` is an `ipc` enum
 * with no `crates/core-model/domain/enum-verbs.toml` rows, so there is no
 * sanctioned verb, glyph or hue for `called`, `said`, `refused`,
 * `unrecognised` or `unreadable` — the spelling renders and nothing is invented
 * here. That gap is the finding; the rows below are what it looks like.
 *
 * **A call and its answer are one row**, joined on the call id by whoever
 * builds these. Fleet puts both on the wire because joining them there would
 * mean holding a call open until its result arrived.
 *
 * **A run of the Drone thinking is one line.** Measured on one real transcript:
 * 106 of 149 rows were kinds the decoder could not place, so three lines in
 * four described the plumbing rather than the work. They collapse, keep their
 * count, and open.
 *
 * **The step is a boundary, not a column.** One Drone works several steps, and
 * a name repeated down every row would be the same string forty times over. The
 * line is drawn where the step changed and every row beneath it is answered by
 * position — including a step that runs, stops and runs again.
 */
const meta: Meta<typeof DroneTurns> = {
  title: "Compositions/Drone turns",
  component: DroneTurns,
};
export default meta;

type Story = StoryObj<typeof DroneTurns>;

const NOTHING_YET = "This job has no turns. It was never dispatched, so no drone has written one.";

/** A run of the Drone thinking, as the wire spells it. */
function thinking(from: number, rows: number, at: string): DroneTurn[] {
  return Array.from({ length: rows }, (_, n) => ({
    id: String(from + n),
    at,
    kind: "unrecognised",
    subject: n % 4 === 3 ? "a turn with nothing in it Armada names" : "system/thinking_tokens",
    quiet: true,
  }));
}

/**
 * The ordinary transcript: a session, some prose, and calls with their answers.
 *
 * The third call carries no detail, which is the wire saying it had no name for
 * that tool's arguments rather than a field that failed to arrive — so the row
 * falls back to the call id, which is then the only thing telling it from the
 * next `Bash`.
 */
const turns: DroneTurn[] = [
  {
    id: "1",
    at: "09:14:02",
    kind: "started",
    // The model is whatever the Job named. A vendor spelling belongs in
    // `adapters` and nowhere else, so the fixture carries a placeholder.
    subject: "sess_01JB4 · the job's model · 2 mcp servers",
  },
  {
    id: "2",
    at: "09:14:03",
    kind: "said",
    said: "Reading the settings module before I split anything, so the public signature survives.",
  },
  {
    id: "3",
    at: "09:14:04",
    kind: "called",
    subject: "Read",
    detail: "src/settings.rs",
    answer: "Answered.",
  },
  {
    id: "4",
    at: "09:14:09",
    kind: "called",
    subject: "Bash",
    detail: "cargo test -p settings --lib",
    answer: "Answered, and the tool itself failed.",
  },
  {
    id: "5",
    at: "09:14:10",
    kind: "called",
    subject: "TodoWrite · call_7f23",
    answer: "Answered.",
  },
  {
    id: "6",
    at: "09:14:11",
    kind: "called",
    subject: "Edit",
    detail: "src/settings.rs +42 -18",
    answer: "No answer yet.",
  },
];

export const ADroneWorking: Story = {
  args: { turns, emptyNote: NOTHING_YET },
};

/**
 * A Job nobody dispatched. **Ordinary, not an error** — the socket opens, says
 * nothing is writing, sends no rows and closes. A blank pane would read as a
 * view that failed to load.
 */
export const AJobWithNoTranscript: Story = {
  args: { turns: [], emptyNote: NOTHING_YET },
};

/**
 * The three rows that are never gaps: a refusal, a kind this build does not
 * know, and a line that did not decode. A view that hid any of them would
 * report a quiet stream where there was a broken one.
 *
 * The middle one is a run of length one and still collapses. Left alone it
 * would render the decoder's own words for a turn it could not place, which is
 * the reading the collapse exists to remove.
 */
export const RefusedUnrecognisedAndUnreadable: Story = {
  args: {
    emptyNote: NOTHING_YET,
    turns: [
      {
        id: "1",
        at: "09:15:40",
        kind: "refused",
        subject: "Bash · call_7f31",
        said: "This command is not on the allowlist for this drone.",
      },
      { id: "2", at: "09:15:41", kind: "unrecognised", subject: "thinking_delta", quiet: true },
      {
        id: "3",
        at: "09:15:42",
        kind: "unreadable",
        subject: '{"type":"assistant","message":{"content":[{"type":"to',
        said: "The line ended mid-object.",
      },
    ],
  },
};

/**
 * What a call did, as the wire carries it.
 *
 * **Not derived from the tool name, ever.** A row that has no detail shows the
 * tool and the opaque call id instead, which is what made twenty-two rows read
 * alike and is why the wire carries this at all. The last row is a value the
 * wire cut short — a `Write` argument is a whole file — and it renders as cut
 * rather than as the whole thing.
 */
export const WhatEachCallDid: Story = {
  args: {
    emptyNote: NOTHING_YET,
    turns: [
      {
        id: "1",
        at: "09:16:02",
        kind: "called",
        subject: "Read",
        detail: "src/settings.rs",
        answer: "Answered.",
      },
      {
        id: "2",
        at: "09:16:04",
        kind: "called",
        subject: "Edit",
        detail: "reducer.rs +42 -18",
        answer: "Answered.",
      },
      {
        id: "3",
        at: "09:16:09",
        kind: "called",
        subject: "Grep",
        detail: "fn observe\\( in crates/",
        answer: "Answered.",
      },
      {
        id: "4",
        at: "09:16:20",
        kind: "called",
        subject: "Write",
        detail: "docs/practices/bridge.md, 412 lines starting # Bridge practices",
        truncated: true,
        answer: "No answer yet.",
      },
    ],
  },
};

/**
 * A Drone thinking now. The last run is the one that can still be happening, so
 * it is the one that says **Working** and the only mark that moves — a run with
 * rows after it already ended, and the contract allows one animated mark per
 * screen.
 */
export const ADroneThinking: Story = {
  args: {
    live: true,
    emptyNote: NOTHING_YET,
    turns: [
      { id: "1", at: "09:14:02", kind: "said", said: "Starting on the settings split." },
      ...thinking(10, 9, "09:14:03"),
      { id: "20", at: "09:14:12", kind: "called", subject: "Read", detail: "src/settings.rs", answer: "Answered." },
      ...thinking(30, 14, "09:14:15"),
      { id: "50", at: "09:14:31", kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18", answer: "Answered." },
      ...thinking(60, 6, "09:14:40"),
    ],
  },
};

/**
 * The same transcript, finished. **No mark moves and no line carries a verb** —
 * a finished transcript is a record, and a record does not narrate. The count
 * is the whole fact once nothing is happening, and a live mark on a gap in the
 * middle of a history that ended would claim work that stopped.
 */
export const AFinishedRun: Story = {
  args: {
    live: false,
    emptyNote: NOTHING_YET,
    turns: [
      { id: "1", at: "09:14:02", kind: "said", said: "Starting on the settings split." },
      ...thinking(10, 9, "09:14:03"),
      { id: "20", at: "09:14:12", kind: "called", subject: "Read", detail: "src/settings.rs", answer: "Answered." },
      ...thinking(30, 14, "09:14:15"),
      { id: "50", at: "09:14:31", kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18", answer: "Answered." },
      { id: "60", at: "09:14:44", kind: "said", said: "The public signature is unchanged. Submitting." },
      { id: "61", at: "09:14:45", kind: "ended", subject: "18 turns · ~$0.42 · no calls refused" },
    ],
  },
};

/**
 * What the run cost, on the run's own last row.
 *
 * **This is the only place a spend figure appears**, and that is the decision
 * rather than an omission. `ended` is one Drone's total, so a Job that retried
 * has one of these per Drone — a number on the Job would be a sum the wire
 * declines to compute, and the board is not where a figure nobody is deciding
 * on belongs. It also arrives on the Observe socket alone, so an outcome region
 * drawn with that socket closed could only carry a labelled blank.
 *
 * **The figure hedges and the counts do not.** P4 of the design contract:
 * spend is estimated and is spelled `~$1.53`, a turn count is measured and
 * speaks flatly. Rendering the two alike would destroy trust in both.
 *
 * The two rows are the pair #161 was filed over — a run that burned a dollar
 * over forty-one turns and one that gave up in four, which read identically
 * until this row existed.
 */
export const WhatTheRunCost: Story = {
  args: {
    live: false,
    emptyNote: NOTHING_YET,
    turns: [
      { id: "1", at: "09:14:02", kind: "said", said: "Starting on the settings split." },
      { id: "2", at: "09:55:10", kind: "ended", subject: "41 turns · ~$1.53 · 6 calls refused" },
      { id: "3", at: "10:02:11", kind: "said", said: "Retrying the step. Reading the refusal first." },
      { id: "4", at: "10:03:40", kind: "ended", subject: "4 turns · ~$0.0018 · no calls refused" },
    ],
  },
};

/** Nothing but tool calls. No run to collapse, so no line is added to the pane. */
export const NothingButToolCalls: Story = {
  args: {
    live: true,
    emptyNote: NOTHING_YET,
    turns: [
      { id: "1", at: "09:20:01", kind: "called", subject: "Bash", detail: "cargo xtask verify-foundations", answer: "Answered." },
      { id: "2", at: "09:20:31", kind: "called", subject: "Bash", detail: "cargo test -p ipc", answer: "Answered, and the tool itself failed." },
      { id: "3", at: "09:20:48", kind: "called", subject: "Read", detail: "crates/ipc/src/turn.rs", answer: "Answered." },
      { id: "4", at: "09:20:52", kind: "called", subject: "Grep", detail: "Saw::Called in crates/", answer: "No answer yet." },
    ],
  },
};

/**
 * The two steps of the transcript below. A `label` Fleet served, so both read
 * as names rather than as identifiers.
 */
const REPRO: TurnStep = { id: "repro", label: "Reproduce the bug" };
const FIX: TurnStep = { id: "fix", label: "Fix the root cause" };

/**
 * One Drone across three runs of two steps.
 *
 * **The boundary is drawn where the step changed, and nowhere else.** A label on
 * every row would be the same string down forty consecutive lines, taking width
 * from the body that carries what the Drone actually did; the question a reader
 * asks is where one step stopped and the next began.
 *
 * **A step that runs twice draws two boundaries.** The transcript records the
 * step that was running when each row was written, not a range, so `fix` failing
 * its gate and being retried is two separate stretches — and a component that
 * marked only first appearances would fold the retry into the original.
 *
 * A boundary also breaks a run of quiet rows, because one collapsed line
 * spanning two steps would attribute the whole of it to whichever the reader
 * guessed at.
 */
export const TurnsUnderTheirSteps: Story = {
  args: {
    live: true,
    emptyNote: NOTHING_YET,
    turns: [
      { id: "1", at: "09:14:02", step: REPRO, kind: "started", subject: "sess_01JB4 · the job's model · 2 mcp servers" },
      { id: "2", at: "09:14:03", step: REPRO, kind: "said", said: "Writing the failing test before I touch the reducer." },
      ...thinking(10, 5, "09:14:04").map((turn) => ({ ...turn, step: REPRO })),
      { id: "20", at: "09:14:22", step: REPRO, kind: "called", subject: "Write", detail: "tests/settings_split.rs", answer: "Answered." },
      { id: "21", at: "09:15:01", step: FIX, kind: "said", said: "The test reproduces it. Splitting the reducer now." },
      { id: "22", at: "09:15:09", step: FIX, kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18", answer: "Answered." },
      { id: "23", at: "09:15:40", step: FIX, kind: "called", subject: "Bash", detail: "cargo test -p settings --lib", answer: "Answered, and the tool itself failed." },
      { id: "24", at: "09:18:02", step: REPRO, kind: "said", said: "The gate sent this back. Widening the reproduction first." },
      { id: "25", at: "09:18:30", step: REPRO, kind: "called", subject: "Edit", detail: "tests/settings_split.rs +11 -0", answer: "Answered." },
      { id: "26", at: "09:19:04", step: FIX, kind: "called", subject: "Edit", detail: "src/settings.rs +6 -2", answer: "No answer yet." },
    ],
  },
};

/**
 * A step whose workflow declares no name of its own.
 *
 * **The `step_id` renders, in mono, and nothing composes a name from it.** That
 * is the rail's answer to the same substitution: Fleet never sends a blank
 * label, it sends the id, and mono is how a reader is told which arrived. See
 * `[workflow-step-human-label]` — no workflow in the repository declares a
 * label yet, so this is what most transcripts look like today.
 */
export const AStepWithNoNameOfItsOwn: Story = {
  args: {
    emptyNote: NOTHING_YET,
    turns: [
      { id: "1", at: "09:22:01", step: { id: "implement", label: "implement", labelIsAnIdentifier: true }, kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18", answer: "Answered." },
      { id: "2", at: "09:24:40", step: { id: "regression_verify", label: "regression_verify", labelIsAnIdentifier: true }, kind: "called", subject: "Bash", detail: "cargo nextest run --workspace", answer: "Answered." },
      { id: "3", at: "09:26:12", step: { id: "write_up", label: "write_up", labelIsAnIdentifier: true }, kind: "said", said: "Submitting the evidence report." },
    ],
  },
};

/**
 * A transcript that begins before Fleet recorded the step.
 *
 * **The leading rows are unlabelled, never the first step.** That is the exact
 * falsehood the field was added to remove — a four-step Job whose whole
 * transcript claimed to have happened under step one. The step those rows ran
 * under cannot be recovered from anything on disk, so no migration invented one
 * and this pane does not either.
 *
 * A transcript where *no* row anywhere carries a step draws no boundary at all:
 * every row of it predates the field, so the line would contrast with nothing.
 * Every story above this one is that case.
 */
export const RowsWrittenBeforeTheStepWasRecorded: Story = {
  args: {
    emptyNote: NOTHING_YET,
    turns: [
      { id: "1", at: "08:59:14", kind: "started", subject: "sess_01J9Z · the job's model · 2 mcp servers" },
      { id: "2", at: "08:59:20", kind: "called", subject: "Read", detail: "src/settings.rs", answer: "Answered." },
      { id: "3", at: "09:01:02", kind: "said", said: "Reading the reducer before I split it." },
      { id: "4", at: "09:12:41", step: FIX, kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18", answer: "Answered." },
      { id: "5", at: "09:13:10", step: FIX, kind: "called", subject: "Bash", detail: "cargo test -p settings --lib", answer: "Answered." },
    ],
  },
};
