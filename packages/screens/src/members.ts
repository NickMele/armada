// A Job whose members are Jobs, read for the Overview region that draws it.
//
// Everything here is arithmetic and copy over `draft/members.ts` and
// `draft/landing.ts`: which branch each member targets, what finishes the
// parent, and what answering one member's question moves. It is a module of
// its own so those can be tested a hundred cases at a time — a `play` that
// computed them would be a unit test paying a browser's price.

import { useState } from "react";

import type { JudgeAnswer } from "@armada/protocol";
import { JOB_STATUS } from "@armada/components";
import type { JobMemberRow, JobMembersProps, JobMemberState } from "@armada/components";

import type { JobMembersView, MemberView } from "./draft/members";
import type { LandingRule } from "./draft/landing";

/** What the Overview region needs from the screen to draw an act. */
export type MemberActs = {
  /** Open one member's own Job. */
  onOpenJob?: (jobId: string) => void;
  /**
   * Open one member's pull request where it lives. **Takes the member's Job
   * id, never the address the card drew** — `opening.ts`'s rule, so the string
   * on screen cannot decide what opens.
   */
  onOpenPullRequest?: (jobId: string) => void;
  /**
   * Answer the Judge refusal a member is holding — `answer_judge`, against
   * that member's Job id. The same verdict as answering on its own screen.
   */
  onAnswerJudge?: (jobId: string, askedAt: string, answer: JudgeAnswer, note?: string) => void;
  /**
   * Drop a member. **Offered only where the caller hands one in**, because no
   * operation serves it: Fleet neither closes a pull request nor rebases what
   * was stacked on the branch.
   */
  onDropMember?: (jobId: string, reason: string) => void;
  /** Whether this Job is live, and whether something is already on its way. */
  stale?: boolean;
  acting?: boolean;
};

/**
 * The region's whole input, or nothing where this Job has no members.
 *
 * **Nothing draws for an ordinary Job**, which is every Job Fleet runs today:
 * one Job, one pull request, and no order to read.
 */
export function membersOf(
  view: JobMembersView | undefined,
  landing: LandingRule | undefined,
  acts: MemberActs = {},
): JobMembersProps | undefined {
  if (view === undefined || view.members.length === 0) return undefined;
  return {
    completeWhen: completeWhenSaid(view.members, landing),
    members: view.members.map((member, at) => rowOf(member, at, view.members, landing, acts)),
  };
}

function rowOf(
  member: MemberView,
  at: number,
  all: readonly MemberView[],
  landing: LandingRule | undefined,
  acts: MemberActs,
): JobMemberRow {
  const targets = targetOf(member, all[at - 1], landing);
  const label = pullRequestLabel(member.pull_request);
  const question = member.question;
  const canDrop =
    acts.onDropMember !== undefined && !member.landed && member.dropped === undefined;
  return {
    id: member.job,
    ordinal: at + 1,
    title: member.title,
    state: stateOf(member.status),
    landed: member.landed,
    ...(member.link === undefined ? {} : { link: member.link }),
    ...(targets === undefined ? {} : { targets }),
    ...(member.pull_request === undefined ? {} : { pullRequest: member.pull_request }),
    ...(label === undefined ? {} : { pullRequestLabel: label }),
    ...(member.branch === undefined ? {} : { branch: member.branch }),
    ...(member.scope === undefined ? {} : { scope: member.scope }),
    ...(tasksSaid(member) === undefined ? {} : { tasks: tasksSaid(member) }),
    ...(member.dropped === undefined ? {} : { dropped: member.dropped }),
    ...(acts.onOpenJob === undefined ? {} : { onOpen: () => acts.onOpenJob?.(member.job) }),
    ...(acts.onOpenPullRequest === undefined
      ? {}
      : { onOpenPullRequest: () => acts.onOpenPullRequest?.(member.job) }),
    ...(canDrop ? { onDrop: (reason: string) => acts.onDropMember?.(member.job, reason) } : {}),
    ...(question === undefined || acts.onAnswerJudge === undefined
      ? {}
      : {
          question: {
            question: question.question,
            expected: question.expected,
            produced: question.produced,
            consequence: question.consequence,
            moves: movesSaid(all.slice(at + 1)),
            disabled: acts.stale === true || acts.acting === true,
            ...(acts.stale === true
              ? { disabledNote: "This job is not live, so nothing can be sent." }
              : acts.acting === true
                ? { disabledNote: "Something sent to this job is still on its way to Fleet." }
                : {}),
            onAnswer: (answer: JudgeAnswer, note?: string) =>
              acts.onAnswerJudge?.(member.job, question.asked_at, answer, note),
          },
        }),
  };
}

/**
 * How a status reads.
 *
 * **The verb, the glyph and the token are the registry's** — a status this
 * build has no row for renders its wire spelling and says so, rather than
 * borrowing a word from a status that happens to look near it.
 */
function stateOf(status: string): JobMemberState {
  const base = JOB_STATUS[status];
  if (base === undefined) {
    return { as: "text", wire: status, missing: `No row in the registry for ${status}` };
  }
  if (base.verb === null || base.icon === null || base.badgeStatus === null) {
    return { as: "text", wire: status, missing: `No verb or glyph in the registry for ${status}` };
  }
  return { as: "badge", status: base.badgeStatus, icon: base.icon, label: base.verb };
}

/**
 * Which branch a member's pull request targets.
 *
 * **A stacked member targets the branch before it; everything else targets
 * where the Job lands** — the issue's own rule, and `published` is not
 * stacked, so it lands where the Job does and waits on a release besides.
 */
function targetOf(
  member: MemberView,
  before: MemberView | undefined,
  landing: LandingRule | undefined,
): string | undefined {
  if (member.link === "stacked") return before?.branch;
  return landing?.target ?? undefined;
}

/**
 * A pull request by its number rather than its whole address — the last
 * segment of the path. A URL drawn in full is a line of chrome across a card.
 */
function pullRequestLabel(address: string | undefined): string | undefined {
  if (address === undefined) return undefined;
  const last = address.split("/").filter((part) => part !== "").pop();
  return last === undefined ? undefined : `#${last}`;
}

/**
 * A member's plan, counted. **Dropped tasks are out of both halves**, the
 * count the plan region already draws: a dropped task is not work owed.
 */
function tasksSaid(member: MemberView): string | undefined {
  const counts = member.tasks;
  if (counts === undefined) return undefined;
  const total = counts.done + counts.working + counts.open;
  return total === 0 ? undefined : `${counts.done} of ${total}`;
}

/**
 * What finishes the parent, and how far along it is.
 *
 * **`all_members_landed` is the default where the Job said nothing**, because
 * a Job with members is done when its members are (#1530). Landed is the
 * merge, so the count is of pull requests in and never of Jobs that reached a
 * successful status.
 */
export function completeWhenSaid(
  members: readonly MemberView[],
  landing: LandingRule | undefined,
): string {
  const counted = members.filter((member) => member.dropped === undefined);
  const landed = counted.filter((member) => member.landed).length;
  const count = `${landed} of ${counted.length} pull ${counted.length === 1 ? "request" : "requests"} merged.`;
  const rule = landing?.complete_when;
  if (rule === "pr_merged") return `Done when this job's own pull request merges. ${count}`;
  if (rule === "pr_opened") return `Done when this job's pull request is open. ${count}`;
  if (rule === "delivered") return `Done when the delivering step has delivered. ${count}`;
  return `Done when every member has landed. ${count}`;
}

/**
 * What answering one member's question moves. **Named, not counted alone** —
 * the press stops or advances work somebody else is waiting on, and which
 * work that is is the thing a person needs before pressing.
 */
export function movesSaid(behind: readonly MemberView[]): string {
  const waiting = behind.filter((member) => member.dropped === undefined);
  if (waiting.length === 0) return "Nothing else is waiting on this answer.";
  const titles = waiting.map((member) => member.title).join(", ");
  const said =
    waiting.length === 1
      ? "1 pull request behind this one is waiting on the answer"
      : `${waiting.length} pull requests behind this one are waiting on the answer`;
  return `${said}: ${titles}.`;
}

/**
 * Which members somebody dropped in this window, and the act that drops one.
 *
 * **Held here because no operation serves it.** Fleet neither closes a pull
 * request nor rebases what was stacked on the branch, so a drop is what this
 * window draws and nothing more — which is why only a reading the mock handed
 * in offers the control at all. It goes the day `drop_member` exists.
 */
export function useDroppedMembers(): {
  over: (view: JobMembersView | undefined) => JobMembersView | undefined;
  drop: (jobId: string, reason: string) => void;
} {
  const [dropped, setDropped] = useState<Readonly<Record<string, string>>>({});
  return {
    over: (view) =>
      view === undefined
        ? undefined
        : {
            ...view,
            members: view.members.map((member) => {
              const reason = dropped[member.job];
              return reason === undefined ? member : { ...member, dropped: reason };
            }),
          },
    drop: (jobId, reason) => setDropped((held) => ({ ...held, [jobId]: reason })),
  };
}
