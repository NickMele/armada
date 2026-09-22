// A Job and the Jobs landing under it. Draft, for `crates/ipc/src/job.rs`.
//
// Source of truth today: `JobSummary.dispatched_by` — the parent Job's id,
// present where `origin` is `sub_dispatched` (protocol 14.2) — the Board row's
// own `branch`, `tasks` and `landed`, and `JobDelivery`, which says whether
// each one opened a pull request and whether that pull request merged.
//
// **No kind name.** A Job is a Job (#1530, 22 Sep): nothing here is called a
// Convoy, a Train or an Epic, and a screen built on it says "three pull
// requests, landing in order" rather than naming a shape. Today's
// one-Job-one-pull-request Job is an ordinary Job with several groups, so it
// has no members and that is a real answer rather than a gap.

import type { JobDetail, JobSummary, JudgeQuestion, TaskCounts } from "@armada/protocol";

/**
 * How a member's work reaches the **one before it**, as
 * `docs/concepts/landing.md` defines the three.
 *
 * | Link | What the later member does |
 * |---|---|
 * | `stacked` | Branches off the earlier member's branch and opens at once. Rebases when that branch lands |
 * | `merged` | Parks until the earlier member lands, and targets where the Job lands |
 * | `published` | Waits on an artifact the earlier member's merge produces, rather than on the merge |
 *
 * **Not served in any form today.** Only `merged` is built, as `depends_on`,
 * and nothing on the wire says which of the three an edge is — so the
 * derivation below reads the one link Fleet can make rather than guessing from
 * what a member happens to have done.
 */
export type MemberLink = "stacked" | "merged" | "published";

/** One Job landing under a parent. */
export type MemberView = {
  job: string;
  title: string;
  status: string;
  /**
   * How it reaches the member before it. **Absent on the first**, which has
   * nothing before it — never a link standing in for none.
   */
  link?: MemberLink;
  /**
   * Whether its pull request merged.
   *
   * **The merge, never `completed_success`** (#1530): a Job whose gate hands
   * off to a person reaches a successful status before anybody merges it, and
   * the parent is done when every member has *landed*.
   */
  landed: boolean;
  /**
   * When it landed. **Absent today for every member**: the wire says *whether*
   * a pull request settled (`JobDelivery.landed`) and never when, and a Job's
   * own `ended_at` is a different instant.
   */
  landed_at?: string;
  /** The branch its work sits on. Absent until a worktree exists. */
  branch?: string;
  /** The address of its pull request, where it has opened one. */
  pull_request?: string;
  /** The paths it writes — the member's own `write_targets`. */
  scope?: readonly string[];
  /** How many of its plan's tasks stand where. Absent is a member with no plan. */
  tasks?: TaskCounts;
  /**
   * The Judge refusal this member is holding a person up on.
   *
   * **The wire's own `JudgeQuestion`**, because answering it from the parent
   * sends `answer_judge` against this member's Job id — the same verdict as
   * answering it on the member itself, rather than a second record of one.
   */
  question?: JudgeQuestion;
  /**
   * Why somebody dropped this member. Present only on a dropped one, whose
   * pull request is closed and whose branch is kept.
   */
  dropped?: string;
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
 * `reads` is each member's own record where Bridge has read one, keyed by Job
 * id. The Board row carries the status, the branch, the task counts and
 * whether the pull request merged; the address of that pull request, the
 * paths the member writes and the question it is holding are only in the
 * member's own `get_job`, and a member nobody opened says nothing about them
 * rather than drawing an empty one.
 */
export function jobMembersOf(
  detail: JobDetail,
  board: readonly JobSummary[],
  reads: Readonly<Record<string, JobDetail | undefined>> = {},
): JobMembersView {
  const members = board
    .filter((row) => row.dispatched_by === detail.job.id)
    .map((row, at): MemberView => {
      const read = reads[row.id];
      return {
        job: row.id,
        title: row.title,
        status: row.status,
        ...(at === 0 ? {} : { link: DERIVED_LINK }),
        landed: row.landed === "merged" || read?.delivery?.landed === "merged",
        ...(row.branch === undefined ? {} : { branch: row.branch }),
        ...(read?.delivery?.pull_request === undefined
          ? {}
          : { pull_request: read.delivery.pull_request }),
        ...(read?.write_targets === undefined ? {} : { scope: read.write_targets }),
        ...(row.tasks === undefined ? {} : { tasks: row.tasks }),
        ...(read?.judge_question === undefined ? {} : { question: read.judge_question }),
      };
    });
  return { job: detail.job.id, title: detail.job.title, members };
}

/**
 * What every member after the first reads, because it is the only link Fleet
 * builds.
 *
 * `docs/concepts/landing.md`: a dependent parks at `blocked_by_dependency`
 * until the one before it reaches `completed_success`, nothing stacks branches
 * and nothing runs after a member's pull request merges. Reading `stacked` off
 * a member that has not opened a pull request yet — which is what this
 * derivation did until #1543 — says the branches are stacked when nothing
 * stacked them, and reading `merged` off one whose own pull request landed
 * spends the word on a different fact than the link carries.
 */
const DERIVED_LINK: MemberLink = "merged";
