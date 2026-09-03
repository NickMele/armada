// The dispatch path where a person describes the work and the Job proposer
// reads it. `crates/ipc/src/job.rs` is the other half of every type here.
//
// Its own file rather than more of `protocol.ts` for that file's own stated
// reason — it is at the length the gate refuses — and it is deliberately not
// re-exported from there: nothing imports `protocol.ts` by path, so a re-export
// would spend lines against that ceiling for a symbol every caller already
// reaches through the package index.
//
// **Hand-mirrored, and nothing compares it to the Rust.** The codegen emits the
// version constant and the enum vocabulary, not the DTOs — `[protocol-codegen]`
// in `docs/practices/protocol.md` is the open question, and until it closes a
// field added on one side of this file is a field missing on the other with
// every check green.

import type { JobSummary } from "./protocol";

/**
 * What a person described, before anything has decided what it is. The request
 * half of `propose_from_request`.
 *
 * **One field, and no `workflow_id` among them.** Naming the workflow is the
 * act this operation exists to remove; a request carrying one would be
 * `propose_job` with a model call in front of it.
 *
 * Carried verbatim: Fleet opens no link and fetches nothing, so what the
 * proposer reads is what was typed.
 */
export type JobRequest = {
  request: string;
  /**
   * A token of the caller's own, echoed back on every `proposal.moved` about
   * this call so the caller can recognise its own.
   *
   * **Opaque to Fleet, which neither reads it nor keeps it.** It is not an id:
   * the same token sent twice is two proposals, each with its own
   * `proposal_id`. Matching on `request` instead would match the wrong call the
   * moment two people dispatch the same words.
   *
   * Absent is ordinary — a caller with no surface has nothing to correlate.
   */
  client_ref?: string;
};

/**
 * What one reading of a request proposed. The answer half of
 * `propose_from_request`.
 *
 * **A list even where it holds one**, because approving is a different act
 * depending on how many: one Job is dispatched by its approval, and several are
 * a plan whose members each take their own approval in turn. A shape that
 * answered with one Job would make the second case unrepresentable rather than
 * merely undrawn.
 *
 * In dependency order — an upstream always before what points at it — and all
 * at `awaiting_approval`. Nothing here has been approved and nothing is
 * running.
 */
export type ProposedPlan = {
  jobs: JobSummary[];
};

/**
 * The two codes this route raises that a surface has to tell apart, mirrored
 * from `crates/fleet/src/refusing.rs`, which declares each beside the failure
 * that raises it.
 *
 * **Read, never minted.** Bridge invents no code — one it invented would be in
 * no manifest and mean nothing to the lookup Bridge does. These are Fleet's own
 * spellings, held here because this is the one place on the seam where a client
 * must branch on a code rather than render it: the request being declined and
 * the call failing are different statuses, different advice and different acts,
 * and the code is the only thing on the wire that separates them once a body is
 * a `WireError`.
 *
 * A code that stops matching falls to `faulted`, which advises asking again —
 * wrong for a decline, and the reason `refusing.rs` says these two must never
 * be rendered as each other.
 */

/** The request was read and no workflow fits. 422. The request comes back. */
export const NO_WORKFLOW_FITS = "fleet.no_workflow_fits";

/** The proposer call could not be made — the network, the quota, the timeout. 500. */
export const PROPOSER_UNREACHABLE = "fleet.proposer_unreachable";

/**
 * Something under the daemon failed. 500, and read here for
 * `PROPOSER_UNREACHABLE`'s reason: it is Fleet's own name for a failure that is
 * not the caller's doing and that asking again is reasonable about, which is
 * the same advice and therefore the same arm.
 */
export const FLEET_FAULT = "fleet.fault";
