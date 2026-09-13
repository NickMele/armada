// Setup's rules — the default ticks, the narrow `missing`, what the state column says, and
// how an answer folds — read here rather than in a browser.

import { describe, expect, it } from "vitest";

import type { ManifestProposal, ManifestProposals, RepositoryScan, ScannedWorkspace } from "@armada/protocol";
import { answered, gridOf, pickerOf, readInto, sheetOf, stateOf, type SetupOpen } from "./setup-view";

function workspace(dir: string, evidence: string): ScannedWorkspace {
  return {
    dir, declared_by: [], evidence, manifests: [{ file: `${dir}/package.json`, tool: "npm" }], lockfiles: [],
    runnables: [], tools: [], services: [], ports: [], missing: [], not_read: [],
  };
}

function proposal(dir: string, checks: string[]): ManifestProposal {
  const scripts = { source: "convention", file: `${dir}/package.json` };
  return {
    dir,
    file: `${dir}/armada.yml`,
    id: { value: dir, provenance: scripts },
    ports: [],
    checks: checks.map((name) => ({ name, run: `pnpm ${name}`, provenance: { ...scripts, key: `scripts.${name}` } })),
    commands: [],
    policy: [{ key: "auto_merge", value: "never", provenance: { source: "default" } }],
  };
}

const SCAN: RepositoryScan = {
  checkout: "/repo",
  workspaces: [workspace(".", "strong"), workspace("web", "strong"), workspace("api", "strong"), workspace("docs", "thin")],
  not_read: [],
};

const PROPOSALS: ManifestProposals = {
  checkout: "/repo",
  proposals: [proposal(".", ["test"]), proposal("web", ["test", "lint"]), proposal("api", ["test"]), proposal("docs", [])],
  caps: { cost_micros: 5_000_000, turns: 200 },
};

function opened(): SetupOpen {
  const held = readInto({ state: "reading" }, SCAN, PROPOSALS);
  if (held.state !== "open") throw new Error("a read opens Setup");
  return held;
}

describe("the picker", () => {
  it("ticks strong evidence and leaves thin unticked", () => {
    expect(opened().ticks).toEqual({ ".": true, web: true, api: true, docs: false });
  });

  it("keeps a person's ticks through a re-read", () => {
    const held = readInto({ ...opened(), ticks: { docs: true, web: false } }, SCAN, PROPOSALS);
    expect(held.state === "open" && held.ticks).toMatchObject({ docs: true, web: false, api: true });
  });

  it("reports each state without instructing", () => {
    expect(stateOf(proposal("docs", []), false)).toBe("no checks proposed");
    expect(stateOf(proposal("web", ["test"]), true)).toBe("open, being edited");
    expect(stateOf({ ...proposal("web", ["test"]), written: { path: "web/armada.yml", at: "2026-09-13T10:00:00Z" } }, true)).toBe("written");
    expect(stateOf(proposal("web", ["test"]), false, { appeared: { onDisk: null } })).toBe("already set up");
    expect(stateOf({ ...proposal(".", ["test"]), present: true }, false)).toBe("already set up");
  });

  it("says a thin workspace names nothing runnable", () => {
    const docs = pickerOf(opened()).rows.find((row) => row.dir === "docs");
    expect(docs?.note).toBe("No file here names a script, so anything proposed is convention.");
  });
});

describe("the Check-names grid", () => {
  it("marks a name every other ticked sibling declares, and never on the root", () => {
    const { names, grid } = gridOf(PROPOSALS.proposals, { ".": true, web: true, api: true });
    expect(names).toEqual(["test", "lint"]);
    expect(grid.find((row) => row.dir === "api")?.cells).toEqual(["declared", "missing"]);
    expect(grid.find((row) => row.dir === ".")?.cells).toEqual(["declared", "absent"]);
  });

  it("recomputes over the batch as ticked, so an unticked sibling marks nothing", () => {
    const withDocs = gridOf(PROPOSALS.proposals, { web: true, api: true, docs: true });
    expect(withDocs.grid.find((row) => row.dir === "api")?.cells).toEqual(["declared", "absent"]);
  });
});

describe("an answer, folded", () => {
  const say = () => "Fleet is not connected. Nothing was sent.";

  it("redraws from the proposal Fleet answered with, and clears the last refusal", () => {
    const edited = { ...proposal("web", ["test"]), checks: [] };
    const was: SetupOpen = { ...opened(), busy: "web", marks: { web: { problem: "no" } } };
    const held = answered(was, "web", "edit", { state: "took", proposal: edited }, say);
    expect(held.state === "open" && [held.busy, held.marks.web, held.proposals.proposals[1]]).toEqual([null, {}, edited]);
  });

  it("keeps a Write's faults for the sheet, and an edit's refusal as one sentence", () => {
    const faults = [{ key: "checks.test.requires", fault: "names no Command" }];
    const wrote = answered(opened(), "web", "write", { state: "refused", saying: "would not load", faults }, say);
    expect(wrote.state === "open" && wrote.marks.web).toEqual({ refused: { saying: "would not load", faults } });
    const edit = answered(opened(), "web", "edit", { state: "refused", saying: "would drop requires", faults: [] }, say);
    expect(edit.state === "open" && edit.marks.web).toEqual({ problem: "would drop requires" });
  });
});

describe("the sheet", () => {
  it("cites each line's file and key, and states the caps in one line", () => {
    const sheet = sheetOf(opened(), PROPOSALS.proposals[1]!);
    expect(sheet.checks[1]?.cited).toEqual({ source: "convention", file: "web/package.json", at: "scripts.lint" });
    expect(sheet.policy[0]?.options.map((one) => one.value)).toEqual(["never", "checks-pass", "always"]);
    expect(sheet.caps).toMatch(/^Jobs here stop at .*5.* or 200 turns/);
    expect(sheet.setUp).toBe(false);
  });

  it("offers no Write where an armada.yml is already there", () => {
    expect(sheetOf(opened(), { ...PROPOSALS.proposals[0]!, present: true }).setUp).toBe(true);
  });
});
