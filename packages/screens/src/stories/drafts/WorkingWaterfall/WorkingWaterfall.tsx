// The Working body as a waterfall — the second of two shapes drawn for the owner
// to pick between, 11 Sep 2026.
//
// **Working has two products, and they are peers.** What the Drone did, and what
// came out of it. The owner's call, 11 Sep 2026: the activity log and the
// produced work are two different things the step yields. That matters more here
// than in the folded draft, because a write takes no time — placed on a clock it
// is one tick between two calls, so the footprint can only be a part of its own.
//
// **What the lane borrows.** A trace viewer: one clock across the top, a bar per
// act placed where it happened and drawn as wide as it took, a collapse by name,
// and a filter down to what failed.
//
// **What the recorded step made it say.** Measured, the `implement` step spent
// 8m 07s inside calls across 43m 36s of attempt, and 6m 31s of that is two
// `TaskOutput` waits. So the empty space in this lane is not a drawing artefact
// — it is 35 minutes of a model thinking, which no folded list can show, and it
// is where four fifths of the step actually went.
import { useState, type ReactNode } from "react";

import { StepActivityMark } from "@armada/components";
import type { StepDetail, Turn } from "@armada/protocol";

import { briefly, clock, lasting } from "../../../duration";
import type { JobFixture } from "../../../fixtures/fixture";
import {
  byToolOf,
  laneOf,
  producedOf,
  workingOf,
  type LaneRow,
  type Produced,
  type ToolTotal,
  type Working,
} from "../../../working";

/** The steps a fixture's Job read carries, or none where it has not arrived. */
function stepsOf(fixture: JobFixture): StepDetail[] {
  return "detail" in fixture.watched ? fixture.watched.detail.steps : [];
}

/** The turns the fixture's socket holds. Kept through `ended` and `failed`. */
function turnsOf(fixture: JobFixture): Turn[] {
  return "turns" in fixture.observed ? fixture.observed.turns.rows : [];
}

export function WorkingWaterfallFrom({
  fixture,
  stepId,
}: {
  fixture: JobFixture;
  stepId?: string;
}) {
  const [wrongOnly, setWrongOnly] = useState(false);
  const [byTool, setByTool] = useState(false);
  const steps = stepsOf(fixture);
  const step =
    steps.find((one) => one.step_id === (stepId ?? fixture.job.current_step_id)) ?? steps[0];
  if (step === undefined) return <p>This fixture&apos;s Job has no steps to draw.</p>;
  const working = workingOf(turnsOf(fixture), step.step_id);
  const rows = laneOf(working, wrongOnly);
  const tools = byToolOf(working.acts);
  const produced = producedOf(working.acts);
  const thinking = Math.max(0, working.wall - working.inCalls);
  return (
    <div style={PANEL}>
      <Head step={step} working={working} produced={produced} />
      <Part
        says="Where the time went"
        meta={`${lasting(working.inCalls)} inside a call · ${lasting(thinking)} with nothing open`}
      >
        <div style={{ display: "flex", gap: "var(--space-2)" }}>
          <Toggle on={byTool} onPress={() => setByTool((was) => !was)} says="By tool" />
          <Toggle
            on={wrongOnly}
            onPress={() => setWrongOnly((was) => !was)}
            says="Only what went wrong"
          />
        </div>
        {byTool ? <Tools tools={tools} /> : <Lane rows={rows} wall={working.wall} />}
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
  return (
    <div style={COLUMN}>
      <span className="caps">Working · {step.label}</span>
      <span style={{ fontSize: "var(--text-sm)" }}>
        {`${calls} calls · ${produced.files.length} files · ${lasting(working.wall)} from end to end`}
      </span>
    </div>
  );
}

/**
 * One of Working's two products.
 *
 * **The same chrome for both**, which is the whole of what makes them peers, and
 * the same component the folded draft uses for the same reason.
 */
function Part({ says, meta, children }: { says: string; meta?: string; children: ReactNode }) {
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

/** One of the two controls. Pressed is state, so it is `aria-pressed`. */
function Toggle({ on, onPress, says }: { on: boolean; onPress: () => void; says: string }) {
  return (
    <button
      type="button"
      aria-pressed={on}
      onClick={onPress}
      style={{
        ...CONTROL,
        color: on ? "var(--fg-default)" : "var(--fg-subtle)",
        borderColor: on ? "var(--border-strong)" : "var(--border-subtle)",
        background: on ? "var(--bg-raised)" : "transparent",
      }}
    >
      {says}
    </button>
  );
}

/** The clock, and every act placed on it. */
function Lane({ rows, wall }: { rows: LaneRow[]; wall: number }) {
  return (
    <div style={WELL}>
      <Axis wall={wall} />
      {rows.length === 0 ? (
        <span style={MUTED}>Nothing went wrong in this step.</span>
      ) : (
        rows.map((row) => <Bar key={row.id} row={row} />)
      )}
    </div>
  );
}

/** Four marks across the attempt, which is what makes a bar's position mean anything. */
function Axis({ wall }: { wall: number }) {
  return (
    <div style={{ ...LINE, position: "sticky", top: 0, background: "var(--surface-well)" }}>
      <span style={{ ...SUBTLE, flex: `0 0 ${LABEL}` }}>Act</span>
      <div style={{ position: "relative", flex: "1 1 auto", height: "var(--leading-2xs)" }}>
        {[0, 0.25, 0.5, 0.75].map((mark) => (
          <span
            key={mark}
            className="mono"
            style={{ ...SUBTLE, position: "absolute", left: `${mark * 100}%` }}
          >
            {lasting(wall * mark)}
          </span>
        ))}
      </div>
      <span style={{ ...SUBTLE, flex: `0 0 ${TOOK}`, textAlign: "right" }}>Took</span>
    </div>
  );
}

/** One act: what it was, where it fell, and how long it held. */
function Bar({ row }: { row: LaneRow }) {
  return (
    <div style={LINE}>
      <span
        className="mono"
        style={{ ...SMALL, flex: `0 0 ${LABEL}`, minWidth: 0, ...CLAMPED }}
        title={row.said}
      >
        {row.wrong === true ? <StepActivityMark activity="failed" label="did not pass" /> : null}
        {row.tool ?? row.kind}
      </span>
      <div style={TRACK}>
        <span
          style={{
            position: "absolute",
            top: 0,
            bottom: 0,
            left: `${row.at * 100}%`,
            width: `${row.width * 100}%`,
            minWidth: "var(--space-1)",
            borderRadius: "var(--radius-sm)",
            background: hueOf(row),
          }}
        />
      </div>
      <span className="mono" style={{ ...SUBTLE, flex: `0 0 ${TOOK}`, textAlign: "right" }}>
        {row.ms === 0 ? "" : briefly(row.ms)}
      </span>
    </div>
  );
}

/**
 * What colour a bar is.
 *
 * **Three hues and no more**, all from the step-activity set: what failed, what
 * was a call, and what was neither. A hue per tool would be a scale this
 * repository does not have and a legend nobody reads.
 */
function hueOf(row: LaneRow): string {
  if (row.wrong === true) return "var(--step-failed)";
  return row.kind === "call" ? "var(--step-running)" : "var(--border-strong)";
}

/**
 * The same acts added up by tool, slowest first.
 *
 * **The collapse that pays for itself on this data**: 351 bars become nine, and
 * the nine say two `TaskOutput` waits hold four fifths of the call time — which
 * is invisible in a lane of 351 rows and invisible in any folded list.
 */
function Tools({ tools }: { tools: ToolTotal[] }) {
  const widest = tools.reduce((most, one) => Math.max(most, one.ms), 0);
  return (
    <div style={WELL}>
      {tools.length === 0 ? (
        <span style={MUTED}>This step made no calls.</span>
      ) : (
        tools.map((tool) => (
          <div key={tool.tool} style={LINE}>
            <span
              className="mono"
              style={{ ...SMALL, flex: `0 0 ${LABEL}`, minWidth: 0, ...CLAMPED }}
            >
              {tool.tool}
            </span>
            <div style={TRACK}>
              <span
                style={{
                  position: "absolute",
                  top: 0,
                  bottom: 0,
                  left: 0,
                  width: `${widest === 0 ? 0 : (tool.ms / widest) * 100}%`,
                  minWidth: "var(--space-1)",
                  borderRadius: "var(--radius-sm)",
                  background: tool.wrong > 0 ? "var(--step-failed)" : "var(--step-running)",
                }}
              />
            </div>
            <span className="mono" style={{ ...SUBTLE, flex: `0 0 ${TOOK}`, textAlign: "right" }}>
              {`${tool.calls} · ${briefly(tool.ms)}`}
            </span>
          </div>
        ))
      )}
    </div>
  );
}

/**
 * What the step wrote.
 *
 * **Identical in the folded draft**, so what is being compared is the activity
 * above it. Every file is drawn: the owner's own Job wrote 42 in a step, and a
 * list that names eight and counts the rest is the reading that sent this back.
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

/** The label column, wide enough for a tool name and never wider. */
const LABEL = "calc(var(--space-12) + var(--space-8))";

/** The duration column. Right-aligned, so the digits line up down the lane. */
const TOOK = "calc(var(--space-12) + var(--space-4))";

const SMALL = { fontSize: "var(--text-2xs)", lineHeight: "var(--leading-2xs)" } as const;
const SUBTLE = { ...SMALL, color: "var(--fg-subtle)" } as const;
const MUTED = { ...SMALL, color: "var(--fg-muted)" } as const;

const CLAMPED = {
  display: "inline-flex",
  alignItems: "center",
  gap: "var(--space-1)",
  whiteSpace: "nowrap",
  overflow: "hidden",
  textOverflow: "ellipsis",
} as const;

const PANEL = {
  display: "flex",
  flexDirection: "column",
  gap: "var(--space-4)",
  padding: "var(--space-4)",
  background: "var(--bg-base)",
  minHeight: "100vh",
} as const;

const COLUMN = { display: "flex", flexDirection: "column", gap: "var(--space-1)" } as const;

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

/** The ground a bar is placed on. Relative, because the bar is absolute in it. */
const TRACK = {
  position: "relative",
  flex: "1 1 auto",
  height: "var(--space-2)",
  borderRadius: "var(--radius-sm)",
  background: "var(--bg-base)",
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

const CONTROL = {
  padding: "var(--space-1) var(--space-3)",
  borderRadius: "var(--radius-sm)",
  borderWidth: "var(--border-width)",
  borderStyle: "solid",
  cursor: "pointer",
  fontFamily: "var(--font-sans)",
  fontSize: "var(--text-2xs)",
  lineHeight: "var(--leading-2xs)",
} as const;
