// The scenario list against the fixtures it is built from, so a builder added to
// `fixtures/build/index.ts` and not named here fails rather than going missing.

import { expect, test } from "vitest";
import { FIXTURES } from "@armada/screens/src/fixtures/build/index";

import { BUILDERS, SCENARIOS, scenarioNamed } from "./scenario";

test("every fixture the build roster lists has a builder here, and no more", () => {
  const built = Object.values(BUILDERS).map((make) => make().name).sort();
  expect(built).toEqual(FIXTURES.map((fixture) => fixture.name).sort());
});

test("every scenario has its own name", () => {
  const names = SCENARIOS.map((one) => one.name);
  expect(new Set(names).size).toBe(names.length);
});

// A Studio on a Manifest the scenario does not serve is never read, so the surface would draw an
// empty list and nothing would say why — #1341.
test("every Studio a scenario keeps belongs to a Manifest that scenario serves", () => {
  for (const scenario of SCENARIOS) {
    const served = (scenario.state.holds.manifests ?? []).map((one) => one.id);
    for (const studio of scenario.studios ?? []) {
      expect(served, `${scenario.name} keeps a Studio on a Manifest it does not serve`).toContain(
        studio.manifest_id,
      );
    }
  }
});

test("every-state holds each Job once, with its own reads", () => {
  const scenario = scenarioNamed("every-state")!;
  const ids = scenario.state.jobs.map((job) => job.id);
  expect(new Set(ids).size).toBe(ids.length);
  expect(Object.keys(scenario.reads).sort()).toEqual([...ids].sort());
  const recordings = SCENARIOS.filter((one) => one.name.startsWith("recorded/"));
  expect(ids.length).toBe(FIXTURES.length + recordings.length);
});
