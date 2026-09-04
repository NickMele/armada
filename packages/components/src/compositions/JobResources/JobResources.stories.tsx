import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
import type { JobExamined, JobResources as Held, Look } from "@armada/protocol";

import { JobResources } from "./JobResources";

const meta: Meta<typeof JobResources> = {
  title: "Compositions/Job resources",
  component: JobResources,
};
export default meta;

type Story = StoryObj<typeof JobResources>;

/** The shape fleet answers with. One place, so a story cannot drift from it. */
function reading(over: Partial<Held> = {}): Held {
  return {
    job_id: "01JOBHOLDS001",
    read_at: "2026-09-04T04:07:00.366Z",
    held: "running",
    processes: [
      {
        pid: 41233,
        command: "node",
        cpu_percent: 12.4,
        memory_bytes: 402_653_184,
        running_for: "06:12",
        recorded: true,
      },
      {
        pid: 41287,
        command: "cargo",
        cpu_percent: 98.7,
        memory_bytes: 838_860_800,
        running_for: "00:41",
        recorded: false,
      },
    ],
    worktree: {
      path: "/Users/user/armada/.armada/worktrees/01JOBHOLDS001",
      branch: "armada/01JOBHOLDS001",
      bytes: 1_073_741_824,
    },
    wrote_last_at: "2026-09-04T04:06:12.001Z",
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
    resources: reading(),
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
  args: { reading: reading(), examined: null, age: "3s", onExamine: () => {} },
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
      worktree: {
        path: "/Users/user/armada/.armada/worktrees/01JOBHOLDS001",
        branch: "armada/01JOBHOLDS001",
      },
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
