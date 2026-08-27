// Watching one Job's turns, drawn from what main published over the second
// socket.
//
// **Read-only, and there is no way for it to be anything else.** The preload
// entry behind this opens a socket that only receives; no route behind it takes
// a message, and nothing on this screen offers to intervene. Pilot ends the
// Drone and moves the Job to `piloted`; observing changes nothing and closing
// the view ends nothing — `docs/concepts/observe.md` is the table.
//
// # The three quiet cases are three sentences
//
// A Job that was never dispatched, a Drone that outlived the Fleet that spawned
// it, and a viewer that fell behind all put fewer rows on screen than a reader
// might expect. Each says which it is, because a transcript that quietly skipped
// rows reads as a Drone that went quiet — the one thing this record exists to
// tell apart.

import type { ReactNode } from "react";
import { WatchingADroneWork, type JobDetailHeading } from "@armada/components";

import type { Observed } from "../../shared/bridge";
import type { JobSummary, ManifestSummary, WorkflowSummary } from "../../shared/protocol";
import { factsOf } from "./facts";
import { readingOf } from "./reading";
import { turnsOf } from "./turns";

/** What a Job with nothing recorded says. Ordinary, and never an error. */
const NOTHING_RECORDED =
  "This job has no turns. Nothing was writing when this opened, so this is the whole history.";

export type ObserveProps = {
  job: JobSummary;
  observed: Observed;
  workflows: readonly WorkflowSummary[];
  manifests: readonly ManifestSummary[];
  /** Now, injected. The header's elapsed is read, so it has to move. */
  now: number;
  /**
   * What sits at the header's trailing edge. **Leaving the view, and nothing
   * else** — a control here that acted on the Drone would be Pilot, which is a
   * different act with a transition on the record. Bridge passes none: the way
   * back sits in the panel head, beside the view's name, where every other
   * leave-this-view control in the window already is.
   */
  actions?: ReactNode;
};

export function Observe({
  job,
  observed,
  workflows,
  manifests,
  now,
  actions,
}: ObserveProps) {
  const reading = readingOf(job);
  const workflow = workflows.find((held) => held.id === job.workflow_id);
  const manifest = manifests.find((held) => held.id === job.owner_manifest_id);

  // The badge is the header, and the list and the detail both refuse a Job the
  // registry has no glyph for rather than half-drawing one. Same answer here.
  if (reading.as !== "badge") {
    return (
      <p className="text-fg-muted">
        {`${job.title} — `}
        <span className="mono">{job.status}</span>
        {". The registry carries no sanctioned rendering for it, so this job has no header to draw."}
      </p>
    );
  }

  const heading: JobDetailHeading = {
    status: reading.status,
    statusIcon: reading.icon,
    statusLabel: reading.verb,
    headline: job.title,
    jobId: job.id,
    fields: factsOf(job, null, workflow, manifest, now),
    actions,
  };

  // Opening, and a socket that would not open. Neither is an empty history: one
  // has not answered yet and the other never will, and a blank pane would read
  // as a Job with nothing to show.
  if (observed.state === "none" || observed.state === "opening") {
    return (
      <WatchingADroneWork
        heading={heading}
        turns={[]}
        emptyNote="Reading this job's turns."
      />
    );
  }
  if (observed.state === "failed") {
    return (
      <WatchingADroneWork
        heading={heading}
        turns={[]}
        emptyNote={NOTHING_RECORDED}
        failure={observed.detail}
      />
    );
  }

  const turns = observed.turns;
  return (
    <WatchingADroneWork
      heading={heading}
      turns={turnsOf(turns.rows)}
      emptyNote={NOTHING_RECORDED}
      live={turns.live}
      skipped={turns.skipped}
      missed={turns.missed}
      // The wire's own spelling. `Silence` has no `enum-verbs.toml` rows, so
      // there is no sanctioned verb for it and none is written here. Reported.
      closedBecause={observed.state === "ended" ? observed.because : undefined}
    />
  );
}
