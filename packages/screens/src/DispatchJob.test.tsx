// One request out at a time, and one way back from the override.
//
// # Why the guard is tested and not just written
//
// There is no in-flight guard on `proposeFromRequest`, matching `proposeJob`.
// Two presses are two model calls and two drafted plans — two of everything at
// the gate, and somebody deleting one by hand — so the form is what has to stop
// it, and a guard nothing exercises is a guard nobody knows broke.
//
// **The button being disabled is not the test.** That is a rendering, and a
// press can reach the handler with the control still drawn live: a key repeat,
// a synthetic click, a frame not yet painted. So every case here presses a
// control that is enabled and asserts on what was sent.
//
// The story in `packages/components` proves the other half — that the control
// goes off while a call is out — because that half is a rendering and belongs
// where renderings are agreed.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { DispatchJob } from "./DispatchJob";
import type { Answered } from "./proposal";
import { mount, unmount } from "./mounted";

afterEach(unmount);

/** What the form is holding when the guard has to work. */
const REQUEST = "The board flickers every time an event lands.";

/** An answer that leaves the request in the field, so a second press has one. */
const UNRESOLVED: Answered = {
  proposal: { at: "unresolved" },
  outcome: null,
  request: REQUEST,
};

/** A promise the test settles, so a call can be held open across two presses. */
function held(): { promise: Promise<Answered>; answer: (read: Answered) => void } {
  let answer!: (read: Answered) => void;
  const promise = new Promise<Answered>((resolve) => {
    answer = resolve;
  });
  return { promise, answer };
}

/**
 * Mount it, and hand back every request that reached the caller.
 *
 * `onPropose` answers with whatever the test hands it, so a call can be left
 * outstanding — which is the only state the guard exists for.
 */
function opened(answering: () => Promise<Answered>): { sent: string[] } {
  const sent: string[] = [];
  mount(
    <DispatchJob
      onPropose={(request) => {
        sent.push(request);
        return answering();
      }}
      onOpen={() => {}}
      byHand={<p>The form, by hand</p>}
      // Nothing published, which is the state before Fleet's first message and
      // the one these cases are about: what the guard does is not a function of
      // how far the call has got.
      watching={null}
      onStop={() => {}}
      disabled={false}
    />,
  );
  return { sent };
}

function field() {
  return page.getByRole("textbox", { name: "Request" });
}

test("two presses in one task are one call", async () => {
  // Never settled: the call stays out for the whole test, which is the window
  // the second press has to be refused in.
  const { sent } = opened(() => new Promise<Answered>(() => {}));
  await userEvent.fill(field(), REQUEST);

  // **Both presses in one task, on purpose.** React has not re-rendered between
  // them, so the second reaches the handler with the button still enabled in
  // the DOM — which is the whole case the ref exists for and the one a click
  // helper that waits for the control to settle can never produce.
  const button = page.getByRole("button", { name: "Dispatch" }).element() as HTMLButtonElement;
  button.click();
  button.click();

  expect(sent, "a second press fired a second model call").toEqual([REQUEST]);
  // And the rendering caught up, which is the half a person sees.
  await expect.element(page.getByRole("button", { name: "Reading the request" })).toBeDisabled();
});

/**
 * The answer releases it, and not before.
 *
 * **The half that keeps the test above honest.** A guard that never let go
 * would pass it and wedge the surface after one dispatch, which on screen reads
 * exactly like the guard working.
 */
test("the answer releases it", async () => {
  const first = held();
  let next: () => Promise<Answered> = () => first.promise;
  const { sent } = opened(() => next());

  await userEvent.fill(field(), REQUEST);
  await userEvent.click(page.getByRole("button", { name: "Dispatch" }));
  expect(sent).toEqual([REQUEST]);

  next = () => Promise.resolve(UNRESOLVED);
  first.answer(UNRESOLVED);
  // The refusal put the request back, so the second press has one to send.
  await expect.element(page.getByRole("button", { name: "Dispatch" })).toBeEnabled();

  await userEvent.click(page.getByRole("button", { name: "Dispatch" }));
  expect(sent, "the answer did not release the guard").toEqual([REQUEST, REQUEST]);
});

/**
 * A call that threw leaves the surface usable. **Wedged on `reading` is worse
 * than the throw** — nothing on screen says the call is dead, and the guard
 * never releases — so the state goes back and the throw carries on.
 */
test("a call that threw gives the surface back", async () => {
  // The rethrow is the point, and `dispatch` is called through `void`, so it
  // lands as an unhandled rejection. In the app that is what `watchUncaught`
  // draws; here it is caught so the runner does not fail the test for the
  // behaviour the test is asserting.
  const expected = (event: PromiseRejectionEvent) => event.preventDefault();
  window.addEventListener("unhandledrejection", expected);
  try {
    const { sent } = opened(() => Promise.reject(new Error("the preload has no proposer")));
    await userEvent.fill(field(), REQUEST);
    await userEvent.click(page.getByRole("button", { name: "Dispatch" }));

    const dispatch = page.getByRole("button", { name: "Dispatch" });
    await expect.element(dispatch).toBeEnabled();
    await userEvent.click(dispatch);
    expect(sent, "a throw left the surface unable to ask again").toEqual([REQUEST, REQUEST]);
  } finally {
    window.removeEventListener("unhandledrejection", expected);
  }
});

/** Nothing is sent for a field holding only spaces, by the button or otherwise. */
test("whitespace is not a request", async () => {
  const { sent } = opened(() => Promise.resolve(UNRESOLVED));
  await userEvent.fill(field(), "   \t ");
  await expect.element(page.getByRole("button", { name: "Dispatch" })).toBeDisabled();
  expect(sent).toEqual([]);
});

/**
 * Hand entry is one press away and never a dead end. **The override has to be
 * leavable** — a form somebody reached by mistake with no way back is the
 * surface that made the proposer worth building.
 */
test("hand entry is reachable and leavable", async () => {
  opened(() => Promise.resolve(UNRESOLVED));
  await userEvent.click(page.getByRole("button", { name: "Enter by hand" }));
  await expect.element(page.getByText("The form, by hand")).toBeVisible();

  await userEvent.click(page.getByRole("button", { name: "Describe the work instead" }));
  await expect.element(field()).toBeVisible();
});
