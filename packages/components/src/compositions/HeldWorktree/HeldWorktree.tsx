import { Folder, GitBranch, GitCommitHorizontal } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import type { BranchDeleted, HeldReason, WorktreeHeld, WorktreeReclaimed } from "@armada/protocol";
import { JOB_STATUS } from "../../generated/vocabulary";
import { Badge } from "../../primitives/Badge/Badge";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";

/** The three acts a row offers, each its own checkbox and its own boolean. */
export type RowChoice = {
  removeCheckout: boolean;
  deleteBranch: boolean;
  forget: boolean;
};

/**
 * One worktree fleet is holding disk for, and the test it did not pass.
 *
 * **No byte count.** Which commits go and which files exist nowhere but this
 * directory is the decision; bytes are not.
 *
 * **Reclaiming the checkout is still never a force.** Deleting the branch is,
 * and it is its own checkbox with its own line in the confirmation.
 *
 * **A piloted job's checkout never reaches this component** — fleet does not
 * serve one.
 */
export type HeldWorktreeProps = {
  /** One row of `GET /worktrees`, exactly as fleet answered it. */
  held: WorktreeHeld;
  /**
   * What is chosen for this row's three acts.
   *
   * Absent draws the row as a record rather than a choice — a job still
   * running, or one fleet is about to take back on its own.
   */
  choice?: RowChoice;
  /**
   * Which of the three this row offers right now.
   *
   * `held.ts`'s `offeredOn`, computed once and handed down — forget is gated
   * on the other two, so this component only draws what it is told.
   */
  offered?: RowChoice;
  onChoose?: (jobId: string, choice: RowChoice) => void;
  /** What reclaiming did, where the checkout has already been given back. */
  reclaimed?: WorktreeReclaimed;
  /**
   * What deleting the branch did, where that act has already been sent.
   *
   * Its own receipt for its own act — `reclaimed.branch` says what the safe
   * reclaim did to the branch, ordinarily nothing; this says what the
   * explicit, forcing delete did. The two never both apply.
   */
  branchDeleted?: BranchDeleted;
  /**
   * How long the checkout has been sitting, as a phrase — `4 days`.
   *
   * Formatted by the caller, like every other elapsed figure in this package.
   * Drawn under `uncommitted` and nowhere else, the one reason that ends
   * something.
   */
  sitting?: string;
  /** A clipboard write is silent, so the surface confirms it. */
  onCopied?: (value: string) => void;
};

export function HeldWorktree({
  held,
  choice,
  offered,
  onChoose,
  reclaimed,
  branchDeleted,
  sitting,
  onCopied,
}: HeldWorktreeProps) {
  const badge = badgeOf(held.status);
  const choosable = choice !== undefined && offered !== undefined && onChoose !== undefined;
  const chosen = choosable && (choice.removeCheckout || choice.deleteBranch || choice.forget);

  return (
    <li className="armada-held" data-selected={chosen || undefined}>
      <div className="armada-held__head">
        <span className="armada-held__title">{held.job_title}</span>
        {badge === null ? (
          /* A status spelling this build's registry has no row for, which is
             Bridge behind Fleet rather than a bad message. **The wire's own
             spelling renders instead**, which is `reading.ts`'s answer to the
             same case: the value is the registry key, so showing it is not a
             second vocabulary, and a blank where a status goes leaves a person
             unable to tell one job from another.

             **`escalated` stood here and no longer does.** It carried no verb
             and no glyph, so this arm was drawing a status the registry does
             name — #400 gave it `needs you` and `megaphone`, and every status
             `enum-verbs.toml` carries now draws a badge. What is left is the
             unknown value, which no registry row can close. */
          <span className="armada-held__unworded">{held.status}</span>
        ) : (
          <Badge status={badge.status} icon={badge.icon}>
            {badge.verb}
          </Badge>
        )}
      </div>

      <div className="armada-held__where">
        <Value glyph={Folder} title={held.path} onCopied={onCopied}>
          {held.path}
        </Value>
        <Value glyph={GitBranch} title={held.branch} onCopied={onCopied}>
          {held.branch}
        </Value>
      </div>

      {held.held.length === 0 ? (
        <Automatic />
      ) : (
        <ul className="armada-held__reasons">
          {held.held.map((reason) => (
            <Reason key={reason.why} reason={reason} sitting={sitting} />
          ))}
        </ul>
      )}

      {!choosable ? null : (
        <Acts
          jobId={held.job_id}
          title={held.job_title}
          choice={choice}
          offered={offered}
          onChoose={onChoose}
        />
      )}

      {reclaimed === undefined && branchDeleted === undefined ? null : (
        <Receipt reclaimed={reclaimed} branchDeleted={branchDeleted} />
      )}
    </li>
  );
}

/** The three checkboxes this row currently offers, each independent of the other two. */
function Acts({
  jobId,
  title,
  choice,
  offered,
  onChoose,
}: {
  jobId: string;
  title: string;
  choice: RowChoice;
  offered: RowChoice;
  onChoose: (jobId: string, choice: RowChoice) => void;
}) {
  return (
    <ul className="armada-held__acts">
      {!offered.removeCheckout ? null : (
        <li>
          <Checkbox
            checked={choice.removeCheckout}
            aria-label={`Remove the checkout — ${title}`}
            onChange={(event) =>
              onChoose(jobId, { ...choice, removeCheckout: event.currentTarget.checked })
            }
          >
            Remove the checkout
          </Checkbox>
        </li>
      )}
      {!offered.deleteBranch ? null : (
        <li>
          <Checkbox
            checked={choice.deleteBranch}
            aria-label={`Delete the branch — ${title}`}
            onChange={(event) =>
              onChoose(jobId, { ...choice, deleteBranch: event.currentTarget.checked })
            }
          >
            Delete the branch
          </Checkbox>
        </li>
      )}
      {!offered.forget ? null : (
        <li>
          <Checkbox
            checked={choice.forget}
            aria-label={`Forget the job — ${title}`}
            onChange={(event) => onChoose(jobId, { ...choice, forget: event.currentTarget.checked })}
          >
            Forget the job
          </Checkbox>
        </li>
      )}
    </ul>
  );
}

/**
 * A status as a badge draws it, from the generated vocabulary rather than typed
 * here — a second copy of a status word is a second vocabulary. `null` where
 * the registry carries no verb, glyph or token, which draws no badge rather
 * than an invented one.
 */
function badgeOf(status: string): { status: string; icon: LucideIcon; verb: string } | null {
  const rendering = JOB_STATUS[status];
  if (rendering === undefined) return null;
  const { badgeStatus, icon, verb } = rendering;
  if (badgeStatus === null || icon === null || verb === null) return null;
  return { status: badgeStatus, icon, verb };
}

/**
 * A machine-derived value with the glyph the registry gives it, copying on
 * click. A path and a branch are things that get pasted into a shell, and the
 * affordance token is the affordance — no `copy` glyph beside a value that
 * copies.
 */
function Value({
  glyph: Glyph,
  title,
  onCopied,
  children,
}: {
  glyph: LucideIcon;
  title: string;
  onCopied?: (value: string) => void;
  children: ReactNode;
}) {
  if (onCopied === undefined) {
    return (
      <span className="armada-held__value" title={title}>
        <Glyph size={12} strokeWidth={2} aria-hidden="true" />
        <span className="armada-held__mono">{children}</span>
      </span>
    );
  }
  return (
    <button
      type="button"
      className="armada-held__value armada-held__value--copies"
      title={title}
      onClick={() => {
        void navigator.clipboard.writeText(title).then(
          // A failed clipboard write is otherwise indistinguishable from a dead
          // control, so the surface is told either way.
          () => onCopied(title),
          () => onCopied(title),
        );
      }}
    >
      <Glyph size={12} strokeWidth={2} aria-hidden="true" />
      <span className="armada-held__mono">{children}</span>
    </button>
  );
}

/**
 * Nothing is holding this one.
 *
 * **Said rather than left blank.** A row with no reason under it would read as
 * a row whose reasons failed to load. What is true is that fleet will give this
 * disk back on its own, which is the other half of the rule and the reason the
 * row carries no control.
 */
function Automatic() {
  return (
    <p className="armada-held__automatic">
      Every safety test passed. Fleet gives this one back on its own sweep, without being
      asked — there is nothing here to decide.
    </p>
  );
}

/** One test, and the facts that particular decision is made on. */
function Reason({ reason, sitting }: { reason: HeldReason; sitting?: string }) {
  switch (reason.why) {
    case "not_terminal":
      return (
        <Held
          title={`The job has not ended — ${JOB_STATUS[reason.status]?.verb ?? reason.status}.`}
        >
          A drone may still be writing here, so fleet refuses to take it. Nothing to decide
          yet: this row is here so an absence is not mistaken for a worktree already gone.
        </Held>
      );
    case "unmerged":
      return (
        <Held title={`${counted(reason.commits)} that ${reason.base} cannot reach.`}>
          <span className="armada-held__safe">
            The branch is kept and the commits stay on it.
          </span>{" "}
          Reclaiming takes the checkout only. Deleting the branch is a separate, explicit
          choice below — nothing here does both at once.
          <span className="armada-held__tip">
            <GitCommitHorizontal size={12} strokeWidth={2} aria-hidden="true" />
            <span className="armada-held__mono">{reason.tip}</span>
          </span>
        </Held>
      );
    case "base_unanswered":
      return (
        <Held title="Nothing could say what this branch would merge into.">
          So nothing can say whether it holds a copy of anything, and it is kept for the
          same reason an unmerged branch is: the cost of guessing wrong is a lost commit.
          <span className="armada-held__detail">{reason.detail}</span>
        </Held>
      );
    case "uncommitted":
      return (
        <Held title={`${filed(reason.files.length)} written and committed nowhere.`}>
          <span className="armada-held__lost">
            Reclaiming destroys these. No branch carries them, so the checkout is the only
            copy.
          </span>{" "}
          {/* How long it has sat, which is half of what makes this decidable —
              work abandoned twenty minutes ago and work abandoned four days ago
              are answered differently. It is the last time armada moved the
              job, said in those words: the dirty reading answers names and not
              times, so nothing here knows when a file was written. */}
          {sitting === undefined
            ? null
            : `Armada last moved this job ${sitting} ago, so they have sat at least that long.`}
          <ul className="armada-held__files">
            {reason.files.map((file) => (
              <li key={file} className="armada-held__mono">
                {file}
              </li>
            ))}
          </ul>
        </Held>
      );
    case "locked":
      return (
        <Held title="Somebody locked this checkout.">
          A lock is a person saying not yet, and the reclaim leaves a locked checkout alone
          and says so.
          <span className="armada-held__detail">{reason.reason}</span>
        </Held>
      );
    case "depended_on":
      return (
        <Held title={`${waiting(reason.by.length)} on this job and has not finished.`}>
          What this one wrote may still be needed, and it is on disk rather than in the
          record.
          <ul className="armada-held__files">
            {reason.by.map((jobId) => (
              <li key={jobId} className="armada-held__mono">
                {jobId}
              </li>
            ))}
          </ul>
        </Held>
      );
    case "unreadable":
      return (
        <Held title="Version control would not say what is in this checkout.">
          Unanswered and clean must never read alike, because only one of them can be taken
          back — so it is held until somebody looks.
          <span className="armada-held__detail">{reason.detail}</span>
        </Held>
      );
  }
}

/** One reason: what the test found, and what it means for the decision. */
function Held({ title, children }: { title: string; children: ReactNode }) {
  return (
    <li className="armada-held__reason">
      <p className="armada-held__what">{title}</p>
      <p className="armada-held__means">{children}</p>
    </li>
  );
}

/**
 * What the reclaim did, half by half.
 *
 * **Two halves and never one flag.** A removed checkout beside a surviving
 * branch is the ordinary outcome here, not a partial failure, and a single line
 * would have to lie about one of them.
 */
function Receipt({
  reclaimed,
  branchDeleted,
}: {
  reclaimed?: WorktreeReclaimed;
  branchDeleted?: BranchDeleted;
}) {
  return (
    <dl className="armada-held__receipt">
      {reclaimed === undefined ? null : (
        <>
          <dt>The checkout</dt>
          <dd>
            {reclaimed.worktree.removed
              ? "Gone from disk."
              : `Still there — ${reclaimed.worktree.why ?? "no reason was given"}.`}
          </dd>
        </>
      )}
      {branchDeleted === undefined && reclaimed === undefined ? null : (
        <>
          <dt>The branch</dt>
          <dd>
            {branchDeleted !== undefined
              ? `Deleted, at ${branchDeleted.tip}.`
              : reclaimed === undefined
                ? null
                : reclaimed.branch.deleted
                  ? `Deleted${reclaimed.branch.tip == null ? "" : `, at ${reclaimed.branch.tip}`}.`
                  : reclaimed.branch.unmerged_commits == null
                    ? `Left standing — ${reclaimed.branch.why ?? "no reason was given"}.`
                    : `Kept, with ${counted(reclaimed.branch.unmerged_commits)} still on it. Merge it or delete it by hand once you have taken what you want.`}
          </dd>
        </>
      )}
    </dl>
  );
}

/** `1 commit`, `4 commits`. A count with its noun, so no row reads `1 commits`. */
function counted(commits: number): string {
  return commits === 1 ? "1 commit" : `${commits} commits`;
}

function filed(files: number): string {
  return files === 1 ? "1 file" : `${files} files`;
}

function waiting(jobs: number): string {
  return jobs === 1 ? "One job depends" : `${jobs} jobs depend`;
}
