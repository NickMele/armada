import { Folder, GitBranch, GitCommitHorizontal } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import type { HeldReason, WorktreeHeld, WorktreeReclaimed } from "@armada/protocol";
import { JOB_STATUS } from "../../generated/vocabulary";
import { Badge } from "../../primitives/Badge/Badge";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";

/**
 * One worktree fleet is holding disk for, and the test it did not pass.
 *
 * **Why is the component, not a label on it.** Not-provably-safe is one word
 * for four situations a person answers differently, and each one wants
 * different facts in front of the decision: how many commits and what they are
 * reachable from; which files were written and committed nowhere; what the job
 * is still doing; which job is still waiting on this one. A row that said
 * "cannot be reclaimed automatically" and stopped would be asking somebody to
 * go and find all of that themselves.
 *
 * **No byte count, and that is the point of the row.** Bytes are not the
 * decision — which commits go, whether anything else has them, and which files
 * exist nowhere but this directory is. A size beside those facts would be the
 * figure read first and meaning least.
 *
 * **Nothing that could be lost is left unnamed.** There is no force on this
 * seam, so a branch holding commits the base cannot reach survives the reclaim
 * and the row says so; uncommitted files do not survive it, and the row names
 * them one by one.
 *
 * **A piloted job's checkout never reaches this component**, because fleet does
 * not serve one. There is no arm for it below and there cannot be: a person is
 * at an unrestricted toolset in that directory.
 */
export type HeldWorktreeProps = {
  /** One row of `GET /worktrees`, exactly as fleet answered it. */
  held: WorktreeHeld;
  /**
   * Chosen to be reclaimed.
   *
   * **Absent draws the row as a record rather than a choice**, which is what a
   * worktree nothing may act on is: a job still running, or one fleet is about
   * to take back on its own.
   */
  selected?: boolean;
  onSelect?: (jobId: string, selected: boolean) => void;
  /**
   * What the reclaim did, where this one has already been given back.
   *
   * **Both halves, because half of it happening is the ordinary outcome.** A
   * kept branch is the safe setting working rather than a failure, and a
   * checkout that would not go is the row staying exactly where it is.
   */
  reclaimed?: WorktreeReclaimed;
  /**
   * How long the checkout has been sitting, as a phrase — `4 days`.
   *
   * **Formatted by the caller, like every other elapsed figure in this
   * package.** A component that held a clock would redraw on somebody else's
   * tick, and the screens own the one `now` every figure on a screen is drawn
   * from.
   *
   * Drawn under `uncommitted` and nowhere else: that is the one reason where
   * the act ends something, and *twenty minutes* and *four days* are answered
   * differently there. Absent where the stamp would not parse, which draws the
   * reason without the age rather than an age measured from zero.
   */
  sitting?: string;
  /** A clipboard write is silent, so the surface confirms it. */
  onCopied?: (value: string) => void;
};

export function HeldWorktree({
  held,
  selected,
  onSelect,
  reclaimed,
  sitting,
  onCopied,
}: HeldWorktreeProps) {
  const badge = badgeOf(held.status);
  const choosable = selected !== undefined && onSelect !== undefined;

  return (
    <li className="armada-held" data-selected={selected || undefined}>
      <div className="armada-held__head">
        {choosable ? (
          <Checkbox
            checked={selected}
            onChange={(event) => onSelect(held.job_id, event.currentTarget.checked)}
          >
            {held.job_title}
          </Checkbox>
        ) : (
          <span className="armada-held__title">{held.job_title}</span>
        )}
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

      {reclaimed === undefined ? null : <Receipt reclaimed={reclaimed} />}
    </li>
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
          Reclaiming takes the checkout only — there is no force on this seam, so nothing
          here can delete work nobody has taken.
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
function Receipt({ reclaimed }: { reclaimed: WorktreeReclaimed }) {
  return (
    <dl className="armada-held__receipt">
      <dt>The checkout</dt>
      <dd>
        {reclaimed.worktree.removed
          ? "Gone from disk."
          : `Still there — ${reclaimed.worktree.why ?? "no reason was given"}.`}
      </dd>
      <dt>The branch</dt>
      <dd>
        {reclaimed.branch.deleted
          ? `Deleted${reclaimed.branch.tip === undefined ? "" : `, at ${reclaimed.branch.tip}`}.`
          : reclaimed.branch.unmerged_commits === undefined
            ? `Left standing — ${reclaimed.branch.why ?? "no reason was given"}.`
            : `Kept, with ${counted(reclaimed.branch.unmerged_commits)} still on it. Merge it or delete it by hand once you have taken what you want.`}
      </dd>
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
