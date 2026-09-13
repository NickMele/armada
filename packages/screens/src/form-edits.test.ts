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
        expect_exit_code: 101,
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
  base: "main",
  evidence: {
    serve: "pnpm storybook",
    ready: "curl -sf localhost:6006",
    run: "pnpm exec playwright test {}",
    frames: ".armada/frames",
    never: ["/__notes"],
  },
  setup_requires: ["bootstrap"],
  quiet_after_seconds: 300,
  poke_limit: 2,
  exclude_paths: ["target"],
};

describe("a draft", () => {
  it("sends nothing where nothing was touched, cap precision included", () => {
    expect(editsOf(DECLARED, draftOf(DECLARED))).toEqual([]);
  });

  it("adds a Check with only the keys it declares", () => {
    const draft = draftOf(DECLARED);
    draft.checks.push({ name: "clippy", run: " cargo clippy ", expectExitCode: "", requires: [], when: "\n", narrow: null });
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

describe("the rest of the schema", () => {
  it("sets the base, each evidence key, both lists, the dials and an exit code", () => {
    const draft = draftOf(DECLARED);
    draft.checks[0]!.expectExitCode = "";
    draft.base = " ";
    const evidence = draft.evidence!;
    evidence.serve = "";
    evidence.ready = "";
    evidence.frames = " .armada/shots ";
    evidence.never = "/__notes\n/settings";
    draft.afterMerge = ["build"];
    draft.setup = [];
    draft.quietAfter = "600";
    draft.pokeLimit = "0";
    draft.excludePaths = "";
    expect(editsOf(DECLARED, draft)).toEqual([
      { edit: "set_check_expect_exit_code", name: "build", expect_exit_code: 0 },
      { edit: "set_base", base: null },
      { edit: "set_evidence_serve", serve: null },
      { edit: "set_evidence_ready", ready: null },
      { edit: "set_evidence_frames", frames: ".armada/shots" },
      { edit: "set_evidence_never", never: ["/__notes", "/settings"] },
      { edit: "set_after_merge_checks", checks: ["build"] },
      { edit: "set_setup_requires", requires: [] },
      { edit: "set_quiet_after_seconds", quiet_after_seconds: 600 },
      { edit: "set_poke_limit", poke_limit: 0 },
      { edit: "set_exclude_paths", exclude_paths: [] },
    ]);
  });

  it("removes evidence first, and declares it with only the keys typed", () => {
    const off = draftOf(DECLARED);
    off.evidence = null;
    off.commands = off.commands.filter((command) => command.name !== "clean");
    expect(editsOf(DECLARED, off)).toEqual([{ edit: "remove_command", name: "clean" }, { edit: "remove_evidence" }]);

    const { evidence: _gone, ...without } = DECLARED;
    const on = draftOf(without);
    on.evidence = { serve: "", ready: "", run: " node capture.js {} ", frames: "out", never: "" };
    expect(editsOf(without, on)).toEqual([
      { edit: "add_evidence", evidence: { run: "node capture.js {}", frames: "out" } },
    ]);
  });

  it("declares a Check with its exit code", () => {
    const draft = draftOf(DECLARED);
    draft.checks.push({ name: "flaky", run: "make flaky", expectExitCode: "1", requires: [], when: "", narrow: null });
    expect(editsOf(DECLARED, draft)).toEqual([
      { edit: "add_check", name: "flaky", check: { run: "make flaky", expect_exit_code: 1 } },
    ]);
  });

  it("keeps Save back for a value the form can tell is wrong, keyed where the file spells it", () => {
    const draft = draftOf(DECLARED);
    draft.checks[0]!.expectExitCode = "1.5";
    draft.evidence = { serve: "pnpm storybook", ready: "", run: "no spec", frames: "", never: "" };
    draft.quietAfter = "0";
    draft.pokeLimit = "-1";
    const problems = problemsOf(draft);
    expect(Object.keys(problems).sort()).toEqual([
      "checks.build.expect_exit_code",
      "drone.poke_limit",
      "drone.quiet_after_seconds",
      "evidence.frames",
      "evidence.ready",
      "evidence.run",
    ]);
  });

  it("leaves a name another section no longer declares to Fleet, as a Check's requires is", () => {
    const draft = draftOf(DECLARED);
    draft.commands = draft.commands.filter((command) => command.name !== "bootstrap");
    draft.afterMerge = ["gone"];
    expect(problemsOf(draft)).toEqual({});
  });
});

describe("what keeps Save back", () => {
  it("is nothing for a file as it was declared", () => {
    expect(problemsOf(draftOf(DECLARED))).toEqual({});
  });

  it("names each field the form can tell is wrong, by where the file spells it", () => {
    const draft = draftOf(DECLARED);
    draft.checks.push({ name: "clippy", run: "", expectExitCode: "", requires: [], when: "", narrow: null });
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
