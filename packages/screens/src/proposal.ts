// What `proposeFromRequest` answered, read into what the dispatch surface
// draws.
//
// **The one place the seam is read.** `DispatchRequest` in `@armada/components`
// knows nothing about the wire and the app does no reading of its own, so a
// change to what Fleet answers lands here and nowhere else.
//
// # Two refusals, and a third that is neither
//
// `unresolved` is Fleet reading the request and declining: no workflow fits, no
// Job was created, and the request comes back so nothing anybody typed is lost.
// **It is not an error** — it takes no code and no red, because Armada worked.
//
// `faulted` is the call not being made at all. That is Armada failing, so it
// takes the error treatment and the code every error carries.
//
// `refused` is neither: it is a command Bridge or Fleet turned down before any
// of this, which every other act in the app already draws one way. It goes back
// to the caller as an `Outcome` rather than being redrawn here, so a proposer
// refused for being disconnected reads exactly like an approval refused for it.
//
// # Where a fault with no code goes
//
// A model call that failed comes back as `refused` inside `faulted`, carrying a
// `WireError` — a code, a message, fields and a chain. That is drawn in the
// surface, inline, because a proposer that could not be called stops this
// surface and reaches nothing else.
//
// A `faulted` carrying anything else has no code, and **nothing here mints
// one**: the `bridge.` namespace is declared in `packages/shell` beside the
// builder that raises it, and a code invented at a call site is the second
// producer that whole arrangement exists to prevent. So it goes back as an
// `Outcome` too, and the app draws it where it draws every other one.

import type { Proposal, ProposedJob } from "@armada/components";
import type { BridgeIdentity, JobSummary, Outcome, WorkflowSummary } from "@armada/protocol";
import { refusalFailure } from "@armada/shell";

/**
 * What `window.armada.proposeFromRequest` answers.
 *
 * **Declared here until `@armada/protocol` carries it.** The two halves of this
 * feature were built either side of the seam at once; this is the surface
 * half's reading of the shape, and it is deleted the moment the protocol
 * package exports the same union.
 */
export type Proposed =
  /**
   * Every Job the request became, in dependency order, **already folded onto
   * the board**. They exist at `awaiting_approval` whether or not this surface
   * draws them, which is why nothing downstream may read as though approving is
   * what creates them.
   */
  | { ok: true; jobs: JobSummary[] }
  /** No workflow fits. No Job was created, and `request` is Fleet's echo. */
  | { ok: false; why: "unresolved"; request: string; outcome: Outcome }
  /** The call could not be made. Says nothing about the request. */
  | { ok: false; why: "faulted"; request: string; outcome: Outcome }
  /** Turned down before sending, or another Fleet refusal. */
  | { ok: false; why: "refused"; outcome: Outcome };

/** What the surface draws, and what the app has to say somewhere else. */
export type Answered = {
  proposal: Proposal;
  /**
   * A refusal the app draws in its own failure pipeline, or `null` where the
   * surface drew the whole answer itself.
   */
  outcome: Outcome | null;
  /**
   * What the request field should hold, or `null` to leave it alone. Fleet's
   * own echo where it sent one — the claim that a refused request comes back
   * unchanged is only true if it is the returned string that comes back.
   */
  request: string | null;
};

/** What the reading needs beyond the answer itself. */
export type ProposalReading = {
  /**
   * The request that was dispatched. **The answer carries none on success** —
   * the Jobs came back and the question did not — and a proposal is only
   * readable against what was asked, so the caller keeps it and hands it here.
   */
  sent: string;
  /** The workflows Fleet holds, so a proposal names one rather than an id. */
  workflows: readonly WorkflowSummary[];
  /** Both protocol versions, for the payload a fault is quoted from. */
  bridge: BridgeIdentity;
  /** When the answer arrived, ISO 8601. Passed in, so this stays a function. */
  at: string;
};

export function answeredAs(answer: Proposed, seen: ProposalReading): Answered {
  if (answer.ok) {
    return {
      proposal: {
        at: "proposed",
        request: seen.sent,
        jobs: answer.jobs.map((job) => became(job, seen.workflows)),
      },
      outcome: null,
      request: null,
    };
  }

  if (answer.why === "unresolved") {
    // The inner outcome is deliberately not rendered. `said()` would answer
    // "A job needs a workflow Fleet holds", which is the composer refusing a
    // pasted id — a different refusal wearing the same words. What this one
    // means is that nothing in the catalogue fits, and the surface says it.
    return { proposal: { at: "unresolved" }, outcome: null, request: answer.request };
  }

  if (answer.why === "faulted" && answer.outcome.ok === false && answer.outcome.why === "refused") {
    const error = answer.outcome.error;
    return {
      proposal: {
        at: "faulted",
        code: error.code,
        message: error.message,
        // Built by the one producer, with the instant stamped when the answer
        // arrived rather than on every render — a standing failure whose
        // timestamp moved would be a fact nobody could quote.
        payload: { ...refusalFailure(error, seen.bridge).payload, at: seen.at },
      },
      outcome: null,
      request: answer.request,
    };
  }

  return {
    proposal: { at: "unasked" },
    outcome: answer.outcome,
    request: answer.why === "faulted" ? answer.request : null,
  };
}

/**
 * One Job the request became, as the surface draws it.
 *
 * **The workflow by name, and the status off the wire.** Neither is asserted
 * here: an id is the one field on a proposal a person cannot check, and a
 * status this surface wrote itself would be Bridge claiming something Fleet
 * told it.
 *
 * **No scope, because none was proposed.** A Job reaches this gate with
 * `write_targets` null, and there is no field here that could carry one.
 */
function became(job: JobSummary, workflows: readonly WorkflowSummary[]): ProposedJob {
  const workflow = workflows.find((held) => held.id === job.workflow_id);
  return {
    id: job.id,
    title: job.title,
    workflow: workflow === undefined ? job.workflow_id : workflow.name,
    status: job.status,
  };
}
