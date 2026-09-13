// A form's draft, sent as edits. The claim under every case: an untouched form
// sends nothing, and a touched one sends only the keys that moved.

import { describe, expect, it } from "vitest";
import type { ManifestDeclared } from "@armada/protocol";

import { budgetWarningsOf, draftOf, editsOf, problemsOf } from "./form-edits";

const DECLARED: ManifestDeclared = {
  checks: [
    {
      name: "build",
      check: {
        run: "cargo build --workspace --locked",
        narrow: { run: "cargo build --locked", each: "-p {}", under: "crates" },
      },
    },
    { name: "bridge_test", check: { run: "pnpm bridge-test", requires: ["bootstrap"], when: ["packages/**", "apps/**"] } },
  ],
  commands: [
    { name: "bootstrap", command: { run: "pnpm install --frozen-lockfile", destructive: false } },
    { name: "clean", command: { run: "cargo clean", destructive: true } },
    {
      name: "storybook_dev",
      command: {
        destructive: false,
        serve: "pnpm storybook",
        ready: "curl -sf localhost:6006",
        links: [{ url: "http://localhost:6006", name: "Storybook" }],
      },
    },
  ],
  ports: [{ name: "web", port: { container: 3000, env: "PORT" } }],
  auto_merge: { written: "never", offered: ["never", "checks-pass", "always"] },
  review_gate: { written: "human_always", offered: ["human_always", "auto_if_judge_passes"] },
  cost_cap_micros_per_job: 1_234_567,
  turn_cap_per_job: 300,
};

describe("a draft", () => {
  it("sends nothing where nothing was touched, cap precision included", () => {
    expect(editsOf(DECLARED, draftOf(DECLARED))).toEqual([]);
  });

  it("adds a Check with only the keys it declares", () => {
    const draft = draftOf(DECLARED);
    draft.checks.push({ name: "clippy", run: " cargo clippy ", requires: [], when: "\n", narrow: null });
    expect(editsOf(DECLARED, draft)).toEqual([
      { edit: "add_check", name: "clippy", check: { run: "cargo clippy" } },
    ]);
  });

  it("edits a name removed and declared again in place, so its comment stays", () => {
    const draft = draftOf(DECLARED);
    draft.commands = draft.commands.filter((command) => command.name !== "clean");
    draft.commands.push({ name: "clean", run: "cargo clean -p fleet", destructive: true, serve: "", ready: "", links: [] });
    expect(editsOf(DECLARED, draft)).toEqual([
      { edit: "set_command_run", name: "clean", run: "cargo clean -p fleet" },
    ]);
  });

  it("removes before it adds", () => {
    const draft = draftOf(DECLARED);
    draft.ports = [{ name: "api", container: "8080", env: "" }];
    expect(editsOf(DECLARED, draft)).toEqual([
      { edit: "remove_port", name: "web" },
      { edit: "add_port", name: "api", port: { container: 8080 } },
    ]);
  });

  it("sets each Check key that moved, a list as its lines and a narrowing whole", () => {
    const draft = draftOf(DECLARED);
    const test = draft.checks[1]!;
    test.when = "packages/**\n\n  crates/ipc/**  ";
    test.requires = [];
    draft.checks[0]!.narrow = null;
    expect(editsOf(DECLARED, draft)).toEqual([
      { edit: "set_check_narrow", name: "build", narrow: null },
      { edit: "set_check_requires", name: "bridge_test", requires: [] },
      { edit: "set_check_when", name: "bridge_test", when: ["packages/**", "crates/ipc/**"] },
    ]);
  });

  it("marks a Command destructive, and clears a server's ready line to null", () => {
    const draft = draftOf(DECLARED);
    draft.commands[0]!.destructive = true;
    draft.commands[2]!.ready = "  ";
    draft.commands[2]!.links.push({ url: "", name: "" });
    expect(editsOf(DECLARED, draft)).toEqual([
      { edit: "set_command_destructive", name: "bootstrap", destructive: true },
      { edit: "set_command_ready", name: "storybook_dev", ready: null },
    ]);
  });

  it("sends a port, a policy word and both caps in the file's units", () => {
    const draft = draftOf(DECLARED);
    draft.ports[0]!.container = "";
    draft.autoMerge = "checks-pass";
    draft.costCap = "5";
    draft.turnCap = "";
    expect(editsOf(DECLARED, draft)).toEqual([
      { edit: "set_port_container", name: "web", container: null },
      { edit: "set_auto_merge", auto_merge: "checks-pass" },
      { edit: "set_cost_cap_micros_per_job", cost_cap_micros_per_job: 5_000_000 },
      { edit: "set_turn_cap_per_job", turn_cap_per_job: null },
    ]);
  });
});

describe("what keeps Save back", () => {
  it("is nothing for a file as it was declared", () => {
    expect(problemsOf(draftOf(DECLARED))).toEqual({});
  });

  it("names each field the form can tell is wrong, by where the file spells it", () => {
    const draft = draftOf(DECLARED);
    draft.checks.push({ name: "clippy", run: "", requires: [], when: "", narrow: null });
    draft.ports[0]!.container = "80a";
    draft.costCap = "-1";
    draft.turnCap = "1.5";
    expect(Object.keys(problemsOf(draft)).sort()).toEqual([
      "budget.cost",
      "budget.turns",
      "checks.clippy.run",
      "ports.web.container",
    ]);
  });
});

describe("the budget warning", () => {
  const SPEND = { jobs: 41, most_cost_micros: 7_120_000, most_turns: 212 };

  it("says where a cap is below the costliest past Job, and names what it cost", () => {
    const draft = { ...draftOf(DECLARED), costCap: "5", turnCap: "300" };
    const warnings = budgetWarningsOf(draft, SPEND);
    expect(warnings).toHaveLength(1);
    expect(warnings[0]).toContain("41 past Jobs");
    expect(warnings[0]).toContain("$7.12");
  });

  it("says nothing at or above it, for an empty cap, or where no Job has run", () => {
    expect(budgetWarningsOf({ ...draftOf(DECLARED), costCap: "7.12", turnCap: "212" }, SPEND)).toEqual([]);
    expect(budgetWarningsOf({ ...draftOf(DECLARED), costCap: "", turnCap: "" }, SPEND)).toEqual([]);
    expect(budgetWarningsOf({ ...draftOf(DECLARED), costCap: "1" }, { ...SPEND, jobs: 0 })).toEqual([]);
  });

  it("warns on turns apart from money", () => {
    const warnings = budgetWarningsOf({ ...draftOf(DECLARED), costCap: "", turnCap: "100" }, SPEND);
    expect(warnings).toEqual([expect.stringContaining("took 212 turns")]);
  });
});
