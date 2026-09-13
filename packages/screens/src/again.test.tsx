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
import type { NamedSpec, Outcome, ShowAgain } from "@armada/protocol";

import { againOf, choicesOf, offerOf, useShowAgain, type ShowAgainCall } from "./again";
import { NO_FRAMES } from "./frames";
import { mount, unmount } from "./mounted";

afterEach(unmount);

const JOB = "01M130Y1380016YK5S0JXBXDQ5";
const SPEC = "e2e/panel.spec.ts";
const OTHER = "e2e/board.spec.ts";

function named(spec: string): NamedSpec {
  return { step_id: "show", attempt: 1, spec, on_disk: true };
}

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
      {...(again.choices === undefined ? {} : { choices: again.choices })}
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
  expect(call).toHaveBeenCalledWith(JOB, undefined);
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

test("one spec draws no chooser, and the press names none", async () => {
  const call = vi.fn<ShowAgainCall>(async () => ({ ok: true, shown: { job_id: JOB } }));
  mount(<OnTheStep call={call} known={facts()} />);

  expect(page.getByRole("combobox", { name: "Spec to run" }).elements()).toHaveLength(0);
  await page.getByRole("button", { name: "Show again" }).click();
  expect(call).toHaveBeenCalledWith(JOB, undefined);
});

test("several specs are offered, and the one picked is what the press runs", async () => {
  const call = vi.fn<ShowAgainCall>(async () => ({ ok: true, shown: { job_id: JOB } }));
  mount(<OnTheStep call={call} known={facts({ specs: [named(SPEC), named(OTHER)] })} />);

  const chooser = page.getByRole("combobox", { name: "Spec to run" });
  await expect.element(chooser).toHaveValue(SPEC);

  await chooser.selectOptions(OTHER);
  await expect.element(chooser).toHaveValue(OTHER);
  await page.getByRole("button", { name: "Show again" }).click();
  expect(call).toHaveBeenCalledWith(JOB, OTHER);
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
  return { pressing: false, press: () => {}, choose: () => {} };
}

test("the chooser is every spec the Job named, and a Fleet that sends none offers one", () => {
  const one = facts();
  expect(choicesOf(one, idle())).toBeUndefined();
  const two = facts({ specs: [named(SPEC), named(OTHER)] });
  expect(choicesOf(two, idle())?.specs).toEqual([SPEC, OTHER]);
  expect(choicesOf(two, { ...idle(), chosen: OTHER })?.chosen).toBe(OTHER);
  expect(offerOf(two, "show", false, OTHER)).toEqual({ state: "ready", spec: OTHER });
});

test("a set says which spec ran, and one kept before Fleet recorded it does not", () => {
  const set = (spec: string | undefined) => ({
    press: 1,
    pressed_at: "2026-09-10T14:02:00Z",
    step_id: "show",
    attempt: 1,
    ...(spec === undefined ? {} : { spec }),
    frames: [],
  });
  const headings = (spec: string | undefined) =>
    againOf(facts({ shown: [set(spec)] }), "show", NO_FRAMES, idle())?.sets.map(
      (one) => one.heading,
    );
  expect(headings(SPEC)?.[0]).toMatch(new RegExp(`^${SPEC.replace(/\./g, "\\.")}, shown again`));
  expect(headings(undefined)?.[0]).toMatch(/^Shown again/);
});
