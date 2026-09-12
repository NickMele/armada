import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fireEvent, fn, waitFor } from "storybook/test";
import type { StagedAttachment } from "@armada/protocol";

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
    onSearchFiles: fn(async () => []),
    attachments: [],
    onStage: fn(async () => ({ path: "/tmp/staged" })),
    onAttach: fn(),
    onRemoveAttachment: fn(),
    onDispatch: fn(),
    onEnterByHand: fn(),
    onReset: fn(),
    onOpen: fn(),
    onApprove: fn(),
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

    // Dispatched rather than clicked. The app's base styles take a disabled
    // control out of pointer reach, so a pointer cannot press it at all; the
    // event still arrives here to prove the handler is not bound either.
    fireEvent.click(dispatch);
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
  play: async ({ args, canvas }) => {
    const dispatch = canvas.getByRole("button", { name: "Dispatch" });
    await expect(dispatch).toBeEnabled();
    // Dispatched rather than clicked. The app's base styles take a disabled
    // control out of pointer reach, so a pointer cannot press it at all; the
    // event still arrives here to prove the handler is not bound either.
    fireEvent.click(dispatch);
    await expect(args.onDispatch).toHaveBeenCalledOnce();
  },
};

/**
 * Typing `@` opens the mention popup, and picking a result inserts it into
 * the field — the way an editor's own file reference works.
 *
 * **A stateful wrapper, not static args.** Every other story here proves a
 * callback fired; this one proves what the field holds afterward, which needs
 * `onRequest` actually feeding back into `request` rather than a `fn()` that
 * drops it.
 */
export const MentionInserted: Story = {
  render: (args) => {
    function Stateful() {
      const [request, setRequest] = useState("");
      return (
        <DispatchRequest
          {...args}
          request={request}
          onRequest={setRequest}
          onSearchFiles={() => Promise.resolve(["README.md", "packages/components/README.md"])}
        />
      );
    }
    return <Stateful />;
  },
  play: async ({ canvas, userEvent }) => {
    const field = canvas.getByRole("textbox", { name: "Request" });
    await userEvent.type(field, "@READ");
    await userEvent.click(await canvas.findByRole("option", { name: "README.md" }));
    await expect(field).toHaveValue("@README.md ");
  },
};

/**
 * Arrow keys move the active row without touching the field's text, and
 * `Enter` inserts whichever row that lands on — the keyboard path beside the
 * mouse `MentionInserted` above already proves.
 */
export const MentionChosenByKeyboard: Story = {
  render: (args) => {
    function Stateful() {
      const [request, setRequest] = useState("");
      return (
        <DispatchRequest
          {...args}
          request={request}
          onRequest={setRequest}
          onSearchFiles={() => Promise.resolve(["README.md", "packages/components/README.md"])}
        />
      );
    }
    return <Stateful />;
  },
  play: async ({ canvas, userEvent }) => {
    const field = canvas.getByRole("textbox", { name: "Request" });
    await userEvent.type(field, "@");
    await canvas.findByRole("option", { name: "README.md" });

    await userEvent.keyboard("{ArrowDown}");
    await expect(
      canvas.getByRole("option", { name: "packages/components/README.md" }),
    ).toHaveAttribute("aria-selected", "true");

    await userEvent.keyboard("{Enter}");
    await expect(field).toHaveValue("@packages/components/README.md ");
  },
};

/**
 * `Escape` closes the popup without touching what was typed — the one way out
 * that leaves the `@` as plain text rather than turning it into a mention.
 */
export const MentionDismissedByEscape: Story = {
  render: (args) => {
    function Stateful() {
      const [request, setRequest] = useState("");
      return (
        <DispatchRequest
          {...args}
          request={request}
          onRequest={setRequest}
          onSearchFiles={() => Promise.resolve(["README.md"])}
        />
      );
    }
    return <Stateful />;
  },
  play: async ({ canvas, userEvent }) => {
    const field = canvas.getByRole("textbox", { name: "Request" });
    await userEvent.type(field, "@READ");
    await canvas.findByRole("option", { name: "README.md" });

    await userEvent.keyboard("{Escape}");

    await expect(canvas.queryByRole("listbox")).toBeNull();
    await expect(field).toHaveValue("@READ");
  },
};

/**
 * An `@` that does not start a word — an email typed into the same field —
 * never opens the popup. `useMention`'s own note says why: an `@` preceded by
 * anything but whitespace stays plain text rather than a mention.
 */
export const MentionNotOpenedInsideAWord: Story = {
  play: async ({ canvas, userEvent }) => {
    await userEvent.type(canvas.getByRole("textbox", { name: "Request" }), "ping me at a@b");
    await expect(canvas.queryByRole("listbox")).toBeNull();
  },
};

/**
 * A screenshot pasted straight into the Request field, without a trip to the
 * file picker — `onRequestPaste`'s own path, proven the way `WithAttachments`
 * below proves the picker's: `onStage` and `onAttach` both fire with what the
 * paste carried.
 *
 * **A real `DataTransfer`, dispatched directly.** A real browser's
 * `ClipboardEvent` constructor requires `clipboardData` to be an actual
 * `DataTransfer` and throws otherwise, before the component ever sees the
 * paste. `fireEvent.paste` cannot carry it: `@testing-library/dom`'s
 * `createEvent` rebuilds `clipboardData` from `Object.getOwnPropertyNames`
 * of whatever is passed — a jsdom-era shim — and a real `DataTransfer`'s
 * `items`/`files` live on its prototype, not as own properties, so that
 * rebuild silently produces an empty one. Dispatching the `ClipboardEvent`
 * ourselves is the seam that keeps the real data.
 */
export const PastedScreenshot: Story = {
  play: async ({ args, canvas }) => {
    const field = canvas.getByRole("textbox", { name: "Request" });
    const pasted = new File(["a screenshot"], "screenshot.png", { type: "image/png" });

    const dt = new DataTransfer();
    dt.items.add(pasted);
    field.dispatchEvent(new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData: dt }));

    // `stage()` reads the file's bytes before calling `onStage`, so the calls
    // land after this event handler returns — `waitFor` rather than a bare
    // assertion.
    await waitFor(() =>
      expect(args.onStage).toHaveBeenCalledWith(expect.anything(), "screenshot.png", "image/png"),
    );
    await expect(args.onAttach).toHaveBeenCalledWith({
      path: "/tmp/staged",
      filename: "screenshot.png",
      mimeType: "image/png",
    });
  },
};

/**
 * Plain text pasted into the Request is not read for images at all — it falls
 * through to the field as text, and nothing stages.
 */
export const PastedTextStagesNothing: Story = {
  play: async ({ args, canvas }) => {
    const field = canvas.getByRole("textbox", { name: "Request" });

    const dt = new DataTransfer();
    dt.setData("text/plain", "some text");
    field.dispatchEvent(new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData: dt }));

    await expect(args.onStage).not.toHaveBeenCalled();
    await expect(args.onAttach).not.toHaveBeenCalled();
  },
};

/**
 * Files staged against the request, drawn as removable chips — the same
 * `AttachmentChip` the hand-entry form already uses, on this field instead.
 */
export const WithAttachments: Story = {
  args: {
    request: REQUEST,
    attachments: [
      { path: "/tmp/a", filename: "before.png", mimeType: "image/png" },
      { path: "/tmp/b", filename: "after.png", mimeType: "image/png" },
    ] satisfies StagedAttachment[],
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Remove before.png" }));
    await expect(args.onRemoveAttachment).toHaveBeenCalledWith("/tmp/a");
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
  play: async ({ args, canvas }) => {
    await expect(canvas.getByRole("textbox", { name: "Request" })).toBeDisabled();
    await expect(canvas.getByRole("button", { name: "Enter by hand" })).toBeDisabled();

    const dispatch = canvas.getByRole("button", { name: "Reading the request" });
    await expect(dispatch).toBeDisabled();
    // Dispatched rather than clicked. The app's base styles take a disabled
    // control out of pointer reach, so a pointer cannot press it at all; the
    // event still arrives here to prove the handler is not bound either.
    fireEvent.click(dispatch);
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
   * Both acts, on one row and sent with the row's own id. **Approving here is
   * what starts the work** — everything the gate approves is on the screen, so
   * the trip to detail is the exception rather than the way through. Review
   * still opens it, for the case where the title is not enough.
   */
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(
      canvas.getByRole("button", { name: "Review Stop the board flickering on every event" }),
    );
    await expect(args.onOpen).toHaveBeenCalledWith("job_2d90bb");

    await userEvent.click(
      canvas.getByRole("button", { name: "Approve Stop the board flickering on every event" }),
    );
    await expect(args.onApprove).toHaveBeenCalledWith("job_2d90bb");
  },
};

/**
 * The approval is out. **The control says so and is dead** — approving twice
 * does not spawn twice, but a control that looks unpressed invites the second
 * press and then says nothing about the first.
 */
export const Approving: Story = {
  args: {
    approving: ["job_2d90bb"],
    proposal: {
      at: "proposed",
      request: REQUEST,
      jobs: [{ id: "job_2d90bb", title: "Stop the board flickering on every event", workflow: "bug", status: "awaiting_approval" }],
    },
  },
  play: async ({ args, canvas }) => {
    const approving = canvas.getByRole("button", { name: /Approving/ });
    await expect(approving).toBeDisabled();
    // Dispatched rather than clicked. The app's base styles take a disabled
    // control out of pointer reach, so a pointer cannot press it at all; the
    // event still arrives here to prove the handler is not bound either.
    fireEvent.click(approving);
    await expect(args.onApprove).not.toHaveBeenCalled();
  },
};

/**
 * Released, and the row says so. **The badge is Fleet's**, so a Job that has
 * left its gate draws no second approval — by this press or by anybody else's.
 */
export const Approved: Story = {
  args: {
    proposal: {
      at: "proposed",
      request: REQUEST,
      jobs: [{ id: "job_2d90bb", title: "Stop the board flickering on every event", workflow: "bug", status: "queued" }],
    },
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button", { name: /^Approve/ })).toBeNull();
  },
};

/**
 * Several, and the order between them.
 *
 * **The order is the whole of the graph.** A proposal of several is a chain —
 * each member waits on the one before it reaching `completed_success` — so
 * position carries it and no second field restates it.
 *
 * **Only the first is approvable, and nothing here approves all three.**
 * Fleet's rule is strictly one by one, and the second is not at its gate until
 * the first completes — so the rows under the head carry Review alone, and the
 * line under the list says why before anybody looks for a control.
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

    // One gate on screen, and it is the head's. Three rows at
    // `awaiting_approval` is what the answer says; which of them a person may
    // release is not the same question.
    await expect(canvas.getAllByRole("button", { name: /^Approve/ })).toHaveLength(1);
    await userEvent.click(
      canvas.getByRole("button", { name: "Approve Move the runtime file into its own crate" }),
    );
    await expect(args.onApprove).toHaveBeenCalledWith("job_11a0");
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
