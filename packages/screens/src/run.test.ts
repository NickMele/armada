// The run tree's own facts, on the Job `#633` was filed from — a tests step
// that stopped `gate_undecided` at attempt 1: every Check passed and the
// Judge timed out.

import { describe, expect, it } from "vitest";

import type { JobDetail as JobWhole, StepDetail } from "@armada/protocol";
import { runOf } from "./run";

const NOW = Date.parse("2026-09-11T10:00:00Z");

/**
 * The tests step of `2-refuse-a-merge-press-whose-chosen-comments`: eight
 * Checks declared behind Fleet's sweep marker, all eight advanced, one
 * criterion declared and none judged — the gate stopped before the panel
 * answered.
 */
function step(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "tests",
    label: "Tests",
    ordinal: 2,
    state: "stopped",
    checks: [
      { kind: "every_manifest_check" },
      { kind: "manifest_check", name: "build", run: "cargo build" },
      { kind: "manifest_check", name: "test", run: "cargo test" },
      { kind: "manifest_check", name: "typecheck", run: "tsc --noEmit" },
      { kind: "manifest_check", name: "bridge_build", run: "pnpm build" },
      { kind: "manifest_check", name: "storybook", run: "pnpm build-storybook" },
      { kind: "manifest_check", name: "bridge_test", run: "pnpm test" },
      { kind: "manifest_check", name: "format", run: "cargo fmt --check" },
      { kind: "diff_nonempty" },
    ],
    check_runs: [
      { attempt: 1, name: "build", outcome: "passed" },
      { attempt: 1, name: "test", outcome: "passed" },
      { attempt: 1, name: "typecheck", outcome: "passed" },
      { attempt: 1, name: "bridge_build", outcome: "passed" },
      { attempt: 1, name: "storybook", outcome: "skipped", produced: "no changed file is under packages/" },
      { attempt: 1, name: "bridge_test", outcome: "skipped", produced: "no changed file is under packages/" },
      { attempt: 1, name: "format", outcome: "skipped", produced: "no changed file is under crates/" },
      { attempt: 1, name: "diff_nonempty", outcome: "passed" },
    ],
    judge_checks: [{ criteria: 1, gaming_check: false }],
    advance_gate: "auto_if_judge_passes",
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [
      { attempt: 1, outcome: "stopped", why: "gate_undecided", started_at: "2026-09-11T09:30:00Z" },
    ],
    verdicts: [{ attempt: 1, named: "failed", trigger: "gate_undecided" }],
    last_verdict: { attempt: 1, named: "failed", trigger: "gate_undecided" },
    entered_at: "2026-09-11T09:30:00Z",
    updated_at: "2026-09-11T09:41:00Z",
    ...over,
  };
}

function whole(showing: StepDetail): JobWhole {
  return {
    job: {
      id: "01M22TYSAE0023MADDP5ZQEYGW",
      handle: "2-refuse-a-merge-press-whose-chosen-comments",
      title: "Refuse a merge press whose chosen comments",
      status: "escalated",
      workflow_id: "feature",
      owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
      origin: "dispatched",
      urgency: "normal",
      atomic: false,
      model: "sonnet",
      created_at: "2026-09-11T09:00:00Z",
    },
    created_at: "2026-09-11T09:00:00Z",
    steps: [showing],
    acceptance_criteria: [],
    dependencies: [],
  };
}

/** The step's own facts, by label — what `runOf` draws under the row. */
function factsOf(showing: StepDetail) {
  const [drawn] = runOf(whole(showing), NOW, showing.step_id, []);
  return new Map((drawn?.facts ?? []).map((fact) => [fact.label, fact.value]));
}

describe("what held a step that stopped gate_undecided", () => {
  it("never says retries are spent on a first attempt the gate declined to rule on", () => {
    // The whole point of #633: attempt one, and the old sentence read
    // "retries spent" regardless.
    expect(factsOf(step()).get("Held")).toBe("the gate could not decide · waiting on you");
  });

  it("says retries spent where gate_failure actually spent them", () => {
    const attempts = [
      { attempt: 1, outcome: "retrying", why: "gate_failure", started_at: "2026-09-11T09:00:00Z" },
      { attempt: 2, outcome: "stopped", why: "gate_failure", started_at: "2026-09-11T09:20:00Z" },
    ];
    const verdicts = [{ attempt: 2, named: "failed", trigger: "gate_failure" }];
    expect(factsOf(step({ attempts, verdicts })).get("Held")).toBe("retries spent · waiting on you");
  });
});

describe("the Checks tier on a step Fleet gates on everything it declares", () => {
  it("counts only the Checks, never the sweep marker ahead of them", () => {
    // Nine declared entries on the wire, one of them the marker with no name
    // and no run — the denominator is the eight that can actually run.
    expect(factsOf(step()).get("Checks")).toBe("8 of 8 passed");
  });
});

/**
 * The `implement` step of `4-make-permission-hold-injectable-and-prove-l`, on
 * the afternoon this was reported: attempt 1 held for a person with two
 * criteria refused, attempt 2 handed back with `test` failing, attempt 3
 * thirty seconds old and its gate not reached. The tree drew all three of
 * those at once — Checks from attempt 2, Judge from attempt 1, Verdict from
 * attempt 2 — in red, over a step whose own mark was running.
 */
function retrying(over: Partial<StepDetail> = {}): StepDetail {
  return step({
    state: "running",
    check_runs: [
      { attempt: 1, name: "build", outcome: "passed" },
      { attempt: 1, name: "test", outcome: "passed" },
      { attempt: 2, name: "build", outcome: "passed" },
      { attempt: 2, name: "test", outcome: "failed", produced: "it exited 100" },
    ],
    judged: [
      {
        attempt: 1,
        criterion_id: "restructures_without_changing",
        verdict: "not_met",
        cited: [],
      },
    ],
    attempts: [
      { attempt: 1, outcome: "awaiting_human", started_at: "2026-09-12T05:10:58Z", ended_at: "2026-09-12T05:30:07Z" },
      { attempt: 2, outcome: "retrying", why: "gate_failure", started_at: "2026-09-12T18:01:29Z", ended_at: "2026-09-12T18:06:02Z" },
      { attempt: 3, outcome: "running", started_at: "2026-09-12T18:06:02Z" },
    ],
    verdicts: [{ attempt: 2, named: "failed", trigger: "gate_failure" }],
    last_verdict: { attempt: 2, named: "failed", trigger: "gate_failure" },
    ...over,
  });
}

describe("a step being worked again", () => {
  it("reads its Checks off the live attempt, which has not reached them", () => {
    expect(factsOf(retrying()).get("Checks")).toBe("not reached");
  });

  it("reads its Judge off the live attempt, which nobody has asked", () => {
    expect(factsOf(retrying()).get("Judge")).toBe("1 criterion, not asked");
  });

  it("says the Judge is being asked while the call is out", () => {
    const judging = {
      look: "criterion",
      model: "sonnet",
      call: 1,
      of: 2,
      since: "2026-09-12T18:10:48Z",
      budget_ms: 120000,
    };
    expect(factsOf(retrying({ judging })).get("Judge")).toBe("asking · 1 criterion");
  });

  it("draws no verdict, because the one on the wire ruled the attempt before", () => {
    expect(factsOf(retrying()).has("Verdict")).toBe(false);
  });

  it("records what the attempt before came to, and never as this step's own state", () => {
    const facts = runOf(whole(retrying()), NOW, "tests", [])[0]?.facts ?? [];
    const before = facts.find((fact) => fact.label === "Attempt 2");
    expect(before?.value).toBe("stopped at the gate");
    // Neutral. A hue here is the screen saying the step is failing right now.
    expect(before?.named).toBeUndefined();
    expect(facts.every((fact) => fact.named !== "failed")).toBe(true);
  });

  it("still reads the newest attempt that answered once the step is at rest", () => {
    // An overrule advances a step whose Checks are the attempt before's, and
    // `gates.ts` has drawn them there all along.
    const rest = retrying({ state: "awaiting_human" });
    expect(factsOf(rest).get("Checks")).toBe("test failed");
    expect(factsOf(rest).get("Verdict")).toBe("stopped at the gate");
  });
});

/**
 * `#813`: the window between a Drone handing in and the gate starting. It used
 * to draw as a step nothing had reached — `job.checking` is the next message
 * and it can be minutes away.
 */
describe("the moment a Drone handed in", () => {
  const handed = {
    job_id: "01M22TYSAE0023MADDP5ZQEYGW",
    step_id: "tests",
    evidence_type: "diff",
    actor: "drone",
    at: "2026-09-11T09:58:00Z",
  };

  function factsWith(showing: StepDetail, moment = handed) {
    const [drawn] = runOf(whole(showing), NOW, showing.step_id, [], moment);
    return new Map((drawn?.facts ?? []).map((fact) => [fact.label, fact.value]));
  }

  it("says what landed and how long it has been waiting", () => {
    const working = step({ state: "running", checking: undefined });
    expect(factsWith(working).get("Handed in")).toBe("a diff \u00b7 2m 00s ago");
  });

  it("stands down the moment the gate starts, which reads the same window", () => {
    const checking = step({
      state: "running",
      checking: { attempt: 1, checks: [{ name: "build" }] },
    });
    expect(factsWith(checking).has("Handed in")).toBe(false);
  });

  it("is not drawn on a step that is no longer being worked", () => {
    expect(factsWith(step()).has("Handed in")).toBe(false);
  });

  it("belongs to the step it names and no other", () => {
    const working = step({ state: "running", checking: undefined });
    expect(factsWith(working, { ...handed, step_id: "implement" }).has("Handed in")).toBe(false);
  });
});

describe("a step that sends the work back", () => {
  it("says which pass it is on, of how many its workflow allows", () => {
    const facts = factsOf(
      step({ step_id: "review", label: "Review the change", state: "awaiting_human", pass: { number: 2, of: 3 } }),
    );
    expect(facts.get("Pass")).toBe("2 of 3");
  });

  it("says nothing about passes on a step that closes no loop", () => {
    expect(factsOf(step()).has("Pass")).toBe(false);
  });

  it("says nothing about passes before the step has started", () => {
    const facts = factsOf(step({ state: "not_started", pass: { number: 1, of: 3 } }));
    expect(facts.has("Pass")).toBe(false);
  });
});

// #1062 — a Drone's own run of the Checks, on the rail, never read as the gate's.
describe("a Drone's own run of the Checks", () => {
  const running = { attempt: 1, checks: [{ name: "build", started_at: "2026-09-11T09:59:00Z" }] };

  it("is the Checks fact until the gate takes the step, marked as the Drone's", () => {
    const asking = step({ state: "running", check_runs: [], dry_run: running });
    const facts = runOf(whole(asking), NOW, "tests", [])[0]?.facts ?? [];
    const checks = facts.find((fact) => fact.label === "Checks");
    expect(checks?.value).toBe("The Drone's run · 1 running");
    expect(checks?.named).toBe("running");
  });

  it("gives way to the gate's own run", () => {
    const both = step({ state: "running", check_runs: [], dry_run: running, checking: running });
    expect(factsOf(both).get("Checks")).toBe("1 running");
  });
});
