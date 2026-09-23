import type { Meta, StoryObj } from "@storybook/react-vite";
import { Check, CircleDot, Clock, Cpu, Folder, GitBranch, OctagonAlert, Power, ScrollText, UserCheck, X } from "lucide-react";
import { expect, fn, waitFor } from "storybook/test";
import { SplitButton } from "../../primitives/SplitButton/SplitButton";
import { ActiveJobsList } from "../ActiveJobsList/ActiveJobsList";
import { StepBar } from "../StepBar/StepBar";
import { JobRowStacked } from "./JobRowStacked";

/**
 * One story per Job state the M1 drawing puts in the list, plus the three row
 * states the focus model names — focused, selected, dimmed — and the row at
 * the 720px floor.
 *
 * **The labels come from the enum→verb map**, `crates/core-model/domain/
 * enum-verbs.toml`, sentence-cased. They are written here because that map is
 * not generated into TypeScript yet, and nowhere that ships.
 *
 * **Every row's field run now carries origin** — display-only, never a filter
 * axis (`docs/concepts/job-board.md`, Origin tagging). The values: dispatched
 * by you, found by Fleet, drafted in Helm, workflow-triggered, drafted by a
 * Drone, and sub-dispatched, which additionally names the parent Job. See
 * issue 218.
 */
const meta: Meta<typeof JobRowStacked> = {
  title: "Compositions/Job row (stacked)",
  component: JobRowStacked,
};
export default meta;

type Story = StoryObj<typeof JobRowStacked>;

const open = (
  <SplitButton
    ground="card"
    items={[
      { label: "Copy job id", shortcut: "⌘C" },
      { label: "Kill", shortcut: "x", danger: true },
    ]}
  >
    Open
  </SplitButton>
);

/**
 * The track list a Job at the gate takes: no branch, step or elapsed reading
 * yet, so a "created" timestamp and origin do the work the running row's
 * time and spend tracks do. Duplicated rather than imported, because a
 * composition's story cannot reach into a screen — but every width is now a
 * named property declared in this component's own stylesheet, which is what
 * makes the duplicate a second reference rather than a second answer.
 *
 * The fourth track was `--armada-track-time`, 76px, while `APPROVAL_TRACKS` in
 * `TheList` drew the drawing's 100px as a bare `calc()`.
 * The drawing wins: the timestamp track is `--armada-track-created`, and both
 * lists say so in the same words.
 */
const GATE_TRACKS = [
  "var(--armada-track-origin)",
  "var(--armada-track-bar)",
  "var(--armada-track-step)",
  "var(--armada-track-created)",
  "var(--armada-track-provenance)",
].join(" ");

/**
 * A Job that has not run. `needs approval` is the verb the enum→verb map
 * carries — the badge means a person must act, not that time is passing.
 *
 * **The field set is different, so the track list is.** No branch, no step and
 * no elapsed yet: a job that has not run has different facts, and the track
 * list belongs to the field set rather than to the row.
 */
export const NeedsApproval: Story = {
  args: {
    status: "awaiting-approval",
    statusIcon: UserCheck,
    statusLabel: "Needs approval",
    headline: "Coalesce concurrent token refreshes",
    jobId: "job_7c31",
    handle: "7-coalesce-token-refreshes",
    tracks: GATE_TRACKS,
    fields: [
      { label: "Workflow", value: "bug, 4 steps", quiet: true },
      { value: <StepBar total={4} current={0} label="Not started, 4 steps" /> },
      { value: "Not started", quiet: true },
      { value: "created 09:12", quiet: true },
      { value: "Dispatched by you" },
    ],
    action: (
      <SplitButton ground="card" items={[{ label: "Reject", danger: true }]}>
        Approve
      </SplitButton>
    ),
  },
};

/**
 * `queued` renders grey whatever its reason, and a reader correctly moves past
 * a grey row. Where a reason is set it supplies the headline verb and the
 * glyph; with none it reads queued, which is this row.
 *
 * **The step field says the step has not started, and nothing more.** It read
 * "Waiting on a drone" — a reason, written into a field on a row whose badge
 * carries none, and the reason it named is not one the registry has. A queued
 * Job blocked on the concurrency cap reads `waiting_on_resources` on the badge;
 * see `Active jobs list`.
 */
export const Queued: Story = {
  args: {
    status: "not-started",
    statusIcon: Clock,
    statusLabel: "Queued",
    headline: "Retire the legacy poke path",
    jobId: "job_8b42",
    handle: "6-retire-the-legacy-poke-path",
    tracks: GATE_TRACKS,
    fields: [
      { label: "Workflow", value: "bug, 4 steps", quiet: true },
      { value: <StepBar total={4} current={0} label="Not started, 4 steps" /> },
      { value: "Not started", quiet: true },
      { value: "approved 09:20", quiet: true },
      { value: "Dispatched by you" },
    ],
    action: open,
  },
};

/**
 * **A queued Job a person put back, which the row above cannot be told from.**
 * Press restart while every place is taken and the Job lands here: `queued`,
 * `waiting_on_resources`, and — until the headline said so — identical to a Job
 * approved an hour ago and never started. The press moved nothing on screen,
 * which is the worst reading of a correct system.
 *
 * It is new. A restart and an override spawned a drone on the spot until
 * re-admission put them behind the concurrency cap.
 *
 * **The word is a suffix on the title, not a sixth field**, which is where the
 * `Escalated, second time` story already puts a qualifier — the field run's
 * tracks are shared down the list and a conditional sixth would land in the
 * track reserved for spend. It comes from `resumption` in `enum-verbs.toml` and
 * carries no glyph, because a headline carries none.
 *
 * **The badge is unchanged and says what the Job is waiting for.** Two facts,
 * two channels: the badge answers "why has nothing started" and the headline
 * answers "did my press land". Folding either into the other loses one of them.
 */
export const QueuedAfterARestart: Story = {
  args: {
    status: "not-started",
    statusIcon: Cpu,
    statusLabel: "Waiting on resources",
    headline: "Retire the legacy poke path, restarted",
    jobId: "job_8b42",
    handle: "6-retire-the-legacy-poke-path",
    tracks: GATE_TRACKS,
    fields: [
      { label: "Workflow", value: "bug, 4 steps", quiet: true },
      { value: <StepBar total={4} current={2} label="Step 2 of 4" /> },
      { value: "Run tests", mono: true, emphasis: true },
      { value: "queued 09:41", quiet: true },
      { value: "Dispatched by you" },
    ],
    action: open,
  },
};

/**
 * The running row standing alone, outside any list: the badge pulses and the
 * step bar beside it stays still.
 *
 * `pulsing` says the Job is running, and a running row pulses the same alone
 * as inside a list, whether or not the cursor is on it (#1276).
 */
export const Running: Story = {
  args: {
    pulsing: true,
    status: "running",
    statusIcon: CircleDot,
    statusLabel: "Running",
    headline: "Split the settings reducer",
    jobId: "job_2d90bb",
    handle: "12-split-the-settings-reducer",
    fields: [
      { value: "fix/settings-split", mono: true, icon: GitBranch, copyValue: "fix/settings-split" },
      { value: <StepBar total={4} current={2} activity="running" label="Step 2 of 4" /> },
      { value: "Implement", emphasis: true },
      { value: "11m 03s", mono: true },
      { value: "~$1.80", mono: true },
      { value: "Dispatched by you" },
    ],
    action: open,
  },
};

/**
 * The same row with the keyboard cursor on it: a 2px `--accent` left edge and
 * `--bg-hover`. Focus adds the edge and nothing else — the pulse was already
 * there.
 */
export const RunningFocused: Story = {
  args: { ...Running.args, focused: true } as never,
};

/**
 * The cursor's row, with the key that fires its control.
 *
 * **One key per verb, and one control per row**, so at most one key ever
 * applies to the row under the cursor: `o` opens, `r` reviews, `t` attests,
 * `d` redirects. Every other verb key no-ops rather than acting on the wrong
 * one. Each key is the verb's own initial except Redirect, because `r` is
 * spent on Review.
 *
 * **The chip is drawn on every row and hidden until the cursor lands.** It
 * holds its width either way, so the run does not reflow as the cursor moves —
 * and the cost of that is a chip's width on rows that are not showing one.
 *
 * The row does not bind the key. A row cannot know whether a text input
 * elsewhere on the screen holds focus, and the contract suppresses every
 * single-key action while one does.
 */
export const FocusedWithItsKey: Story = {
  args: { ...Running.args, focused: true, actionKey: "o" } as never,
};

/**
 * The same key on a row nothing has the cursor on. It renders as a reserved
 * gap rather than a chip: the width is what keeps the list from moving, and
 * the caption is what would say the same thing fourteen times.
 */
export const UnfocusedWithItsKey: Story = {
  args: { ...Running.args, actionKey: "o" } as never,
};

/** `--accent-muted` fill. Selected and focused are different states and coexist. */
export const Selected: Story = {
  args: { ...Running.args, selected: true } as never,
};

/**
 * De-emphasised: `--border-subtle` and `--fg-subtle`, never an `opacity`.
 * Dimming is a token.
 */
export const Dimmed: Story = {
  args: { ...Running.args, dimmed: true } as never,
};

/**
 * A Job that started running 18 seconds ago, 40% of the way through `--duration-decay`: the tint sits
 * between `--row-tint-recent` and `--row-tint`, and the note fades with it. The age is held still
 * here; on a screen `useRecentChanges` walks it.
 */
export const ChangedMidDecay: Story = {
  name: "Changed, mid-decay",
  args: { ...Running.args, changed: { note: "Running · 18s ago", remaining: 0.6 } } as never,
};

/**
 * `escalated` renders its reason where one is set, never its own name — nobody
 * says a Job escalated at step 3. The status row has carried `needs you` behind
 * `megaphone` since #400, and it stands only where no reason reaches a surface.
 */
export const EscalatedStalled: Story = {
  args: {
    status: "escalated",
    statusIcon: OctagonAlert,
    statusLabel: "Stalled",
    headline: "Job 12 stalled at step 3",
    jobId: "job_12",
    handle: "12-fix-auth-session-race",
    fields: [
      { value: "auth/session.rs", mono: true, icon: GitBranch, copyValue: "auth/session.rs" },
      { value: <StepBar total={5} current={3} activity="stopped" label="Step 3 of 5" /> },
      { value: "3 pokes", emphasis: true },
      { value: "12m", mono: true },
      { value: "~$1.80", mono: true },
      { value: "Found by Fleet" },
    ],
    action: (
      <SplitButton ground="card" items={[{ label: "Kill", danger: true }]}>
        Pilot
      </SplitButton>
    ),
  },
};

/**
 * A second stall at the same step reads "2nd time" in the headline, and the
 * detail view surfaces the prior attempt. Presentation only — recurrence
 * changing behaviour is a separate decision.
 */
export const EscalatedSecondTime: Story = {
  args: {
    ...EscalatedStalled.args,
    headline: "Job 12 stalled at step 3, 2nd time",
  } as never,
};

/**
 * A failed segment is loud. At M1 a failed Check ends the Job, so this row is
 * the entire reason a person opened the screen.
 */
export const Failed: Story = {
  args: {
    status: "completed-failed",
    statusIcon: X,
    statusLabel: "Failed",
    headline: "Cache the manifest read",
    jobId: "job_91ab",
    handle: "14-cache-the-manifest-read",
    fields: [
      { value: "feat/manifest-cache", mono: true, icon: GitBranch, copyValue: "feat/manifest-cache" },
      { value: <StepBar total={4} current={3} activity="failed" label="Step 3 of 4" /> },
      { value: "Run tests", emphasis: true },
      { value: "22m 41s", mono: true },
      { value: "~$2.10", mono: true },
      { value: "Found by Fleet" },
    ],
    action: open,
  },
};

/**
 * A killed segment is not loud. Killing is a human decision rather than a
 * system failure and must not read as an error — the segment keeps
 * `--fg-default` and the badge takes the grey `killed` hue.
 */
export const Killed: Story = {
  args: {
    status: "killed",
    statusIcon: Power,
    statusLabel: "Killed",
    headline: "Rename the session token field",
    jobId: "job_5e88",
    handle: "18-rename-session-token",
    fields: [
      { value: "feat/session-rename", mono: true, icon: GitBranch, copyValue: "feat/session-rename" },
      { value: <StepBar total={4} current={2} activity="killed" label="Step 2 of 4" /> },
      { value: "Implement", emphasis: true },
      { value: "4m 09s", mono: true },
      { value: "~$0.60", mono: true },
      { value: "Workflow-triggered" },
    ],
    action: open,
  },
};

/** A finished Job: every segment past, and no hue left for a current step. */
export const Done: Story = {
  args: {
    status: "completed-success",
    statusIcon: Check,
    statusLabel: "Done",
    headline: "Add a retry ceiling to the poke loop",
    jobId: "job_4f10",
    handle: "21-retry-ceiling-on-poke-loop",
    fields: [
      { value: "fix/poke-ceiling", mono: true, icon: GitBranch, copyValue: "fix/poke-ceiling" },
      { value: <StepBar total={4} current={5} activity="advanced" label="All 4 of 4 steps advanced" /> },
      { value: "Summarise" },
      { value: "18m 22s", mono: true },
      { value: "~$2.40", mono: true },
      { value: "Drafted in Helm" },
    ],
    action: open,
  },
};

/**
 * Spend follows the active billing mode, and the visible number is always the
 * number that gates dispatch. The track is sized for the wider of the two
 * strings rather than for whichever example came first.
 */
export const SpendAsQuota: Story = {
  args: {
    ...Running.args,
    fields: [
      { value: "fix/settings-split", mono: true, icon: GitBranch, copyValue: "fix/settings-split" },
      { value: <StepBar total={4} current={2} activity="running" label="Step 2 of 4" /> },
      { value: "Implement", emphasis: true },
      { value: "11m 03s", mono: true },
      { value: "68% quota", mono: true },
      { value: "Dispatched by you" },
    ],
  } as never,
};

/**
 * At the 768px floor, with the rail at 48px: 720px of content. **The same
 * row.** Nothing reshapes and no field is dropped — secondary values truncate
 * with a tooltip carrying the full string, and the badge stays leading so
 * status is still the first thing caught.
 */
export const AtTheWidthFloor: StoryObj = {
  render: () => (
    <div style={{ width: "calc(var(--window-floor) - var(--sidebar-rail))" }}>
      <JobRowStacked
        status="running"
        statusIcon={CircleDot}
        statusLabel="Running"
        headline="Split the settings reducer so the selectors can be tested alone"
        jobId="job_2d90bb"
        handle="12-split-the-settings-reducer-selectors"
        fields={[
          { value: "fix/settings-split-selectors", mono: true, icon: GitBranch, copyValue: "fix/settings-split-selectors" },
          { value: <StepBar total={4} current={2} activity="running" label="Step 2 of 4" /> },
          { value: "Implement", emphasis: true },
          { value: "11m 03s", mono: true },
          { value: "~$1.80", mono: true },
          { value: "Dispatched by you" },
        ]}
        action={open}
      />
    </div>
  ),
};

/**
 * A Job that writes in several places has no single workspace, so its row
 * **names its first write target and counts the rest** — `+2` where three
 * Workspaces are declared. Every other row names a place in that column, so
 * this one does too, and the folder glyph keeps meaning a workspace. A bare
 * "3 workspaces" was drawn and rejected: it puts a count where the column
 * holds an identifier.
 *
 * **No chip and no hue.** A bordered pill is a Job state and nothing else, so
 * shape reads as plain text. The Board computes it from `write_targets` and
 * `atomic` — nothing on Job stores a shape.
 */
export const SeveralWriteTargets: Story = {
  args: {
    ...Running.args,
    headline: "Retire the poke path across the fleet",
    handle: "16-retire-poke-path-fleet",
    fields: [
      { value: "crates/fleet +2", mono: true, icon: Folder },
      { value: <StepBar total={4} current={2} activity="running" label="Step 2 of 4" /> },
      { value: "Implement", emphasis: true },
      { value: "11m 03s", mono: true },
      { value: "~$4.20", mono: true },
      { value: "Workflow-triggered" },
    ],
  } as never,
};

/**
 * **The one exception on the Board: a sub-dispatched Job that is out of
 * headroom.** Almost every sub-dispatched Job is already running before
 * anything could render it — this is the single reason one appears here,
 * `waiting_on_resources`, still `queued` (`docs/concepts/job-board.md`,
 * "A sub-dispatched Job is usually already running").
 *
 * **No Approve, no Dispatch** — the approval already happened at the parent
 * Job named in the origin field, so a decision control here would offer a
 * choice that has no content. **Kill stays available**, and the row is
 * dimmed, the same visually-distinct treatment a blocked Job gets. Without
 * the origin field naming `job_2d90bb` — the parent, running in the
 * `Running` story above — a read-only row with no primary action would look
 * arbitrary; that is the whole reason this issue put origin on the row
 * rather than only on job detail.
 *
 * **The step field says the step, and does not repeat the reason.** It read
 * "Held for CPU headroom", which is a value the real surface cannot emit twice
 * over: no vocabulary in the repository contains those words, and the badge
 * beside it already carries the reason — which is exactly the correction
 * `Active jobs list` made to its own queued rows. The registry does have a word
 * for a Job the machine is holding, `admission_hold.memory`, and it reads
 * "waiting on memory"; it renders in the status bar, once for the whole fleet,
 * because how full the machine is is not a fact about one row.
 */
export const SubDispatchedWaitingOnResources: Story = {
  args: {
    status: "not-started",
    statusIcon: Cpu,
    statusLabel: "Waiting on resources",
    headline: "Precompute embeddings for the batch import step",
    jobId: "job_9f21",
    handle: "23-precompute-embeddings-batch-import",
    dimmed: true,
    tracks: GATE_TRACKS,
    fields: [
      { label: "Workflow", value: "chore, 3 steps", quiet: true },
      { value: <StepBar total={3} current={1} label="Step 2 of 3, waiting" /> },
      { value: "embed-batch", mono: true, emphasis: true },
      { value: "queued 09:41", quiet: true },
      { value: "Sub-dispatched by job_2d90bb" },
    ],
    action: (
      <SplitButton ground="card" variant="destructive" items={[]}>
        Kill
      </SplitButton>
    ),
  },
};

/**
 * **The longest verb in the registry, at the width floor.** `a required
 * command did not succeed` is 34 characters — `enum-verbs.toml`'s longest, and
 * more than twice what the drawn 132px badge column holds.
 *
 * The column was a bare fixed value until #262 and this painted across the
 * headline: `Badge` is `nowrap` and nothing clips it. It is `minmax(132px,
 * max-content)` now, so the drawn width is a floor like every other track's
 * and the list sizes the column to its widest badge.
 *
 * **This story is against the registry, not against the bug report.** The
 * reported case was `evidence disputed` at 17 characters, and a story pinned to
 * that would pass again the next time a longer verb is added. Widening the
 * fixed value would also have passed it — which is why the drawing's own 184px
 * is not the fix.
 *
 * **Past this length the badge takes the row's own first line (#984)** —
 * `minmax(132px, max-content)` alone still starved the headline beside it.
 */
export const TheLongestVerbAtTheWidthFloor: StoryObj = {
  render: () => (
    // A story has no list frame to be the container, so it is its own —
    // `containerType` is what the wide-badge container query reads (#984).
    <div
      style={{ width: "calc(var(--window-floor) - var(--sidebar-rail))", containerType: "inline-size" }}
    >
      <JobRowStacked
        status="escalated"
        statusIcon={OctagonAlert}
        statusLabel="A required command did not succeed"
        headline="Reconcile orphaned drones on Fleet start"
        jobId="job_31c7"
        handle="31-reconcile-orphaned-drones-fleet-start"
        fields={[
          { value: "fix/orphan-reconcile", mono: true, icon: GitBranch, copyValue: "fix/orphan-reconcile" },
          { value: <StepBar total={5} current={3} activity="failed" label="Step 4 of 5" /> },
          { value: "Regression check", emphasis: true },
          { value: "1h 12m", mono: true },
          { value: "~$3.40", mono: true },
          { value: "Dispatched by you" },
        ]}
        action={open}
      />
    </div>
  ),
  play: async ({ canvasElement }) => {
    const wrapper = canvasElement.querySelector<HTMLElement>("div")!.getBoundingClientRect();
    const row = canvasElement.querySelector<HTMLElement>(".armada-job-row")!;
    const badge = row.querySelector<HTMLElement>(".armada-badge")!;
    await expect(badge).toHaveTextContent("A required command did not succeed");
    await expect(badge.getBoundingClientRect().right).toBeLessThanOrEqual(wrapper.right);
    const title = row.querySelector<HTMLElement>(".armada-job-row__title")!;
    await expect(title).toHaveTextContent("Reconcile orphaned drones on Fleet start");
    await expect(row.querySelector(".armada-job-row__action")!.getBoundingClientRect().right).toBeLessThanOrEqual(
      wrapper.right,
    );
  },
};

/** Three titles of wildly different lengths, so nothing here lines up by accident. */
const HANDLE_ROWS = [
  { handle: "3-runningWaitingOnACommand", headline: "Reuse one HTTP client across every query" },
  { handle: "14-cache-the-manifest-read", headline: "Cache the manifest read" },
  {
    handle: "1-board-s-clear-button-should-reclaim-worktr",
    headline: "Refuse a merge press whose chosen comments were never posted",
  },
] as const;

const handleCopied = fn();

/**
 * **The handle is a column, not the tail of the title.** It sat at the end of
 * the headline, so it started wherever that row's title happened to end while
 * the facts under it stood in columns — "none of the ids align nicely like the
 * rest of the job row text". It leads the field run now, labelled the way
 * Workflow and Progress are.
 *
 * `Job`, not `ID`: the handle is what a person calls a Job
 * (`crates/core-model/src/job/handle.rs`), and the id is the ULID the row
 * carries and never draws.
 *
 * The list is here because the tracks are: a row standing alone sizes its own
 * handle column, and the frame is what spreads one column across every row.
 */
export const HandlesInTheirOwnColumn: StoryObj = {
  render: () => (
    <ActiveJobsList
      heading="Active jobs"
      summary="3 jobs."
      selectable
      label="Active jobs"
      columns={["Workflow", "Progress", "Run time"]}
    >
      {HANDLE_ROWS.map((row) => (
        <JobRowStacked
          key={row.handle}
          status="running"
          statusIcon={CircleDot}
          statusLabel="Running"
          headline={row.headline}
          jobId={row.handle}
          handle={row.handle}
          onOpen={() => {}}
          onCopied={handleCopied}
          fields={[
            { label: "Workflow", icon: ScrollText, value: "Bug, 6 steps" },
            {
              label: "Progress",
              value: (
                <>
                  <StepBar total={6} current={2} activity="running" label="Step 2 of 6" />
                  <span className="armada-row-step">regression_verify</span>
                </>
              ),
            },
            { label: "Run time", value: "11m 03s", mono: true },
          ]}
          action={
            <SplitButton ground="card" items={[{ label: "Kill", danger: true }]}>
              Open
            </SplitButton>
          }
        />
      ))}
    </ActiveJobsList>
  ),
  play: async ({ canvasElement, userEvent }) => {
    const handles = [...canvasElement.querySelectorAll<HTMLElement>(".armada-job-row__id")];
    const titles = [...canvasElement.querySelectorAll<HTMLElement>(".armada-job-row__title")];
    await expect(handles).toHaveLength(HANDLE_ROWS.length);

    // The claim a rendering cannot make: every handle starts on one x.
    const starts = handles.map((handle) => handle.getBoundingClientRect().left);
    for (const start of starts) await expect(start).toBeCloseTo(starts[0]!, 0);

    // And the titles end nowhere near each other, which is what the handles
    // used to follow. Without this, the assertion above would pass on three
    // rows that happened to be the same width.
    const ends = titles.map((title) => title.getBoundingClientRect().right);
    await expect(Math.max(...ends) - Math.min(...ends)).toBeGreaterThan(24);

    // Each handle is under its own title rather than beside it.
    for (const [at, handle] of handles.entries()) {
      await expect(handle.getBoundingClientRect().top).toBeGreaterThanOrEqual(
        titles[at]!.getBoundingClientRect().bottom,
      );
    }

    // Still an identifier, so it still copies (docs/contracts/design-system.md).
    handleCopied.mockClear();
    await userEvent.click(handles[1]!);
    // The clipboard write is a promise, and the surface is told either way it settles.
    await waitFor(() => expect(handleCopied).toHaveBeenCalledWith(HANDLE_ROWS[1].handle));
  },
};
