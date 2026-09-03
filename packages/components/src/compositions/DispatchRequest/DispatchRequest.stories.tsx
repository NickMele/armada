import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { DispatchRequest } from "./DispatchRequest";
import type { Proposal } from "./DispatchRequest";

/**
 * Dispatching by describing the work, which is the path — the form behind
 * `Enter by hand` is the override.
 *
 * The complaint this answers, in the owner's words: *"I hate having to come up
 * with a title and brief and determine the workflow for a set of work."* Three
 * fields, and the Job proposer answers all three off one reading.
 *
 * **Every state here is one a person meets.** Nothing typed, the wait, a
 * proposal of one, a proposal of several with the order between them, and each
 * of the two refusals. There is no partial proposal, because the call is asked
 * once and answers once — see the component's own note on why no skeleton is
 * drawn for the wait.
 */
const meta: Meta<typeof DispatchRequest> = {
  title: "Compositions/Dispatch request",
  component: DispatchRequest,
  args: {
    request: "",
    onRequest: fn(),
    onDispatch: fn(),
    onEnterByHand: fn(),
    onReset: fn(),
    onOpen: fn(),
    onStop: fn(),
    proposal: { at: "unasked" } satisfies Proposal,
  },
};
export default meta;

type Story = StoryObj<typeof DispatchRequest>;

/** The request one of these stories was written from, kept in one place. */
const REQUEST =
  "The board flickers every time an event lands. Find out why and stop it — it has been " +
  "doing it since the resync change.";

/**
 * Nothing typed. The one control that spends money is off, and the field says
 * what it takes: prose, or a link.
 *
 * **The button is off rather than absent.** A control that appears once the
 * field is filled teaches nothing about what the surface is for; one that is
 * visibly off says a request is what it is waiting for.
 */
export const NothingTyped: Story = {
  /**
   * Typed into, it comes alive, and dispatching sends exactly once. A rendering
   * shows the button greyed; only a press shows that nothing went out.
   */
  play: async ({ args, canvas, userEvent }) => {
    const dispatch = canvas.getByRole("button", { name: "Dispatch" });
    await expect(dispatch).toBeDisabled();

    await userEvent.click(dispatch);
    await expect(args.onDispatch).not.toHaveBeenCalled();

    await userEvent.type(canvas.getByRole("textbox", { name: "Request" }), "Fix the flicker");
    await expect(args.onRequest).toHaveBeenCalled();
  },
};

/**
 * Typed, and ready. The same state as above with a request in it — the control
 * this surface exists for is live.
 */
export const Typed: Story = {
  args: { request: REQUEST },
  play: async ({ args, canvas, userEvent }) => {
    const dispatch = canvas.getByRole("button", { name: "Dispatch" });
    await expect(dispatch).toBeEnabled();
    await userEvent.click(dispatch);
    await expect(args.onDispatch).toHaveBeenCalledOnce();
  },
};

/**
 * The wait, drawn honestly.
 *
 * `job-proposer.md` says the proposal is "visible filling in as it is worked
 * out". **What shipped is one request and one response**, with no stream, so
 * nothing here fills in and nothing pretends to. The control takes the
 * present-participle label every in-flight act in Bridge takes — `Approving`,
 * `Reading the request` — and the field goes inert so a second request cannot
 * be typed over one already sent.
 */
export const Reading: Story = {
  args: { request: REQUEST, proposal: { at: "reading" } },
  /**
   * Inert on every path out, which is the thing a rendering cannot show: the
   * field, the primary and the override are all dead while a call is out.
   */
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("textbox", { name: "Request" })).toBeDisabled();
    await expect(canvas.getByRole("button", { name: "Enter by hand" })).toBeDisabled();

    const dispatch = canvas.getByRole("button", { name: "Reading the request" });
    await expect(dispatch).toBeDisabled();
    await userEvent.click(dispatch);
    await expect(args.onDispatch).not.toHaveBeenCalled();
  },
};

/**
 * The call has reached the vendor and is thinking. **What a wait is for**: the
 * reach, the elapsed figure against Fleet's ceiling, and how much thinking
 * there has been — none of which an elapsed count alone can say.
 *
 * Well inside `slowAfterMs`, so no question is asked and no stop is offered.
 * Waiting is what should happen here and the surface says nothing else.
 */
export const ReadingWithProgress: Story = {
  args: {
    request: REQUEST,
    proposal: {
      at: "reading",
      watch: {
        reached: "thinking",
        elapsedMs: 41_000,
        budgetMs: 600_000,
        model: "haiku",
        thinkingTokens: 763,
      },
    },
    slowAfterMs: 120_000,
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("The model is thinking")).toBeVisible();
    await expect(canvas.getByText("41s")).toBeVisible();
    // No question and no stop: the wait is ordinary.
    await expect(canvas.queryByRole("button", { name: "Stop the proposer" })).toBeNull();
  },
};

/**
 * Past the mark, and the surface asks.
 *
 * **The stop is the only control offered.** Waiting is what happens if nothing
 * is pressed, so a `Keep waiting` button would perform no act — and dismissing
 * the notice would hide the one way out of the wait.
 */
export const ReadingAndSlow: Story = {
  args: {
    request: REQUEST,
    proposal: {
      at: "reading",
      watch: {
        reached: "thinking",
        elapsedMs: 142_000,
        budgetMs: 600_000,
        model: "haiku",
        thinkingTokens: 4_210,
      },
    },
    slowAfterMs: 120_000,
  },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByText(/taking longer than expected/)).toBeVisible();
    await expect(canvas.getByText("2m 22s")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Stop the proposer" }));
    await expect(args.onStop).toHaveBeenCalled();
  },
};

/**
 * **The case worth telling apart from every other.** Two minutes in and the
 * harness has still not announced itself, so the call never reached the vendor
 * at all — a credential or a harness problem, which will not resolve by
 * waiting. Under an elapsed count alone this is indistinguishable from a model
 * thinking hard, and the two take opposite decisions.
 */
export const ReadingAndStuckStarting: Story = {
  args: {
    request: REQUEST,
    proposal: {
      at: "reading",
      watch: {
        reached: "starting",
        elapsedMs: 130_000,
        budgetMs: 600_000,
        model: "haiku",
      },
    },
    slowAfterMs: 120_000,
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Starting the proposer")).toBeVisible();
    // Nothing has been thought and nothing answered, so neither count is drawn.
    // Absent rather than zeroed: `0 tokens` would read as a model that thought
    // about nothing, which is a different and much less alarming fact.
    await expect(canvas.queryByText(/tokens of thinking/)).toBeNull();
    await expect(canvas.getByRole("button", { name: "Stop the proposer" })).toBeVisible();
  },
};

/**
 * The answer is arriving. **Nearly over** — stopping here would throw away work
 * about to land, which is what the reach is for.
 */
export const ReadingAndAnswering: Story = {
  args: {
    request: REQUEST,
    proposal: {
      at: "reading",
      watch: {
        reached: "answering",
        elapsedMs: 88_000,
        budgetMs: 600_000,
        model: "haiku",
        thinkingTokens: 2_100,
        answeredCharacters: 340,
      },
    },
    slowAfterMs: 120_000,
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("The answer is arriving")).toBeVisible();
    await expect(canvas.getByText(/340 characters of answer/)).toBeVisible();
  },
};

/**
 * One job, which is the ordinary case. It exists already, at
 * `awaiting_approval` — the badge is the same one the Job Board draws — and
 * approving it is what starts the work.
 *
 * **No file is named, and the line under it says so.** A job reaches this gate
 * with `write_targets` null, which is scope not yet determined rather than a
 * claim that it writes nothing. Naming paths credibly needs the repository, and
 * the proposer has not read it.
 */
export const OneJob: Story = {
  args: {
    proposal: {
      at: "proposed",
      request: REQUEST,
      jobs: [{ id: "job_2d90bb", title: "Stop the board flickering on every event", workflow: "bug", status: "awaiting_approval" }],
    },
  },
  /**
   * The row opens the job. **It does not approve it** — nothing on a list
   * approves, and a control here that dispatched would be a second gate over a
   * proposal the first one already holds. A rendering cannot show which of the
   * two a press does.
   */
  play: async ({ args, canvas, userEvent }) => {
    await expect(
      canvas.queryByRole("button", { name: /approve/i }),
    ).toBeNull();

    await userEvent.click(
      canvas.getByRole("button", { name: "Review Stop the board flickering on every event" }),
    );
    await expect(args.onOpen).toHaveBeenCalledWith("job_2d90bb");
  },
};

/**
 * Several, and the order between them.
 *
 * **The order is the whole of the graph.** A proposal of several is a chain —
 * each member waits on the one before it reaching `completed_success` — so
 * position carries it and no second field restates it.
 *
 * **Nothing here approves all three.** Fleet's rule is strictly one by one, so
 * each job takes its own approval when its turn comes, and the line under the
 * list is what says so before anybody looks for a control that is not there.
 */
export const SeveralJobs: Story = {
  args: {
    proposal: {
      at: "proposed",
      request:
        "Move the runtime file to its own crate and make Bridge verify the pid before it " +
        "connects, so an unreachable Fleet stops reading as a missing one.",
      jobs: [
        { id: "job_11a0", title: "Move the runtime file into its own crate", workflow: "refactor", status: "awaiting_approval" },
        { id: "job_11a1", title: "Verify the pid before connecting", workflow: "feature", status: "awaiting_approval" },
        { id: "job_11a2", title: "Tell an unreachable Fleet from a missing one", workflow: "feature", status: "awaiting_approval" },
      ],
    },
  },
  /**
   * The third row opens the third job. An implementation keyed on an index
   * rather than the row's own id looks identical here and sends the wrong id
   * the moment Fleet reorders anything.
   */
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(
      canvas.getByRole("button", { name: "Review Tell an unreachable Fleet from a missing one" }),
    );
    await expect(args.onOpen).toHaveBeenCalledWith("job_11a2");
  },
};

/**
 * Refusal one: no workflow resolved.
 *
 * **Armada working, not Armada failing.** Fleet read the request, could not
 * resolve a workflow and returned the request unchanged; no job was created.
 * So it takes no red, no code chip and no solid fill — the rule on the left is
 * `--step-waiting`, which means needs you and not urgent.
 *
 * **Nothing is assigned by default**, and the copy says why: the resolved
 * definition is frozen into the job at creation and becomes the yardstick the
 * work is judged against, so a default would be the standard a drone is held
 * to rather than a guess somebody could correct.
 *
 * The two ways on are both here: the request is still in the field, and
 * `Enter by hand` is the override it always was.
 */
export const NoWorkflowResolved: Story = {
  args: { request: REQUEST, proposal: { at: "unresolved" } },
  play: async ({ args, canvas, userEvent }) => {
    // The request came back unchanged, which is the whole claim of this refusal.
    await expect(canvas.getByRole("textbox", { name: "Request" })).toHaveValue(REQUEST);

    await userEvent.click(canvas.getByRole("button", { name: "Enter by hand" }));
    await expect(args.onEnterByHand).toHaveBeenCalledOnce();
  },
};

/**
 * Refusal two: the call could not be made.
 *
 * **Armada failing, so it is the error treatment.** It carries the code every
 * error carries, it is the one solid fill on this surface, and it renders
 * inline because blast radius picks the placement — a proposer that could not
 * be called stops this surface and reaches nothing else.
 *
 * **What to do about it is Fleet's own sentence.** Fleet is what knows whether
 * a budget ran out, a key is missing or the provider was down, and a second
 * sentence written here would be Bridge guessing at a cause it was told.
 *
 * Told apart from the refusal above on both channels the design contract gives:
 * the red is the only solid fill on a data surface, and an error always carries
 * a code where a status never does.
 */
export const CallRefused: Story = {
  args: {
    request: REQUEST,
    proposal: {
      at: "faulted",
      code: "fleet.model.budget_exhausted",
      message: "The proposer was not called: this manifest's model budget is spent for today.",
      payload: {
        code: "fleet.model.budget_exhausted",
        message: "The proposer was not called: this manifest's model budget is spent for today.",
        run_id: "run_8f21c0",
        fields: [{ key: "budget_window", value: "day" }],
        bridgeProtocol: "5.2",
        fleetProtocol: "5.2",
        at: "2026-09-02T22:14:03Z",
      },
    },
    onCopied: fn(),
  },
};
