import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { WhereRow } from "./WhereRow";
import { Button } from "../../primitives/Button/Button";

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
        value=".armada/logs/12-the-drone-count-is-wrong.jsonl"
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
        value=".armada/transcripts/12-the-drone-count-is-wrong/01M10B1V2A0011VRS6RA2SKPQ7.jsonl"
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

/**
 * The worktree row plus **Run…**, the run sheet's entry point. The row still
 * opens where it lives — `Run…` is a second control beside it, never a
 * replacement for the first.
 */
export const WithRun: Story = {
  args: {
    label: "Worktree",
    value: ".armada/worktrees/job_2d90bb",
    act: "open",
    onAct: fn(),
    run: { onRun: fn() },
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Run…" }));
    await expect(args.run!.onRun).toHaveBeenCalled();
    // The row's own act is untouched by adding a second control beside it.
    await userEvent.click(canvas.getByRole("button", { name: /Worktree|worktree/ }));
    await expect(args.onAct).toHaveBeenCalled();
  },
};

/**
 * **The worktree is gone.** `Run…` stays in place, disabled, and says why —
 * a control that vanished would read as a capability nobody ever had.
 */
export const WithRunWorktreeGone: Story = {
  args: {
    label: "Worktree",
    value: ".armada/worktrees/job_2d90bb",
    act: "open",
    run: { disabledReason: "This Job's worktree was given back." },
  },
  play: async ({ canvas }) => {
    const control = canvas.getByRole("button", { name: "Run…" });
    await expect(control).toBeDisabled();
  },
};

/**
 * **A server outlives the sheet.** Closing the run sheet leaves a server
 * running, so *Where things are* gains a *Serving* row per server, with its
 * link buttons — `actions`, generalised from `run` for a row whose control
 * is not the fixed shape of a single `Run…` button.
 */
export const WithAServingRow: Story = {
  render: () => (
    <div style={{ display: "flex", flexDirection: "column" }}>
      <WhereRow
        label="Worktree"
        value=".armada/worktrees/job_2d90bb"
        act="open"
        onAct={() => {}}
        run={{ onRun: () => {} }}
      />
      <WhereRow
        label="Serving"
        value="storybook · localhost:41207"
        act="open"
        actions={
          <Button variant="secondary" size="sm" onClick={() => {}}>
            Open Storybook
          </Button>
        }
      />
    </div>
  ),
};
