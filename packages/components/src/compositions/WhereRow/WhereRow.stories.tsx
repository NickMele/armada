import type { Meta, StoryObj } from "@storybook/react-vite";
import { WhereRow } from "./WhereRow";

const meta: Meta<typeof WhereRow> = {
  title: "Compositions/Where row",
  component: WhereRow,
  decorators: [
    (Story) => (
      <div style={{ width: "calc(var(--space-12) * 8)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof WhereRow>;

/**
 * A path, which opens where it lives. `external-link` at the trailing edge —
 * the act leaves Bridge, and the glyph is the one that says so everywhere
 * else.
 *
 * The label column is the point of the row. The build drew a glyph in its
 * place and left a reader deducing from the shape of a string whether it was a
 * worktree or a log.
 */
export const APathThatOpens: Story = {
  args: {
    label: "Worktree",
    value: ".armada/worktrees/job_2d90bb",
    act: "open",
    onAct: () => {},
  },
};

/**
 * An identifier, which copies. A Drone id names something rather than locating
 * it, so there is nowhere to open and the clipboard is the act.
 */
export const AnIdentifierThatCopies: Story = {
  args: {
    label: "Drone",
    value: "01M10B1V2A0011VRS6RA2SKPQ7",
    act: "copy",
    onCopied: () => {},
  },
};

/**
 * The region as the drawing has it. Two acts, mixed, and the trailing mark is
 * the only thing saying which is which — a branch copies, a log opens.
 *
 * `Workflow` is the third act: it leads to another surface inside Bridge, and
 * its note says which version of the workflow the Job is running, because a
 * workflow edited since dispatch is not the one this Job was given.
 */
export const TheWholeRegion: Story = {
  render: () => (
    <div style={{ display: "flex", flexDirection: "column" }}>
      <WhereRow
        label="Worktree"
        value=".armada/worktrees/job_2d90bb"
        act="open"
        onAct={() => {}}
      />
      <WhereRow
        label="Branch"
        value="fix/settings-split-selectors"
        act="copy"
        onCopied={() => {}}
      />
      <WhereRow label="Manifest" value="armada.yml" act="open" onAct={() => {}} />
      <WhereRow
        label="Workflow"
        value="bug"
        note="as it was at 14:20"
        act="into"
        onAct={() => {}}
      />
      <WhereRow
        label="Job log"
        value=".armada/logs/job_2d90bb.jsonl"
        act="open"
        onAct={() => {}}
      />
      <WhereRow
        label="Drone"
        value="01M10B1V2A0011VRS6RA2SKPQ7"
        act="copy"
        onCopied={() => {}}
      />
    </div>
  ),
};

/**
 * A value wider than the column. It clips from the right, which is correct
 * here and nowhere else on this screen: a worktree path is read from its
 * start, and it is the produced-file column that needs its end kept.
 */
export const WiderThanTheColumn: Story = {
  render: () => (
    <div style={{ width: "calc(var(--space-12) * 5)" }}>
      <WhereRow
        label="Transcript"
        value=".armada/transcripts/01M10B1V2A0011VRS6RA2SKPQ7.jsonl"
        act="open"
        onAct={() => {}}
      />
    </div>
  ),
};

/**
 * A row with nothing behind it. Drawn as a label, not as a control that does
 * nothing — a dead affordance is worse than an absent one.
 */
export const NothingToDo: Story = {
  args: {
    label: "Manifest",
    value: "armada.yml",
    act: "open",
  },
};
