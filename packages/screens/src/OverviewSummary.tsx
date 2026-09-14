// Overview's summary strip, wired from the same read `OverviewLists` draws its panels from — so
// the strip's counts and the panels beneath it can never disagree. #1091, Overview 27.
//
// Recently ended is not drawn until Overview 28 (#1092) gives a Job an end time; `SECTIONS` below
// names the three this build can read.

import { OverviewSummaryStrip } from "@armada/components";
import type { JobSummary, RepositorySummary } from "@armada/protocol";
import type { BoardSection } from "./board";
import { overviewListsOf } from "./overview-lists";

/** The strip names three of the four panels `OverviewLists` can draw — Other is not one of them. */
const SECTIONS: { id: Extract<BoardSection, "needs-you" | "running" | "queued">; label: string }[] = [
  { id: "needs-you", label: "Needs you" },
  { id: "running", label: "Running" },
  { id: "queued", label: "Queued" },
];

export type OverviewSummaryProps = {
  jobs: readonly JobSummary[];
  /** Every repository Fleet serves. */
  repositories: readonly RepositorySummary[];
  /** The rail's pick, by root. `null` is All repositories. */
  picked: string | null;
  /** A count pressed — Overview opens that panel and scrolls it into view. */
  onJump: (section: "needs-you" | "running" | "queued") => void;
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
        // Needs you is the only one of the three the strip tones — Recently
        // ended is the other, and it does not exist here yet.
        tone: id === "needs-you" ? "awaiting-review" : undefined,
        onPress: () => onJump(id),
      }))}
    />
  );
}
