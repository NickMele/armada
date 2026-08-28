import type { Meta, StoryObj } from "@storybook/react-vite";
import { JobBrief } from "../JobBrief/JobBrief";
import { JobLogReference } from "../JobLogReference/JobLogReference";
import { JobRecord } from "./JobRecord";

/**
 * Everything about a Job that is not what it was or what it produced, folded
 * into one strip. Each section is one interaction away and only the open one is
 * rendered, so the fold costs nothing and a section can own a subscription.
 *
 * The sections are the caller's, and the order is the caller's. This component
 * decides where they go and what a closed one costs, nothing else.
 */
const meta: Meta<typeof JobRecord> = {
  title: "Compositions/Job record",
  component: JobRecord,
};
export default meta;

type Story = StoryObj<typeof JobRecord>;

const sections = [
  { id: "steps", label: "Steps and checks", panel: <Panel>The workflow rail goes here.</Panel> },
  { id: "turns", label: "The drone's turns", panel: <Panel>The transcript goes here.</Panel> },
  {
    id: "told",
    label: "What it was told",
    panel: (
      <JobBrief
        criteria={[]}
        only="facts"
        facts="The refresh path is in `auth/session.ts`. Keep the public signature."
      />
    ),
  },
  {
    id: "paths",
    label: "Where the work is",
    panel: (
      <JobLogReference
        rows={[
          { value: "/w/api/.armada/worktrees/01M130Y1380016YK5S0JXBXDQ5" },
          { value: "/w/api/.armada/logs/01M130Y1380016YK5S0JXBXDQ5.jsonl", separated: true },
        ]}
      >
        The worktree, the log and the transcripts directory follow from this job&apos;s id.
      </JobLogReference>
    ),
  },
];

/** The strip as a finished Job draws it, opened on the first section. */
export const FoldedRecord: Story = {
  args: { sections, defaultValue: "steps" },
};

/**
 * A later section open. The strip does not reorder and the sections that are
 * not drawn have mounted nothing.
 */
export const ASectionOpened: Story = {
  args: { sections, defaultValue: "told" },
};

/** No sections at all — a job read before its detail arrived. */
export const NothingRecorded: Story = {
  args: {
    sections: [],
    emptyNote: "Fleet has not answered for this job, so there is no record to fold.",
  },
};

/** Standing in for a component another issue builds, so the strip can be read. */
function Panel({ children }: { children: string }) {
  return <p className="armada-record__note">{children}</p>;
}
