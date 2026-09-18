// Carrying a bad Helm answer to somebody who could fix it, through `App` — #1367.
//
// The dock's own control opens the record, the record is read on screen, and
// the control inside it puts that same string on the clipboard. Fleet's answer
// is the scenario's, because a mock Fleet hosts no session.

import type { HelmDebugInfo } from "@armada/protocol";
import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { connected } from "./moment";
import type { Scenario } from "./moment";
import { MANIFEST_ID, repository, workflow } from "@armada/screens/src/fixtures/build/base";
import { entered, mount, unmountAfterEach } from "./testing";

unmountAfterEach();

const RECORD: HelmDebugInfo = {
  manifest_id: MANIFEST_ID,
  checkout: "/Users/user/armada",
  authority: "acting",
  model: "a-model",
  brief: "You are Helm, in Armada.",
  door: "armada-fleet",
  tools: ["list_jobs", "get_events_since"],
  servers: 19,
  session: "a-session",
  thread: [
    { at: "2026-09-17T14:29:40.000Z", line: "asked", text: { text: "why did you refuse that" } },
    { at: "2026-09-17T14:29:46.000Z", line: "refused", tool: "Read", because: "the person refused it" },
    { at: "2026-09-17T14:29:52.000Z", line: "ended", turns: 3, cost_micros: 24_300, refusals: 1 },
  ],
  cut: 0,
  run_id: "01K5RJ0F5H7TZ8QK6M9R1V2WXY",
  protocol_version: { major: 15, minor: 1 },
  at: "2026-09-17T14:31:02.117Z",
};

/** Helm pointed at a repository, with a thread in it and a record to answer with. */
function talking(): Scenario {
  return {
    name: "helm-record",
    says: "Helm pointed at a repository, with a session to report",
    state: {
      ...connected([], [workflow()], [repository()]),
      helm: { state: "open", manifestId: MANIFEST_ID, replying: false, skipped: 0, missed: 0, items: [] },
    },
    reads: {},
    behaves: () => ({ helmDebugInfo: async () => ({ ok: true, record: RECORD }) }),
  };
}

const dock = () => page.getByRole("complementary", { name: "Helm" });

test("the dock opens the session record, and the control copies what is on screen", async () => {
  const written: string[] = [];
  // `navigator.clipboard` is a getter, so the write is replaced rather than the object.
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText: (text: string) => (written.push(text), Promise.resolve()) },
  });
  mount(talking());
  await expect.element(dock()).toBeVisible();

  await page.getByRole("button", { name: "Session record" }).click();
  const sheet = page.getByRole("dialog", { name: "Session record" });
  await entered(sheet);

  // What decided the answer, on screen before anything leaves the machine.
  const shown = page.getByText(/armada helm session/);
  await expect.element(shown).toBeVisible();
  const text = shown.element().textContent ?? "";
  expect(text).toContain("You are Helm, in Armada.");
  expect(text).toContain("get_events_since");
  expect(text).toContain("refused Read · the person refused it");
  expect(text).toContain("$0.0243 · 3 turns · 1 refused");

  await page.getByRole("button", { name: "Copy debug info" }).click();
  await expect.poll(() => written).toHaveLength(1);
  // The same string, which is what one producer buys.
  expect(written[0]).toBe(text);
});

test("Helm pointed at no repository offers no record, because there is no session to report", async () => {
  mount("empty-store");
  await expect.element(dock()).toBeVisible();
  expect(page.getByRole("button", { name: "Session record" }).query()).toBeNull();
});
