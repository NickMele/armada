// The Land board: what a finished Job shows, drawn from `landed.ts`. #1542.
//
// Two regions and no third: the outcome, and the groups that produced it. The
// run, the plan and the record are the destinations under it — a board that
// redrew them would be a second copy of each, one tab away from the first.

import { Button, JobOutcome, ProducedGroups, ProducedPanel } from "@armada/components";
import type { JobOutcomePart } from "@armada/components";
import { File, Folder, GitBranch, GitCommitHorizontal, GitPullRequest } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import type { LandedPart, LandedRead } from "./landed";

/** The glyph each part takes, by the registry's own name for it. */
const MARKS: Record<NonNullable<LandedPart["mark"]>, LucideIcon> = {
  branch: GitBranch,
  commit: GitCommitHorizontal,
  "pull-request": GitPullRequest,
  // `folder` means workspace in the registry and a worktree has no row of its
  // own; `work.tsx` draws the same row the same way rather than inventing one.
  worktree: Folder,
  log: File,
};

export type LandBoardProps = {
  read: LandedRead;
  /** Opens the pull request, in whatever the machine opens addresses with. */
  onOpenPullRequest: () => void;
  /** Opens the composer, which is where a follow-up is dispatched from. */
  onCompose: () => void;
  onCopied: (value: string) => void;
};

export function LandBoard({ read, onOpenPullRequest, onCompose, onCopied }: LandBoardProps) {
  return (
    <div className="armada-detail-tab__region">
      <JobOutcome
        headline={{
          verb: read.verb,
          count: read.count,
          says: read.says,
          criteria: read.criteria,
          completes: read.completes,
        }}
        sections={read.sections.map((section) => ({
          name: section.name,
          ...(section.meta === undefined ? {} : { meta: section.meta }),
          parts: section.parts.map((part) => partOf(part, onOpenPullRequest)),
          ...(section.note === undefined ? {} : { note: section.note }),
        }))}
        steps={read.steps}
        cost={read.cost}
        runs={read.runs}
        act={
          <Button variant="secondary" ground="sunken" onClick={onCompose}>
            {read.followUp}
          </Button>
        }
        onCopied={onCopied}
      />
      <ProducedPanel summary={read.groupsSummary}>
        <ProducedGroups groups={read.groups} emptyNote={read.groupsAbsent} note={read.groupsNote} />
      </ProducedPanel>
    </div>
  );
}

/** One part, with its glyph and — on the pull request alone — its way out. */
function partOf(part: LandedPart, onOpenPullRequest: () => void): JobOutcomePart {
  const icon = part.mark === undefined ? undefined : MARKS[part.mark];
  return {
    name: part.name,
    ...(icon === undefined ? {} : { icon, iconLabel: part.name }),
    ...(part.value === undefined ? {} : { value: part.value }),
    ...(part.meta === undefined ? {} : { meta: part.meta }),
    ...(part.absent === undefined ? {} : { absent: part.absent }),
    ...(part.opens === undefined || part.value === undefined
      ? {}
      : {
          action: (
            <Button variant="secondary" size="sm" ground="sunken" onClick={onOpenPullRequest}>
              Open
            </Button>
          ),
        }),
  };
}
