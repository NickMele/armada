// What the step's story builds, tested as the answer it is.
//
// **Rendered to markup rather than mounted.** `packages/screens` tests answers,
// and both defects here are structural: a brief that arrived as blocks and left
// as one paragraph, and a chapter that counted files and then reported on a
// document. Neither needs a browser.
//
// **The other half of #306 is a story, and it has to be.** `pre-wrap` is a
// declaration, so whether a reader sees the newlines is a question about a
// rendering — `DroneBrief.stories.tsx` reads it off the browser, and this file
// does not pretend to. What is here is the block boundary, which is arithmetic:
// a hundred briefs cost what one costs.

import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";

import type { Diff, Footprint, JobSummary, StepDetail, Turn } from "@armada/protocol";

// The grouping lives with the component that draws it, so the two cannot
// disagree about where a block stops. Tested here because this is the node
// runner: `packages/components` runs its stories in a browser, and a `play`
// that computed this would be a unit test paying a browser's price.
import { briefBlocks, widenIndent } from "@armada/components";

import { chaptersOf } from "./chapters";
// The phase strip, because the last describe in this file is the two surfaces
// against each other and there is no third place both are reachable from.
import { phasesOf, type Opens } from "./phases";

/** The brief as `crates/fleet/src/briefing.rs` writes it, three blocks of it. */
const BRIEF = [
  "JOB BRIEF",
  "",
  "Coalesce concurrent token refreshes",
  "",
  "This is done when:",
  "  - two refreshes in flight make one network call",
  "",
  "WHERE YOU ARE",
  "",
  "This task runs in 4 parts. You are on part 1.",
  "",
  "  1. Plan the change — you are here",
  "     STOP. Submit when this part is done, then wait.",
  "  2. Implement — not yours — do not do it",
  "",
  "STEP: Plan the change",
  "",
  "What you claim should be what the work now does, not that you finished.",
].join("\n");

function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01M130Y1380016YK5S0JXBXDQ5",
    title: "Coalesce concurrent token refreshes",
    status: "awaiting_review",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-09-02T09:00:00Z",
    assigned_drone: "01M1HHJ6XB001BZJZ4BE2XKY34",
    ...over,
  };
}

function step(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "plan",
    label: "Plan the change",
    ordinal: 1,
    state: "advanced",
    check_runs: [],
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [],
    entered_at: "2026-09-02T13:11:00Z",
    updated_at: "2026-09-02T13:18:00Z",
    ...over,
  };
}

/**
 * The turn Armada opened the step with, carrying the whole brief.
 *
 * `headings` is what Fleet stamps on the row — the line numbers it wrote the
 * block headings at. Omitted is a turn with no headed blocks, and a row written
 * before the field existed; the two are the same to a reader.
 */
function instructed(text: string, headings?: number[]): Turn {
  return {
    ts: "2026-09-02T13:11:00Z",
    seq: 1,
    step: "plan",
    by: "armada",
    saw: { event: "instructed", occasion: "opening", text, ...(headings ? { headings } : {}) },
  };
}

/** Where BRIEF's three block headings are, as Fleet would have stamped them. */
const HEADED = [0, 7, 15];

/** How a document is opened, stubbed. Shared, because both surfaces take it. */
const OPENS: Opens = {
  jobId: "01M130Y1380016YK5S0JXBXDQ5",
  open: async () => ({ ok: true }),
  onSaid: () => {},
};

/** The chapters, with everything the panel owns stubbed to a no-op. */
function chapters(over: { rows?: Turn[]; step?: StepDetail; transcript?: string } = {}) {
  return chaptersOf({
    job: job(),
    step: over.step ?? step(),
    render: "reviewing",
    watching: { rows: over.rows ?? [], skipped: 0 },
    footprint: { state: "none" } as Footprint,
    kept: { files: [], recorded_at: "2026-09-02T13:18:00Z", plans: [] },
    diff: { state: "none" } as Diff,
    live: false,
    transcript: over.transcript,
    log: (region) => ({ region, openId: null, onOpen: () => {} }),
    calls: { of: () => undefined, fetch: () => {} },
    sheet: null,
    opens: OPENS,
    onOpenSheet: () => {},
  });
}

/** Every `<p>`'s own text, so "on its own line" is a thing a test can read. */
function paragraphs(markup: string): string[] {
  return [...markup.matchAll(/<p\b[^>]*>(.*?)<\/p>/gs)].map((found) => found[1] ?? "");
}

/** Every heading element's own text. What a `<p>` is not. */
function headings(markup: string): string[] {
  return [...markup.matchAll(/<h[1-6]\b[^>]*>(.*?)<\/h[1-6]>/gs)].map((found) => found[1] ?? "");
}

describe("the brief's blocks", () => {
  it("groups the lines at the blank lines the author wrote", () => {
    expect(briefBlocks(["JOB BRIEF", "", "The title"])).toEqual([
      { text: "JOB BRIEF", heading: false },
      { text: "The title", heading: false },
    ]);
  });

  it("keeps a block's own line breaks, because the parts rail is one part a line", () => {
    expect(
      briefBlocks([
        "  1. Plan the change — you are here",
        "     STOP. Submit when this part is done, then wait.",
      ]),
    ).toEqual([
      {
        text: "  1. Plan the change — you are here\n     STOP. Submit when this part is done, then wait.",
        heading: false,
      },
    ]);
  });

  it("swallows a run of blank lines rather than emitting an empty block", () => {
    expect(briefBlocks(["one", "", "  ", "two"])).toEqual([
      { text: "one", heading: false },
      { text: "two", heading: false },
    ]);
  });

  it("answers nothing for a payload that is all blank", () => {
    expect(briefBlocks(["", ""])).toEqual([]);
  });

  // A bare string says nothing about the line, so nothing is a heading above
  // and everything here turns on the marker the wire carried.
  it("makes a block a heading only where the line it holds is named one", () => {
    expect(
      briefBlocks([{ text: "JOB BRIEF", named: "heading" }, { text: "" }, { text: "The title" }]),
    ).toEqual([
      { text: "JOB BRIEF", heading: true },
      { text: "The title", heading: false },
    ]);
  });

  it("leaves a named line that has body beside it as body", () => {
    // Not a shape `briefing.rs` writes — every heading it writes has a blank
    // line under it. Asserted so that if one ever arrives it does not drag the
    // body into a heading, which is what deciding by the first line would do.
    expect(
      briefBlocks([{ text: "JOB BRIEF", named: "heading" }, { text: "The title" }]),
    ).toEqual([{ text: "JOB BRIEF\nThe title", heading: false }]);
  });
});

describe("the rail's indent, widened for the panel", () => {
  it("doubles every leading space and leaves the rest of the line alone", () => {
    expect(widenIndent("  1. Plan the change — you are here")).toBe(
      "    1. Plan the change — you are here",
    );
  });

  it("keeps the stop deeper than the part it sits under", () => {
    const drawn = widenIndent("  1. Plan\n     STOP. Submit when this part is done.");
    expect(drawn).toBe("    1. Plan\n          STOP. Submit when this part is done.");
  });

  it("leaves an unindented line at the margin", () => {
    expect(widenIndent("WHERE YOU ARE")).toBe("WHERE YOU ARE");
  });

  it("never touches a space that is not leading", () => {
    expect(widenIndent("a  b")).toBe("a  b");
  });
});

describe("drone instructions", () => {
  it("puts each block heading on a line of its own", () => {
    const markup = renderToStaticMarkup(chapters({ rows: [instructed(BRIEF)] })[0]!.preview);
    const said = paragraphs(markup);
    // The defect: every one of these ran inline with the prose around it,
    // because the payload was joined back into one paragraph.
    expect(said).toContain("JOB BRIEF");
    expect(said).toContain("WHERE YOU ARE");
    expect(said).toContain("STEP: Plan the change");
  });

  // **This is the test that fails if the wiring is reverted**, and it is the
  // whole reason the marker crossing the wire reaches a person. `chapters.tsx`
  // hands `DroneBrief` the payload; mapped back to `line.text` the `named`
  // values are dropped on the way and every heading below draws as a `<p>`
  // again, with the gap above it as the only thing marking it — which is #318
  // shipped as a seam nothing reaches.
  it("hands the brief's marked lines to the component, so a heading is a heading", () => {
    const markup = renderToStaticMarkup(
      chapters({ rows: [instructed(BRIEF, HEADED)] })[0]!.preview,
    );
    expect(headings(markup)).toEqual(["JOB BRIEF", "WHERE YOU ARE", "STEP: Plan the change"]);
  });

  it("draws a short body line as body, whatever it looks like", () => {
    // The definition of done's own case, one surface further out than the
    // story: `This is done when:` is shorter than two of the headings above it
    // and opens a block of its own, so both cheap rules would mark it.
    const markup = renderToStaticMarkup(
      chapters({ rows: [instructed(BRIEF, HEADED)] })[0]!.preview,
    );
    expect(headings(markup)).not.toContain("This is done when:");
    expect(paragraphs(markup).some((said) => said.startsWith("This is done when:"))).toBe(true);
  });

  it("marks nothing on a turn the wire said nothing about", () => {
    // A row written before Fleet stamped the field. It draws as it drew after
    // #306 — blocks, no marked heading — rather than throwing or guessing.
    const markup = renderToStaticMarkup(chapters({ rows: [instructed(BRIEF)] })[0]!.preview);
    expect(headings(markup)).toEqual([]);
    expect(paragraphs(markup)).toContain("JOB BRIEF");
  });

  it("keeps the parts rail's lines and its indent, so STOP is still a boundary", () => {
    const markup = renderToStaticMarkup(chapters({ rows: [instructed(BRIEF)] })[0]!.preview);
    const rail = paragraphs(markup).find((said) => said.includes("1. Plan the change"));
    expect(rail).toBeDefined();
    // Doubled from the five spaces Fleet wrote. `widenIndent` is why, and it
    // states the cost: this rail and the transcript are no longer one string.
    expect(rail).toContain("\n          STOP. Submit when this part is done, then wait.");
    expect(rail?.startsWith("    1.")).toBe(true);
    // Whether a reader sees those newlines is `white-space`, which is a
    // rendering — `DroneBrief.stories.tsx` reads it off the browser.
  });

  it("keeps the parts list a list rather than a sentence", () => {
    const markup = renderToStaticMarkup(chapters({ rows: [instructed(BRIEF)] })[0]!.preview);
    const done = paragraphs(markup).find((said) => said.includes("This is done when:"));
    expect(done).toContain("\n    - two refreshes in flight make one network call");
  });
});

describe("produced, on a step whose product is a document", () => {
  const kept = step({ deliverables: [{ attempt: 1, path: ".armada/deliverables/plan.1.plan.md" }] });

  it("counts the document beside the files and never inside them", () => {
    const produced = chapters({ step: kept })[2]!;
    expect(produced.summary).toBe("0 files · 1 document");
  });

  it("says nothing about documents on a step that kept none", () => {
    expect(chapters()[2]!.summary).toBe("0 files");
  });

  it("stops claiming the drone changed nothing", () => {
    const markup = renderToStaticMarkup(chapters({ step: kept })[2]!.preview);
    expect(markup).not.toContain("has not changed anything");
    expect(markup).toContain("This step&#x27;s product is the document below.");
  });

  it("offers the document as a control that opens it", () => {
    const markup = renderToStaticMarkup(chapters({ step: kept })[2]!.preview);
    expect(markup).toContain("plan.1.plan.md");
    expect(markup).toContain('title=".armada/deliverables/plan.1.plan.md"');
    expect(markup).toContain("attempt 1");
  });

  it("draws the newest run first, because that is the one being read about", () => {
    const three = step({
      deliverables: [
        { attempt: 1, path: ".armada/deliverables/plan.1.plan.md" },
        { attempt: 2, path: ".armada/deliverables/plan.2.plan.md" },
      ],
    });
    const markup = renderToStaticMarkup(chapters({ step: three })[2]!.preview);
    expect(markup.indexOf("plan.2.plan.md")).toBeLessThan(markup.indexOf("plan.1.plan.md"));
    expect(chapters({ step: three })[2]!.summary).toBe("0 files · 2 documents");
  });
});

/**
 * The two surfaces that draw one step's documents, read against each other.
 *
 * **The defect was two readings, not one wrong one.** `keptRows` reversed and
 * argued for it; this chapter reversed and said so separately. Both were right
 * and nothing held them together, so the next edit to either was free to move
 * one. What this pins is the agreement — a step retried twice listing the same
 * documents in the same sequence on both. #321.
 */
describe("a step retried twice, on both surfaces", () => {
  const twice = step({
    deliverables: [
      { attempt: 1, path: ".armada/deliverables/plan.1.plan.md" },
      { attempt: 2, path: ".armada/deliverables/plan.2.plan.md" },
    ],
  });

  /** Each document a surface drew, with its attempt, in the order it drew it. */
  function documentsIn(markup: string): string[] {
    return [
      ...markup.matchAll(/title="(\.armada\/deliverables\/[^"]+)"[\s\S]*?(attempt \d+)/g),
    ].map((found) => `${found[1]} · ${found[2]}`);
  }

  /** What the Produced chapter lists. */
  function inProduced(one: StepDetail): string[] {
    return documentsIn(renderToStaticMarkup(chapters({ step: one })[2]!.preview));
  }

  /** What the strip's Submitted tier lists. The result is beside the label. */
  function onTheStrip(one: StepDetail): string[] {
    const stage = phasesOf(one, [], OPENS).stages.find((held) => held.id === "submitted");
    return (stage?.rows ?? []).flatMap((row) =>
      documentsIn(renderToStaticMarkup(row.label) + renderToStaticMarkup(row.result)),
    );
  }

  it("lists the same documents in the same order on both", () => {
    expect(onTheStrip(twice)).toEqual(inProduced(twice));
  });

  // Named rather than only compared, so a change that reversed *both* surfaces
  // still fails here. Two surfaces agreeing on the wrong order is the reading
  // the test above cannot tell from the right one.
  it("puts the last run at the top of both", () => {
    expect(inProduced(twice)).toEqual([
      ".armada/deliverables/plan.2.plan.md · attempt 2",
      ".armada/deliverables/plan.1.plan.md · attempt 1",
    ]);
  });
});

/**
 * **The defect this file's third answer exists for.** A socket that had failed
 * or closed drew as a step that had not started — the same two sentences a step
 * nobody has opened yet gets, which is how nine minutes of an escalated
 * implement step read as `Nothing has happened on this step yet.` #324.
 */
describe("a step whose transcript is not being read", () => {
  const NOT_READ = "The transcript could not be read. Fleet is not connected.";

  it("says which reading failed instead of that nothing has happened", () => {
    const markup = renderToStaticMarkup(chapters({ transcript: NOT_READ })[1]!.preview);
    expect(markup).toContain("The transcript could not be read.");
    expect(markup).not.toContain("Nothing has happened on this step yet.");
  });

  it("says it in chapter one too, where the instruction would be", () => {
    const markup = renderToStaticMarkup(chapters({ transcript: NOT_READ })[0]!.preview);
    expect(paragraphs(markup)).toContain(NOT_READ);
    expect(markup).not.toContain("Armada has not opened this step yet.");
  });

  it("keeps saying it with rows in hand, because the rows do not answer for the socket", () => {
    const markup = renderToStaticMarkup(
      chapters({ rows: [instructed("go")], transcript: NOT_READ })[1]!.preview,
    );
    expect(markup).toContain("The transcript could not be read.");
    expect(markup).toContain("Armada opened the step.");
  });

  it("leaves the ordinary sentence alone while the socket is reading", () => {
    const markup = renderToStaticMarkup(chapters()[1]!.preview);
    expect(markup).toContain("Nothing has happened on this step yet.");
  });
});
