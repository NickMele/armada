import { File, GitBranch } from "lucide-react";
import type { ReactNode } from "react";
import { EvidenceTrail, type EvidenceTrailEntry } from "../../compositions/EvidenceTrail/EvidenceTrail";
import { JobBrief } from "../../compositions/JobBrief/JobBrief";
import { JobDetailHeaderActions } from "../../compositions/JobDetailHeaderActions/JobDetailHeaderActions";
import { JobLogReference } from "../../compositions/JobLogReference/JobLogReference";
import { Absent } from "../absent";
import type { JobDetailHeading, JobDetailLog } from "../detail";

/**
 * A finished job — a branch and an evidence trail. The screen hands over a
 * branch name and gets out of the way: no approve, no reject, no merge, no
 * in-app diff.
 *
 * The trail is the reason to open this screen, so it is the largest element
 * rather than a panel to expand. The header carries no action: the branch and
 * the log each carry their own, beside the panel that names them.
 */
export type AFinishedJobABranchAndAnEvidenceTrailProps = {
  heading: JobDetailHeading;
  /** The branch the work is on, what it came from, and what opens it. */
  handover?: {
    branch: string;
    /** `from main · 3 files +214 −96`. Machine-derived. */
    meta?: ReactNode;
    action?: ReactNode;
    log?: { path: string; meta?: ReactNode; action?: ReactNode };
    /** What the person still owes. Armada does not push and does not merge. */
    note?: ReactNode;
  };
  /** Why there is no branch to hand over, where there is none. */
  handoverAbsent?: string;
  /**
   * The brief, and the paths the work left behind. Separate from the handover
   * above it: the handover is what you take away, this is where to go looking.
   * The branch is not repeated here — the handover names it.
   */
  work?: JobDetailLog;
  /** Why there is nothing to name there, where there is nothing. */
  workAbsent?: string;
  /** One entry per submission, in order. */
  trail?: EvidenceTrailEntry[];
  /** How many submissions, said where the count is known. */
  trailMeta?: ReactNode;
  /** Why there is no trail, where there is none. */
  trailAbsent?: string;
  onCopied?: (value: string) => void;
};

/** The branch mark runs at Job level: 16px, like the header's own glyphs. */
const BRANCH_ICON = 16;
const BRANCH_STROKE = 2;
/** The log mark sits a level below it, like every mark below Job level. */
const ROW_ICON = 12;

export function AFinishedJobABranchAndAnEvidenceTrail({
  heading,
  handover,
  handoverAbsent = "Nothing serves a branch or a worktree yet.",
  work,
  workAbsent = "Nothing serves this Job's paths or its brief.",
  trail,
  trailMeta,
  trailAbsent = "Nothing serves a work submission yet.",
  onCopied,
}: AFinishedJobABranchAndAnEvidenceTrailProps) {
  return (
    <div className="armada-screen__detail">
      <JobDetailHeaderActions {...heading} onCopied={onCopied} />

      {handover === undefined ? (
        <div className="armada-screen__slot">
          <Absent name="Where the work is" note={handoverAbsent} />
        </div>
      ) : (
        <div className="armada-screen__sunken">
          <div className="armada-screen__branch-line">
            <span className="armada-screen__mark">
              <GitBranch size={BRANCH_ICON} strokeWidth={BRANCH_STROKE} aria-hidden />
            </span>
            <span className="armada-screen__branch">{handover.branch}</span>
            {handover.meta ? <span className="armada-screen__tag">{handover.meta}</span> : null}
            {handover.action ? (
              <div className="armada-screen__push-right">{handover.action}</div>
            ) : null}
          </div>
          {handover.log === undefined ? null : (
            <div className="armada-screen__log-line">
              {/* `file` is the registry's log mark — the file-* family's
                  unmarked member, reserved to this row. */}
              <span className="armada-screen__mark">
                <File size={ROW_ICON} strokeWidth={BRANCH_STROKE} aria-hidden />
              </span>
              <span className="armada-screen__log-path">{handover.log.path}</span>
              {handover.log.meta ? (
                <span className="armada-screen__tag">{handover.log.meta}</span>
              ) : null}
              {handover.log.action ? (
                <div className="armada-screen__push-right">{handover.log.action}</div>
              ) : null}
            </div>
          )}
          {handover.note ? <p className="armada-screen__handover">{handover.note}</p> : null}
        </div>
      )}

      <div className="armada-screen__col">
        <span className="armada-screen__eyebrow">Where the work is</span>
        {work === undefined ? (
          <div className="armada-screen__slot">
            <Absent name="Where the work is" note={workAbsent} />
          </div>
        ) : (
          <>
            {work.brief === undefined ? null : <JobBrief {...work.brief} />}
            <JobLogReference rows={work.rows} actions={work.actions} onCopied={onCopied}>
              {work.note}
            </JobLogReference>
          </>
        )}
      </div>

      <div className="armada-screen__col">
        <div className="armada-screen__head-row">
          <span className="armada-screen__eyebrow">Evidence</span>
          {trailMeta ? <span className="armada-screen__tag">{trailMeta}</span> : null}
        </div>
        {trail === undefined ? (
          <div className="armada-screen__slot">
            <Absent name="Evidence trail" note={trailAbsent} />
          </div>
        ) : (
          <EvidenceTrail entries={trail} />
        )}
      </div>
    </div>
  );
}
