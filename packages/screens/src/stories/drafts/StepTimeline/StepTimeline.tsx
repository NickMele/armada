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
    // **Bounded, because a real step is not small.** The recorded Job's
    // implement step carries 1763 turns, 770 of them readable, and a row that
    // draws them all is a row nobody can scroll past. The panel proper opens
    // the log chapter, which is bounded and followable; this says the counts
    // and draws the end of the work, which is where a person looks first.
    const shown = rows.slice(-TURNS_DRAWN);
    return (
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-1)" }}>
        <span style={SUBTLE}>{turnsSaid(rows.length, unread, shown.length)}</span>
        {shown.map((one) => (
          <span key={one.id} style={SMALL}>
            <span className="mono" style={{ color: "var(--fg-subtle)" }}>
              {one.at}
            </span>{" "}
            {one.actor} · {one.message}
          </span>
        ))}
        <Wrote row={row} />
      </div>
    );
  }
  if (row.phase === "checks") {
    return (
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-1)" }}>
        {(row.runs ?? []).map((run) => (
          <span key={`${run.name}-${run.attempt}`} style={SMALL}>
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
          <span key={`${one.criterion_id}-${one.member ?? 0}`} style={SMALL}>
            {one.verdict} · <span className="mono">{one.criterion_id}</span>
          </span>
        ))}
      </div>
    );
  }
  const opened = (row.turns ?? [])[0];
  return (
    <span style={MUTED}>
      {opened === undefined
        ? "The words Armada opened this attempt with are in the brief."
        : `Opened at ${clock(opened.ts)}. The brief itself draws here in the panel.`}
    </span>
  );
}

/**
 * The body's own small type, and the two steps down from it.
 *
 * **Written as style rather than as utilities, because utilities are inert
 * here.** `text-2xs` and `text-fg-muted` are Tailwind's, generated where the
 * whole of Tailwind is imported — which the desktop renderer does and Storybook
 * does not: its preview loads the preflight alone, and says so. So a draft
 * written in them reads correctly in the app and draws at the inherited size in
 * the one place it exists to be looked at.
 */
const SMALL = { fontSize: "var(--text-2xs)", lineHeight: "var(--leading-2xs)" } as const;
const SUBTLE = { ...SMALL, color: "var(--fg-subtle)" } as const;
const MUTED = { ...SMALL, color: "var(--fg-muted)" } as const;

/** How many turns the draft draws, newest last. The log chapter has the rest. */
const TURNS_DRAWN = 12;

/** How many files it names before it counts the rest. The diff has them all. */
const FILES_DRAWN = 8;

/** What the body says about the turns it is not drawing. */
function turnsSaid(readable: number, unread: number, shown: number): string {
  const held = readable + unread;
  const counted = `${held} ${held === 1 ? "turn" : "turns"}`;
  const hidden = unread === 0 ? "" : `, ${unread} of them unread rows`;
  return shown >= readable
    ? `${counted}${hidden}`
    : `${counted}${hidden} · the last ${shown} are here, and the log has the rest`;
}

/**
 * What the attempt wrote, under the turns that wrote it — the owner's call:
 * what a Drone did and what came out of it are one reading.
 */
function Wrote({ row }: { row: TimelineRow }) {
  const produced = row.produced ?? [];
  const kept = row.kept ?? [];
  if (produced.length === 0 && kept.length === 0) return null;
  const files = produced.slice(0, FILES_DRAWN);
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
      <span className="armada-screen__eyebrow">
        {`Produced · ${produced.length} ${produced.length === 1 ? "file" : "files"}`}
      </span>
      {files.map((file) => (
        <span key={file.path} style={SMALL}>
          <span className="mono">{file.path}</span>{" "}
          <span style={{ color: "var(--fg-subtle)" }}>{file.change}</span>
        </span>
      ))}
      {produced.length <= FILES_DRAWN ? null : (
        <span style={SUBTLE}>{`${produced.length - FILES_DRAWN} more, in the diff`}</span>
      )}
      {kept.map((one) => (
        <span key={one} className="mono" style={SMALL}>
          {one}
        </span>
      ))}
    </div>
  );
}
