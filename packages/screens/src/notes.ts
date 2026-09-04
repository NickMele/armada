// What Armada has done to the Job itself — the second per-Job socket, as rows.
//
// **Its own file beside `story.ts`, not inside it.** That one is a step's
// story, bounded to the step and read off a Drone's transcript. This is the
// Job's own log, which belongs to no step and exists when no Drone does — the
// distinction the owner settled on 4 Sep, and the reason the two are drawn in
// two places.
//
// **One row type for both, deliberately.** A note and a turn draw through the
// same component under the same rules: the instant, then who, then the one
// line, then what it opens to. A second row type would be a second grammar in
// a column whose whole claim is that it has one, and a log file rendered as
// text is what that becomes.

import type { Journalled, Noted } from "@armada/protocol";

import { clock } from "./duration";
import type { LogLine, LogRow } from "./story";

/**
 * The lines of a Job's own log, as rows — what Fleet did, off the second
 * socket.
 *
 * **One row type for both streams, deliberately.** A note and a turn are drawn
 * by the same component under the same rules — the instant, then who, then the
 * one line, then what it opens to — and a second row type would be a second
 * grammar in a column whose whole claim is that it has one.
 *
 * `by` is the wire's, exactly as it is for a turn. Nothing here infers Fleet
 * from the fact that this is Fleet's log: three voices in one column with
 * attribution inferred for one of them is the failure the component was written
 * against, one layer up.
 */
export function notesOf(notes: readonly Noted[]): LogRow[] {
  return notes.map((note) => ({
    id: `note-${note.seq}`,
    at: clock(note.at),
    actor: note.by,
    kind: "note" as const,
    message: note.msg,
    // The fields are what the row opens to, and a note with none opens to
    // nothing rather than to a sentence about itself. **Every entry opens to
    // its payload** is the component's rule; a Fleet event drawn as grey prose
    // in a column of openable rows is the second-class citizen this stream was
    // built not to be.
    payload: (note.fields ?? []).map((field) => ({
      text: `${field.name}  ${field.value}`,
      named: named(note.level),
    })),
  }));
}

/**
 * How a note's fields are named, from the level the line was written at.
 *
 * **Only the two levels that mean something went wrong are named.** `named` is
 * a hue on a payload line and most of what happens is not a verdict, so an
 * ordinary note draws as body — which is the same restraint `ranTo` shows about
 * a Check that passed.
 */
function named(level: Noted["level"]): LogLine["named"] {
  if (level === "error") return "failed";
  if (level === "warn") return "echo";
  return "meta";
}

/**
 * What the log socket says about its own reading, or nothing while it is
 * reading. [`whyNotWatching`]'s twin, and its sentences are about the Job's own
 * log rather than about a transcript.
 *
 * **A Job Fleet has done nothing to yet is not one of these.** That is
 * `watching` with no notes, and it draws the ordinary empty line — the state
 * these five exist to keep apart from it.
 */
export function whyNoNotes(journalled: Journalled): string | undefined {
  switch (journalled.state) {
    case "watching":
      return undefined;
    case "opening":
      return "Armada is opening this job's log.";
    case "none":
      return "Armada is not reading this job's log.";
    case "failed":
      return `This job's log could not be read. ${journalled.detail}`;
    case "ended":
      return journalled.because === "unreadable"
        ? "This job's log is there and Armada could not read it."
        : `The log stopped: ${journalled.because}.`;
  }
}

/** What a Job Fleet has recorded nothing about says. Ordinary, never an error. */
export const NOTHING_FROM_FLEET_YET = "Armada has not recorded anything about this job yet.";
