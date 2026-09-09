// How a Job's state reads, from the map that owns the answer.
//
// **No verb, glyph or hue is chosen here.** All three come from
// `crates/core-model/domain/enum-verbs.toml` through the generated module, and
// the only thing this file holds is the rule the status grammar states: for
// `escalated` and `queued` the headline is the reason rather than the state,
// because nobody says a Job escalated at step 3.
//
// Where the registry carries no verb or no glyph, the variant renders as what
// is there and is named in the report — never filled in with copy invented at
// the call site.

import type { LucideIcon } from "lucide-react";

import { BUDGET_HOLD, ESCALATION_REASON, JOB_STATUS, QUEUED_REASON } from "@armada/components";
import type { Rendering } from "@armada/components";
import type { JobSummary } from "@armada/protocol";

export type Reading =
  /** Everything the badge needs. The only shape that renders as a pill. */
  | { as: "badge"; status: string; icon: LucideIcon; verb: string }
  /**
   * A variant the registry has no sanctioned copy or glyph for. It renders as
   * whatever is there plus the wire spelling, which is recoverable and never
   * primary — a queue that reads like a stack trace is the other failure.
   */
  | { as: "text"; verb: string | null; wire: string; missing: readonly string[] };

/**
 * The vocabulary a status takes its reason from, where it takes one.
 *
 * **Two tables for `queued`, because the finer word is its own.** Their keys
 * are disjoint, so the order below is one lookup across both rather than a
 * precedence between them.
 */
function reasonOf(status: string, named: string | undefined): Rendering | undefined {
  if (named === undefined) return undefined;
  if (status === "queued") return BUDGET_HOLD[named] ?? QUEUED_REASON[named];
  if (status === "escalated") return ESCALATION_REASON[named];
  return undefined;
}

/**
 * Which field a status keeps its reason in.
 *
 * **`queued` keeps it in its own**, because it is computed from the board at
 * read time rather than recorded by a transition — so it is not in the log
 * `reason` is read from, and reading it there answered `undefined` on every
 * queued Job.
 */
function namedOn(job: JobSummary): string | undefined {
  if (job.status === "queued") {
    // **`over_budget` reads finer where the wire says which ceiling caught
    // it.** The money cap and the turn cap fold to one word, only one of them
    // has the control a person is about to press, and a header saying `Over
    // budget` leaves them to work out which. `budget_hold` refines the reason
    // the way `admission_hold` refines `waiting on resources`.
    //
    // The fold stays where the field is absent — an older Fleet — because the
    // coarse word is still true there.
    const finer = job.queued_reason === OVER_BUDGET ? job.budget_hold : undefined;
    // Only where this build can render it. A spelling from a newer Fleet falls
    // back to the coarse word rather than past both tables to the bare status,
    // which would say `queued` about a job nothing is going to start.
    if (finer !== undefined && BUDGET_HOLD[finer] !== undefined) return finer;
    return job.queued_reason;
  }
  return job.reason?.named;
}

/**
 * The one queued reason two words can be true of. `enum-verbs.toml`'s
 * spelling, named once rather than typed at the comparison.
 */
const OVER_BUDGET = "over_budget";

export function readingOf(job: JobSummary): Reading {
  const base = JOB_STATUS[job.status];
  if (base === undefined) {
    // A spelling this build's registry does not have. Fleet refuses one it does
    // not know, so this is Bridge behind Fleet rather than a bad message.
    return { as: "text", verb: null, wire: job.status, missing: ["variant"] };
  }

  const named = namedOn(job);
  const reason = reasonOf(job.status, named);
  const verb = reason?.verb ?? base.verb;
  const icon = reason?.icon ?? base.icon;
  const wire = reason === undefined ? job.status : (named ?? job.status);

  if (verb === null || icon === null || base.badgeStatus === null) {
    const missing = [
      ...(verb === null ? ["verb"] : []),
      ...(icon === null ? ["icon"] : []),
      ...(base.badgeStatus === null ? ["token"] : []),
    ];
    return { as: "text", verb, wire, missing };
  }
  return { as: "badge", status: base.badgeStatus, icon, verb };
}

/**
 * A registry verb, opening a sentence.
 *
 * **The word is still the registry's.** `enum-verbs.toml` spells its verbs for
 * the sentence they usually sit in the middle of; capitalising the first letter
 * where one leads is presentation, not a second spelling — nothing here
 * chooses, shortens or rewrites the word.
 *
 * Here rather than in the two files that call it, because both are in this
 * package and a helper copied inside one package is the drift this module
 * exists to argue against. `badge.ts` in `@armada/components` spells it a third
 * time; that one is across a package boundary and stays. Reported.
 */
export function leading(verb: string): string {
  return verb.charAt(0).toUpperCase() + verb.slice(1);
}
