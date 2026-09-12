import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import type { ServerState } from "@armada/protocol";
import type { JobFixture } from "../../../fixtures/fixture";
import { propsFor } from "../../../fixtures/props";
import {
  awaitingApproval,
  awaitingAttestation,
  awaitingRepair,
  completedFailed,
  completedSuccess,
  escalatedBlockedByPolicy,
  escalatedEvidenceSuspect,
  escalatedGateFailure,
  escalatedInterrupted,
  escalatedLoopCap,
  escalatedNoReport,
  escalatedSilent,
  killed,
  piloted,
  preparing,
  queued,
  reading,
  rejected,
  retryingCheckFailure,
  review,
  reviewAtDelivery,
  running,
  runningAtGate,
  runningWaitingOnACommand,
  superseded,
  unreadable,
} from "../../../fixtures/build/index";
import { JOB_ID, spend, watchedRead } from "../../../fixtures/build/base";
import { WAITING_CALL } from "../../../fixtures/build/running";
import { recorded } from "../../../fixtures/recorded";
import { JobDetailFrom } from "./JobDetail";

/**
 * Job detail in every state a Job can be in, drawn by the app's own screen from
 * wire data.
 *
 * A recorded story is a real Job, captured off a Fleet by
 * `scripts/record-job.mjs` and replayed through the fold Bridge's main process
 * runs. A built story is composed in code from the wire types, for states a
 * real Job rarely sits in long enough to catch. The type checker keeps those
 * honest: a field Fleet stops sending stops compiling here.
 *
 * Every region is live, so a story can be clicked through like the app. A press
 * that would change the Job does nothing, because there is no Fleet behind it.
 */
const meta: Meta<typeof JobDetailFrom> = {
  title: "Screens/Job detail",
  component: JobDetailFrom,
  parameters: { layout: "fullscreen" },
};
export default meta;

type Story = StoryObj<typeof JobDetailFrom>;

/** A story that draws one fixture, with no controls. A whole Job is not an arg. */
function drawing(fixture: () => JobFixture): Story["render"] {
  return () => <JobDetailFrom fixture={fixture()} />;
}

/** A real Job that landed. Its worktree was reclaimed, so there is no diff to read. */
export const DoneRecorded: Story = {
  name: "Done (recorded)",
  render: drawing(() => recorded("done-worktree-given-back")),
};

/** The recorded Job at the narrowest window. The title and the job id stack, and the acts are one control. */
export const DoneRecordedNarrow: Story = {
  name: "Done (recorded), narrow window",
  render: () => (
    <JobDetailFrom fixture={recorded("done-worktree-given-back")} width="var(--window-floor)" />
  ),
};

/** The header's one control with its menu open: the act that leads on its face, the rest behind it. */
export const HeaderActionsOpen: Story = {
  name: "Header actions open",
  render: drawing(() => recorded("done-worktree-given-back")),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(
      await canvas.findByRole("button", { name: "Everything else this job can do" }),
    );
    await expect(await within(document.body).findByRole("menuitem", { name: /record/i })).toBeVisible();
  },
};

/** Midway through Fix, before its Check has run. */
export const Running: Story = { name: "Running", render: drawing(running) };

/** What the panel's choice sends. A module's spy, cleared by the play that reads it. */
const setWhenBlocked = fn();

/**
 * The running Job, changed: a model chosen for its later steps and one command
 * allowed for it. What the header counts, and what the panel lists.
 */
function runningWithSettings(): JobFixture {
  const fixture = running();
  if (fixture.watched.state !== "read") return fixture;
  return {
    ...fixture,
    watched: watchedRead({
      ...fixture.watched.detail,
      model_override: "opus",
      allowed_commands: [
        {
          run: "pnpm add -D reselect@5.1.1",
          reach: "job",
          allowed_at: "2026-09-10T14:29:40Z",
          by: "human",
        },
      ],
    }),
  };
}

/**
 * **The header's way in, and the panel it opens.** The count on the button is
 * the model and the allow; the panel says what each setting does and when a
 * change to it takes. Nothing on the screen behind it restarts.
 *
 * **What goes is this Job's id and the wire's word.** The panel's own story
 * proves a choice names `ask_me`; this proves the screen hands it on against the
 * Job on screen, which is the half a composition cannot show. Broken once by
 * sending the Job's handle rather than its id.
 */
export const JobSettingsOpen: Story = {
  name: "Job settings open",
  render: () => (
    <JobDetailFrom fixture={runningWithSettings()} on={{ onSetWhenBlocked: setWhenBlocked }} />
  ),
  play: async ({ canvas, userEvent }) => {
    setWhenBlocked.mockClear();
    await userEvent.click(await canvas.findByRole("button", { name: /^Job settings/ }));
    const panel = within(await canvas.findByRole("dialog", { name: "Job settings" }));
    await userEvent.click(panel.getByRole("radio", { name: "Ask me first" }));
    await expect(setWhenBlocked).toHaveBeenCalledWith(JOB_ID, "ask_me");
  },
};

/** What a waiting command's answer sends. A module's spy, cleared by the play that reads it. */
const answerCommand = fn();

/**
 * A Job at Ask me first whose Drone reached for a command it was not given, and
 * is waiting on a person — still `running`.
 * The command is what is asked; the answers are the three Fleet offered, each
 * saying what it commits to.
 *
 * **What goes is the call and the answer's wire name**, never the words on
 * the control. A label is copy, and Fleet takes `allow_for_job`.
 */
export const WaitingOnACommand: Story = {
  name: "Waiting on a command",
  render: () => (
    <JobDetailFrom fixture={runningWaitingOnACommand()} on={{ onAnswerCommand: answerCommand }} />
  ),
  play: async ({ canvas, userEvent }) => {
    answerCommand.mockClear();
    await userEvent.click(await canvas.findByRole("radio", { name: "Allow for this job" }));
    await userEvent.click(canvas.getByRole("button", { name: "Send this answer" }));
    await expect(answerCommand).toHaveBeenCalledWith(JOB_ID, WAITING_CALL, "allow_for_job");
  },
};

/**
 * Before the first Drone turn, while the worktree is cut and the repository's
 * preparation commands run. Fleet's own log is all there is to read.
 */
export const Preparing: Story = { name: "Preparing", render: drawing(preparing) };

/** A Check failed on the first attempt, and the Drone is on its second. */
export const CheckFailedRetrying: Story = {
  name: "Check failed, retrying",
  render: drawing(retryingCheckFailure),
};

/**
 * One Check has reported and the next is queued behind it, with the Judge after
 * both. The wire has no outcome for a Check still running, so this is what in
 * flight looks like.
 */
export const AtTheGate: Story = { name: "At the gate", render: drawing(runningAtGate) };

/** Every Check passed, and the Judge refused two criteria. */
export const JudgeRefused: Story = { name: "Judge refused", render: drawing(escalatedEvidenceSuspect) };

/** Every Check passed and the Judge met every criterion. A person decides next. */
export const Review: Story = { name: "Review", render: drawing(review) };

/**
 * The review moment on a Job whose names are as long as real ones get, with the
 * spend and turns a long run reaches. The header a person reported on 11 Sep
 * 2026: the facts wrap into an orphaned line, and the blocked-command setting
 * reads as the screen's main button with nothing saying what pressing it does.
 */
function reviewWithLongNames(): JobFixture {
  const fixture = reviewAtDelivery();
  if (fixture.watched.state !== "read") return fixture;
  const handle = "3-support-attaching-screenshots-and-file-sea";
  const renamed = {
    ...fixture.job,
    title: "Support attaching screenshots and file search in job context",
    handle,
    branch: `armada/${handle}`,
  };
  const whole = fixture.watched.detail;
  return {
    ...fixture,
    job: renamed,
    watched: watchedRead({
      ...whole,
      job: renamed,
      branch: renamed.branch,
      spend: spend({
        cost_micros: 29_630_000,
        cost_cap_micros: 60_000_000,
        unpriced: 2,
        turns: 580,
        turn_cap: 1000,
      }),
      ...(whole.delivery === undefined
        ? {}
        : {
            delivery: {
              ...whole.delivery,
              pull_request: "https://forge.example/armada/armada/pull/660",
            },
          }),
    }),
  };
}

/** The review header with real-length names, as reported. Kept to iterate the header against. */
export const ReviewLongNames: Story = {
  name: "Review, long names",
  render: drawing(reviewWithLongNames),
};

/** At review with a pull request open and comments on it. Merge becomes a fourth answer. */
export const ReviewWithPullRequest: Story = {
  name: "Review, pull request open",
  render: drawing(reviewAtDelivery),
};

/**
 * Merge pressed. Cancel holds focus and Enter presses whatever holds focus, so
 * Enter here cancels. A merge Bridge cannot take back never goes out by default.
 */
export const MergeConfirmation: Story = {
  name: "Merge confirmation",
  render: drawing(reviewAtDelivery),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /^Merge/ }));
    const layer = within(await canvas.findByRole("dialog"));
    await expect(layer.getByRole("button", { name: "Cancel" })).toHaveFocus();
  },
};

/** Two comments picked to send to a Drone, which makes Send live. */
export const CommentsPicked: Story = {
  name: "Comments picked",
  render: drawing(reviewAtDelivery),
  play: async ({ canvas, userEvent }) => {
    const remarks = within(
      await canvas.findByRole("region", { name: "Comments on the pull request" }),
    );
    const picks = remarks.getAllByRole("checkbox");
    const send = remarks.getByRole("button", { name: "Send to a drone" });
    await expect(send).toBeDisabled();
    await userEvent.click(picks[0]!);
    await userEvent.click(picks[1]!);
    await expect(send).toBeEnabled();
  },
};

/** A Check failed on its last attempt, and the Job stopped at the gate. */
export const StoppedAtTheGate: Story = {
  name: "Stopped at the gate",
  render: drawing(escalatedGateFailure),
};

/** A step spent its retries and holds for a person to repair it. */
export const NeedsRepair: Story = { name: "Needs repair", render: drawing(awaitingRepair) };

/** Waiting for room to run. Nothing has started. */
export const Queued: Story = { name: "Queued", render: drawing(queued) };

/** Proposed, and waiting for a person to approve it. */
export const NeedsApproval: Story = { name: "Needs approval", render: drawing(awaitingApproval) };

/** Waiting for a person to attest to what the work did. */
export const NeedsAttestation: Story = {
  name: "Needs attestation",
  render: drawing(awaitingAttestation),
};

/** A person is driving the Drone directly. */
export const Piloted: Story = { name: "Piloted", render: drawing(piloted) };

/** The Drone reached for something the policy refuses. */
export const BlockedByPolicy: Story = {
  name: "Blocked by policy",
  render: drawing(escalatedBlockedByPolicy),
};

/** Fleet stopped during a running step, and the Drone was gone when it came back. */
export const Interrupted: Story = { name: "Interrupted", render: drawing(escalatedInterrupted) };

/** The Drone stopped writing for longer than a step allows. */
export const Silent: Story = { name: "Silent", render: drawing(escalatedSilent) };

/** The step looped more times than its cap allows. */
export const LoopCap: Story = { name: "Loop cap", render: drawing(escalatedLoopCap) };

/** The Drone ended without submitting a report. */
export const NoReport: Story = { name: "No report", render: drawing(escalatedNoReport) };

/** Fleet would not answer for this Job's detail. Each region says what it could not read. */
export const FleetUnreachable: Story = { name: "Fleet unreachable", render: drawing(unreadable) };

/**
 * This Job's own detail asked for and not back yet. **What the Board already
 * holds draws at once** — the run's step names, the open step's name, every row
 * of where things are — and only what the read answers waits.
 */
export const StillReading: Story = {
  name: "Still reading",
  render: drawing(reading),
  play: async ({ canvas }) => {
    const run = canvas.getByRole("status", { name: "Reading the run" });
    await expect(within(run).getByText("Reproduction")).toBeVisible();
    await expect(canvas.getByText("Branch")).toBeVisible();
    await expect(canvas.queryByText("Reading this job.")).toBeNull();
  },
};

/** Landed. */
export const Landed: Story = { name: "Landed", render: drawing(completedSuccess) };

/** A held or escalated Job that a person closed as failed. */
export const Failed: Story = { name: "Failed", render: drawing(completedFailed) };

/** A person declined the work. That is a decision, and it is drawn as one. */
export const Rejected: Story = { name: "Rejected", render: drawing(rejected) };

/** A person stopped it. That is a decision, and it is drawn as one. */
export const Killed: Story = { name: "Killed", render: drawing(killed) };

/** Replaced by a redispatch that carries the work on as a new Job. */
export const Superseded: Story = { name: "Superseded", render: drawing(superseded) };

/**
 * The log, opened from its chapter's own control, the way a person opens it.
 * The control leaves the chapter once its sheet is open, which is what this
 * checks.
 */
export const LogOpen: Story = {
  name: "Log open",
  render: drawing(running),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /Open the log/ }));
    await expect(canvas.queryByRole("button", { name: /Open the log/ })).toBeNull();
  },
};

/** The Job's patch, opened from the Produced chapter. */
export const DiffOpen: Story = {
  name: "Diff open",
  render: drawing(running),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /Open the diff/ }));
  },
};

/** The log open at the narrowest window Bridge lays out for. */
export const LogOpenNarrow: Story = {
  name: "Log open, narrow window",
  render: () => <JobDetailFrom fixture={running()} width="var(--window-floor)" />,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /Open the log/ }));
  },
};

/** The log open on a Job a failed Check stopped. */
export const LogOpenStopped: Story = {
  name: "Log open, stopped",
  render: drawing(escalatedGateFailure),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /Open the log/ }));
  },
};

/** The failed Check's output, opened from the Checks chapter. */
export const CheckOutputOpen: Story = {
  name: "Check output open",
  render: drawing(escalatedGateFailure),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /Open the output/ }));
  },
};

/** Pulse, read in full: the Details control on its title line opens the sheet. */
export const FullReadingOpen: Story = {
  name: "Full reading open",
  render: drawing(running),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /^Details/ }));
  },
};

// Journey 9's run sheet — #624. `r` opens it, one sheet replaces another, and
// a server's link never navigates.

/**
 * This repository's own `armada.yml` — Setup, Checks and Commands as they are
 * declared at the root of this repository — read as the Job's frozen
 * Manifest. A story with an empty rail draws what shipping the wire and
 * never loading it into a fixture looks like on screen, so this is the real
 * file rather than a name invented for the story.
 */
const ARMADA_RUN_SHEET_READ = {
  state: "read" as const,
  jobId: JOB_ID,
  sheet: {
    job_id: JOB_ID,
    setup: [
      { name: "bootstrap", run: "pnpm install --frozen-lockfile", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "browsers", run: "pnpm -C packages/components exec playwright install chromium --only-shell", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
    ],
    checks: [
      { name: "build", run: "cargo build --workspace --locked", narrows: true, narrow_run: "cargo build --locked -p screens", requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "test", run: "cargo nextest run --workspace --exclude acceptance", narrows: true, narrow_run: "cargo nextest run -p screens", requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "typecheck", run: "pnpm typecheck", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "bridge_build", run: "pnpm -C apps/desktop build", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "storybook", run: "pnpm -C packages/components build-storybook", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "bridge_test", run: "pnpm bridge-test", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "format", run: "cargo fmt --all --check", narrows: true, narrow_run: "rustfmt --check --edition 2021 packages/screens/src/JobDetail.tsx", requires: ["fmt"], expect_exit_code: 0, destructive: false, frozen: true },
    ],
    commands: [
      { name: "fmt", run: "cargo fmt --all", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "gate", run: "cargo xtask verify-foundations", narrows: false, requires: [], expect_exit_code: 0, destructive: true, frozen: true },
    ],
    manifest_edited_at: "2026-09-08T16:40:00Z",
    worktree_on_disk: true,
    worktree_differs: false,
    drone_working: true,
  },
};

/** `r` opens the run sheet, nothing selected — Setup, Checks and Commands as
 * this repository's own `armada.yml` declares them. */
export const RunOpen: Story = {
  name: "Run sheet open",
  render: () => (
    <JobDetailFrom
      fixture={running()}
      on={{ rehearsal: { ...propsFor(running()).rehearsal, runSheet: ARMADA_RUN_SHEET_READ } }}
    />
  ),
  play: async ({ userEvent }) => {
    await userEvent.keyboard("r");
    const dialog = within(await within(document.body).findByRole("dialog", { name: "Run" }));
    await expect(dialog.findByText("bridge_test")).resolves.toBeVisible();
  },
};

/** Opening the run sheet while the log is open replaces it — one sheet at a time. */
export const RunReplacesLog: Story = {
  name: "Run sheet replaces the log",
  render: drawing(running),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /Open the log/ }));
    await expect(within(document.body).findByRole("dialog", { name: "Activity log" })).resolves.toBeVisible();

    await userEvent.keyboard("r");
    await expect(within(document.body).findByRole("dialog", { name: "Run" })).resolves.toBeVisible();
    expect(within(document.body).queryByRole("dialog", { name: "Activity log" })).toBeNull();
  },
};

/** A server this Job started, serving. Its link hands the address to the
 * system browser rather than navigating — the design system's hard rule. */
const SERVING: ServerState = {
  id: "srv-storybook",
  name: "storybook",
  job_id: propsFor(running()).job.id,
  phase: "serving",
  serve: "pnpm storybook",
  ports: [{ name: "PORT", port: 41207 }],
  links: [{ url: "http://localhost:41207", name: "Storybook" }],
  started_by: "person",
  started_at: "2026-09-10T18:00:00Z",
  serving_since: "2026-09-10T18:00:05Z",
  stopped: false,
  log: "run/storybook.log",
};

const openServerLink = fn(async () => ({ ok: true }) as const);

/** The sheet's own reading of the Job's frozen Manifest, naming the Check
 * `escalatedGateFailure` failed — `cargo_nextest` — so selecting it has a row
 * to select. */
const RUN_SHEET_READ = {
  state: "read" as const,
  jobId: JOB_ID,
  sheet: {
    job_id: JOB_ID,
    setup: [],
    checks: [
      {
        name: "cargo_nextest",
        run: "cargo nextest run --workspace",
        narrows: false,
        requires: [],
        expect_exit_code: 0,
        destructive: false,
        frozen: true,
      },
    ],
    commands: [],
    worktree_on_disk: true,
    worktree_differs: false,
    drone_working: false,
  },
};

/** A refused Check's `Run it here` opens the run sheet with that Check selected. */
export const RunItHere: Story = {
  name: "Run it here selects the Check",
  render: () => (
    <JobDetailFrom
      fixture={escalatedGateFailure()}
      on={{ rehearsal: { ...propsFor(escalatedGateFailure()).rehearsal, runSheet: RUN_SHEET_READ } }}
    />
  ),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: "Run it here" }));
    const dialog = within(await within(document.body).findByRole("dialog", { name: "Run" }));
    await expect(dialog.findByRole("button", { current: true })).resolves.toHaveTextContent("cargo_nextest");
  },
};

/** `test` running, streaming its output as it prints. */
export const RunStreaming: Story = {
  name: "Running test, streaming",
  render: () => (
    <JobDetailFrom
      fixture={escalatedGateFailure()}
      on={{
        rehearsal: {
          ...propsFor(escalatedGateFailure()).rehearsal,
          runSheet: {
            ...RUN_SHEET_READ,
            sheet: {
              ...RUN_SHEET_READ.sheet,
              running: {
                id: "run-1",
                job_id: JOB_ID,
                name: "cargo_nextest",
                command: "cargo nextest run --workspace",
                narrowed: false,
                started_at: "2026-09-11T14:05:00Z",
              },
            },
          },
          runFollowed: {
            state: "following",
            jobId: JOB_ID,
            runId: "run-1",
            name: "cargo_nextest",
            path: ".armada/runs/run-1/output.log",
            fromLine: 1,
            lines: ["running 2034 tests", "test settings::selectors::visible_manifests_memoises ... FAIL"],
          },
        },
      }}
    />
  ),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: "Run it here" }));
    await expect(
      within(await within(document.body).findByRole("dialog", { name: "Run" })).findByText(/FAIL/),
    ).resolves.toBeVisible();
  },
};

export const ServingRow: Story = {
  name: "Serving row, link opens the system browser",
  render: () => (
    <JobDetailFrom
      fixture={running()}
      on={{
        rehearsal: {
          ...propsFor(running()).rehearsal,
          servers: { servers: [SERVING] },
          onOpenServerLink: openServerLink,
        },
      }}
    />
  ),
  play: async ({ canvas, userEvent }) => {
    openServerLink.mockClear();
    const link = await canvas.findByRole("button", { name: "Storybook" });
    await userEvent.click(link);
    await expect(openServerLink).toHaveBeenCalledWith(SERVING.id, SERVING.links[0]!.url);
  },
};
