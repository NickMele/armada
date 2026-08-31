import type { Meta, StoryObj } from "@storybook/react-vite";
import { Check, File, FileCheck, Folder, GitBranch, GitCommitHorizontal, GitPullRequest } from "lucide-react";
import { JobBrief } from "../../compositions/JobBrief/JobBrief";
import { JobLogReference } from "../../compositions/JobLogReference/JobLogReference";
import { AFinishedJobWhatItWasAndWhatItProduced } from "./AFinishedJobWhatItWasAndWhatItProduced";

/**
 * Journey · Read the work and merge by hand. A finished Job is read once, to
 * decide whether to take the work, so the screen answers what it was and what
 * it produced at full weight and folds the rest into a record.
 *
 * The header is `Job detail header actions`, the same component the running and
 * failed jobs render. A finished job reports what it ran rather than what step
 * it is on.
 */
const meta: Meta<typeof AFinishedJobWhatItWasAndWhatItProduced> = {
  title: "Screens/A finished job — what it was and what it produced",
  component: AFinishedJobWhatItWasAndWhatItProduced,
};
export default meta;

type Story = StoryObj<typeof AFinishedJobWhatItWasAndWhatItProduced>;

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

const brief = {
  criteria: [
    { text: "The poke loop stops after the configured number of attempts.", source: "check" },
    { text: "A ceiling of zero is refused at load rather than at run.", source: "check" },
  ],
};

const NOTE =
  "The branch is pushed and a review is open. Armada has no merge action — read the diff in your own tools and land it yourself.";

/** Where the work is, folded into the record: where to go looking, not what to take away. */
const paths = (
  <JobLogReference
    rows={[
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
        separated: true,
      },
      {
        iconLabel: "Transcript",
        value: "/repos/armada/.armada/transcripts/",
        copyValue: "/repos/armada/.armada/transcripts/",
        meta: "named by a drone id nothing serves",
      },
    ]}
  >
    The worktree, the log and the transcripts directory follow from the job id and the repository
    its manifest was read from.
  </JobLogReference>
);

const record = [
  { id: "steps", label: "Steps and checks", panel: <Stub>The workflow rail goes here.</Stub> },
  { id: "turns", label: "The drone's turns", panel: <Stub>The transcript goes here.</Stub> },
  {
    id: "told",
    label: "What it was told",
    panel: (
      <JobBrief
        criteria={[]}
        only="facts"
        facts="The loop is in `fleet::poke`. The ceiling is a Machine setting, not a Kit one."
      />
    ),
  },
  { id: "paths", label: "Where the work is", panel: paths },
];

/**
 * The screen as Bridge draws it today: a branch, and four parts of "produced"
 * that nothing serves. Each keeps its row and names what would have to serve
 * it, so the gap is a finding rather than a silence.
 */
export const AsBridgeDrawsItToday: Story = {
  render: () => (
    <div className="armada-screen">
      <AFinishedJobWhatItWasAndWhatItProduced
        heading={heading}
        brief={brief}
        outcome={{
          note: NOTE,
          parts: [
            {
              name: "Branch",
              icon: GitBranch,
              iconLabel: "Branch",
              value: "fix/poke-ceiling",
            },
            {
              name: "Commit",
              icon: GitCommitHorizontal,
              iconLabel: "Commit",
              value: "5375d705cb7713a21a91681c1028166b98a0d6de",
              meta: "origin/armada/01M1CNPKTV0018H2M1CXDNBK06",
            },
            {
              name: "Pull request",
              icon: GitPullRequest,
              iconLabel: "Pull request",
              value: "https://example.invalid/armada/pull/229",
            },
            {
              /* No glyph: `file` is reserved to the log row and `file-check` to
                 a submission that landed, so a changed-file row has nothing in
                 the registry to take. The mark column stays and renders empty. */
              name: "Files changed",
              absent:
                "job.files_changed is published while a drone is working. Nothing serves a finished job's footprint.",
            },
            {
              name: "Evidence",
              icon: FileCheck,
              iconLabel: "Evidence",
              absent: "No operation serves a work submission, so there is nothing to draw.",
            },
          ],
        }}
        record={record}
        recordValue="steps"
      />
    </div>
  ),
};

/** Every part of "produced" served — what the lead region becomes as they land. */
export const EveryPartServed: Story = {
  render: () => (
    <div className="armada-screen">
      <AFinishedJobWhatItWasAndWhatItProduced
        heading={heading}
        brief={brief}
        outcome={{
          note: NOTE,
          parts: [
            {
              name: "Branch",
              icon: GitBranch,
              iconLabel: "Branch",
              value: "fix/poke-ceiling",
              meta: "from main",
            },
            {
              name: "Commit",
              icon: GitCommitHorizontal,
              iconLabel: "Commit",
              value: "9f2c1ab",
            },
            {
              name: "Pull request",
              icon: GitPullRequest,
              iconLabel: "Pull request",
              value: "armada#42",
            },
            { name: "Files changed", value: "3 files", meta: "+214 −96" },
            {
              name: "Evidence",
              icon: FileCheck,
              iconLabel: "Evidence",
              value: "4 submissions",
            },
          ],
        }}
        record={record}
        recordValue="told"
      />
    </div>
  ),
};

/**
 * A Job read before its detail arrived. Both lead regions say which of the two
 * silences this is, and the record folds nothing rather than showing a strip
 * with no panel under it.
 */
export const BeforeTheDetailArrives: Story = {
  render: () => (
    <div className="armada-screen">
      <AFinishedJobWhatItWasAndWhatItProduced
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
        briefAbsent="Reading this job."
        outcomeAbsent="Reading this job."
        recordAbsent="Reading this job, so there is no record to fold yet."
      />
    </div>
  ),
};

/** Standing in for a component another issue builds, so the strip can be read. */
function Stub({ children }: { children: string }) {
  return <p className="armada-record__note">{children}</p>;
}
