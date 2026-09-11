// The verdict sheet's own data, tested as the answer it is.
//
// **Three-way logic, so it is tested at three values.** `neverDelivers` and
// `neverAsksAPerson` each answer `true`, `false` or `undefined`, and a case
// that only checked the two-value shortcut would have missed the one the
// wire actually calls "cannot say".

import { describe, expect, it, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { JobDetail as JobWhole, PullRequestDetail, StepDetail, Submitted } from "@armada/protocol";

import { mount, unmount } from "./mounted";
import {
  briefOf,
  cameBackOf,
  currencyLineOf,
  leftAloneOf,
  neverAsksAPerson,
  neverDelivers,
  provesItNoteOf,
  provesItOf,
  pullRequestBlockOf,
  risksOf,
} from "./verdict";

function step(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "land",
    label: "Land",
    ordinal: 3,
    state: "awaiting_human",
    check_runs: [],
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [],
    verdicts: [],
    entered_at: "2026-09-09T09:30:00Z",
    updated_at: "2026-09-09T09:41:00Z",
    ...over,
  };
}

/** A wall clock reading, for the one row that elapses while it runs. */
const NOW = Date.parse("2026-09-09T09:45:00Z");

describe("whether a workflow ever delivers", () => {
  it("is true where every step says it does not deliver", () => {
    expect(neverDelivers([step({ delivers: false }), step({ delivers: false })])).toBe(true);
  });

  it("is false where any step delivers", () => {
    expect(neverDelivers([step({ delivers: false }), step({ delivers: true })])).toBe(false);
  });

  it("is undefined where a step cannot say and none delivers", () => {
    expect(neverDelivers([step({ delivers: false }), step({ delivers: undefined })])).toBeUndefined();
  });

  it("is undefined on a Job with no steps", () => {
    expect(neverDelivers([])).toBeUndefined();
  });
});

describe("whether a workflow ever stops for a person", () => {
  it("is false where any step gates on a person", () => {
    expect(
      neverAsksAPerson([step({ advance_gate: "auto" }), step({ advance_gate: "human_always" })]),
    ).toBe(false);
  });

  it("is true where every step's gate is known and none is human", () => {
    expect(
      neverAsksAPerson([step({ advance_gate: "auto" }), step({ advance_gate: "auto_if_judge_passes" })]),
    ).toBe(true);
  });

  it("is undefined where a step's gate cannot be said and none is human", () => {
    expect(neverAsksAPerson([step({ advance_gate: "auto" }), step({ advance_gate: undefined })])).toBeUndefined();
  });
});

describe("what proves it", () => {
  it("surfaces a mechanical check with no declared counterpart", () => {
    const rows = provesItOf(
      step({ check_runs: [{ attempt: 1, name: "artifact_exists", outcome: "passed" }] }),
      [],
      NOW,
    );
    expect(rows[0]?.identifier).toBe("artifact_exists");
    expect(rows[0]?.named).toBe("passed");
  });

  it("carries a declared Check and an undeclared one together", () => {
    const rows = provesItOf(
      step({
        checks: [{ kind: "manifest_check", name: "build" }],
        check_runs: [
          { attempt: 1, name: "build", outcome: "passed" },
          { attempt: 1, name: "artifact_exists", outcome: "passed" },
        ],
      }),
      [],
      NOW,
    );
    expect(rows.map((row) => row.identifier).sort()).toEqual(["artifact_exists", "build"]);
  });

  it("adds the 'nothing checked' row where the only tier is the deliverable's existence", () => {
    const rows = provesItOf(
      step({ check_runs: [{ attempt: 1, name: "artifact_exists", outcome: "passed" }] }),
      [],
      NOW,
    );
    const nothing = rows.find((row) => row.says === "Nothing checked what it says");
    expect(nothing?.identifier).toBe("no Judge · no Checks");
  });

  it("does not add the row where a Judge is declared", () => {
    const rows = provesItOf(
      step({
        check_runs: [{ attempt: 1, name: "artifact_exists", outcome: "passed" }],
        judge_checks: [{ criteria: 1, gaming_check: false }],
        judged: [{ attempt: 1, criterion_id: "c1", verdict: "met" }],
      }),
      [{ criterion_id: "c1", text: "The draft names a limit.", source: "brief" }],
      NOW,
    );
    expect(rows.some((row) => row.says === "Nothing checked what it says")).toBe(false);
  });

  it("does not add the row where a real Check is declared beside artifact_exists", () => {
    const rows = provesItOf(
      step({
        checks: [{ kind: "manifest_check", name: "build" }],
        check_runs: [
          { attempt: 1, name: "build", outcome: "passed" },
          { attempt: 1, name: "artifact_exists", outcome: "passed" },
        ],
      }),
      [],
      NOW,
    );
    expect(rows.some((row) => row.says === "Nothing checked what it says")).toBe(false);
  });

  it("does not add the row where the step has no checks at all", () => {
    const rows = provesItOf(step(), [], NOW);
    expect(rows.some((row) => row.says === "Nothing checked what it says")).toBe(false);
  });
});

describe("the line under the checklist", () => {
  it("names the missing workflow where nothing on the step can say", () => {
    expect(provesItNoteOf(step({ checks: undefined, judge_checks: undefined }), "reviewing")).toMatch(
      /cannot say what gates this step/,
    );
  });

  it("says the answer is the only verdict, at a gate with no Judge", () => {
    expect(
      provesItNoteOf(
        step({ check_runs: [{ attempt: 1, name: "artifact_exists", outcome: "passed" }] }),
        "reviewing",
      ),
    ).toMatch(/is the review/);
  });

  it("says nobody was asked, once the Job is over", () => {
    expect(
      provesItNoteOf(
        step({ check_runs: [{ attempt: 1, name: "artifact_exists", outcome: "passed" }] }),
        "finished",
      ),
    ).toMatch(/advanced on its own/);
  });

  it("says nothing where a Judge is declared", () => {
    expect(
      provesItNoteOf(step({ judge_checks: [{ criteria: 1, gaming_check: false }] }), "reviewing"),
    ).toBeUndefined();
  });
});

describe("what came back, and what it left alone", () => {
  const claim: Submitted = {
    step_id: "land",
    evidence_type: "diff",
    claimed: "The branch is pushed and the pull request is open.",
    shown_by: "pull request #4711",
    not_claimed: "Nothing about the after-merge Checks.",
  };

  it("reads the Drone's own claim", () => {
    expect(cameBackOf(claim)).toBe(claim.claimed);
  });

  it("says a step has not submitted yet, where there is no claim", () => {
    expect(cameBackOf(undefined)).toMatch(/has not submitted/);
  });

  it("reads not_claimed where the submission drew a boundary", () => {
    expect(leftAloneOf(claim)).toBe(claim.not_claimed);
  });

  it("names the absence where the submission drew none", () => {
    expect(leftAloneOf({ ...claim, not_claimed: undefined })).toMatch(/drew no boundary/);
  });
});

/** A `whole` fixture carrying only the served review — the one field these two read. */
function withReview(review: JobWhole["review"]): JobWhole {
  return { review } as JobWhole;
}

describe("the served review — one builder, issue 665", () => {
  it("reads the brief Fleet composed, with the pull request's Markdown stripped", () => {
    const whole = withReview({
      why: "The **export** button the brief asked for, over `src/export.ts`.",
      outcome: "",
      risks: "",
      evidence: "",
    });
    expect(briefOf(whole)).toBe("The export button the brief asked for, over src/export.ts.");
  });

  it("is absent where Fleet has composed no review yet", () => {
    expect(briefOf(withReview(undefined))).toBeUndefined();
    expect(briefOf(null)).toBeUndefined();
  });

  it("reads the risks Fleet composed, trimmed and with Markdown stripped", () => {
    const whole = withReview({
      why: "",
      outcome: "",
      risks: "Every line below is something Fleet ran.\n\n",
      evidence: "",
    });
    expect(risksOf(whole)).toBe("Every line below is something Fleet ran.");
  });

  it("is absent where the review carries nothing to say", () => {
    expect(risksOf(withReview(undefined))).toBeUndefined();
  });
});

describe("what proves it, where the step carries an overruled verdict", () => {
  function overriddenStep(): StepDetail {
    return step({
      overridden: true,
      judge_checks: [{ criteria: 1, gaming_check: true }],
      judged: [
        { attempt: 1, criterion_id: "c1", verdict: "met" },
        {
          attempt: 1,
          criterion_id: "c2",
          verdict: "not_met",
          expected: "The step should not touch crates/ipc/operations.toml.",
        },
      ],
    });
  }

  it("draws the judge row overruled, naming the finding and the person's own words", () => {
    const rows = provesItOf(overriddenStep(), [], NOW, undefined, "The note is correct and needed.");
    const judge = rows.find((row) => row.named === "overruled");
    expect(judge?.identifier).toMatch(/overruled by you$/);
    expect(judge?.detail).toMatch(/The step should not touch crates\/ipc\/operations\.toml\./);
    expect(judge?.detail).toMatch(/You: “The note is correct and needed\.”/);
  });

  it("draws the finding alone where the log kept no reason", () => {
    const rows = provesItOf(overriddenStep(), [], NOW);
    const judge = rows.find((row) => row.named === "overruled");
    expect(judge?.detail).toBe("Not met: The step should not touch crates/ipc/operations.toml.");
  });

  it("draws the ordinary refused row where the step was not overridden", () => {
    const rows = provesItOf({ ...overriddenStep(), overridden: false }, [], NOW);
    expect(rows.some((row) => row.named === "overruled")).toBe(false);
    expect(rows.some((row) => row.named === "refused")).toBe(true);
  });
});

// `#663`: what the last attempt to keep a pull request's branch current
// against a moved base says. Plain words, settled 2026-09-11: no commit id,
// no "rebase", no "push", no "forge".
describe("the currency line a moved base leaves", () => {
  it("is undefined where the base has never moved", () => {
    expect(currencyLineOf(undefined, NOW)).toBeUndefined();
  });

  it("says the branch is up to date, with a relative time in whole units, where it caught up cleanly", () => {
    const said = currencyLineOf(
      {
        rebased_onto: "8c2ce681000000000000000000000000000000",
        rebased_at: "2026-09-09T09:40:00Z",
        conflict_files: [],
      },
      NOW,
    );
    expect(said?.conflicted).toBe(false);
    expect(said?.said).toBe("Up to date with main, checked 5 minutes ago.");
  });

  it("says 'just now' rather than 'checked under a minute ago'", () => {
    const said = currencyLineOf(
      {
        rebased_onto: "8c2ce681000000000000000000000000000000",
        rebased_at: "2026-09-09T09:44:45Z",
        conflict_files: [],
      },
      NOW,
    );
    expect(said?.said).toBe("Up to date with main, checked just now.");
  });

  it("says 'just now' rather than 'ago' where the timestamp will not parse", () => {
    const said = currencyLineOf(
      {
        rebased_onto: "8c2ce681000000000000000000000000000000",
        rebased_at: "not a date",
        conflict_files: [],
      },
      NOW,
    );
    expect(said?.said).toBe("Up to date with main, checked just now.");
  });

  it("names the files and says the branch was left as it was where it clashed", () => {
    const said = currencyLineOf(
      {
        rebased_onto: "8c2ce681000000000000000000000000000000",
        rebased_at: "2026-09-09T09:40:00Z",
        conflict_files: ["src/parse.rs", "src/lex.rs"],
      },
      NOW,
    );
    expect(said?.conflicted).toBe(true);
    expect(said?.said).toBe(
      "Main has changes that clash with this branch in src/parse.rs, src/lex.rs. Fleet left the branch as it was.",
    );
  });

  it("is not conflicted where `conflict_files` is absent", () => {
    const said = currencyLineOf(
      {
        rebased_onto: "8c2ce681000000000000000000000000000000",
        rebased_at: "2026-09-09T09:40:00Z",
      },
      NOW,
    );
    expect(said?.conflicted).toBe(false);
  });
});

// `#663`: the control sits with the pull request block, directly under the
// sentence naming the clash — never after the decision card, and never where
// nothing has clashed.
describe("the pull request block's own resolve-conflicts control", () => {
  const ADDRESS = "https://forge.example/armada/pull/533";
  const now = Date.parse("2026-09-09T09:45:00Z");

  function detail(currency: PullRequestDetail["currency"]): PullRequestDetail {
    return { number: 533, reviews: [], currency };
  }

  test("is offered under the clash sentence, and sends on the press", async () => {
    const sent: string[] = [];
    mount(
      <>
        {pullRequestBlockOf(
          ADDRESS,
          detail({
            rebased_onto: "8c2ce681000000000000000000000000000000",
            rebased_at: "2026-09-09T09:40:00Z",
            conflict_files: ["src/parse.rs"],
          }),
          now,
          undefined,
          () => sent.push("resolved"),
        )}
      </>,
    );
    await expect
      .element(page.getByText("Main has changes that clash with this branch in src/parse.rs."))
      .toBeVisible();
    await userEvent.click(page.getByRole("button", { name: "Resolve conflicts" }));
    expect(sent).toEqual(["resolved"]);
    unmount();
  });

  test("is not offered where the branch is current", async () => {
    mount(
      <>
        {pullRequestBlockOf(
          ADDRESS,
          detail({
            rebased_onto: "8c2ce681000000000000000000000000000000",
            rebased_at: "2026-09-09T09:40:00Z",
          }),
          now,
          undefined,
          () => {},
        )}
      </>,
    );
    await expect
      .element(page.getByRole("button", { name: "Resolve conflicts" }))
      .not.toBeInTheDocument();
    unmount();
  });

  test("is not offered where the base has never moved, even with a handler given", async () => {
    mount(<>{pullRequestBlockOf(ADDRESS, detail(undefined), now, undefined, () => {})}</>);
    await expect
      .element(page.getByRole("button", { name: "Resolve conflicts" }))
      .not.toBeInTheDocument();
    unmount();
  });
});
