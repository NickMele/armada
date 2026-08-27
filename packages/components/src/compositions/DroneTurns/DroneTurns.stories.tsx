import type { Meta, StoryObj } from "@storybook/react-vite";
import { DroneTurns, type DroneTurn } from "./DroneTurns";

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

/** The ordinary transcript: a session, some prose, and calls with their answers. */
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
    subject: "Read · call_7f21",
    answer: "Answered.",
  },
  {
    id: "4",
    at: "09:14:09",
    kind: "called",
    subject: "Bash · call_7f22",
    answer: "Answered, and the tool itself failed.",
  },
  {
    id: "5",
    at: "09:14:11",
    kind: "called",
    subject: "Edit · call_7f23",
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
 * What a call did, where the wire carries it.
 *
 * **Not derived from the tool name, ever.** Until `Saw::Called` carries it a
 * row shows the tool and the opaque call id, which is what made twenty-two
 * rows read alike. The last row is a value the wire cut short — a `Write`
 * argument is a whole file — and it renders as cut rather than as the whole
 * thing.
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
