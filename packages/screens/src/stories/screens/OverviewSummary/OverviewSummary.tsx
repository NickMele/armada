import type { JobSummary, RepositorySummary } from "@armada/protocol";
import { OverviewSummary } from "../../../OverviewSummary";
import { ARMADA, JOBS } from "../OverviewLists/OverviewLists";

const noop = () => {};

/**
 * Overview's summary strip, drawn by the app's own `OverviewSummary` from the same Jobs
 * `OverviewListsFrom` draws its panels from, so a story never shows the two disagreeing.
 */
export function OverviewSummaryFrom({
  jobs = JOBS,
  repositories = [ARMADA],
  picked = null,
  onJump = noop,
}: {
  jobs?: readonly JobSummary[];
  repositories?: readonly RepositorySummary[];
  /** The rail's pick, by root. `null` is All repositories. */
  picked?: string | null;
  onJump?: (section: "needs-you" | "running" | "queued") => void;
}) {
  return <OverviewSummary jobs={jobs} repositories={repositories} picked={picked} onJump={onJump} />;
}
