import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { Check, CircleDot, OctagonAlert, UserCheck, X } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { SplitButton } from "../../primitives/SplitButton/SplitButton";
import { JobDetailHeaderActions } from "./JobDetailHeaderActions";

/**
 * The header on every state job detail reaches: working, stopped badly, stopped
 * well. One story each, because the three are only comparable side by side —
 * this block was hand-built three times before it was converged, and the copies
 * had drifted on wrapping, on shrink order and on whether mono was the default.
 *
 * The act shape is the same on both terminals: none. Everything that ends
 * something is one outlined destructive split button, and the one primary the
 * header ever carries is `Approve dispatch`, on the approval gate alone.
 *
 * **The labels come from the enum→verb map**, `crates/core-model/domain/
 * enum-verbs.toml`, sentence-cased, and the glyphs from the same rows. They
 * are written here because that map is not generated into TypeScript yet, and
 * nowhere that ships.
 */
const meta: Meta<typeof JobDetailHeaderActions> = {
  title: "Compositions/Job detail header actions",
  component: JobDetailHeaderActions,
};
export default meta;

type Story = StoryObj<typeof JobDetailHeaderActions>;

/**
 * A running job. The badge is static: the workflow rail beneath it on this
 * screen carries the one pulse, on its current step.
 *
 * The branch copies on click. A Job with no assigned Drone has one act, so the
 * split button is a button — and nothing to approve or merge while a Job works,
 * so no primary.
 */
export const ARunningJob: Story = {
  args: {
    status: "running",
    statusIcon: CircleDot,
    statusLabel: "Running",
    headline: "Split the settings reducer",
    jobId: "job_2d90bb",
    fields: [
      { label: "Step", value: "2 of 4", mono: true },
      {
        label: "Branch",
        value: "fix/settings-split",
        mono: true,
        copyValue: "fix/settings-split",
      },
      { label: "Elapsed", value: "11m 03s", mono: true },
      { label: "Spend, estimated", value: "~$1.80", mono: true },
      { label: "Dispatched by you" },
    ],
    actions: (
      <>
        <Button variant="ghost">Watch the turns</Button>
        <Button variant="destructive">Kill job</Button>
      </>
    ),
  },
};

/**
 * A failed job. `Stopped at Run tests, step 3 of 4` is one fact, not two — the
 * comma joins them, and the step name stays sans beside its mono sibling
 * because a step name is a label and not a machine-derived value.
 *
 * No action here either. What you can do with a dead end is read its log and
 * its worktree, and those sit beside the branch further down the screen.
 */
export const AFailedJob: Story = {
  args: {
    status: "completed-failed",
    statusIcon: X,
    statusLabel: "Failed",
    headline: "Cache the manifest read",
    jobId: "job_91ab",
    fields: [
      { label: "Stopped at", value: "Run tests" },
      { label: "step", value: "3 of 4", mono: true, continues: true },
      { label: "Ran", value: "22m 41s", mono: true },
      { label: "Spend, estimated", value: "~$2.10", mono: true },
      { label: "Dispatched by you" },
    ],
  },
};

/**
 * A finished job carries no action in the header at all: the acts on a
 * finished job are about its branch and its log, and they sit beside those
 * rather than up here. The field run changes with the state — a job that has
 * stopped reports what it ran, not what step it is on.
 *
 * **`Pull request #4711` is a fact and not an act**, which is why it is in the
 * run rather than in the trailing group. Going to read something is not one of
 * the things that end a job, and it belongs beside the branch it was opened
 * from. It is `--accent`, the token the design contract gives links, and it
 * underlines only on hover.
 *
 * **The number, never the address.** The address is sixty characters of which
 * a person reads four; the whole of it is on the link's `title`.
 */
export const AFinishedJob: Story = {
  args: {
    status: "completed-success",
    statusIcon: Check,
    statusLabel: "Done",
    headline: "Add a retry ceiling to the poke loop",
    jobId: "job_4f10",
    fields: [
      { label: "All", value: "4 of 4", mono: true, suffix: "steps advanced" },
      { label: "Branch", value: "fix/poke-ceiling", mono: true, copyValue: "fix/poke-ceiling" },
      {
        label: "Pull request",
        value: "#4711",
        mono: true,
        href: "https://forge.invalid/org/repo/pull/4711",
      },
      { label: "Ran", value: "18m 22s", mono: true },
      { label: "Spend, estimated", value: "~$2.40", mono: true },
      { label: "Dispatched by you" },
    ],
  },
};

/**
 * The same job once somebody has taken the work. **What became of the pull
 * request continues the fact that names it** — `Pull request #4711, merged` —
 * rather than standing beside it as a second fact. They are one thing said to
 * the depth the record can say it, and the run's gap would read them as two.
 *
 * Nothing is drawn here while a pull request is merely open: that is the state
 * every one of them is in from the moment it exists, so a word for it would be
 * a slot on every finished job saying that nothing has happened.
 */
export const ThePullRequestOnceItLanded: Story = {
  args: {
    ...AFinishedJob.args,
    fields: [
      { label: "All", value: "4 of 4", mono: true, suffix: "steps advanced" },
      { label: "Branch", value: "fix/poke-ceiling", mono: true, copyValue: "fix/poke-ceiling" },
      {
        label: "Pull request",
        value: "#4711",
        mono: true,
        href: "https://forge.invalid/org/repo/pull/4711",
      },
      { label: "merged", continues: true },
      { label: "Ran", value: "18m 22s", mono: true },
      { label: "Spend, estimated", value: "~$2.40", mono: true },
      { label: "Dispatched by you" },
    ],
    onFollowed: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    const address = "https://forge.invalid/org/repo/pull/4711";
    const link = canvas.getByRole("link", { name: "#4711" });

    // The number is what is on screen and the address is what is behind it.
    await expect(link).not.toHaveTextContent(address);
    await expect(link).toHaveAttribute("title", address);

    // One fact, not two: the fold puts the comma inside the same run.
    await expect(link.parentElement).toHaveTextContent("Pull request #4711, merged");

    // The click leaves, and this window does not. `preventDefault` is what
    // keeps a forge address from loading over the app, and the host is what
    // decides where it actually goes.
    await userEvent.click(link);
    await expect(args.onFollowed).toHaveBeenCalledWith(address);
  },
};

/**
 * The two kills, as Bridge draws them. **They are two acts, not one control
 * with a mode** — killing the drone leaves the job open with its worktree held,
 * killing the job ends it at `killed`. Two outlined reds side by side read as
 * one control with two labels, which is the thing they are least like; one
 * split button separates them, and each menu label says what survives.
 *
 * The drone act is drawn only where a drone is assigned, which is why the
 * `Drone` fact and that menu entry appear together.
 */
export const BothKills: Story = {
  args: {
    ...ARunningJob.args,
    fields: [
      { label: "Step", value: "2 of 4", mono: true },
      { label: "at", value: "implement", mono: true, continues: true },
      { label: "Elapsed", value: "11m 03s", mono: true },
      { label: "Branch", value: "fix/settings-split", mono: true, copyValue: "fix/settings-split" },
      { label: "Drone", value: "drn_7c21", mono: true, copyValue: "drn_7c21" },
      { label: "Writes", value: "src/settings/reducer.ts", mono: true },
    ],
    actions: (
      <>
        <Button variant="ghost">Watch the turns</Button>
        <SplitButton
          variant="destructive"
          menuLabel="What else ends this job"
          items={[{ label: "Kill drone, the job stays open" }]}
        >
          Kill job
        </SplitButton>
      </>
    ),
  },
};

/**
 * The same pair with the menu open, which is the only view where the
 * distinction can be read. **The face is the act that ends the Job** — killing
 * the Drone leaves it open with its worktree held, so the milder act is the one
 * behind the caret and never the other way round.
 */
export const BothKillsMenuOpen: Story = {
  args: {
    ...BothKills.args,
    actions: (
      <>
        <Button variant="ghost">Watch the turns</Button>
        <SplitButton
          variant="destructive"
          defaultOpen
          menuLabel="What else ends this job"
          items={[{ label: "Kill drone, the job stays open" }]}
        >
          Kill job
        </SplitButton>
      </>
    ),
  },
};

/**
 * A stopped job, with the one recovery. **The label says what happens** — a
 * redispatch mints a replacement and kills this job, so "retry" or "run again"
 * would name an act Fleet does not perform.
 *
 * `Replaces` is the lineage the new job carries back. Without it a board reads
 * every second failure as a first one.
 */
export const StoppedWithARedispatch: Story = {
  args: {
    status: "escalated",
    statusIcon: OctagonAlert,
    statusLabel: "stalled",
    headline: "Cache the manifest read",
    jobId: "job_91ab04",
    fields: [
      { label: "Step", value: "3 of 4", mono: true },
      { label: "at", value: "verify", mono: true, continues: true },
      { label: "Elapsed", value: "22m 41s", mono: true },
      { label: "Branch", value: "feat/manifest-cache", mono: true, copyValue: "feat/manifest-cache" },
      { label: "Scope undetermined" },
    ],
    actions: (
      <>
        <Button variant="ghost">Watch the turns</Button>
        <SplitButton
          variant="destructive"
          defaultOpen
          menuLabel="What else ends this job"
          items={[{ label: "Kill job, it ends here", danger: true }]}
        >
          Redispatch as a new job
        </SplitButton>
      </>
    ),
  },
};

/**
 * A Job at the approval gate. **The only forward act the header ever carries**,
 * and the only primary — everything else on this block stops something.
 *
 * It is last, where the shell head puts its own primary, and it is the accent
 * fill rather than a third outline: the fill and the distance are what keep it
 * from reading as a peer of the red group. **It does not confirm.** Approving
 * is the ordinary path and is reversible by killing, and a gate that costs two
 * clicks for the common case is a gate in the wrong place.
 */
export const AtTheApprovalGate: Story = {
  args: {
    status: "awaiting-approval",
    statusIcon: UserCheck,
    statusLabel: "needs approval",
    headline: "Cache the manifest read",
    jobId: "job_91ab04",
    fields: [
      { label: "Step", value: "1 of 4", mono: true },
      { label: "at", value: "plan", mono: true, continues: true },
      { label: "Waiting", value: "4m 12s", mono: true },
      { label: "Writes", value: "crates/config/src/manifest.rs", mono: true },
    ],
    actions: (
      <>
        <Button variant="ghost">Watch the turns</Button>
        <Button variant="destructive">Kill job</Button>
        <Button variant="primary">Approve dispatch</Button>
      </>
    ),
  },
};

/**
 * **The window narrowing, at four widths.** The reported defect and what
 * replaces it.
 *
 * The block used to hold one row at every width, and only the title column
 * could give: the acts are fixed-width controls, so the headline wrapped to
 * four lines beside three buttons and the fact run broke with `Spend` alone on
 * the end. Now the acts drop under the title when the two stop fitting, and the
 * facts read as one run again the moment they have the width.
 *
 * `--window-floor` is the narrowest window Bridge opens, and the two below it
 * are what the panel is given inside one — the header is not the window.
 * **Nothing is dropped at any of them**, which is the rule this block has
 * carried from the start.
 */
export const AsTheWindowNarrows: Story = {
  args: AtTheApprovalGate.args,
  render: (args) => (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-8)" }}>
      {(
        [
          ["the narrowest window Bridge opens", "var(--window-floor)"],
          ["the panel inside one", "var(--w-dialog-wide)"],
          ["narrower still", "var(--w-sheet)"],
          ["narrower than anything ships", "var(--w-dialog)"],
        ] as [string, string][]
      ).map(([said, width]) => (
        <div key={width} style={{ width, maxWidth: "100%" }}>
          <span className="armada-screen__eyebrow">{said}</span>
          <JobDetailHeaderActions {...args} />
        </div>
      ))}
    </div>
  ),
};
