// What the grouped log folds, and what it refuses to fold.
//
// **Rendered to markup rather than measured as data**, because the claim is
// about what a reader sees: a run behind one line, a failure never behind one,
// and the rows still in the document when a group is closed.
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { Turn, WorkPlan } from "@armada/protocol";

import type { Calls } from "./calls";
import { answered, called, said } from "./fixtures/build/base";
import { OUTSIDE_ANY_TASK, PlanBar, WorkGrouped, WorkNarrated } from "./grouped";
import { entriesOf, hideUnread } from "./story";

const STEP = "implement";

/** No call has been fetched, and nothing asks for one. */
const CALLS: Calls = { of: () => undefined, fetch: () => {} };

/** The log's keyboard binding, with nothing open. */
const LOG = { region: "log", openId: null, onOpen: () => {} };

function at(seconds: number): string {
  return new Date(Date.parse("2026-09-10T14:00:00Z") + seconds * 1000).toISOString();
}

function drawn(turns: Turn[]): string {
  const { rows } = hideUnread(entriesOf(turns, STEP));
  return renderToStaticMarkup(
    <WorkGrouped
      rows={rows}
      turns={turns}
      stepId={STEP}
      emptyNote="Nothing yet"
      calls={CALLS}
      log={LOG}
    />,
  );
}

/**
 * How many log rows a rendering drew. Every row's control names its own
 * payload, so counting those counts the rows — and it counts them whether the
 * fold above them is open or shut, which is the claim the bound makes.
 */
function rowsIn(markup: string): number {
  return markup.split('aria-controls="log-payload-').length - 1;
}

/** Three reads in a row, all of them fine. */
function threeReads(): Turn[] {
  return [
    called(STEP, at(0), "c1", "Read", "a.ts"),
    answered(STEP, at(1), "c1"),
    called(STEP, at(2), "c2", "Read", "b.ts"),
    answered(STEP, at(3), "c2"),
    called(STEP, at(4), "c3", "Read", "c.ts"),
    answered(STEP, at(5), "c3"),
  ];
}

describe("a run of one tool folds to a line", () => {
  it("draws one heading carrying the count and the time", () => {
    const markup = drawn(threeReads());
    expect(markup).toContain("armada-work__head");
    expect(markup).toContain("3 calls");
  });

  it("keeps the rows in the document, hidden, so an opened row survives a fold", () => {
    const markup = drawn(threeReads());
    expect(markup).toContain('hidden=""');
    expect(markup).toContain("a.ts");
  });

  it("draws a lone turn with no heading at all", () => {
    const markup = drawn([said(STEP, at(1), "I have the shape of it now.")]);
    expect(markup).not.toContain("armada-work__head");
    expect(markup).toContain("I have the shape of it now.");
  });
});

describe("a failure is never folded", () => {
  it("draws the run holding it bare, with no count in front of it", () => {
    const failing = [
      ...threeReads(),
      called(STEP, at(6), "c4", "Read", "d.ts"),
      answered(STEP, at(7), "c4", true),
    ];
    const markup = drawn(failing);
    // One run, four calls, one of them failed — so no heading is drawn for it.
    expect(markup).not.toContain("armada-work__head");
    expect(markup).toContain("The call failed");
  });
});

describe("no row is ever dropped", () => {
  it("draws a row no group claims, in the order it arrived", () => {
    // `unreadable` survives `hideUnread`, so it reaches the log — and a mapper
    // that selected rows by their groups would lose it silently.
    const unreadable: Turn = {
      ts: at(4),
      seq: 8100,
      step: STEP,
      by: "fleet",
      saw: { event: "unreadable", line: "{ not json", why: "unexpected token" },
    };
    const markup = drawn([...threeReads(), unreadable]);
    expect(markup).toContain("A line the reader could not parse");
  });

  it("draws every row it is handed, once", () => {
    const turns = [...threeReads(), said(STEP, at(8), "Done.")];
    const { rows } = hideUnread(entriesOf(turns, STEP));
    const markup = renderToStaticMarkup(
      <WorkGrouped
        rows={rows}
        turns={turns}
        stepId={STEP}
        emptyNote="Nothing yet"
        calls={CALLS}
        log={LOG}
      />,
    );
    // Every row's own payload id appears exactly once — the log writes one per
    // row, so counting them counts the rows that were drawn.
    for (const row of rows) {
      expect(markup.split(`log-payload-${row.id}"`).length - 1).toBeGreaterThanOrEqual(1);
    }
  });
});

describe("what it does not draw", () => {
  it("says nothing about unread rows unless it is given them", () => {
    expect(drawn(threeReads())).not.toContain("does not draw");
  });

  it("counts them where it is", () => {
    // **Built once.** The fixture builders carry a module-level sequence, so a
    // second call names the same work with different ids — and a group whose
    // rows are not in hand draws nothing, which is the mapper behaving.
    const turns = threeReads();
    const { rows } = hideUnread(entriesOf(turns, STEP));
    const markup = renderToStaticMarkup(
      <WorkGrouped
        rows={rows}
        turns={turns}
        stepId={STEP}
        unread={[{ kind: "system/thinking_tokens", count: 757 }]}
        emptyNote="Nothing yet"
        calls={CALLS}
        log={LOG}
      />,
    );
    expect(markup).toContain("757 rows this Bridge does not draw");
  });
});

describe("the Working area reads as the Drone's sentences", () => {
  function narrated(turns: Turn[], plan?: WorkPlan, mostEntries?: number): string {
    const { rows } = hideUnread(entriesOf(turns, STEP));
    return renderToStaticMarkup(
      <WorkNarrated
        rows={rows}
        turns={turns}
        stepId={STEP}
        plan={plan}
        live
        emptyNote="Nothing yet"
        calls={CALLS}
        log={LOG}
        {...(mostEntries === undefined ? {} : { mostEntries })}
      />,
    );
  }

  const sentences = (): Turn[] => [
    said(STEP, at(0), "Now let's add `EvidenceInbox::reloaded`:"),
    called(STEP, at(1), "n1", "Edit", "evidence.rs"),
    answered(STEP, at(2), "n1"),
    said(STEP, at(3), "Now wire this into `Fleet::assembled` in fittings.rs:"),
    called(STEP, at(4), "n2", "Edit", "fittings.rs"),
    answered(STEP, at(5), "n2"),
  ];

  it("draws no task heading where the Job has no plan", () => {
    const markup = narrated(sentences());
    expect(markup).not.toContain(OUTSIDE_ANY_TASK);
    expect(markup).toContain("Now wire this into `Fleet::assembled` in fittings.rs:");
    expect(markup).toContain("1 call so far · Edit");
  });

  it("keeps an older sentence's calls in the document behind a closed fold", () => {
    const markup = narrated(sentences());
    // The log's own rows carry `aria-expanded` too, so only the fold lines count.
    expect(markup.match(/narration__calls" aria-expanded="false"/g)).toHaveLength(1);
    expect(markup.match(/narration__calls" aria-expanded="true"/g)).toHaveLength(1);
    expect(markup).toContain("evidence.rs");
  });

  it("opens the task holding the newest work and folds the one finished", () => {
    const plan: WorkPlan = {
      approach: "a",
      recorded_by: { by: "person" },
      recorded_at: at(0),
      tasks: [
        {
          id: "T1",
          title: "Give it a row",
          state: "done",
          working_windows: [{ entered: at(0), left: at(3) }],
        },
        { id: "T2", title: "Reload it", state: "working", working_windows: [{ entered: at(3) }] },
        { id: "T3", title: "Test a restart", state: "open" },
      ],
    };
    const markup = narrated(sentences(), plan);
    const t1 = markup.indexOf("Give it a row");
    const t2 = markup.indexOf("Reload it");
    const t3 = markup.indexOf("Test a restart");
    expect(t1).toBeGreaterThan(-1);
    expect(t2).toBeGreaterThan(t1);
    expect(t3).toBeGreaterThan(t2);
    expect(markup.slice(0, t1)).toContain('aria-expanded="false"');
    expect(markup.slice(t1, t2)).toContain('aria-expanded="true"');
    expect(markup).not.toContain(OUTSIDE_ANY_TASK);
  });

  // The owner's decision of 18 Sep 2026: the preview is the last ten entries,
  // and *Open the log* is where everything is read. What the bound counts is
  // the claim — eight groups, and eight sentences after them, each let one
  // preview draw a hundred rows.
  describe("the preview draws ten rows and never more", () => {
    /** A sentence, then that many `Read` calls — one row each, the answer folded in. */
    function reading(calls: number): Turn[] {
      const turns = [said(STEP, at(0), "Reading the reducer to find the selector.")];
      for (let one = 0; one < calls; one += 1) {
        turns.push(called(STEP, at(one * 2 + 1), `p${one}`, "Read", `src/file${one}.ts`));
        turns.push(answered(STEP, at(one * 2 + 2), `p${one}`));
      }
      return turns;
    }

    it("holds ten of a dozen consecutive calls of one tool", () => {
      // The case group counting existed to avoid, taken on purpose.
      const markup = narrated(reading(12), undefined, 10);
      expect(rowsIn(markup)).toBe(10);
      expect(markup).toContain("3 earlier, in the log");
    });

    it("says what is under a run the bound cut in half, not what the run held", () => {
      const markup = narrated(reading(12), undefined, 10);
      // The sentence went with the rows it lost, so the fold line is the
      // heading — and it counts the calls in hand rather than all twelve.
      expect(markup).not.toContain("Reading the reducer to find the selector.");
      expect(markup).toContain("10 calls so far · Read");
    });

    it("counts a sentence as one of the ten, so sentences cannot fill it", () => {
      const turns = Array.from({ length: 6 }, (_, one) => [
        said(STEP, at(one * 3), `Sentence ${one}.`),
        called(STEP, at(one * 3 + 1), `s${one}`, "Edit", "crates/fleet/src/evidence.rs"),
        answered(STEP, at(one * 3 + 2), `s${one}`),
      ]).flat();
      const markup = narrated(turns, undefined, 10);
      expect(rowsIn(markup)).toBe(5);
      expect(markup).not.toContain("Sentence 0.");
      expect(markup).toContain("Sentence 5.");
    });

    it("draws every row where nothing bounds it, which is how the log sheet reads", () => {
      expect(rowsIn(narrated(reading(12)))).toBe(12);
    });
  });
});

describe("the plan bar", () => {
  it("says done over not dropped, and nothing without a plan", () => {
    expect(renderToStaticMarkup(<PlanBar plan={undefined} />)).toBe("");
    const markup = renderToStaticMarkup(
      <PlanBar
        plan={{
          approach: "a",
          recorded_by: { by: "person" },
          recorded_at: at(0),
          tasks: [
            { id: "T1", title: "a", state: "done" },
            { id: "T2", title: "b", state: "working" },
            { id: "T3", title: "c", state: "dropped", reason: "no" },
          ],
        }}
      />,
    );
    expect(markup).toContain("1 of 2");
  });
});
