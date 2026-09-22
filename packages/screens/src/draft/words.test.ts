// Every draft word is a real token, and no draft word is a second copy.

import {
  ADVANCE_GATE,
  JOB_STATUS,
  STEP_STATE,
} from "@armada/components/src/generated/vocabulary";
import { describe, expect, it } from "vitest";

import {
  CASE_RUN_OUTCOME_WORDS,
  CLASSIFYING_WORD,
  CRITERION_NO_VERDICT_WORD,
  DRAFT_VOCABULARIES,
  GROUP_STATE_WORDS,
  TASK_STATE_WORDS,
} from "./words";

/** The tokens `packages/tokens/src/status.css` defines, by the stem it names. */
const STATUS_TOKENS = [
  "not-started",
  "running",
  "awaiting-review",
  "awaiting-approval",
  "escalated",
  "completed-success",
  "completed-failed",
  "rejected",
  "killed",
  "piloted",
  "awaiting-attestation",
];

describe("every draft word renders from the token scale", () => {
  it("names a status token that exists, and never a hex", () => {
    for (const { vocabulary, words } of DRAFT_VOCABULARIES) {
      for (const [value, word] of Object.entries(words)) {
        expect(
          STATUS_TOKENS.includes(word.badgeStatus ?? ""),
          `${vocabulary}.${value} names ${word.badgeStatus}`,
        ).toBe(true);
        expect(word.statusToken).toBe(`--status-${word.badgeStatus}`);
      }
    }
  });

  it("gives every value a verb, so nothing renders as a wire spelling", () => {
    for (const { words } of DRAFT_VOCABULARIES) {
      for (const word of Object.values(words)) {
        expect(word.verb?.length ?? 0).toBeGreaterThan(0);
      }
    }
  });

  it("carries no glyph, because none of these is in the icon registry", () => {
    for (const { words } of DRAFT_VOCABULARIES) {
      for (const word of Object.values(words)) {
        expect("icon" in word).toBe(false);
      }
    }
  });
});

describe("what the registry already answers is not restated here", () => {
  it("leaves the gate words to the generated vocabulary", () => {
    expect(ADVANCE_GATE["manifest_rule:auto_merge"]?.verb).toBeTruthy();
    for (const { vocabulary } of DRAFT_VOCABULARIES) {
      expect(vocabulary).not.toBe("advance_gate");
    }
  });

  it("spells classifying, which the registry knows only as another word", () => {
    expect(JOB_STATUS["classifying"]).toBeUndefined();
    expect(CLASSIFYING_WORD.verb).toBe("classifying");
  });
});

describe("the values that have no registry row at all", () => {
  // `running` and `retrying` are spelled the same in `step-states.toml`, and
  // that is a word in common rather than a meaning: a group running is not the
  // step running, and a group can be `running` inside a step that is
  // `awaiting_human`. The other six have no row anywhere.
  it("covers the six group states no registry spells at all", () => {
    const unknown = Object.keys(GROUP_STATE_WORDS).filter(
      (state) => STEP_STATE[state] === undefined,
    );

    expect(unknown).toEqual([
      "pending",
      "joining",
      "checking",
      "passed",
      "failed",
      "landed",
    ]);
  });

  it("gives failed a word, which is the one task state the wire cannot send", () => {
    expect(TASK_STATE_WORDS.failed.verb).toBe("failed");
  });

  it("calls a case that did not run not covered, never passing", () => {
    expect(CASE_RUN_OUTCOME_WORDS.not_run.verb).toBe("not covered");
    expect(CASE_RUN_OUTCOME_WORDS.not_run.badgeStatus).not.toBe("completed-success");
  });

  // Two facts, two sentences: a case with no spec has nothing to run, and a
  // criterion with no verdict was never answered in the record.
  it("keeps an unanswered criterion apart from a case with no spec", () => {
    expect(CRITERION_NO_VERDICT_WORD.verb).toBe("no verdict recorded");
    expect(CRITERION_NO_VERDICT_WORD.verb).not.toBe(CASE_RUN_OUTCOME_WORDS.not_run.verb);
    expect(CRITERION_NO_VERDICT_WORD.badgeStatus).not.toBe("completed-success");
  });
});

describe("the list #1545 reads", () => {
  it("names every map in this module, so none is promoted by being forgotten", () => {
    expect(DRAFT_VOCABULARIES.map((entry) => entry.vocabulary)).toEqual([
      "task_state",
      "group_state",
      "case_run_outcome",
      "case_state",
      "job_status",
      "criterion_reading",
    ]);
  });
});
