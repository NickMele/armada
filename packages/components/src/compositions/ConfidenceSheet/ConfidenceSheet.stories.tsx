import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { ConfidenceSheet } from "./ConfidenceSheet";

/**
 * Armada's review, drawn above the verdict sheet at the gate. #903. The verdict and Needs you
 * are always open; the rest folds, and Tests in the change opens itself on a removed test.
 */
const meta: Meta<typeof ConfidenceSheet> = {
  title: "Compositions/Confidence sheet",
  component: ConfidenceSheet,
};
export default meta;

type Story = StoryObj<typeof ConfidenceSheet>;

const REMOVED = "a_loaded_machine_holds_a_job_back_too";

/** A confident review, with one change in behaviour for the person and a test removed without a reason. */
export const ARemovedTest: Story = {
  args: {
    confidence: {
      says: "confident",
      reasons: [
        "Every Check passed and the Judge met every criterion.",
        "One change in behaviour needs your yes before it lands.",
      ],
      areas: [
        {
          name: "Admission",
          what: "CPU no longer holds a Job",
          files: ["crates/fleet/src/headroom.rs", "crates/fleet/src/admitting.rs"],
        },
        {
          name: "Tests",
          what: "The CPU hold's test asserts the opposite",
          files: ["crates/fleet/src/tests/headroom.rs"],
        },
      ],
      tests: {
        opened_because: { kind: "test_removed", name: REMOVED },
        proves: [{ area: "Admission", what: "A saturated CPU holds nothing back", tests: 2 }],
        changed: [
          {
            name: REMOVED,
            change: "removed",
            replaced_by: "a_machine_whose_cpu_is_saturated_still_admits",
            flagged: true,
          },
        ],
        untested: [
          { code: "Opening Fleet settings from the status bar", why: "No test opens the sheet from it" },
        ],
      },
      needs_you: [
        {
          finding: `\`${REMOVED}\` was removed with no reason given`,
          why: "A test taken out or weakened is the reviewer's to explain",
        },
        { finding: "A busy CPU no longer delays a Job", why: "People may rely on the old behaviour" },
      ],
      small_fixes: [],
      for_context: [
        { finding: "The pull request asks for a check of the lock order when saving", why: "The author flagged it" },
      ],
    },
  },
  play: async ({ canvas, userEvent }) => {
    // The verdict is the heading; no label repeats it above.
    await expect(canvas.queryByText("Armada's review")).toBeNull();
    const callout = canvas.getByRole("note");
    await expect(callout).toHaveTextContent("A test was removed");
    await expect(callout).toHaveTextContent(REMOVED);
    await expect(canvas.getByRole("button", { name: /Tests in the change/ })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    const shape = canvas.getByRole("button", { name: /Shape of the change/ });
    await expect(shape).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(shape);
    await expect(shape).toHaveAttribute("aria-expanded", "true");
  },
};

/** Confident, with nothing for the person: Needs you says so, and the empty sections are left out. */
export const NothingNeedsYou: Story = {
  args: {
    confidence: {
      says: "confident",
      reasons: ["It only adds code: one new route and one new message."],
      areas: [{ name: "New code", what: "The repository scan", files: ["crates/fleet/src/scanning.rs"] }],
      needs_you: [],
      small_fixes: [],
      for_context: [],
    },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Nothing needs you.")).toBeVisible();
    await expect(canvas.queryByRole("button", { name: /Small fixes for a Drone/ })).toBeNull();
    await expect(canvas.queryByRole("note")).toBeNull();
  },
};

/** Not confident, with a small fix a Drone can make. */
export const NotConfident: Story = {
  args: {
    confidence: {
      says: "not_confident",
      reasons: ["The change reaches every repository that uses the workflows, and it changes a database trigger."],
      areas: [
        { name: "Engine", what: "Walking back through a finished step", files: ["crates/fleet/src/dispatch.rs"] },
      ],
      needs_you: [
        { finding: "Undoing the trigger change needs a second migration", why: "Reverting the code does not undo it" },
      ],
      small_fixes: [
        { finding: "The store module is over 500 lines", why: "In scope. Move the migration into its own file." },
      ],
      for_context: [],
    },
  },
};

/** A finding and an area with a View. Only the rows that have one carry the button. #904. */
export const WithAView: Story = {
  args: {
    onView: fn(),
    confidence: {
      says: "confident",
      reasons: ["One change in behaviour needs your yes before it lands."],
      areas: [
        {
          name: "Admission",
          what: "CPU no longer holds a Job",
          files: ["crates/fleet/src/headroom.rs"],
          view: [
            {
              file: "crates/fleet/src/headroom.rs",
              hunk: "@@ -40,6 +40,4 @@",
              summary: "Removes CPU as a way to be short of room.",
            },
          ],
        },
        { name: "Docs", what: "Doctor says why", files: ["docs/concepts/doctor.md"] },
      ],
      needs_you: [
        {
          finding: "A busy CPU no longer delays a Job",
          why: "People may rely on the old behaviour",
          view: [
            {
              file: "crates/fleet/src/admitting.rs",
              hunk: "@@ -212,7 +212,6 @@",
              summary: "The hold goes.",
            },
          ],
        },
        { finding: "The status bar says nothing about it", why: "A person would look there first" },
      ],
      small_fixes: [],
      for_context: [],
    },
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: /Shape of the change/ }));
    const views = canvas.getAllByRole("button", { name: "View" });
    await expect(views).toHaveLength(2);
    await userEvent.click(views[1]!);
    await expect(args.onView).toHaveBeenCalledWith({
      title: "A busy CPU no longer delays a Job",
      finding: "A busy CPU no longer delays a Job",
      steps: [
        {
          file: "crates/fleet/src/admitting.rs",
          hunk: "@@ -212,7 +212,6 @@",
          summary: "The hold goes.",
        },
      ],
    });
  },
};

/** A finding a person dismissed is off the lists, and kept with its reason under Dismissed. #907. */
export const WithADismissal: Story = {
  args: {
    confidence: {
      says: "confident",
      reasons: ["It only moves the lock into the store."],
      areas: [{ name: "Store", what: "Saving takes one lock", files: ["crates/store/src/open.rs"] }],
      needs_you: [],
      small_fixes: [],
      for_context: [],
      dismissed: [
        {
          finding: "The lock order when saving",
          reason: "Saving takes one lock, so there is no order.",
        },
      ],
    },
  },
  play: async ({ canvas, userEvent }) => {
    const fold = canvas.getByRole("button", { name: /Dismissed/ });
    await expect(fold).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(fold);
    await expect(canvas.getByText("Saving takes one lock, so there is no order.")).toBeVisible();
  },
};

/**
 * A pull request whose CI failed: What the verdict rests on opens itself on that row, with
 * Investigate under it and Re-run the failed runs under the caret. #905.
 *
 * **No callout above the table.** The row it would have named is right there once the
 * section is open, which the auto-open already does.
 */
export const WithFailedCi: Story = {
  args: {
    confidence: {
      says: "not_confident",
      reasons: ["The unit tests failed on the pull request."],
      areas: [],
      needs_you: [],
      small_fixes: [],
      for_context: [],
    },
    grounds: {
      checks: { result: "7 of 7 passed", tone: "met", detail: "build, format, tests" },
      judge: { result: "3 of 3 criteria met", tone: "met" },
      evidence: { result: "Within scope", tone: "met" },
      plan: { result: "1 file outside the plan", tone: "quiet", detail: "`setup.rs`" },
    },
    ci: {
      kind: "some_failed",
      said: "1 of 3 failed",
      failed: ["unit tests"],
      conflicted: false,
      onInvestigate: fn(),
      onRerun: fn(),
    },
  },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("button", { name: /What the verdict rests on/ })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    // The section opened itself on the failing row; no callout repeats it.
    await expect(canvas.queryByRole("note")).toBeNull();
    const row = canvas.getByRole("row", { name: /Pull request CI/ });
    await expect(row).toHaveTextContent("unit tests");
    await userEvent.click(canvas.getByRole("button", { name: "Investigate" }));
    await expect(args.ci?.onInvestigate).toHaveBeenCalled();
    await userEvent.click(canvas.getByRole("button", { name: "More about CI" }));
    await userEvent.click(await canvas.findByRole("menuitem", { name: "Re-run the failed runs" }));
    await expect(args.ci?.onRerun).toHaveBeenCalled();
  },
};

/**
 * A pull request that conflicts with main: the row names the conflict and
 * draws no act, `#1131`. Fleet sends a Drone to clear it on its own — there
 * is nothing here to press, which is what this guards.
 */
export const WithConflictedPullRequest: Story = {
  args: {
    confidence: {
      says: "not_confident",
      reasons: ["The branch conflicts with main."],
      areas: [],
      needs_you: [],
      small_fixes: [],
      for_context: [],
    },
    grounds: {
      checks: { result: "7 of 7 passed", tone: "met", detail: "build, format, tests" },
      judge: { result: "3 of 3 criteria met", tone: "met" },
      evidence: { result: "Within scope", tone: "met" },
    },
    ci: {
      kind: "unreadable",
      said: "Conflicts with main",
      failed: [],
      conflicted: true,
    },
  },
  play: async ({ canvas }) => {
    const row = canvas.getByRole("row", { name: /Pull request CI/ });
    await expect(row).toHaveTextContent("Conflicts with main");
    await expect(canvas.queryByRole("button", { name: "Resolve conflicts" })).toBeNull();
  },
};

/**
 * Everything the verdict rests on held, so the section stays folded and says so in one line.
 * What the Job captured opens on the frames and the Drone's claim.
 */
export const WhatTheJobCaptured: Story = {
  args: {
    confidence: {
      says: "confident",
      reasons: ["Every Check passed and the Judge met every criterion."],
      areas: [],
      needs_you: [],
      small_fixes: [],
      for_context: [],
    },
    grounds: {
      checks: { result: "7 of 7 passed", tone: "met", detail: "build, format, tests" },
      judge: { result: "4 of 4 criteria met", tone: "met" },
      evidence: { result: "Within scope", tone: "met" },
      plan: { result: "Inside the plan", tone: "met" },
    },
    ci: { kind: "all_passed", said: "4 of 4 passed", failed: [], conflicted: false },
    captured: {
      frames: [{ kept: "k1", name: "fleet-settings.png", attempt: 1, weight: "84 KB" }],
      claim: {
        claimed: "Fleet settings opens from the Board's menu and saves a limit.",
        shownBy: "`fleet-settings.png`",
        notClaimed: "The status bar's own way in.",
      },
    },
  },
  play: async ({ canvas, userEvent }) => {
    const rests = canvas.getByRole("button", { name: /What the verdict rests on/ });
    await expect(rests).toHaveAttribute("aria-expanded", "false");
    await expect(rests).toHaveTextContent(
      "Checks 7 of 7 passed · CI 4 of 4 passed · Judge 4 of 4 criteria met · Evidence within scope · Plan inside the plan",
    );
    await expect(canvas.queryByRole("note")).toBeNull();
    const captured = canvas.getByRole("button", { name: /What the Job captured/ });
    await expect(captured).toHaveTextContent("1 captured · the Drone's claim");
    await userEvent.click(captured);
    await expect(canvas.getByText("Fleet settings opens from the Board's menu and saves a limit.")).toBeVisible();
    await expect(canvas.getByText("The status bar's own way in.")).toBeVisible();
  },
};

/** For context findings a person can follow up: one already queued, one still open. #906. */
export const WithFollowUps: Story = {
  args: {
    confidence: {
      says: "confident",
      reasons: ["Every Check passed."],
      areas: [],
      needs_you: [],
      small_fixes: [],
      for_context: [
        {
          finding: "`headroom.rs` has grown past what one reader holds",
          why: "The author flagged it for later.",
        },
        { finding: "The retry count is a guess", why: "Nothing measured it." },
      ],
      followed: [{ finding: "The retry count is a guess", job: "01J9Z3M5X8Q2V7R4T6W1Y0ZABC" }],
    },
    followUp: { onQueueAfter: fn(), onFileIssue: fn() },
  },
  play: async ({ args, canvas, userEvent }) => {
    const open = "`headroom.rs` has grown past what one reader holds";
    await userEvent.click(canvas.getByRole("button", { name: /For context/ }));
    await expect(canvas.getByText("Job queued. It starts when this one lands.")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Queue after this lands" }));
    await expect(args.followUp?.onQueueAfter).toHaveBeenCalledWith(open);
    await userEvent.click(canvas.getByRole("button", { name: "More ways to follow up" }));
    await userEvent.click(await canvas.findByRole("menuitem", { name: "File an issue" }));
    await expect(args.followUp?.onFileIssue).toHaveBeenCalledWith(open);
  },
};
