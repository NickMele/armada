// The Working body with the activity folded by kind — one of two shapes drawn
// for the owner to pick between, 11 Sep 2026.
//
// **Working has two products, and they are peers.** What the Drone did, and what
// came out of it. The owner's call, 11 Sep 2026: the activity log and the
// produced work are two different things the step yields, so neither is the
// other's footnote — the footprint is not a row at the bottom of a list two
// hundred rows long, and it does not wait behind a press.
//
// **What the folding borrows.** Every continuous-integration product that folds
// a build log does the same three things: consecutive work of one kind collapses
// to a heading carrying a count and a duration, the group that failed opens
// itself, and the rows a reader never wants are counted rather than drawn. On
// the recorded `implement` step that turns 1763 rows into 203.
//
// **What it cannot say.** Where the time went — every row is the same height
// whether it took 200ms or four minutes. `Drafts/Working waterfall` is the other
// half of that question, and its Produced part is deliberately identical to this
// one so the comparison is of the activity above it.
import { useState, type ReactNode } from "react";

import { ChevronDown, ChevronRight } from "lucide-react";

import { StepActivityMark } from "@armada/components";
import type { StepDetail, Turn } from "@armada/protocol";

import { briefly, clock, lasting } from "../../../duration";
import type { JobFixture } from "../../../fixtures/fixture";
import {
  producedOf,
  runsOf,
  workingOf,
  type Produced,
  type Working,
  type WorkingAct,
  type WorkingRun,
} from "../../../working";

/** The steps a fixture's Job read carries, or none where it has not arrived. */
function stepsOf(fixture: JobFixture): StepDetail[] {
  return "detail" in fixture.watched ? fixture.watched.detail.steps : [];
}

/** The turns the fixture's socket holds. Kept through `ended` and `failed`. */
function turnsOf(fixture: JobFixture): Turn[] {
  return "turns" in fixture.observed ? fixture.observed.turns.rows : [];
}

export function WorkingGroupedFrom({ fixture, stepId }: { fixture: JobFixture; stepId?: string }) {
  const steps = stepsOf(fixture);
  const step =
    steps.find((one) => one.step_id === (stepId ?? fixture.job.current_step_id)) ?? steps[0];
  if (step === undefined) return <p>This fixture&apos;s Job has no steps to draw.</p>;
  const working = workingOf(turnsOf(fixture), step.step_id);
  const runs = runsOf(working.acts);
  const produced = producedOf(working.acts);
  const unread = working.unread.reduce((sum, one) => sum + one.count, 0);
  return (
    <div style={PANEL}>
      <Head step={step} working={working} produced={produced} />
      <Part
        says="What the Drone did"
        meta={`${runs.length} groups from ${working.acts.length + unread} rows`}
      >
        <div style={WELL}>
          {runs.map((run) => (
            <Run key={run.id} run={run} />
          ))}
          <Unread working={working} />
        </div>
      </Part>
      <Part says="What it produced" meta={changesSaid(produced)}>
        <Footprint produced={produced} />
      </Part>
    </div>
  );
}

/** The step, and its two products in one line each. */
function Head({
  step,
  working,
  produced,
}: {
  step: StepDetail;
  working: Working;
  produced: Produced;
}) {
  const calls = working.acts.filter((act) => act.kind === "call").length;
  const said = working.acts.filter((act) => act.kind === "said").length;
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-1)" }}>
      <span className="caps">Working · {step.label}</span>
      <span style={{ fontSize: "var(--text-sm)" }}>
        {`${calls} calls · ${said} replies · ${produced.files.length} files · ${lasting(working.wall)}`}
      </span>
    </div>
  );
}

/**
 * One of Working's two products.
 *
 * **The same chrome for both**, which is the whole of what makes them peers: a
 * heading, a fact about the part, and a bordered body. A band tucked under a
 * scrolling list reads as that list's footer however it is worded.
 */
function Part({
  says,
  meta,
  children,
}: {
  says: string;
  meta?: string;
  children: ReactNode;
}) {
  return (
    <section style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
      <div style={{ ...LINE, justifyContent: "space-between" }}>
        <span className="caps">{says}</span>
        {meta === undefined ? null : <span style={SUBTLE}>{meta}</span>}
      </div>
      {children}
    </section>
  );
}

/** One run: a folded heading, or the acts themselves where it does not fold. */
function Run({ run }: { run: WorkingRun }) {
  const [open, setOpen] = useState(false);
  if (run.acts[0] === undefined) return null;
  if (!run.folded) {
    return (
      <div style={COLUMN}>
        {run.acts.map((act) => (
          <Act key={act.id} act={act} />
        ))}
      </div>
    );
  }
  return (
    <div style={COLUMN}>
      <button type="button" aria-expanded={open} onClick={() => setOpen((was) => !was)} style={FOLD}>
        {open ? (
          <ChevronDown size={13} strokeWidth={2} aria-hidden />
        ) : (
          <ChevronRight size={13} strokeWidth={2} aria-hidden />
        )}
        <span className="mono">{run.tool}</span>
        <span style={{ color: "var(--fg-subtle)" }}>
          {`${run.acts.length} calls · ${briefly(run.ms)}`}
        </span>
      </button>
      {!open ? null : (
        <div style={NESTED}>
          {run.acts.map((act) => (
            <Act key={act.id} act={act} />
          ))}
        </div>
      )}
    </div>
  );
}

/** One act as a line: when, what, how long, and what it came to. */
function Act({ act }: { act: WorkingAct }) {
  // A failure opens itself. It is the row the reader came for, and a count is
  // the one thing that must never stand in front of it.
  const [open, setOpen] = useState(act.wrong === true);
  const shows = act.wrong === true || act.kind === "checked";
  return (
    <div style={COLUMN}>
      <div style={LINE}>
        <span className="mono" style={{ ...SUBTLE, flex: "0 0 auto" }}>
          {clock(act.ts)}
        </span>
        {act.wrong === true ? <StepActivityMark activity="failed" label="did not pass" /> : null}
        <span
          {...(act.mono === true ? { className: "mono" } : {})}
          style={{ ...SMALL, flex: "1 1 auto", minWidth: 0, ...CLAMPED }}
        >
          {act.said}
        </span>
        {act.ms === undefined ? null : (
          <span style={{ ...SUBTLE, flex: "0 0 auto" }}>{briefly(act.ms)}</span>
        )}
        {!shows ? null : (
          <button
            type="button"
            aria-expanded={open}
            onClick={() => setOpen((was) => !was)}
            style={MORE}
          >
            {open ? "less" : "more"}
          </button>
        )}
      </div>
      {!open ? null : <Inside act={act} />}
    </div>
  );
}

/** What an opened act shows. A reading, not the chapter the panel would open. */
function Inside({ act }: { act: WorkingAct }) {
  if (act.kind === "checked" && act.run !== undefined) {
    return (
      <div style={NESTED}>
        <span style={SMALL}>
          <span className="mono">{act.run.outcome}</span>
          {act.run.produced === undefined ? "" : ` · ${act.run.produced}`}
        </span>
      </div>
    );
  }
  return (
    <div style={NESTED}>
      <span style={MUTED}>
        {act.because ?? "The log holds the whole of this row in the panel proper."}
      </span>
    </div>
  );
}

/**
 * The rows nothing draws, counted by the wire's own kind.
 *
 * **Closed, and it says how many.** 993 of the recorded step's 1763 rows are
 * these — 757 of one kind — and the count is what keeps the hiding honest.
 */
function Unread({ working }: { working: Working }) {
  const [open, setOpen] = useState(false);
  const total = working.unread.reduce((sum, one) => sum + one.count, 0);
  if (total === 0) return null;
  return (
    <div style={COLUMN}>
      <button type="button" aria-expanded={open} onClick={() => setOpen((was) => !was)} style={FOLD}>
        {open ? (
          <ChevronDown size={13} strokeWidth={2} aria-hidden />
        ) : (
          <ChevronRight size={13} strokeWidth={2} aria-hidden />
        )}
        <span style={{ color: "var(--fg-subtle)" }}>
          {`${total} rows this Bridge does not draw`}
        </span>
      </button>
      {!open ? null : (
        <div style={NESTED}>
          {working.unread.map((one) => (
            <span key={one.kind} style={SMALL}>
              <span className="mono">{one.kind}</span>{" "}
              <span style={{ color: "var(--fg-subtle)" }}>{one.count}</span>
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

/**
 * What the step wrote.
 *
 * **Identical in both drafts**, so what is being compared is the activity above
 * it. Every file is drawn: the owner's own Job wrote 42 in a step, and a list
 * that names eight and counts the rest is the reading that sent this back.
 */
function Footprint({ produced }: { produced: Produced }) {
  if (produced.files.length === 0) {
    return (
      <div style={WELL}>
        <span style={MUTED}>This attempt wrote nothing.</span>
      </div>
    );
  }
  return (
    <div style={WELL}>
      <div style={{ ...LINE, justifyContent: "space-between" }}>
        <span style={SMALL}>
          {`${produced.files.length} ${produced.files.length === 1 ? "file" : "files"}`}
          {produced.at === undefined ? "" : ` · read at ${clock(produced.at)}`}
        </span>
        <button type="button" style={MORE}>
          Open the diff
        </button>
      </div>
      {produced.outsidePlan === 0 ? null : (
        <span style={{ ...SMALL, color: "var(--step-waiting)" }}>
          {`${produced.outsidePlan} outside the plan this step declared`}
        </span>
      )}
      <div style={FILES}>
        {produced.files.map((file) => (
          <span key={file.path} style={SMALL}>
            <span className="mono">{file.path}</span>{" "}
            <span style={{ color: "var(--fg-subtle)" }}>{file.change}</span>
            {file.outside_plan === true ? (
              <span style={{ color: "var(--step-waiting)" }}> outside the plan</span>
            ) : null}
          </span>
        ))}
      </div>
    </div>
  );
}

/** `22 modified · 12 added · 2 deleted`, in the wire's own spellings. */
function changesSaid(produced: Produced): string {
  return produced.changes.map((one) => `${one.files} ${one.change}`).join(" · ");
}

/**
 * The body's small type.
 *
 * **Style rather than classes, because the classes do not exist** — `.mono` and
 * `.caps` in the token layer are the only utilities this repository declares.
 */
const SMALL = { fontSize: "var(--text-2xs)", lineHeight: "var(--leading-2xs)" } as const;
const SUBTLE = { ...SMALL, color: "var(--fg-subtle)" } as const;
const MUTED = { ...SMALL, color: "var(--fg-muted)" } as const;

/** One line held to one line, however long the argument was. */
const CLAMPED = { whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" } as const;

const PANEL = {
  display: "flex",
  flexDirection: "column",
  gap: "var(--space-4)",
  padding: "var(--space-4)",
  background: "var(--bg-base)",
  minHeight: "100vh",
} as const;

const COLUMN = { display: "flex", flexDirection: "column", gap: "var(--space-1)" } as const;

/** A part's body. It scrolls: 203 rows is what this step is, and hiding that lies. */
const WELL = {
  ...COLUMN,
  maxHeight: "55vh",
  overflowY: "auto",
  padding: "var(--space-3)",
  background: "var(--surface-well)",
  borderRadius: "var(--radius-md)",
  border: "var(--border-width) solid var(--border-subtle)",
} as const;

/** The footprint scrolls inside its own part rather than being cut to a count. */
const FILES = {
  ...COLUMN,
  maxHeight: "calc(var(--space-12) * 4)",
  overflowY: "auto",
  marginTop: "var(--space-1)",
  paddingTop: "var(--space-2)",
  borderTop: "var(--border-width) solid var(--border-subtle)",
} as const;

const LINE = { display: "flex", alignItems: "center", gap: "var(--space-2)", minWidth: 0 } as const;

const FOLD = {
  display: "inline-flex",
  alignItems: "center",
  gap: "var(--space-2)",
  alignSelf: "flex-start",
  padding: 0,
  border: 0,
  background: "none",
  cursor: "pointer",
  color: "var(--fg-default)",
  fontFamily: "var(--font-sans)",
  fontSize: "var(--text-2xs)",
  lineHeight: "var(--leading-2xs)",
} as const;

const MORE = {
  flex: "0 0 auto",
  padding: 0,
  border: 0,
  background: "none",
  cursor: "pointer",
  color: "var(--accent)",
  fontFamily: "var(--font-sans)",
  fontSize: "var(--text-2xs)",
  lineHeight: "var(--leading-2xs)",
} as const;

const NESTED = {
  ...COLUMN,
  marginLeft: "var(--space-4)",
  paddingLeft: "var(--space-3)",
  borderLeft: "var(--border-width) solid var(--border-subtle)",
} as const;
