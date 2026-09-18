import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { Check, CircleDot, OctagonAlert, UserCheck, X } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { HoldButton } from "../../primitives/HoldButton/HoldButton";
import { JobSettingsButton } from "../JobSettings/JobSettings";
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
 * A running job. The badge is static: the rail's current step is the running
 * mark, so it carries the loop and the badge does not.
 *
 * **The id is behind a word and the run is readings only**, since #1481. The
 * repository and the branch were on this line and are drawn a second time by
 * *Where things are*, one column down; Spend and Turns went to Pulse. What is
 * left is what a person reads rather than what they came to fetch.
 *
 * A Job with no assigned Drone has one act, so the split button is a button,
 * held rather than asked because it is a kill alone. Nothing to approve or
 * merge while a Job works, so no primary.
 */
export const ARunningJob: Story = {
  args: {
    status: "running",
    statusIcon: CircleDot,
    statusLabel: "Running",
    headline: "Split the settings reducer",
    jobId: "12-split-the-settings-reducer",
    jobIdLabel: "Job",
    fields: [
      { label: "Step", value: "2 of 4", mono: true },
      { label: "Run time", value: "11m 03s", mono: true },
      { label: "Dispatched by you" },
    ],
    actions: (
      <>
        <Button variant="ghost">Watch the turns</Button>
        <HoldButton
          askLabel="Kill job"
          description="Kills the job once held until it fills. Letting go sooner kills nothing."
          onAsk={fn()}
          onCommit={fn()}
        >
          Hold to kill job
        </HoldButton>
      </>
    ),
  },
  play: async ({ canvas }) => {
    // The id says what it is. It was the first value on a line of bare
    // strings, and the owner could not tell it from the branch beside it.
    await expect(canvas.getByRole("heading", { name: "Split the settings reducer" })).toBeVisible();
    await expect(canvas.getByText("Job")).toBeVisible();

    // And no way back that does not go anywhere — #1481.
    await expect(canvas.queryByRole("button", { name: "Overview" })).toBeNull();
  },
};

/**
 * A running job somebody has changed. The way into its settings sits left of
 * the act group — quiet, because it ends nothing — and counts what differs from
 * how a Job starts, so a changed Job reads as one without opening anything.
 */
export const ARunningJobWithSettings: Story = {
  args: {
    ...ARunningJob.args,
    actions: (
      <>
        <JobSettingsButton changed={2} onOpen={fn()} />
        <HoldButton
          askLabel="Kill job"
          description="Kills the job once held until it fills. Letting go sooner kills nothing."
          onAsk={fn()}
          onCommit={fn()}
        >
          Hold to kill job
        </HoldButton>
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
 * its worktree, and those sit further down the screen.
 *
 * **Run time on a Job that is over is the figure it stopped at.** It ticks
 * while the Job runs and freezes at `ended_at`; a killed Job whose figure kept
 * climbing read as one still working, which is how the owner met a ten-hour
 * `Elapsed` on a Job dead since the morning.
 */
export const AFailedJob: Story = {
  args: {
    status: "completed-failed",
    statusIcon: X,
    statusLabel: "Failed",
    headline: "Cache the manifest read",
    jobId: "91-cache-the-manifest-read",
    jobIdLabel: "Job",
    fields: [
      { label: "Stopped at", value: "Run tests" },
      { label: "step", value: "3 of 4", mono: true, continues: true },
      { label: "Run time", value: "22m 41s", mono: true },
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
 * the things that end a job. It is `--accent`, the token the design contract
 * gives links, and it underlines only on hover.
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
    jobId: "40-add-a-retry-ceiling-to-the-poke-loop",
    jobIdLabel: "Job",
    fields: [
      { label: "All", value: "4 of 4", mono: true, suffix: "steps advanced" },
      {
        label: "Pull request",
        value: "#4711",
        mono: true,
        href: "https://forge.invalid/org/repo/pull/4711",
      },
      { label: "Run time", value: "18m 22s", mono: true },
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
      {
        label: "Pull request",
        value: "#4711",
        mono: true,
        href: "https://forge.invalid/org/repo/pull/4711",
      },
      { label: "merged", continues: true },
      { label: "Run time", value: "18m 22s", mono: true },
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
 * **Drawn as `Acts.tsx` draws it:** mildest first, so `Kill drone` is the held
 * face and `Kill job` asks from behind the caret. Secondary, because a running
 * Job waits on its drone rather than a person. The drone act appears only where
 * a drone is assigned, which is why the `Drone` fact and that face go together.
 */
export const BothKills: Story = {
  args: {
    ...ARunningJob.args,
    fields: [
      { label: "Step", value: "2 of 4", mono: true },
      { label: "at", value: "implement", mono: true, continues: true },
      { label: "Run time", value: "11m 03s", mono: true },
      { label: "Dispatched by you" },
    ],
    actions: (
      <>
        <SplitButton
          variant="secondary"
          menuLabel="Everything else this job can do"
          items={[{ label: "Kill job, it ends here", danger: true, onSelect: fn() }]}
          onAction={fn()}
          hold={{
            label: "Hold to kill drone",
            description:
              "Kills the drone once held until it fills. Letting go sooner kills nothing. The job stays open.",
            onCommit: fn(),
          }}
        >
          Kill drone
        </SplitButton>
      </>
    ),
  },
};

/**
 * The same pair with the menu open, which is the only view where the
 * distinction can be read. **The act that ends the Job is behind the caret** —
 * the screen leads with the milder kill, which leaves the Job open with its
 * worktree held, and the face holds rather than asks. The caret never starts a
 * hold.
 */
export const BothKillsMenuOpen: Story = {
  args: {
    ...BothKills.args,
    actions: (
      <>
        <SplitButton
          variant="secondary"
          defaultOpen
          menuLabel="Everything else this job can do"
          items={[{ label: "Kill job, it ends here", danger: true, onSelect: fn() }]}
          onAction={fn()}
          hold={{
            label: "Hold to kill drone",
            description:
              "Kills the drone once held until it fills. Letting go sooner kills nothing. The job stays open.",
            onCommit: fn(),
          }}
        >
          Kill drone
        </SplitButton>
      </>
    ),
  },
};

/**
 * A job minted by a redispatch, naming the one it replaced. **The handle, and
 * pressable** — it drew a bare ULID, which was the one fact on the screen about
 * where the job came from and the one fact a person could not use. `#1474`.
 *
 * **`opensJob` rather than `href`.** The destination is inside Armada, so the
 * value is a job id and the press goes to `onOpenJob`; an address would make
 * `onFollowed` answer two questions and send this one to a browser.
 *
 * The drawing is the link's, because the affordance a person reads is the
 * same. What differs is the element: a button, because there is nowhere to go
 * without the app.
 */
export const ItReplacedAnEarlierJob: Story = {
  args: {
    ...AFinishedJob.args,
    fields: [
      { label: "Branch", value: "fix/poke-ceiling", mono: true, copyValue: "fix/poke-ceiling" },
      { label: "Ran", value: "18m 22s", mono: true },
      { label: "Dispatched by you" },
      {
        label: "Redispatched from",
        value: "118-add-a-retry-ceiling-to-the-poke-loop",
        mono: true,
        opensJob: "01M2C1TJ8G0099REDISPATCHED",
      },
    ],
    onOpenJob: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    const handle = "118-add-a-retry-ceiling-to-the-poke-loop";
    const said = canvas.getByRole("button", { name: handle });

    // What is on screen is the handle, and no id is anywhere near it.
    await expect(said).toHaveTextContent(handle);
    await expect(said).not.toHaveTextContent("01M2C1TJ8G0099REDISPATCHED");

    // The press sends the id, and this component navigates nowhere.
    await userEvent.click(said);
    await expect(args.onOpenJob).toHaveBeenCalledWith("01M2C1TJ8G0099REDISPATCHED");
  },
};

/**
 * The same fact with nowhere to go. **No control where nothing opens** — the
 * rule the callout on the replaced job already keeps, rather than a link that
 * looks pressable and is not.
 */
export const ItReplacedAJobNothingCanOpen: Story = {
  args: {
    ...ItReplacedAnEarlierJob.args,
    onOpenJob: undefined,
  },
  play: async ({ canvas }) => {
    const handle = "118-add-a-retry-ceiling-to-the-poke-loop";
    await expect(canvas.queryByRole("button", { name: handle })).toBeNull();
    await expect(canvas.getByText(handle)).toBeVisible();
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
    jobId: "91-cache-the-manifest-read",
    jobIdLabel: "Job",
    fields: [
      { label: "Step", value: "3 of 4", mono: true },
      { label: "at", value: "verify", mono: true, continues: true },
      { label: "Run time", value: "22m 41s", mono: true },
      { label: "Dispatched by you" },
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
    jobId: "91-cache-the-manifest-read",
    jobIdLabel: "Job",
    fields: [
      { label: "Step", value: "1 of 4", mono: true },
      { label: "at", value: "plan", mono: true, continues: true },
      { label: "Waiting", value: "4m 12s", mono: true },
      { label: "Dispatched by you" },
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
 * four lines beside three buttons and the fact run broke with one fact alone
 * on the end. Now the acts drop under the title when the two stop fitting, and
 * the facts read as one run again the moment they have the width.
 *
 * `--window-floor` is the narrowest window Bridge opens, and the two below it
 * are what the panel is given inside one — the header is not the window.
 * **Below `--w-sheet` the trailing two facts give way**, since #1093 replaced
 * the rule that nothing here was ever dropped. The two that go are the two
 * least urgent, which is what fixed the run's order.
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
