// Overview's summary strip, wired from the same read `OverviewLists` draws its panels from — so
// the strip's counts and the panels beneath it can never disagree. #1091, Overview 27.
//
// Recently ended joined the strip in Overview 28 (#1092), once `JobSummary.ended_at` gave a Job
// an end time to carry: `SECTIONS` below names all four panels this build can read, of the five
// `OverviewLists` can draw — Other is the one left out.

import { OverviewSummaryStrip } from "@armada/components";
import type { JobSummary, RepositorySummary } from "@armada/protocol";
import type { BoardSection } from "./board";
import { overviewListsOf } from "./overview-lists";

type StripSection = Extract<BoardSection, "needs-you" | "running" | "queued" | "recently-ended">;

const SECTIONS: { id: StripSection; label: string }[] = [
  { id: "needs-you", label: "Needs you" },
  { id: "running", label: "Running" },
  { id: "queued", label: "Queued" },
  { id: "recently-ended", label: "Recently ended" },
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
      items={SECTIONS.map(({ id, label }) => ({
        id,
        label,
        count: countOf(id),
        // Needs you and Recently ended are the strip's two tones — the two
        // states `OverviewSummaryStripTone` carries, and the only two rows
        // here that mean something is owed rather than something in flight.
        tone: id === "needs-you" ? "awaiting-review" : id === "recently-ended" ? "completed-failed" : undefined,
        onPress: () => onJump(id),
      }))}
    />
  );
}
