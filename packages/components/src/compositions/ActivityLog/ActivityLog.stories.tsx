import type { Meta, StoryObj } from "@storybook/react-vite";
import { ActivityLog, type ActivityEntry } from "./ActivityLog";

/**
 * One story per shape the log has to carry. The stream is the drawing's own —
 * a Drone editing, a Drone running a command, Fleet noticing it went quiet, and
 * Armada's injected turn at the top of the step.
 */
const meta: Meta<typeof ActivityLog> = {
  title: "Compositions/Activity log",
  component: ActivityLog,
};
export default meta;

type Story = StoryObj<typeof ActivityLog>;

const BUILD_OUTPUT = [
  "$ cargo build --workspace --locked",
  "   Compiling armada-settings v0.1.0 (packages/settings)",
  "   Compiling armada-fleet v0.1.0 (crates/fleet)",
  "    Finished `dev` profile [unoptimized] in 47.61s",
].join("\n");

const STREAM: ActivityEntry[] = [
  {
    id: "1",
    at: "14:22:07",
    actor: "armada",
    summary: "Go on to Implement.",
    payload: "The injected turn that opens the step. Armada writes it; the Drone answers it.",
  },
  {
    id: "2",
    at: "14:22:44",
    actor: "drone",
    summary:
      "Splitting the selector block into its own module so the tests can import it without the store.",
  },
  { id: "3", at: "14:23:11", actor: "drone", summary: "Read", subject: "packages/settings/src/reducer.ts" },
  { id: "4", at: "14:26:31", actor: "drone", summary: "Edit", subject: "packages/settings/src/selectors.ts" },
  {
    id: "5",
    at: "14:29:40",
    actor: "drone",
    summary: "Bash",
    subject: "cargo build --workspace --locked",
    output: BUILD_OUTPUT,
    ran: "exit 0 · 47.61s · in .armada/worktrees/job_2d90bb",
  },
  {
    id: "6",
    at: "14:30:28",
    actor: "fleet",
    summary: "Heartbeat — the Drone has been quiet for 48 seconds",
  },
  { id: "7", at: "14:31:58", actor: "drone", summary: "thinking" },
];

/**
 * The stream as it reads on a running step. **Every entry names who** — that is
 * what keeps one stream honest when the Drone, Armada and Fleet all write into
 * it.
 */
export const OneStream: Story = {
  args: { entries: STREAM },
};

/**
 * A command opened. **Its full text, its output, its exit code and where it
 * ran** — which is what makes the log an answer rather than a pointer at a
 * transcript.
 */
export const AnEntryOpened: Story = {
  args: { entries: STREAM, openId: "5" },
};

/**
 * Fleet handing a failed Check back to the Drone. Fleet's own events are what a
 * reader is scanning for in a stream of the Drone's, so Fleet's name is the
 * brighter neutral — and the Check id takes the failed hue, which is the
 * verdict rather than the Job's state.
 */
export const AFleetEvent: Story = {
  args: {
    openId: "f1",
    entries: [
      {
        id: "f0",
        at: "14:46:02",
        actor: "drone",
        summary: "Bash",
        subject: "cargo nextest run --workspace",
      },
      {
        id: "f1",
        at: "14:47:09",
        actor: "fleet",
        summary: "Check failed — 3 of 2034 tests. Handed back to the Drone, attempt 2 of 3.",
        subject: "test",
        named: "failed",
        output: [
          "FAIL settings::selectors::visible_manifests_memoises",
          "  expected the same reference on repeat calls, got a new object",
          "FAIL settings::selectors::hidden_manifests_excluded",
          "FAIL settings::reducer::identity_stable_across_actions",
        ].join("\n"),
        ran: "exit 101 · 1m 22s · in .armada/worktrees/job_2d90bb",
      },
    ],
  },
};

/**
 * **A cut says so and names where the rest is.** The bound is the diff's, for
 * the diff's reason — a block long enough to need virtualizing is the freeze
 * the whole surface exists to escape. Drawn here at ten lines so the treatment
 * is visible without a fixture nobody would read.
 */
export const APayloadCut: Story = {
  args: {
    maxLines: 10,
    openId: "long",
    entries: [
      {
        id: "long",
        at: "14:47:09",
        actor: "fleet",
        summary: "Check failed — 218 of 2034 tests.",
        subject: "test",
        named: "failed",
        output: Array.from({ length: 218 }, (_, i) => `FAIL settings::selectors::case_${i + 1}`).join("\n"),
        outputAt: ".armada/logs/job_2d90bb/checks/test.log",
        ran: "exit 101 · 4m 02s · in .armada/worktrees/job_2d90bb",
      },
    ],
  },
};

/**
 * A cut with nothing to name. **It says that too** rather than trailing off:
 * a reader who cannot finish reading here needs to know there is nowhere else
 * to finish, which is a finding about Fleet and not about the Job.
 */
export const ACutThatNamesNoFile: Story = {
  args: {
    maxLines: 10,
    openId: "long",
    entries: [
      {
        id: "long",
        at: "14:47:09",
        actor: "drone",
        summary: "Bash",
        subject: "cargo nextest run --workspace",
        output: Array.from({ length: 60 }, (_, i) => `line ${i + 1}`).join("\n"),
      },
    ],
  },
};

/**
 * The stream itself bounded. The same sentence shape one level up — what is not
 * here, and where the rest of it is.
 */
export const TheStreamCut: Story = {
  args: {
    entries: STREAM,
    cut: "The newest 7 of 126 entries. The whole log is in .armada/logs/job_2d90bb.jsonl.",
  },
};

/**
 * A step nothing has been recorded against. Ordinary, and never an error — a
 * Drone that has just started has done nothing yet.
 */
export const NothingYet: Story = {
  args: { entries: [] },
};
