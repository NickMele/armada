import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
import type { JobExamined, Look } from "@armada/protocol";

import { JobResources, type PulseReading } from "./JobResources";

const meta: Meta<typeof JobResources> = {
  title: "Compositions/Job resources",
  component: JobResources,
};
export default meta;

type Story = StoryObj<typeof JobResources>;

/** The branch one Job's Drone is working on. Every row here belongs to it. */
const BRANCH = "armada/01JOBHOLDS001";

/**
 * What the Job is running and what it is spending. **The caller's rows**, so
 * this component holds no rule about which five they are.
 */
const FIGURES = [
  { label: "Drones", value: "1 running" },
  { label: "Checks", value: "2 running" },
  { label: "Judges", value: "none out" },
  { label: "Spend", value: "~$2.41 of ~$5.00" },
  { label: "Turns", value: "34 of 120" },
];

/** The board, as the caller derives it. One place, so a story cannot drift. */
function reading(over: Partial<PulseReading> = {}): PulseReading {
  return {
    held: "running",
    readAt: "2026-09-04T04:07:00.366Z",
    processes: [
      {
        pid: 41233,
        command: "node",
        owner: BRANCH,
        cpuPercent: 12.4,
        memoryBytes: 402_653_184,
        runningFor: "06:12",
        recorded: true,
      },
      {
        pid: 41287,
        command: "cargo",
        owner: BRANCH,
        cpuPercent: 98.7,
        memoryBytes: 838_860_800,
        runningFor: "00:41",
        recorded: false,
      },
    ],
    worktrees: [
      {
        path: "/Users/user/armada/.armada/worktrees/01JOBHOLDS001",
        branch: BRANCH,
        state: "on disk",
        bytes: 1_073_741_824,
      },
    ],
    logs: [{ kind: "job", owner: null, writing: true }],
    ...over,
  };
}

function look(over: Partial<Look> & Pick<Look, "asked" | "found">): Look {
  return { said: "", fields: [], ...over };
}

function examined(found: JobExamined["found"], looks: Look[]): JobExamined {
  return {
    job_id: "01JOBHOLDS001",
    looked_at: "2026-09-04T04:07:00.366Z",
    found,
    looks,
    // The wire's own reading, which the look carries back with it — not the
    // board above, which is what this component draws.
    resources: {
      job_id: "01JOBHOLDS001",
      read_at: "2026-09-04T04:07:00.366Z",
      held: "running",
      processes: [],
    },
  };
}

/**
 * **Nobody has pressed yet.** The figures are drawn and the question is not
 * answered, because looking walks a process table and a directory and an answer
 * that appeared unasked would be the automatic bound rather than the person's
 * half of it. The line says the act is free, since a person who thinks it costs
 * a model call will not press it.
 */
export const NobodyHasAsked: Story = {
  args: {
    reading: reading(),
    figures: FIGURES,
    examined: null,
    age: "3s",
    refreshed: "Taken again whenever this job moves.",
    onExamine: () => {},
  },
};

/**
 * **The reading a person came for.** A drone and the build it started, the one
 * fleet wrote down leading the list, and the disk the checkout has taken —
 * which is the figure with a second reason to exist, since seventy-four
 * worktrees once took 220 GB and nothing said so.
 */
export const WorkingAndSaidSo: Story = {
  args: {
    reading: reading(),
    age: "3s",
    examined: examined("working", [
      look({
        asked: "process",
        found: "working",
        said: "the process Fleet recorded is running",
        fields: [{ name: "pid", value: "41233" }],
      }),
      look({ asked: "worktree", found: "working", said: "the worktree is on disk" }),
      look({ asked: "span", found: "working", said: "waiting for the step to finish" }),
    ]),
    onExamine: () => {},
  },
};

/**
 * **The state that took a terminal to establish.** The job reads running and
 * fleet holds no process for it — which as an empty table under a heading is
 * exactly how it went unnoticed, so it is a sentence in the error treatment
 * instead. This is the 4 Sep 2026 job, drawn.
 */
export const NothingIsRunning: Story = {
  args: {
    reading: reading({ held: "none", processes: [] }),
    age: "1s",
    examined: examined("not_working", [
      look({
        asked: "process",
        found: "not_working",
        said: "this Job is running and Fleet recorded no process for it",
        fields: [{ name: "processes", value: "0" }],
      }),
      look({
        asked: "writing",
        found: "cannot_tell",
        said: "nothing has been written to this Job's log lately, which settles nothing on its own",
        fields: [{ name: "seconds_ago", value: "372" }],
      }),
    ]),
    onExamine: () => {},
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText(/is not doing what it should be/)).toBeVisible();
    await expect(canvas.getByText(/Fleet holds no process for this job/)).toBeVisible();
  },
};

/**
 * **The answer that has to be said rather than implied.** Two of the five looks
 * can never report a fault — a job's log is quiet while a drone works steadily,
 * and a quiet drone is what a long command looks like — so an examination that
 * finds nothing wrong says which checks came back short rather than reporting
 * that everything looks fine.
 */
export const SomeChecksCouldNotTell: Story = {
  args: {
    reading: reading(),
    age: "5s",
    examined: examined("cannot_tell", [
      look({
        asked: "process",
        found: "working",
        said: "the process Fleet recorded is running",
      }),
      look({
        asked: "writing",
        found: "cannot_tell",
        said: "nothing has been written to this Job's log lately, which settles nothing on its own",
      }),
      look({
        asked: "silence",
        found: "cannot_tell",
        said: "no Drone is in the slot",
      }),
    ]),
    onExamine: () => {},
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText(/could not tell working from not/)).toBeVisible();
  },
};

/**
 * **A checkout too large to walk inside the bound.** Said rather than reported
 * as nothing: a size that did not arrive is a walk that ran long, which on a
 * worktree is itself worth knowing — and a zero here would be the one figure a
 * person acts on directly, wrong.
 */
export const TheWorktreeWasNotMeasuredInTime: Story = {
  args: {
    reading: reading({
      worktrees: [
        {
          path: "/Users/user/armada/.armada/worktrees/01JOBHOLDS001",
          branch: BRANCH,
          state: "on disk",
        },
      ],
    }),
    age: "2s",
    examined: null,
    onExamine: () => {},
  },
};

/** A look already out. A second press does not send a second act. */
export const LookingNow: Story = {
  args: { reading: reading(), age: "0s", examined: null, looking: true, onExamine: () => {} },
};

/**
 * **Nothing has been read.** Not the same as a job holding nothing, which is
 * the distinction the whole panel turns on — so the note says which rather than
 * drawing an empty table.
 *
 * Fleet is answering here and this one read did not come back, so the act
 * stays: another attempt is a reasonable move.
 */
export const NothingHasBeenRead: Story = {
  args: {
    reading: null,
    note: "Fleet did not answer, so what this job holds is unknown.",
    examined: null,
    onExamine: () => {},
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: /Look now/ })).toBeVisible();
  },
};

/**
 * **Fleet is the thing that did not answer, so there is nothing to ask.** The
 * act is gone rather than greyed: it asks Fleet, and pressing it could only
 * ask the thing that is silent. A disabled button with no sentence beside it
 * is the same dead end drawn quieter, which is what this state was before.
 *
 * **Degraded and not a fault.** Amber, no red, because restarting Fleet is the
 * wrong move when the process is alive and only the connection stopped — and
 * the status bar is the one surface that says which of the two this is.
 *
 * **Not `lookFailed`.** That says one attempt did not come back, which invites
 * another. This says attempts are not the shape of the problem.
 */
export const FleetIsNotAnswering: Story = {
  args: { reading: null, examined: null, nothingToAsk: "no_answer", onExamine: () => {} },
  play: async ({ canvas }) => {
    await expect(
      canvas.getByText(/Fleet is not answering, so there is nothing to ask/),
    ).toBeVisible();
    await expect(canvas.getByText(/Nothing here is a reading of this job/)).toBeVisible();
    // The whole point: no control that asks Fleet, disabled or otherwise.
    await expect(canvas.queryByRole("button", { name: /Look now/ })).toBeNull();
    await expect(canvas.queryByText(/Looking costs no model call/)).toBeNull();
  },
};

/**
 * **Fleet answered, and Bridge could not read it.** The second reading, and it
 * is not the first said differently: Fleet is demonstrably up here, so "not
 * answering" would be false and pointing at the status bar would send a reader
 * to a line saying Fleet running.
 *
 * **Fleet being alive is not a reason to keep the act.** What stops the answer
 * is the two builds disagreeing about this route, so the same request meets
 * the same disagreement and a restart brings back the Fleet that caused it.
 * Attempts are not the shape of this problem either, which is why it withdraws
 * the control rather than reading as one failed look.
 *
 * **Still degraded and still not red**, for the reason an unreachable Fleet is
 * not: what has failed is this panel's ability to show a reading.
 */
export const BridgeCouldNotReadTheAnswer: Story = {
  args: { reading: null, examined: null, nothingToAsk: "unreadable", onExamine: () => {} },
  play: async ({ canvas }) => {
    await expect(
      canvas.getByText(/Fleet answered, and Bridge could not read the answer/),
    ).toBeVisible();
    await expect(canvas.getByText(/rebuilding both is what settles it/)).toBeVisible();
    await expect(canvas.queryByRole("button", { name: /Look now/ })).toBeNull();
    // The two readings must not draw as one message. Neither the sentence that
    // says Fleet is silent nor the pointer at a bar reading "Fleet running".
    await expect(canvas.queryByText(/Fleet is not answering/)).toBeNull();
    await expect(canvas.queryByText(/The status bar names/)).toBeNull();
  },
};

/**
 * **A Job running several Drones.** One checkout per member, one process per
 * member, and the owner column is what joins them — *which of these is eating
 * the machine* cannot be answered by a table that names no owner.
 *
 * The first process belongs to no checkout. It is drawn as unplaced rather
 * than folded into the first row, which would name the wrong member.
 */
export const SeveralMembers: Story = {
  args: {
    reading: reading({
      processes: [
        {
          pid: 52_118,
          command: "node",
          owner: "armada/22-give-the-store-one-shape",
          cpuPercent: 12.4,
          memoryBytes: 486_539_264,
          runningFor: "06:12",
          recorded: true,
        },
        {
          pid: 52_640,
          command: "node",
          owner: "armada/24-drop-the-store-singleton",
          cpuPercent: 41.9,
          memoryBytes: 712_179_712,
          runningFor: "01:40",
          recorded: false,
        },
        {
          pid: 52_711,
          command: "cargo",
          owner: null,
          cpuPercent: 98.7,
          memoryBytes: 838_860_800,
          runningFor: "00:41",
          recorded: false,
        },
      ],
      worktrees: [
        {
          path: "/Users/user/armada/.armada/worktrees/22-give-the-store-one-shape",
          branch: "armada/22-give-the-store-one-shape",
          state: "merged",
          bytes: 1_020_054_016,
        },
        {
          path: "/Users/user/armada/.armada/worktrees/23-read-the-store-through-selectors",
          branch: "armada/23-read-the-store-through-selectors",
          state: "waiting on you",
          bytes: 980_321_280,
        },
        {
          path: "/Users/user/armada/.armada/worktrees/24-drop-the-store-singleton",
          branch: "armada/24-drop-the-store-singleton",
          state: "on disk",
        },
      ],
      logs: [
        { kind: "job", owner: null, bytes: 184_320, writing: false },
        { kind: "drone", owner: "armada/24-drop-the-store-singleton", bytes: 2_097_152, writing: true },
      ],
    }),
    figures: [
      { label: "Drones", value: "2 running" },
      { label: "Checks", value: "1 running" },
      { label: "Judges", value: "1 out" },
    ],
    age: "4s",
    refreshed: "Taken again whenever this job moves.",
    examined: null,
    onExamine: () => {},
  },
  /**
   * **The owner is the reason this table has five columns.** A row that lost
   * it would render identically to the one-Drone case and read as correct, so
   * the assertion is that each branch is on screen and that the process
   * nothing placed says so instead of borrowing one.
   */
  play: async ({ canvas }) => {
    // Twice: once as a process's owner, once as a checkout. One occurrence
    // is the owner column gone, which renders as the one-Drone case and reads
    // as correct.
    expect(canvas.getAllByText("armada/22-give-the-store-one-shape")).toHaveLength(2);
    await expect(canvas.getByText("not placed")).toBeVisible();
    await expect(canvas.getByText("still being written")).toBeVisible();
  },
};

/**
 * **A Job that has written nothing.** Its own sentence rather than an empty
 * list: a region that vanished when it held nothing would make "no logs yet"
 * and "this build does not draw logs" the same screen.
 */
export const NothingHasBeenWritten: Story = {
  args: {
    reading: reading({ logs: [] }),
    figures: FIGURES,
    age: "1s",
    examined: null,
    onExamine: () => {},
  },
};
