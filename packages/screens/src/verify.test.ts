// Drift and Verify on the Manifest surface. What each panel is offered is a
// rule from Journey 9, so each is read here rather than in a browser.

import { describe, expect, it } from "vitest";

import type {
  CheckoutRunSheet,
  CheckoutRunRecord,
  CheckoutVerify,
  Declaration,
  ManifestDriftRead,
} from "@armada/protocol";
import { driftGoneOf, driftPanelOf, endedOf, verifyPanelOf, verifySaidOf, type VerifyInputs } from "./verify";

const SHEET: CheckoutRunSheet = { setup: [], checks: [], commands: [] };

function record(name: string, exit: number): CheckoutRunRecord {
  return {
    id: `run-${name}`,
    name,
    command: name,
    required: [],
    started_at: "2026-09-12T14:00:00Z",
    ended_at: "2026-09-12T14:00:02Z",
    duration_ms: 2000,
    exit_code: exit,
    expect_exit_code: 0,
    ended: `exited ${exit}`,
    stopped: false,
    changed: [],
    undoable: false,
    log: `runs/${name}/output.log`,
  };
}

function inputs(sheet: CheckoutRunSheet, over: Partial<VerifyInputs> = {}): VerifyInputs {
  return {
    sheet: { state: "read", sheet },
    now: Date.parse("2026-09-12T14:01:00Z"),
    dismissed: null,
    refused: null,
    onVerify: () => {},
    onStopRun: () => {},
    onDismiss: () => {},
    ...over,
  };
}

const UNDERWAY: CheckoutVerify = {
  id: "v1",
  started_at: "2026-09-12T14:00:00Z",
  steps: [
    { group: "setup", name: "bootstrap", run: "pnpm install", state: "ran", record: record("bootstrap", 0) },
    { group: "checks", name: "test", run: "cargo test", state: "running", run_id: "run-test" },
    { group: "checks", name: "lint", run: "cargo clippy", state: "waiting" },
  ],
};

describe("drift", () => {
  it("draws a gone line with what it is missing, and a current one without", () => {
    const panel = driftPanelOf({
      state: "read",
      drift: {
        path: "armada.yml",
        checkout: "/repo",
        declarations: [
          {
            section: "checks",
            name: "lint",
            key: "run",
            run: "sh scripts/lint.sh",
            drift: { verdict: "gone", missing: ["scripts/lint.sh"] },
            unfollowed: [],
          },
          { section: "checks", name: "test", key: "run", run: "cargo test", drift: { verdict: "current", checked: 0 }, unfollowed: [] },
        ],
      },
    });
    expect(panel.rows?.map((row) => [row.where, row.verdict, row.missing])).toEqual([
      ["checks.lint.run", "gone", ["scripts/lint.sh"]],
      ["checks.test.run", "current", undefined],
    ]);
  });

  it("says it is reading before Fleet has answered, rather than drawing no rows", () => {
    expect(driftPanelOf({ state: "none" }).rows).toBeUndefined();
    expect(driftPanelOf({ state: "reading" }).note).toMatch(/Reading/);
  });
});

describe("verify", () => {
  it("is offered where nothing is out, and has run nothing", () => {
    const panel = verifyPanelOf(inputs(SHEET));
    expect(panel.onVerify).toBeDefined();
    expect(panel.steps).toBeUndefined();
  });

  it("is not offered while a person's run holds the checkout, and says whose", () => {
    const panel = verifyPanelOf(
      inputs({ ...SHEET, running: { id: "r", name: "fmt", command: "cargo fmt", started_at: "2026-09-12T14:00:30Z" } }),
    );
    expect(panel.onVerify).toBeUndefined();
    expect(panel.unavailable).toMatch(/`fmt` is running/);
  });

  it("while underway offers Stop on the step that is out, and no second Verify", () => {
    const stopped: string[] = [];
    const panel = verifyPanelOf(
      inputs(
        { ...SHEET, verify: UNDERWAY, running: { id: "run-test", name: "test", command: "cargo test", started_at: "2026-09-12T14:00:30Z" } },
        { onStopRun: (id) => stopped.push(id) },
      ),
    );
    expect(panel.onVerify).toBeUndefined();
    expect(panel.unavailable).toBeUndefined();
    expect(panel.steps?.map((step) => step.state.kind)).toEqual(["ran", "running", "waiting"]);
    expect(panel.steps?.[1]?.state).toEqual({ kind: "running", elapsed: "30s" });
    panel.onStop?.();
    expect(stopped).toEqual(["run-test"]);
  });

  it("once ended counts what ran, what ended otherwise and what did not run — and can be put away", () => {
    const ended: CheckoutVerify = {
      ...UNDERWAY,
      ended_at: "2026-09-12T14:00:09Z",
      steps: [
        { group: "setup", name: "bootstrap", run: "pnpm install", state: "ran", record: record("bootstrap", 0) },
        { group: "checks", name: "test", run: "cargo test", state: "ran", record: record("test", 2) },
        { group: "checks", name: "lint", run: "cargo clippy", state: "not_run", why: "Verify was stopped during `test`" },
      ],
    };
    expect(endedOf(ended)).toBe("Ran 2 of 3. 1 ended with a code other than the one it expects. 1 did not run.");
    const panel = verifyPanelOf(inputs({ ...SHEET, verify: ended }));
    expect(panel.onVerify).toBeDefined();
    expect(panel.steps?.[1]?.state).toEqual({ kind: "ran", result: "exit 2 (expects 0)", duration: "2.0s" });
    expect(verifyPanelOf(inputs({ ...SHEET, verify: ended }, { dismissed: "v1" })).steps).toBeUndefined();
  });
});

describe("verify of a workspace's own file", () => {
  const WEB_ENDED: CheckoutVerify = {
    id: "w1",
    started_at: "2026-09-12T14:00:00Z",
    ended_at: "2026-09-12T14:00:02Z",
    workspace: "apps/web",
    steps: [{ group: "checks", name: "test", run: "pnpm test", state: "ran", record: record("test", 0) }],
  };

  it("draws a Verify on the sheet of the file it ran, and not on another's", () => {
    const web = verifyPanelOf(inputs({ ...SHEET, verify: WEB_ENDED }, { workspace: "apps/web" }));
    expect(web.steps?.map((step) => step.name)).toEqual(["test"]);
    const root = verifyPanelOf(inputs({ ...SHEET, verify: WEB_ENDED }));
    expect(root.steps).toBeUndefined();
    expect(root.onVerify).toBeDefined();
  });

  it("is not offered while another file's Verify holds the checkout, and says which", () => {
    const panel = verifyPanelOf(inputs({ ...SHEET, verify: UNDERWAY }, { workspace: "apps/web" }));
    expect(panel.onVerify).toBeUndefined();
    expect(panel.unavailable).toBe("Verify is running armada.yml in this checkout. This file can be verified once it ends.");
    expect(panel.steps).toBeUndefined();
    expect(panel.onStop).toBeUndefined();
  });
});

describe("what the two acts on the tab row carry", () => {
  const line = (name: string, verdict: "gone" | "current"): Declaration => ({
    section: "checks",
    name,
    key: "run",
    run: `pnpm ${name}`,
    drift: verdict === "gone" ? { verdict, missing: ["package.json: scripts.x"] } : { verdict, checked: 1 },
    unfollowed: [],
  });
  const read = (declarations: Declaration[]): ManifestDriftRead => ({
    state: "read",
    drift: { path: "/r/armada.yml", checkout: "/r", declarations },
  });

  it("says nothing has gone before the read answers, rather than nothing is gone", () => {
    expect(driftGoneOf({ state: "reading" }).size).toBe(0);
    expect(driftGoneOf({ state: "none" }).size).toBe(0);
  });

  it("names the entries whose own line went, and only those", () => {
    expect([...driftGoneOf(read([line("test", "gone"), line("fmt", "current")]))]).toEqual(["test"]);
  });

  it("says nothing about a Verify nobody has run in this checkout", () => {
    expect(verifySaidOf(inputs(SHEET))).toBeUndefined();
  });

  it("says a Verify is out while it is, so pressing is not how you find out", () => {
    const verify: CheckoutVerify = {
      id: "01V",
      started_at: "2026-09-12T14:00:00Z",
      steps: [{ group: "checks", name: "build", run: "cargo build", state: "running", run_id: "r1" }],
    };
    expect(verifySaidOf(inputs({ ...SHEET, verify }))).toBe("running");
  });

  it("counts what an ended Verify ran, short, and forgets one a person put away", () => {
    const verify: CheckoutVerify = {
      id: "01V",
      started_at: "2026-09-12T14:00:00Z",
      ended_at: "2026-09-12T14:01:00Z",
      steps: [
        { group: "checks", name: "build", run: "cargo build", state: "ran", record: record("build", 0) },
        { group: "checks", name: "test", run: "cargo test", state: "not_run", why: "setup `bootstrap` exited 1" },
      ],
    };
    expect(verifySaidOf(inputs({ ...SHEET, verify }))).toBe("ran 1 of 2");
    // Put away, the act has nothing to open onto and says so by saying nothing.
    expect(verifySaidOf(inputs({ ...SHEET, verify }, { dismissed: "01V" }))).toBeUndefined();
  });
});
