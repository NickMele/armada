// Asking a Job to show its work again, from the control to the call.
//
// # Why this is a browser test
//
// `offerOf` is arithmetic and is tested as arithmetic below. What is worth a
// browser is that a person pressing the control reaches the call the screen was
// handed, with the Job it is about, and that the answer lands beside the
// control — a hook, an effect and a button, none of which a function test sees.

import { useState } from "react";
import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { ShownAgain } from "@armada/components";
import type { Outcome, ShowAgain } from "@armada/protocol";

import { againOf, offerOf, useShowAgain, type ShowAgainCall } from "./again";
import { NO_FRAMES } from "./frames";
import { mount, unmount } from "./mounted";

afterEach(unmount);

const JOB = "01M130Y1380016YK5S0JXBXDQ5";
const SPEC = "e2e/panel.spec.ts";

function facts(over: Partial<ShowAgain> = {}): ShowAgain {
  return {
    harness: true,
    worktree_on_disk: true,
    spec: { step_id: "show", attempt: 1, spec: SPEC, on_disk: true },
    drone_working: false,
    shown: [],
    ...over,
  };
}

/** The step's Shown chapter's second half, wired the way `JobDetail` wires it. */
function OnTheStep({ call, known }: { call: ShowAgainCall; known: ShowAgain }) {
  const [held] = useState(known);
  const pressing = useShowAgain(call, JOB, held, "show", NO_FRAMES);
  const again = againOf(held, "show", NO_FRAMES, pressing);
  if (again === undefined) return null;
  return (
    <ShownAgain
      {...(again.offer === undefined ? {} : { offer: again.offer })}
      sets={again.sets}
      {...(again.said === undefined ? {} : { said: again.said })}
      onShow={again.onShow}
    />
  );
}

test("a press reaches the call, for the Job it is about, and waits for the answer", async () => {
  let answer: (outcome: Outcome) => void = () => {};
  const call = vi.fn<ShowAgainCall>(
    () => new Promise<Outcome>((resolve) => (answer = resolve)),
  );
  mount(<OnTheStep call={call} known={facts()} />);

  await page.getByRole("button", { name: "Show again" }).click();
  expect(call).toHaveBeenCalledWith(JOB);
  await expect
    .element(page.getByRole("button", { name: "Showing…" }))
    .toBeDisabled();

  answer({
    ok: true,
    shown: {
      job_id: JOB,
      nothing: "`evidence.run` exited 0 and `evidence.frames` held no file",
    },
  });
  await expect.element(page.getByRole("status")).toHaveTextContent("held no file.");
  await expect.element(page.getByRole("button", { name: "Show again" })).toBeEnabled();
});

test("a refusal Fleet answered with is said beside the control, as a sentence", async () => {
  const call = vi.fn<ShowAgainCall>(async () => ({
    ok: false,
    why: "refused",
    error: {
      code: "fleet.cannot_show_again",
      message: "this Job's worktree is no longer on disk, so there is nowhere to run the harness",
      run_id: "01M1RUN0000000000000000000",
      fields: {},
      chain: [],
    },
  }));
  mount(<OnTheStep call={call} known={facts()} />);

  await page.getByRole("button", { name: "Show again" }).click();
  await expect
    .element(page.getByRole("status"))
    .toHaveTextContent("This Job's worktree is no longer on disk");
});

test("a control that cannot run is never pressed through to Fleet", async () => {
  const call = vi.fn<ShowAgainCall>();
  mount(<OnTheStep call={call} known={facts({ worktree_on_disk: false })} />);

  await expect.element(page.getByRole("button", { name: "Show again" })).toBeDisabled();
  await expect.element(page.getByText(/worktree is gone/)).toBeInTheDocument();
  expect(call).not.toHaveBeenCalled();
});

// --------------------------------------------------- the reading, as arithmetic

test("the reason is chosen in Fleet's own refusal order", () => {
  const all = facts({
    harness: false,
    worktree_on_disk: false,
    drone_working: true,
    spec: { step_id: "show", attempt: 1, spec: SPEC, on_disk: false },
  });
  const reason = (known: ShowAgain) => {
    const offer = offerOf(known, "show", false);
    return offer?.state === "cannot" ? String(offer.why) : offer?.state;
  };
  expect(reason(all)).toMatch(/declares no evidence harness/);
  expect(reason({ ...all, harness: true })).toMatch(/worktree is gone/);
  expect(reason({ ...all, harness: true, worktree_on_disk: true })).toMatch(/Drone is working/);
  expect(
    reason({ ...all, harness: true, worktree_on_disk: true, drone_working: false }),
  ).toMatch(/no longer in this Job's worktree/);
});

test("no control is drawn where no Drone named a spec, or on another step", () => {
  expect(offerOf(facts({ spec: undefined }), "show", false)).toBeUndefined();
  expect(offerOf(facts(), "review", false)).toBeUndefined();
  expect(againOf(facts({ spec: undefined }), "show", NO_FRAMES, idle())).toBeUndefined();
});

test("a press out from any window reads as showing, whatever else is true", () => {
  const out = facts({ showing_since: "2026-09-10T14:02:00Z", drone_working: true });
  expect(offerOf(out, "show", false)?.state).toBe("showing");
});

function idle() {
  return { pressing: false, press: () => {} };
}
