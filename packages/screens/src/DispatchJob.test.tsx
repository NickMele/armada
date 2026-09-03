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
// rendering is only as good as the caller re-rendering in time: a key repeat, a
// synthetic click or an app that awaits before it moves the proposal all reach
// the handler with the control still drawn live. So every case here presses a
// control that is *enabled* and asserts on what was sent.
//
// The story in `packages/components` proves the other half — that the control
// goes off while a call is out — because that half is a rendering and belongs
// where renderings are agreed.

import { useState } from "react";
import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { Proposal } from "@armada/components";

import { DispatchJob } from "./DispatchJob";
import { mount, unmount } from "./mounted";

afterEach(unmount);

/** What the form is holding when the guard has to work. */
const REQUEST = "The board flickers every time an event lands.";

/**
 * Mount it, and hand back every request that reached the caller.
 *
 * **`onDispatch` deliberately moves nothing.** A caller that answered by
 * setting `reading` would disable the control and prove the rendering rather
 * than the guard; this is the app at its slowest, which is the case the guard
 * exists for.
 */
function opened(proposal: Proposal = { at: "unasked" }): { sent: string[] } {
  const sent: string[] = [];
  mount(
    <DispatchJob
      proposal={proposal}
      onDispatch={(request) => sent.push(request)}
      onReset={() => {}}
      onOpen={() => {}}
      echoed={null}
      byHand={<p>The form, by hand</p>}
      disabled={false}
    />,
  );
  return { sent };
}

function field() {
  return page.getByRole("textbox", { name: "Request" });
}

test("two presses on a live control are one call", async () => {
  const { sent } = opened();
  await userEvent.fill(field(), REQUEST);

  const dispatch = page.getByRole("button", { name: "Dispatch" });
  // Still enabled on the second press, which is the whole point: the caller has
  // not answered, so nothing about the rendering has changed.
  await userEvent.click(dispatch);
  await expect.element(dispatch).toBeEnabled();
  await userEvent.click(dispatch);

  expect(sent, "a second press fired a second model call").toEqual([REQUEST]);
});

/**
 * The guard releases when the answer arrives, and not before.
 *
 * **The half that keeps the test above honest.** A guard that never let go
 * would pass it and wedge the surface after one dispatch, which on screen reads
 * exactly like the guard working.
 */
test("the answer releases it", async () => {
  const sent: string[] = [];
  mount(<Answering sent={sent} />);

  await userEvent.fill(field(), REQUEST);
  await userEvent.click(page.getByRole("button", { name: "Dispatch" }));
  expect(sent).toEqual([REQUEST]);
  // A call is out, so the control says so and sends nothing.
  await expect.element(page.getByRole("button", { name: "Reading the request" })).toBeDisabled();

  await userEvent.click(page.getByRole("button", { name: "Answer it" }));
  await userEvent.click(page.getByRole("button", { name: "Dispatch" }));
  expect(sent, "the answer did not release the guard").toEqual([REQUEST, REQUEST]);
});

/**
 * The caller, moving the proposal the way the app does: `reading` the moment a
 * request goes out, and an answer when one comes back. The refusal it answers
 * with is the one that leaves the request in the field, so the second dispatch
 * has something to send.
 */
function Answering({ sent }: { sent: string[] }) {
  const [proposal, setProposal] = useState<Proposal>({ at: "unasked" });
  return (
    <>
      <button type="button" onClick={() => setProposal({ at: "unresolved" })}>
        Answer it
      </button>
      <DispatchJob
        proposal={proposal}
        onDispatch={(request) => {
          sent.push(request);
          setProposal({ at: "reading" });
        }}
        onReset={() => {}}
        onOpen={() => {}}
        echoed={null}
        byHand={<p>The form, by hand</p>}
        disabled={false}
      />
    </>
  );
}

/** Nothing is sent for a field holding only spaces, by the button or otherwise. */
test("whitespace is not a request", async () => {
  const { sent } = opened();
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
  opened();
  await userEvent.click(page.getByRole("button", { name: "Enter by hand" }));
  await expect.element(page.getByText("The form, by hand")).toBeVisible();

  await userEvent.click(page.getByRole("button", { name: "Describe the work instead" }));
  await expect.element(field()).toBeVisible();
});
