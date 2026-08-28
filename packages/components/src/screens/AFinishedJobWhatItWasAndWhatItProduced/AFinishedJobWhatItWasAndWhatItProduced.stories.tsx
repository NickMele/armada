import type { Meta, StoryObj } from "@storybook/react-vite";
import type { LucideIcon } from "lucide-react";
import { Check, File, Folder } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { AFinishedJobABranchAndAnEvidenceTrail } from "./AFinishedJobABranchAndAnEvidenceTrail";

/**
 * Journey · Read the work and merge by hand. The screen hands over a branch
 * name and gets out of the way: no approve, no reject, no merge, no in-app
 * diff.
 *
 * The header is `Job detail header actions`, the same component the running and
 * failed jobs render. A finished job reports what it ran rather than what step
 * it is on, and carries no action in the header.
 */
const meta: Meta<typeof AFinishedJobABranchAndAnEvidenceTrail> = {
  title: "Screens/A finished job — a branch and an evidence trail",
  component: AFinishedJobABranchAndAnEvidenceTrail,
};
export default meta;

type Story = StoryObj<typeof AFinishedJobABranchAndAnEvidenceTrail>;

/* `file` and `file-check` have no entry in `packages/icons/icons.toml`. The log
   row and every trail entry render a channel short rather than reaching for an
   unregistered glyph. Reported. */
const NO_GLYPH_IN_REGISTRY = undefined as unknown as LucideIcon;

const entries = [
  {
    step: "Plan the change",
    provenance: "14:02 · facts_note · no check",
    icon: NO_GLYPH_IN_REGISTRY,
    iconLabel: "Evidence",
    claimed: "The poke loop stops after 3 attempts and the job records how many it spent.",
    shownBy: "core/fleet/src/lease.rs · the loop has no ceiling today",
    notClaimed:
      "Does not change the poke interval, and does not decide what happens at the third failure.",
  },
  {
    step: "Implement",
    provenance: "14:11 · diff · build exit 0 · diff_nonempty passed",
    icon: NO_GLYPH_IN_REGISTRY,
    iconLabel: "Evidence",
    claimed:
      "A drone that stops answering is poked at most 3 times, and the count is on the job record.",
    shownBy: "core/fleet/src/lease.rs +38 −7 · core/model/src/job.rs +14 −0",
    notClaimed:
      "The count is not surfaced in Bridge. Nothing acts on reaching the ceiling yet — the loop exits and the job keeps its status.",
  },
  {
    step: "Run tests",
    provenance: "14:16 · test_suite_run · test exit 0",
    icon: NO_GLYPH_IN_REGISTRY,
    iconLabel: "Evidence",
    claimed: "The ceiling holds at 3 and the counter increments once per poke.",
    shownBy:
      "cargo test --workspace · 86 passed 0 failed 5.1s · lease::poke_ceiling_holds, lease::poke_count_increments",
    notClaimed:
      "No test covers a drone that answers on the third poke. The suite was green before this change and is green after, so it does not prove the ceiling is reached in practice.",
  },
  {
    step: "Summarise",
    provenance: "14:20 · facts_note · no check",
    icon: NO_GLYPH_IN_REGISTRY,
    iconLabel: "Evidence",
    claimed: "The change is on fix/poke-ceiling and ready to read.",
    shownBy: "3 files +214 −96 · branch fix/poke-ceiling",
    notClaimed:
      "The value 3 is a constant rather than config. Whether it is the right number is not established by anything here.",
  },
];

const heading = {
  status: "completed-success",
  statusIcon: Check,
  statusLabel: "Done",
  headline: "Add a retry ceiling to the poke loop",
  jobId: "job_4f10",
  fields: [
    // The fact reads as a sentence around its value, which is what `suffix` is
    // for: `All 4 of 4 steps advanced`.
    { label: "All", value: "4 of 4", mono: true, suffix: "steps advanced" },
    { label: "Ran", value: "18m 22s", mono: true },
    { label: "Spend, estimated", value: "~$2.40", mono: true },
    { label: "Dispatched by you" },
  ],
};

/**
 * Where the work is, on a Job that is over. **The branch is not repeated** —
 * the handover above names it, and this region is where to go looking rather
 * than what to take away. No path carries a count: nothing counts lines.
 */
const WORK = {
  brief: {
    criteria: [
      { text: "The poke loop stops after the configured number of attempts.", source: "check" },
      { text: "A ceiling of zero is refused at load rather than at run.", source: "check" },
    ],
    facts: "The loop is in `fleet::poke`. The ceiling is a Machine setting, not a Kit one.",
  },
  rows: [
    {
      icon: Folder,
      iconLabel: "Worktree",
      value: "/repos/armada/.armada/worktrees/job_4f10",
      copyValue: "/repos/armada/.armada/worktrees/job_4f10",
    },
    {
      icon: File,
      iconLabel: "Log",
      value: "/repos/armada/.armada/logs/job_4f10.jsonl",
      copyValue: "/repos/armada/.armada/logs/job_4f10.jsonl",
    },
    {
      iconLabel: "Transcript",
      value: "/repos/armada/.armada/transcripts/",
      copyValue: "/repos/armada/.armada/transcripts/",
      meta: "named by a drone id nothing serves",
    },
  ],
  note: "The worktree and the log are derived from the job id and the repository the manifest was read from.",
};

export const FinishedJob: Story = {
  render: () => (
    <div className="armada-screen">
      <AFinishedJobABranchAndAnEvidenceTrail
        heading={heading}
        handover={{
          branch: "fix/poke-ceiling",
          meta: "from main · 3 files +214 −96",
          action: <Button ground="sunken">Open the worktree</Button>,
          // No `log` here: the region beneath names the log along with the
          // worktree and the transcript, and one path drawn twice is two
          // places to keep in step.
          // Operator copy, not commentary: it is addressed to the person using
          // the app and states the one thing this screen exists to say.
          note: "The branch is unpushed and unmerged. Armada does not push and has no merge action — read the diff in your own tools and land it yourself.",
        }}
        work={WORK}
        trail={entries}
        trailMeta="4 submissions · in order"
      />
    </div>
  ),
};

/**
 * The same screen with only what `GET /jobs` carries — which is what Bridge
 * draws today. Neither the branch nor the evidence trail is on the wire, so
 * both regions say what is missing instead of closing up.
 */
export const AsBridgeDrawsItToday: Story = {
  render: () => (
    <div className="armada-screen">
      <AFinishedJobABranchAndAnEvidenceTrail
        heading={{
          status: "completed-success",
          statusIcon: Check,
          statusLabel: "done",
          headline: "Add a retry ceiling to the poke loop",
          jobId: "job_4f10",
          fields: [
            { label: "All", value: "4 of 4", mono: true, suffix: "steps advanced" },
            { label: "Model", value: "sonnet", mono: true },
          ],
        }}
      />
    </div>
  ),
};
