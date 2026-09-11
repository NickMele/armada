// Every fixture, proved two ways: `renderFor` lands where its name says it
// does, and `JobDetail` draws the whole thing without throwing.
//
// **`.test.ts`, not `.test.tsx`** — `vitest.config.ts` routes a file by what it
// imports, and this one calls `createElement` rather than writing JSX, so it
// stays on the node project alongside `chapters.test.ts` rather than paying for
// a browser to render markup nothing here looks at pixel by pixel.

import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { JobDetail } from "../JobDetail";
import { renderFor, type Render } from "../render";
import { FIXTURES } from "./build/index";
import { propsFor } from "./props";


/**
 * What `renderFor` should answer for each fixture's status, keyed by the
 * fixture's own name so a roster addition without an entry here fails loudly
 * rather than silently skipping the check.
 */
const EXPECTED_RENDER: Record<string, Render> = {
  "running — mid-step on Fix, a Check not yet run": "working",
  "awaiting_review — every Check passed, the Judge met every criterion": "reviewing",
  "escalated · gate_failure — a Check failed and ended the Job": "stopped",
  "queued — approved, waiting on a free drone": "working",
  "awaiting_approval — a person must approve the dispatch": "working",
  "awaiting_repair — the retry budget is spent and the work is unfinished": "stopped",
  "awaiting_attestation — the work landed, a criterion needs a person's action outside Armada":
    "working",
  "piloted — a person is working it now, and the drone is gone": "working",
  "escalated · blocked_by_policy — the drone reached for a command not on the allowlist": "stopped",
  "escalated · interrupted — fleet restarted mid-run and lost track of the drone": "stopped",
  "escalated · silent — the drone stopped writing and nothing explains why": "stopped",
  "escalated · loop_cap — the step hit its iteration cap": "stopped",
  "escalated · no_report — the drone's run ended and it never submitted": "stopped",
  "completed_success — every step advanced, every criterion verified": "finished",
  "completed_failed — a person accepted the failure as the outcome": "stopped",
  "rejected — a person declined the work at the human gate": "stopped",
  "killed — cleared from the Board, carrying no verdict": "stopped",
  "superseded — the work landed outside this Job, and there is nothing left for it to do":
    "stopped",
  "escalated · evidence_suspect — every Check passed, and the panel refused two criteria":
    "stopped",
  "running — before the first Drone turn, the worktree is being prepared": "working",
  "running — Fleet would not answer for this Job's own detail": "working",
  "running — a Check failed and the Drone is retrying, attempt 2 of 3": "working",
  "running — Regression check: nextest passed, the build Check is queued behind it": "working",
  "awaiting_review — the branch is pushed, a pull request is open, and the fourth answer appears":
    "reviewing",
};

describe("each fixture's own render", () => {
  for (const fixture of FIXTURES) {
    it(`${fixture.name} takes the render its name claims`, () => {
      const expected = EXPECTED_RENDER[fixture.name];
      expect(expected, `no expected render named for "${fixture.name}"`).toBeDefined();
      expect(renderFor(fixture.job)).toBe(expected);
    });
  }
});

describe("JobDetail on every fixture", () => {
  for (const fixture of FIXTURES) {
    it(`draws ${fixture.name} without throwing`, () => {
      const markup = renderToStaticMarkup(createElement(JobDetail, propsFor(fixture)));
      expect(markup.length).toBeGreaterThan(0);
    });
  }
});
