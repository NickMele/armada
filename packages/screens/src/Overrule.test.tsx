// An override will not go on the record without a reason — by the button and
// by the key.
//
// # Why the reason is not optional
//
// `Overrule.tsx` says it: a person is recording that a verifier was wrong and
// that they took responsibility for going past it, the reason is written to an
// append-only log beside the refusal, and `#154` will read those reasons to
// learn whether the Judge or the criterion was at fault. An unexplained
// override is the one outcome there is no path back from — which is why the
// keyboard half below matters more here than anywhere else this rule appears.
//
// # Both triggers, because the words are chosen off the trigger
//
// A Judge's refusal overruled and a gaming flag overruled are two different
// things a person is doing, so `OVERRULING` keys every word — including the
// field's own label — off `trigger`. A gate reading one label would prove the
// rule for one of the two screens this file draws.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { StepDetail } from "@armada/protocol";

import { mount, unmount } from "./mounted";
import { OverruleControl } from "./Overrule";
import { OVERRULING, type Overruled } from "./recovery";

afterEach(unmount);

/** The same shape `phases.test.ts` builds, for the same reason: it is the wire's. */
function step(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "implement",
    label: "Implement",
    ordinal: 2,
    state: "failed",
    check_runs: [],
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [],
    verdicts: [],
    entered_at: "2026-09-02T09:00:00Z",
    updated_at: "2026-09-02T09:12:00Z",
    ...over,
  };
}

/** Mount the control for one trigger, and hand back what the override was told. */
function opened(trigger: Overruled): { said: [string, string][] } {
  const said: [string, string][] = [];
  mount(
    <OverruleControl
      jobId="job_2d90bb"
      overrule={{ step: step(), trigger, commits: false }}
      disabled={false}
      onOverrule={(jobId, reason) => said.push([jobId, reason])}
    />,
  );
  return { said };
}

/**
 * The confirm, scoped to the dialog.
 *
 * **The opener and the confirm carry the same words on purpose** — an action
 * keeps its name through the flow — so a query by name alone finds two buttons,
 * and the one outside the dialog is never disabled.
 */
function confirm(trigger: Overruled) {
  return page.getByRole("dialog").getByRole("button", { name: OVERRULING[trigger].label });
}

const TRIGGERS: Overruled[] = ["gate_failure", "evidence_suspect"];

for (const trigger of TRIGGERS) {
  const words = OVERRULING[trigger];

  test(`${trigger}: a blank reason is refused by the button and by Enter`, async () => {
    const { said } = opened(trigger);
    await userEvent.click(page.getByRole("button", { name: words.label }));

    await expect.element(confirm(trigger)).toBeDisabled();

    await userEvent.keyboard("{Enter}");
    expect(said, "Enter recorded an override with no reason").toEqual([]);
    // Still up. Closing on a refused press would read as having gone through,
    // and nothing takes an override back.
    await expect.element(page.getByRole("dialog")).toBeVisible();
  });

  test(`${trigger}: whitespace is not a reason`, async () => {
    const { said } = opened(trigger);
    await userEvent.click(page.getByRole("button", { name: words.label }));

    await userEvent.fill(page.getByRole("textbox", { name: words.field }), "  \t ");

    await expect.element(confirm(trigger)).toBeDisabled();
    await userEvent.keyboard("{Enter}");
    expect(said, "spaces counted as a reason").toEqual([]);
  });

  test(`${trigger}: a reason satisfies it, and Enter then records it`, async () => {
    const { said } = opened(trigger);
    await userEvent.click(page.getByRole("button", { name: words.label }));

    // **The half that keeps the two above honest.** A dialog that refused every
    // press would pass both, and the refusal is only right if the same key
    // works the moment the field is filled.
    await userEvent.fill(page.getByRole("textbox", { name: words.field }), "it read the wrong diff");
    await expect.element(confirm(trigger)).toBeEnabled();

    await userEvent.keyboard("{Enter}");
    expect(said).toEqual([["job_2d90bb", "it read the wrong diff"]]);
  });
}
