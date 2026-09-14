import { ClipboardList, HardDrive } from "lucide-react";
import type { JobSummary } from "@armada/protocol";
import { DockQuestions, TheShell } from "@armada/components";
import { OverviewListsFrom } from "../OverviewLists/OverviewLists";
import { OverviewTilesFrom } from "../OverviewTiles/OverviewTiles";

const noop = () => {};

/**
 * Overview's whole surface, drawn from its two pieces: the tile band `OverviewTiles` draws and the
 * panels `OverviewLists` draws beneath it, inside the shell with Helm's dock open. #920, #921.
 *
 * **Not routed yet, so this composes what the app renders rather than what mounts it.** The rail
 * roster and the head are stand-ins — #921 is what puts Overview in the rail — and the dock's
 * questions are `DockQuestions`'s own fixture, not this surface's arithmetic.
 *
 * `jobs` overrides `OverviewListsFrom`'s own fixture, where a story needs a specific roster in the
 * shell's own width — the rail and Helm's dock beside it, which the lists' own story stands alone
 * without.
 */
export function OverviewSurfaceFrom({ jobs }: { jobs?: readonly JobSummary[] } = {}) {
  return (
    <div style={{ height: "100vh", display: "flex", flexDirection: "column" }}>
      <TheShell
        surfaces={[
          { id: "board", label: "Job Board", icon: ClipboardList, count: 6 },
          { id: "worktrees", label: "Cleanup", icon: HardDrive },
        ]}
        title="Overview"
        summary="Everything in flight for armada."
        status={{
          fleet: "running",
          fleetLabel: "Fleet running",
          detail: "pid 4417 · port 7411",
          items: ["6 jobs"],
          approvals: 1,
        }}
        dock={{
          open: true,
          binding: "⌘J",
          questions: 1,
          onOpen: noop,
          children: (
            <DockQuestions
              questions={[
                {
                  id: "b:judge",
                  repository: "armada",
                  job: "31",
                  title: "Split the settings reducer",
                  label: "Helm is asking",
                  asked:
                    "Two callers outside settings read the reducer directly. Move them in, or export the shape and leave them?",
                  waiting: "2m",
                  answers: [
                    { id: "move", label: "Move them in" },
                    { id: "export", label: "Export the shape" },
                  ],
                  note: "Open job 31 to answer.",
                },
              ]}
            />
          ),
        }}
      >
        <div className="armada-screen__mounted">
          <div className="armada-screen__overview">
            <OverviewTilesFrom />
            {jobs === undefined ? <OverviewListsFrom /> : <OverviewListsFrom jobs={jobs} />}
          </div>
        </div>
      </TheShell>
    </div>
  );
}
