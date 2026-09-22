// The scenario list against the fixtures it is built from, so a builder added to
// `fixtures/build/index.ts` and not named here fails rather than going missing.
//
// **Three rosters now.** `FIXTURES` is the Bug Job at every state and needs a
// line in `BUILDERS`; `ARC_MOMENTS` and `KIND_FIXTURES` are walked, so a
// moment or a kind added there is a scenario with no edit here — what these
// tests hold is that the walk happened and that no name went missing.

import { expect, test } from "vitest";
import { FIXTURES } from "@armada/screens/src/fixtures/build/index";
import { ARC_MOMENTS } from "@armada/screens/src/fixtures/build/arc";
import { KIND_FIXTURES } from "@armada/screens/src/fixtures/build/kinds";

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

test("every arc moment is a scenario, and carries what its boards draw", () => {
  const arc = SCENARIOS.filter((one) => one.name.startsWith("arc/"));
  expect(arc).toHaveLength(ARC_MOMENTS.length);
  for (const one of arc) {
    expect(one.draft, `${one.name} carries no draft`).toBeDefined();
  }
});

test("the moments that are several Jobs hold every one of them", () => {
  for (const name of ["members/stacked", "members/merged", "epic/wave"]) {
    const scenario = scenarioNamed(name)!;
    expect(scenario, `no scenario named ${name}`).toBeDefined();
    const ids = scenario.state.jobs.map((job) => job.id);
    expect(ids.length).toBeGreaterThan(1);
    expect(Object.keys(scenario.reads).sort()).toEqual([...ids].sort());
    expect(scenario.draft?.members?.members.length).toBeGreaterThan(1);
  }
});

test("one Job per workflow kind, on one Board and one at a time", () => {
  const board = scenarioNamed("kinds")!;
  expect(board.state.jobs).toHaveLength(KIND_FIXTURES.length);
  const workflows = new Set(board.state.jobs.map((job) => job.workflow_id));
  expect(workflows.size).toBe(KIND_FIXTURES.length);
  // Every workflow a row names is one this scenario's Fleet serves, or the
  // detail draws a run with no step labels at all.
  const served = new Set((board.state.holds.workflows ?? []).map((one) => one.id));
  for (const id of workflows) expect(served, `${id} is not served`).toContain(id);
  expect(SCENARIOS.filter((one) => one.name.startsWith("kind/"))).toHaveLength(
    KIND_FIXTURES.length,
  );
});

test("no scenario names a shape", () => {
  for (const one of SCENARIOS) {
    const said = `${one.name} ${one.says}`.toLowerCase();
    for (const word of ["convoy", "train", "atomic"]) {
      expect(new RegExp(`\\b${word}\\b`).test(said), `${one.name}: ${one.says}`).toBe(false);
    }
  }
});
