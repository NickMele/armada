// Overview's lists: Needs you, Running, Queued and Other, over the Jobs in scope. #920.
//
// **Read from `sectionsOf`, never restated.** `overviewListsOf` in `overview-lists.ts` scopes the
// board and applies the Board's own fold and sort ahead of it.
//
// **Rows are the Board's own `Row`** — same field run, same act, and on All with more than one
// repository served the row names it, exactly as `Jobs.tsx` does.
//
// **Disconnected reads exactly as `BoardEmpty`'s own fault state.** Overview holds whatever Bridge
// last received, so a Needs you row from before an outage stays on screen; only a board with
// nothing on it at all draws the disconnected message.
//
// Not routed yet — #921 mounts this beneath the tile band — so a story draws it directly.

import { ActiveJobsList } from "@armada/components";
import type { JobSummary, RepositorySummary, WorkflowSummary } from "@armada/protocol";
import { BoardEmpty } from "./BoardEmpty";
import { repositoryOf } from "./board";
import { headlineOf } from "./lineage";
import { overviewListsOf } from "./overview-lists";
import { readingOf } from "./reading";
import { Row } from "./Row";

export type OverviewListsProps = {
  jobs: readonly JobSummary[];
  /** True while what is held is not live — every row reads de-emphasised, the Board's own rule. */
  stale: boolean;
  now: number;
  workflows: readonly WorkflowSummary[];
  /** Every repository Fleet serves. */
  repositories: readonly RepositorySummary[];
  /** The rail's pick, by root. `null` is All repositories. */
  picked: string | null;
  /** The connection's own statement, where Fleet cannot be reached — `BoardEmpty`'s fault state. */
  disconnected: string | null;
  /** The Job whose detail is open, where one is. */
  selected: string | null;
  /** Open a Job. Every row is a control, so every row calls this. */
  onOpen: (jobId: string) => void;
  /** Ask to kill the Job a row's own control names. It asks; it never kills — `Jobs.tsx`'s rule. */
  onKill: (jobId: string) => void;
  /** A clipboard write is silent, so the surface confirms every one with a toast. */
  onCopied: (value: string) => void;
};

export function OverviewLists({
  jobs,
  stale,
  now,
  workflows,
  repositories,
  picked,
  disconnected,
  selected,
  onOpen,
  onKill,
  onCopied,
}: OverviewListsProps) {
  const pickedRepository = repositories.find((one) => one.root === picked) ?? null;
  const all = picked === null;
  const { sections, dispatch, undrawable } = overviewListsOf(jobs, pickedRepository);

  const rowOf = (job: JobSummary) => (
    <Row
      key={job.id}
      job={job}
      headline={headlineOf(job, dispatch.get(job.id))}
      stale={stale}
      now={now}
      workflows={workflows}
      repository={repositoryOf(job, repositories, all)}
      selected={job.id === selected}
      focused={false}
      onOpen={onOpen}
      onKill={onKill}
      onCopied={onCopied}
    />
  );

  return (
    <div className="armada-screen__stack">
      <ActiveJobsList
        sections={sections.map((section) => ({
          id: section.id,
          label: section.label,
          count: section.jobs.length,
          rows: section.jobs.map(rowOf),
        }))}
        selectable
        label="Overview"
        empty={
          <BoardEmpty
            disconnected={disconnected}
            why={null}
            suspended={false}
            nothingServed={repositories.length === 0}
            onClear={() => {}}
          />
        }
      />

      {/* The registry has no glyph for this state, so the row shape cannot draw it — named
          rather than dropped, `Jobs.tsx`'s own choice for the same case. */}
      {undrawable.map((job) => {
        const reading = readingOf(job);
        if (reading.as === "badge") return null;
        return (
          <p key={job.id} className="text-fg-muted">
            {`${headlineOf(job, dispatch.get(job.id))} — `}
            <span className="mono">{reading.wire}</span>
            {`. The registry carries no ${reading.missing.join(" and no ")} for it, so the row shape cannot draw it.`}
          </p>
        );
      })}
    </div>
  );
}
