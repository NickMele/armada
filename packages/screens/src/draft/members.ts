// A Job and the Jobs landing under it. Draft, for `crates/ipc/src/job.rs`.
//
// Source of truth today: `JobSummary.dispatched_by` — the parent Job's id,
// present where `origin` is `sub_dispatched` (protocol 14.2) — and
// `JobDelivery`, which says whether each one opened a pull request and whether
// that pull request merged.
//
// **No kind name.** A Job is a Job (#1530, 22 Sep): nothing here is called a
// Convoy, a Train or an Epic, and a screen built on it says "three pull
// requests, landing in order" rather than naming a shape. Today's
// one-Job-one-pull-request Job is an ordinary Job with several groups, so it
// has no members and that is a real answer rather than a gap.

import type { JobDelivery, JobDetail, JobSummary } from "@armada/protocol";

/**
 * How a member's work reaches the parent's.
 *
 * **Not served in any form today.** Fleet records one pull request per Job and
 * nothing about how two of them relate, so the derivation below reads the link
 * off what each member *did* rather than off a declared intent.
 */
export type MemberLink = "stacked" | "merged" | "published";

/** One Job landing under a parent. */
export type MemberView = {
  job: string;
  title: string;
  status: string;
  link: MemberLink;
  /**
   * When it landed. **Absent today for every member**: the wire says *whether*
   * a pull request settled (`JobDelivery.landed`) and never when, and a Job's
   * own `ended_at` is a different instant — a member counts as landed when its
   * pull request merged, not at `completed_success` (#1530).
   */
  landed_at?: string;
};

/** A parent Job and its members. */
export type JobMembersView = {
  /** The parent Job's id. */
  job: string;
  title: string;
  members: MemberView[];
};

/**
 * The members of a Job, from the Board's own rows.
 *
 * `board` is every Job Bridge is holding; a member is one whose `dispatched_by`
 * names this Job. **A Job with no such row has no members** and draws as the
 * ordinary Job it is.
 *
 * `deliveries` is what the caller already read per member, keyed by Job id. A
 * Board row carries `landed` and not the pull request, so a member nothing was
 * read for reads `stacked` rather than guessing it was published.
 */
export function jobMembersOf(
  detail: JobDetail,
  board: readonly JobSummary[],
  deliveries: Readonly<Record<string, JobDelivery | undefined>> = {},
): JobMembersView {
  const members = board
    .filter((row) => row.dispatched_by === detail.job.id)
    .map((row) => ({
      job: row.id,
      title: row.title,
      status: row.status,
      link: linkOf(row, deliveries[row.id]),
    }));
  return { job: detail.job.id, title: detail.job.title, members };
}

// A member that merged reads `merged`; one that opened a pull request and has
// not, `published`; one that has neither, `stacked` — the honest word for work
// that exists and has not been offered anywhere yet.
//
// `JobSummary.landed` carries the same `Settled` the detail's delivery does, so
// the merged case is answered off the Board row alone.
function linkOf(row: JobSummary, delivery: JobDelivery | undefined): MemberLink {
  if (row.landed === "merged" || delivery?.landed === "merged") {
    return "merged";
  }
  return delivery?.pull_request === undefined ? "stacked" : "published";
}
