// Overview's summary tiles, wired from the same read `OverviewLists` draws its panels from — so
// the tiles' counts and the panels beneath them can never disagree. #1091, Overview 27.
//
// Recently ended joined them in Overview 28 (#1092), once `JobSummary.ended_at` gave a Job
// an end time to carry: `SECTIONS` below names all four panels this build can read, of the five
// `OverviewLists` can draw — Other is the one left out.

import { OverviewSummaryStrip, type OverviewSummaryStripTone } from "@armada/components";
import type { JobSummary, RepositorySummary } from "@armada/protocol";
import type { BoardSection } from "./board";
import { overviewListsOf } from "./overview-lists";

type StripSection = Extract<BoardSection, "needs-you" | "running" | "queued" | "recently-ended">;

// `hue` is the status stem a tile wears at every count — design-system.md → Overview summary
// tiles. `tone` colours a count past zero only, on the two rows that mean something is owed.
const SECTIONS: { id: StripSection; label: string; hue: string; tone?: OverviewSummaryStripTone }[] = [
  { id: "needs-you", label: "Needs you", hue: "awaiting-review", tone: "awaiting-review" },
  { id: "running", label: "Running", hue: "running" },
  { id: "queued", label: "Queued", hue: "not-started" },
  { id: "recently-ended", label: "Recently ended", hue: "completed-success", tone: "completed-failed" },
];

export type OverviewSummaryProps = {
  jobs: readonly JobSummary[];
  /** Every repository Fleet serves. */
  repositories: readonly RepositorySummary[];
  /** The rail's pick, by root. `null` is All repositories. */
  picked: string | null;
  /** A count pressed — Overview opens that panel and scrolls it into view. */
  onJump: (section: StripSection) => void;
};

export function OverviewSummary({ jobs, repositories, picked, onJump }: OverviewSummaryProps) {
  const pickedRepository = repositories.find((one) => one.root === picked) ?? null;
  const { sections } = overviewListsOf(jobs, pickedRepository);
  const countOf = (id: BoardSection) => sections.find((section) => section.id === id)?.jobs.length ?? 0;

  return (
    <OverviewSummaryStrip
      items={SECTIONS.map(({ id, label, hue, tone }) => ({
        id,
        label,
        count: countOf(id),
        hue,
        tone,
        onPress: () => onJump(id),
      }))}
    />
  );
}
