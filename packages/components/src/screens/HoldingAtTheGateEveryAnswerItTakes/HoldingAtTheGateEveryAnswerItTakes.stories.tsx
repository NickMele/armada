import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { Button } from "../../primitives/Button/Button";
import { OnAScreen, WHERE, BRIEF } from "../InsideAJobOneArrangementAtEveryState/fixtures";
import { HoldingAtTheGate } from "./HoldingAtTheGateEveryAnswerItTakes";
import {
  GATE_CHAPTERS,
  GATE_HEADING,
  PHASES_DELIVERED,
  PLAN_BRIEF,
  PLAN_CHAPTERS,
  PLAN_HEADING,
  PLAN_PHASES,
  PLAN_RUN,
  PLAN_WHERE,
  REMARKS,
  RUN_AT_THE_GATE,
} from "./gate";

/**
 * **The end of a Job, which is the state every finished Job passes through and
 * the one nothing on a sheet could show.** Reaching it meant driving a real Job
 * to an open pull request — minutes of work and a forge — every time somebody
 * wanted to look at a button.
 *
 * Four moments, and one fact separates the first three from the fourth: whether
 * there is a pull request. With one, the fourth answer appears, the accent fill
 * moves from Approve to Merge, and what people wrote on it is drawn under the
 * decision. Without one, all three of those are absent rather than pending —
 * `design-plan` and `epic` declare no delivering step, so nothing was ever
 * pushed and there is nothing anybody could have commented on.
 *
 * **The arrangement is the sheet next door's, unchanged.** Nothing about a gate
 * moves a region: the run is still on the left, the step is still the panel,
 * and the decision is the block after the story rather than a second surface.
 * Review and reply are one loop, which is the v1 failure Bridge exists to
 * escape and the one constraint that survives every redraw.
 */
const meta: Meta<typeof HoldingAtTheGate> = {
  title: "Screens/Holding at the gate — every answer it takes",
  component: HoldingAtTheGate,
  decorators: [(Story) => <OnAScreen><Story /></OnAScreen>],
};
export default meta;

type Story = StoryObj<typeof HoldingAtTheGate>;

const nothingPressedYet = () => {};

/**
 * The Job-level acts on a Job that is holding but not over: a Drone to kill and
 * a Job to kill, each its own quiet button.
 *
 * **Neither is a lead.** `Acts.tsx` gives this render no split button because a
 * lead is never destructive and the non-destructive one — *Review* — is the
 * decision block under the story rather than a header control.
 */
const JOB_ACTS = (
  <>
    <Button variant="secondary">Kill drone</Button>
    <Button variant="secondary">Kill job</Button>
  </>
);

/**
 * The step's own short facts. **Two, because `fieldsOf` builds two** — how long
 * the step has been in this state and which run this is. How long you have had
 * it is on the tree's `Waiting` row and is not restated here.
 */
const GATE_FIELDS = [
  { label: "Took", value: "3m 41s", mono: true },
  { label: "Attempt", value: "1", mono: true },
];

/** The band above the story, in `noticeOf`'s own words for the reviewing render. */
const AT_THE_GATE = {
  tone: "waiting" as const,
  title: "This Job needs your review before it can go on.",
  children: "Every step passed its gates. Nothing advances until you answer.",
};

/**
 * **A Job holding with its pull request open, and the fourth answer is what
 * this state is for.** Merge sits in the group beside Approve and takes the
 * primary fill off it, because a Job with a pull request open has one ordinary
 * ending and it is not *record this done and leave the branch on the forge*.
 *
 * **Read the block bottom-up and it is three costs in order.** Merge and
 * Approve both take the work and differ by what happens to the branch; Request
 * changes keeps the drone, the worktree and the step and is off until the note
 * has something in it, which is what Fleet would answer anyway; Reject is under
 * a rule, alone, because it is the only one of the four that leaves nothing
 * behind. The rule is load-bearing — it is what says the control under it is
 * not a fifth answer in the group.
 *
 * **The note is on the surface and never behind a control.** Reviewing and
 * replying are one interaction, so the field is already open where the diff is,
 * with nothing to press to reach it.
 *
 * **And the comments are under it, in the same block.** A reviewer's comment
 * and the answer to it are the same loop; a second surface for them would be a
 * place to forget they exist.
 */
export const WithAPullRequest: Story = {
  render: () => (
    <HoldingAtTheGate
      heading={{ ...GATE_HEADING, actions: JOB_ACTS }}
      run={RUN_AT_THE_GATE}
      runElapsed="34m 18s"
      where={WHERE}
      brief={BRIEF}
      onCopied={nothingPressedYet}
      step={{
        label: "Land",
        fields: GATE_FIELDS,
        acts: <Button variant="secondary">Restart step</Button>,
        notice: AT_THE_GATE,
        phases: PHASES_DELIVERED,
        chapters: GATE_CHAPTERS,
      }}
      answers={{ onMerge: nothingPressedYet, comments: REMARKS }}
    />
  ),
  /**
   * **Which control carries the accent is the whole claim of this state**, and
   * a rendering cannot say it: primary and secondary differ by a fill, and a
   * screenshot of the wrong one looks exactly like a screenshot of the right
   * one to anything that is not a person looking at it.
   *
   * The assertion is on the accessible names and their order, not on a class:
   * Merge is drawn, it leads the group, and Reject is outside it. Checked once
   * against `onMerge` removed, where the first line fails.
   */
  play: async ({ canvas }) => {
    const merge = canvas.getByRole("button", { name: "Merge and take the work" });
    const approve = canvas.getByRole("button", { name: "Approve the work" });
    await expect(merge.compareDocumentPosition(approve)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    // Off until the note has something in it, which is what Fleet answers.
    await expect(canvas.getByRole("button", { name: "Request changes" })).toBeDisabled();
    // The comments are in the same block, not behind anything.
    await expect(canvas.getByRole("button", { name: "Send to a drone" })).toBeVisible();
  },
};

/**
 * **The one act in Armada that writes into a repository Armada does not own.**
 * Everything else a press on this screen does is confined to a worktree Fleet
 * cut; this changes what everybody else builds on, so it is the only answer
 * here besides Reject that costs a second press.
 *
 * **Neutral and not destructive.** Nothing ends and nothing is destroyed, and
 * the confirm keeps the accent fill the control a person just pressed had — a
 * red confirm would make a merge read as an error state on its way in.
 *
 * **It states the mechanism and promises no outcome.** Where the commits land,
 * that Armada runs the repository's after-merge checks against what landed —
 * which is the whole reason this button exists rather than merging on the forge
 * — and, plainly, that Bridge cannot take it back, because there is no undo to
 * offer and a dialog implying one would be worse than the sentence.
 *
 * **It does not ask the title again in the body.** The button already said it.
 */
export const TheMergeConfirmation: Story = {
  render: () => (
    <HoldingAtTheGate
      heading={{ ...GATE_HEADING, actions: JOB_ACTS }}
      run={RUN_AT_THE_GATE}
      runElapsed="34m 18s"
      where={WHERE}
      brief={BRIEF}
      onCopied={nothingPressedYet}
      step={{
        label: "Land",
        fields: GATE_FIELDS,
        notice: AT_THE_GATE,
        phases: PHASES_DELIVERED,
        chapters: GATE_CHAPTERS,
      }}
      answers={{ onMerge: nothingPressedYet, comments: REMARKS, confirming: "merge" }}
    />
  ),
  /**
   * Cancel holds initial focus, and `Enter` fires whatever holds focus — so on
   * this layer `Enter` cancels. That is the keyboard contract, and it is the
   * property worth asserting on the one dialog here that writes outside the
   * machine: a regression where `Enter` reached past Cancel merges a pull
   * request somebody was declining to merge, and it renders identically.
   */
  play: async ({ canvas }) => {
    const layer = within(canvas.getByRole("dialog"));
    await expect(layer.getByRole("button", { name: "Cancel" })).toHaveFocus();
    await expect(
      canvas.getByText(/Bridge cannot take a merge back/),
    ).toBeVisible();
  },
};

/**
 * **Picking is the whole reason this is a surface and not a payload.** Not
 * every comment on a pull request is a change request — one of these three is
 * somebody asking about the density setting, and a drone handed all three would
 * go and answer it. So nothing is preselected and the send is off until
 * something is.
 *
 * **A comment already sent is drawn and cannot be picked.** The forge has no
 * memory of what Armada did, so it reads the same forever; hiding it would
 * leave a person looking at a review with holes in it, and offering it would be
 * offering a press Fleet refuses.
 *
 * **What goes back is the handles and never the words.** Every word here was
 * written by whoever can see the pull request, and it is drawn as a text node
 * and as nothing else — no markdown rendered, no link made clickable.
 */
export const CommentsPickedUp: Story = {
  render: () => (
    <HoldingAtTheGate
      heading={{ ...GATE_HEADING, actions: JOB_ACTS }}
      run={RUN_AT_THE_GATE}
      runElapsed="34m 18s"
      where={WHERE}
      brief={BRIEF}
      onCopied={nothingPressedYet}
      step={{
        label: "Land",
        fields: GATE_FIELDS,
        notice: AT_THE_GATE,
        phases: PHASES_DELIVERED,
        chapters: GATE_CHAPTERS,
      }}
      answers={{ onMerge: nothingPressedYet, comments: REMARKS }}
    />
  ),
  /**
   * The state a rendering cannot hold: two comments picked and the third left
   * alone, with the send now live. **The one already sent has no control at
   * all** — asserted as absent rather than as disabled, because a checkbox a
   * person can tick and Fleet then refuses is the failure this shape avoids.
   */
  play: async ({ canvas, userEvent }) => {
    const remarks = within(canvas.getByRole("region", { name: "Comments on the pull request" }));
    const picks = remarks.getAllByRole("checkbox", { name: "Act on this" });
    // Three choosable and four drawn: the fourth is marked sent and offers none.
    await expect(picks).toHaveLength(3);
    await expect(remarks.getByText("Already sent to a drone")).toBeVisible();

    const send = remarks.getByRole("button", { name: "Send to a drone" });
    await expect(send).toBeDisabled();
    await userEvent.click(picks[0]!);
    await userEvent.click(picks[1]!);
    await expect(send).toBeEnabled();
    // The third stays alone, which is the whole point of picking.
    await expect(picks[2]!).not.toBeChecked();
  },
};

/**
 * **A gate with nothing to merge, and the absences are the design.** This is a
 * Design Plan Job holding at `present` on its second pass — two steps, neither
 * of which delivers, so no branch was ever pushed.
 *
 * **Three things are not on this screen and none of them is drawn pending.**
 * There is no `Pull request` fact in the header, because `facts.ts` draws one
 * only where Fleet opened one. There is no Merge control, because `pullRequest`
 * absent is the whole of what decides it — an act a person can press to be
 * refused is a worse surface than one that is not drawn. And there is no
 * comments block, because there is nothing anybody could have commented on;
 * *Nobody has commented on this pull request* would be a sentence about a pull
 * request that does not exist.
 *
 * **So Approve takes the primary fill back**, which is the other half of the
 * same rule. A Job with no pull request has one ordinary ending and it is
 * Approve.
 *
 * **The step gates on nothing and the strip says so in words.** Design Plan
 * carries no Judge at all — the back-and-forth is what a person ends — and
 * `present` declares no Check either, so the tiers are absent rather than
 * greyed out and the note under the strip is what advances the step.
 */
export const WithNoPullRequest: Story = {
  render: () => (
    <HoldingAtTheGate
      heading={{ ...PLAN_HEADING, actions: JOB_ACTS }}
      run={PLAN_RUN}
      runElapsed="22m 06s"
      where={PLAN_WHERE}
      brief={PLAN_BRIEF}
      onCopied={nothingPressedYet}
      step={{
        label: "Present",
        // `fieldsOf`'s two. **The iteration is not one of them and is not
        // added here** — a loop's pass number is on this step's tree row and on
        // no panel field, which is a gap in the product rather than one to
        // paper over in a fixture. Reported.
        fields: [
          { label: "Took", value: "4m 55s", mono: true },
          { label: "Attempt", value: "1", mono: true },
        ],
        notice: AT_THE_GATE,
        phases: PLAN_PHASES,
        chapters: PLAN_CHAPTERS,
      }}
      answers={{}}
    />
  ),
  /**
   * Three absences and one promotion, and a rendering can show none of them:
   * an absent control and a control drawn somewhere off-screen look the same,
   * and which button carries the accent is a fill.
   *
   * Checked against `answers={{ onMerge, comments: REMARKS }}`, where every
   * line fails.
   */
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button", { name: "Merge and take the work" })).toBeNull();
    await expect(canvas.queryByRole("region", { name: "Comments on the pull request" })).toBeNull();
    await expect(canvas.queryByText("Pull request")).toBeNull();
    // Approve is the group's lead again, which is the other half of the rule.
    const approve = canvas.getByRole("button", { name: "Approve the work" });
    const changes = canvas.getByRole("button", { name: "Request changes" });
    await expect(approve.compareDocumentPosition(changes)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
  },
};
