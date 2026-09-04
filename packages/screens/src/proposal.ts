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

import type { Proposal, ProposalWatch, ProposedJob } from "@armada/components";
import type {
  BridgeIdentity,
  JobSummary,
  Outcome,
  ProposalInFlight,
  Proposed,
  WorkflowSummary,
} from "@armada/protocol";
import { refusalFailure } from "@armada/shell";

export type { Proposed };

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


/**
 * How long a proposal may run before the surface asks whether to keep waiting.
 *
 * **A prompt and not a limit.** Nothing happens at this mark: the call keeps
 * running until Fleet's own budget or until somebody presses stop. What it
 * decides is when the question is put in front of a person rather than left for
 * them to wonder about.
 *
 * Two minutes, chosen against the wait it replaced — Bridge used to abort the
 * request at five seconds, and before that a proposal that took this long was
 * simply lost. It is deliberately well inside Fleet's own proposer budget
 * (`PROVISIONAL_PROPOSER_BUDGET`, ten minutes): a question asked as the call
 * dies is not a question, it is an epitaph.
 *
 * **Unmeasured, like the budget it sits inside.** What would settle it is a
 * distribution of real proposal latencies, which nothing collects yet.
 */
export const PROPOSAL_IS_SLOW = 120_000;

/**
 * What Fleet says about the call in flight, as the surface draws it.
 *
 * `null` where nothing is out, or where the instant will not read — a wait that
 * cannot say how long it has been is drawn as a wait with nothing known about
 * it rather than as one that has been running since the epoch.
 *
 * **The clock is the caller's.** Elapsed is resolved here, against a `now` the
 * app already ticks for everything else, so this figure and every other elapsed
 * figure on screen come from one reading.
 */
export function watchOf(proposing: ProposalInFlight | null, now: number): ProposalWatch | null {
  if (proposing === null) return null;
  const since = Date.parse(proposing.since);
  if (Number.isNaN(since)) return null;
  return {
    reached: proposing.reached,
    // Never negative. A clock a few milliseconds behind Fleet's would otherwise
    // draw a call that has not started yet.
    elapsedMs: Math.max(0, now - since),
    budgetMs: proposing.budget_ms,
    model: proposing.model,
    ...(proposing.thinking_tokens === undefined
      ? {}
      : { thinkingTokens: proposing.thinking_tokens }),
    ...(proposing.answered_characters === undefined
      ? {}
      : { answeredCharacters: proposing.answered_characters }),
  };
}
