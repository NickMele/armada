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
 */
const meta: Meta<typeof DroneTurns> = {
  title: "Compositions/Drone turns",
  component: DroneTurns,
};
export default meta;

type Story = StoryObj<typeof DroneTurns>;

const NOTHING_YET = "This job has no turns. It was never dispatched, so no drone has written one.";

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
      {
        id: "2",
        at: "09:15:41",
        kind: "unrecognised",
        subject: "thinking_delta",
      },
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
