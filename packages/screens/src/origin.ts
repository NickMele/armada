// Where a job came from, in the registry's own words. The board row and job
// detail both draw it, so the reading is here rather than in either of them.
//
// **The words are never written here.** `enum-verbs.toml`'s `origin` table is
// the only place an origin is spelled, carried to Bridge as the generated
// `ORIGIN`; this fills the one slot the registry leaves — `sub_dispatched`
// reads `Sub-dispatched by job_2d90bb`, and the parent's id is part of what a
// person reads rather than a decoration around a word.

import { ORIGIN } from "@armada/components";
import type { JobSummary } from "@armada/protocol";

/**
 * What the row and the header say about where this job came from, or
 * `undefined` where there is nothing to say.
 *
 * **Undefined on an origin the vocabulary does not hold**, which is a Fleet
 * ahead of this Bridge. The map already answers `undefined` for a key it does
 * not have, and drawing nothing is what that build can truthfully say.
 *
 * **Undefined too on a form with nothing to fill it.** A Fleet built before
 * `JobSummary.dispatched_by` sends `sub_dispatched` with no parent id, and the
 * literal template is worse than a blank.
 */
export function originReading(job: JobSummary): string | undefined {
  const verb = ORIGIN[job.origin]?.verb;
  if (verb === null || verb === undefined) return undefined;
  if (!verb.includes("{")) return verb;
  if (job.dispatched_by === undefined) return undefined;
  return verb.replace("{dispatched_by.job_id}", job.dispatched_by);
}

/**
 * Whether this job reached the board off a Studio — the two values
 * `crates/fleet/src/promoting.rs` writes when dispatch leaves one.
 *
 * **Read off `origin` and not off `from_studio`.** The edge that names which
 * Studio is deleted with the Studio; this is not, so it is what lets the
 * detail say *that Studio is no longer there* rather than saying nothing.
 */
export function fromAStudio(job: JobSummary): boolean {
  return job.origin === "studio_dispatched" || job.origin === "studio_helm_drafted";
}
