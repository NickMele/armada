// What else is running where this work would write. Draft, for
// `crates/ipc/src/overlap.rs`.
//
// Source of truth today: `JobDetail.write_scope_overlaps`, `ScopeOverlap` and
// `SharedPath`. The draft's only change is **when** it can be asked: this is
// for dispatch, before a Job exists, and the wire's field hangs off a Job.
//
// **A fact, never a verdict** — `docs/concepts/fleet.md`, "surfaced, never
// serialised". There is no `blocked` flag here and no severity, because there
// is no field a Bridge could grey the approve button from. A person dispatching
// anyway is the ordinary case.

import type { JobDetail, ScopeOverlap } from "@armada/protocol";

/** One other Job claiming paths this work claims. One entry per Job, not per path. */
export type PeerView = {
  job: string;
  title: string;
  /** Where the other Job is. `running` and `awaiting_review` are different remedies. */
  status: string;
  /** The narrower of the two claims, per shared path. Never empty. */
  shared_paths: string[];
};

/** What was asked, and who else claims it. */
export type PeerOverlapView = {
  /** The paths the comparison was made against. */
  paths_asked: string[];
  peers: PeerView[];
};

/**
 * Nothing was compared, versus a comparison that found nobody.
 *
 * **`null` and `{ peers: [] }` are different sentences and a surface must say
 * each of them** — the rule `write_scope_overlaps` states in its own doc
 * comment, mirrored exactly. `null` is nobody having looked, which is every
 * Job at its approval gate, because the proposer does not fill `write_targets`
 * in. An empty `peers` is a comparison that ran and found nobody. Drawing them
 * the same way says "no overlap" about work nothing looked at.
 */
export type PeerOverlapAnswer = PeerOverlapView | null;

/**
 * The overlap on an existing Job.
 *
 * Absent `write_scope_overlaps` is `null`, absent `write_targets` is no paths
 * asked. Both absences are carried rather than flattened, for the reason above.
 */
export function peerOverlapOf(detail: JobDetail): PeerOverlapAnswer {
  const overlaps = detail.write_scope_overlaps;
  if (overlaps === undefined) {
    return null;
  }
  return {
    paths_asked: detail.write_targets ?? [],
    peers: overlaps.map(peerOf),
  };
}

/**
 * The same answer at dispatch, where there is no Job yet.
 *
 * `overlaps` is what a comparison came back with, and `undefined` is no
 * comparison having been made — which is what a form draws before anybody has
 * typed a path.
 */
export function peerOverlapAsked(
  pathsAsked: readonly string[],
  overlaps?: readonly ScopeOverlap[],
): PeerOverlapAnswer {
  if (overlaps === undefined) {
    return null;
  }
  return { paths_asked: [...pathsAsked], peers: overlaps.map(peerOf) };
}

function peerOf(overlap: ScopeOverlap): PeerView {
  return {
    job: overlap.job_id,
    title: overlap.title,
    status: overlap.status,
    shared_paths: overlap.paths.map((shared) => shared.path),
  };
}
