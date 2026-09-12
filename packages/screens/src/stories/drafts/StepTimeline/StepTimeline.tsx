// A draft of the step panel as one timeline, from a real Job's wire data.
//
// **What this is trying out.** Job detail draws `Where this step is` as a strip
// and then the step's chapters under it, and the two say the same thing twice:
// the strip says a Check failed, and the chapter below says what it printed.
// The owner asked, on 11 Sep 2026, for one list instead — a row per phase, in
// the order they happened, repeated where the step was handed back, with the
// previous attempts readable rather than hinted at.
//
// **The shape is the point, not the bodies.** Each row opens to a short reading
// here; in the panel proper they would open to the chapters that already exist
// — the brief, the activity log, the Check's output, the Judge's verdicts.
// Drawing those again in a draft would be building the thing before it is
// agreed, which is what a draft is for avoiding.
import { useState } from "react";

import { ChevronDown, ChevronRight } from "lucide-react";

import { Chapter, StepActivityMark } from "@armada/components";
import type { StepDetail, Turn } from "@armada/protocol";

import type { JobFixture } from "../../../fixtures/fixture";
import { clock } from "../../../duration";
import { entriesOf, hideUnread } from "../../../story";
import { timelineOf, type TimelineAttempt, type TimelineRow } from "../../../timeline";

/** The steps a fixture's Job read carries, or none where it has not arrived. */
function stepsOf(fixture: JobFixture): StepDetail[] {
  return "detail" in fixture.watched ? fixture.watched.detail.steps : [];
}

/** The turns the fixture's socket holds. Kept through `ended` and `failed`. */
function turnsOf(fixture: JobFixture): Turn[] {
  return "turns" in fixture.observed ? fixture.observed.turns.rows : [];
}

export function StepTimelineFrom({ fixture, stepId }: { fixture: JobFixture; stepId?: string }) {
  const steps = stepsOf(fixture);
  const step =
    steps.find((one) => one.step_id === (stepId ?? fixture.job.current_step_id)) ?? steps[0];
  if (step === undefined) return <p>This fixture's Job has no steps to draw.</p>;
  const attempts = timelineOf(step, turnsOf(fixture), fixture.now);
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-4)",
        padding: "var(--space-4)",
        background: "var(--bg-base)",
        minHeight: "100vh",
      }}
    >
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-1)" }}>
        <span className="armada-screen__eyebrow">Step</span>
        <span style={{ fontSize: "var(--text-lg)", fontWeight: "var(--weight-heading)" }}>
          {step.label}
        </span>
      </div>
      {attempts.map((attempt) => (
        <Attempt key={attempt.id} attempt={attempt} step={step} />
      ))}
    </div>
  );
}

/** One attempt: its own heading, and the phases nested under it. */
function Attempt({ attempt, step }: { attempt: TimelineAttempt; step: StepDetail }) {
  // The attempt a person is reading is open; the ones before it are folded,
  // which is the whole of what "easier to view the previous attempts" asks for.
  const [open, setOpen] = useState(attempt.current);
  return (
    <section style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((was) => !was)}
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: "var(--space-2)",
          alignSelf: "flex-start",
          padding: 0,
          border: 0,
          background: "none",
          cursor: "pointer",
          color: "var(--fg-muted)",
          fontFamily: "var(--font-sans)",
          fontSize: "var(--text-xs)",
          lineHeight: "var(--leading-xs)",
        }}
      >
        {open ? <ChevronDown size={14} strokeWidth={2} aria-hidden /> : <ChevronRight size={14} strokeWidth={2} aria-hidden />}
        {`Attempt ${attempt.attempt}`}
        <span style={{ color: "var(--fg-subtle)" }}>{saidOf(attempt)}</span>
      </button>
      {!open ? null : (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "var(--space-2)",
            marginLeft: "var(--space-2)",
            paddingLeft: "var(--space-3)",
            borderLeft: "var(--border-width) solid var(--border-subtle)",
          }}
        >
          {attempt.rows.map((row) => (
            <Row key={row.id} row={row} step={step} />
          ))}
        </div>
      )}
    </section>
  );
}

/**
 * What became of an attempt, in words rather than in the wire's own.
 *
 * **`retrying` is "handed back".** It is the step's word for what happens next,
 * and on an attempt that has ended it reads as a Drone still working — which is
 * the same thing the run tree said until the owner had it changed.
 */
function saidOf(attempt: TimelineAttempt): string {
  const said =
    attempt.outcome === "retrying"
      ? "handed back"
      : attempt.outcome === "advanced"
        ? "advanced"
        : attempt.outcome === "stopped"
          ? "stopped"
          : attempt.outcome;
  return attempt.took === undefined ? said : `${said} · ${attempt.took}`;
}

/** One phase of one attempt, as a row that opens. */
function Row({ row, step }: { row: TimelineRow; step: StepDetail }) {
  const [open, setOpen] = useState(row.live === true);
  return (
    <Chapter
      name={
        <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-2)" }}>
          <StepActivityMark activity={row.mark} label={row.name} pulsing={row.live} />
          {row.name}
        </span>
      }
      {...(row.meta === undefined ? {} : { meta: row.meta })}
      {...(row.live === true ? { live: true } : {})}
      open={open}
      onToggle={() => setOpen((was) => !was)}
    >
      <Body row={row} step={step} />
    </Chapter>
  );
}

/** What a row says when it is opened. A reading, not the chapter itself. */
function Body({ row, step }: { row: TimelineRow; step: StepDetail }) {
  if (row.phase === "working") {
    const { rows, unread } = hideUnread(entriesOf(row.turns ?? [], step.step_id));
    return (
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-1)" }}>
        {rows.map((one) => (
          <span key={one.id} className="text-2xs">
            <span className="mono text-fg-subtle">{one.at}</span> {one.actor} · {one.message}
          </span>
        ))}
        {unread === 0 ? null : <span className="text-2xs text-fg-subtle">{`${unread} hidden`}</span>}
        <Wrote row={row} />
      </div>
    );
  }
  if (row.phase === "checks") {
    return (
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-1)" }}>
        {(row.runs ?? []).map((run) => (
          <span key={`${run.name}-${run.attempt}`} className="text-2xs">
            <span className="mono">{run.name}</span> · {run.outcome}
            {run.produced === undefined ? "" : ` · ${run.produced}`}
          </span>
        ))}
      </div>
    );
  }
  if (row.phase === "judge") {
    return (
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-1)" }}>
        {(row.judged ?? []).map((one) => (
          <span key={`${one.criterion_id}-${one.member ?? 0}`} className="text-2xs">
            {one.verdict} · <span className="mono">{one.criterion_id}</span>
          </span>
        ))}
      </div>
    );
  }
  const opened = (row.turns ?? [])[0];
  return (
    <span className="text-2xs text-fg-muted">
      {opened === undefined
        ? "The words Armada opened this attempt with are in the brief."
        : `Opened at ${clock(opened.ts)}. The brief itself draws here in the panel.`}
    </span>
  );
}

/**
 * What the attempt wrote, under the turns that wrote it — the owner's call:
 * what a Drone did and what came out of it are one reading.
 */
function Wrote({ row }: { row: TimelineRow }) {
  const produced = row.produced ?? [];
  const kept = row.kept ?? [];
  if (produced.length === 0 && kept.length === 0) return null;
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-1)",
        marginTop: "var(--space-2)",
        paddingTop: "var(--space-2)",
        borderTop: "var(--border-width) solid var(--border-subtle)",
      }}
    >
      <span className="armada-screen__eyebrow">Produced</span>
      {produced.map((file) => (
        <span key={file.path} className="text-2xs">
          <span className="mono">{file.path}</span>{" "}
          <span className="text-fg-subtle">{file.change}</span>
        </span>
      ))}
      {kept.map((one) => (
        <span key={one} className="text-2xs mono">
          {one}
        </span>
      ))}
    </div>
  );
}
