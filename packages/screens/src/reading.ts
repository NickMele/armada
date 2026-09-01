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

import { ESCALATION_REASON, JOB_STATUS, QUEUED_REASON } from "@armada/components";
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

/** The vocabulary a status takes its reason from, where it takes one. */
function reasonOf(status: string, named: string | undefined): Rendering | undefined {
  if (named === undefined) return undefined;
  if (status === "queued") return QUEUED_REASON[named];
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
  if (job.status === "queued") return job.queued_reason;
  return job.reason?.named;
}

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
