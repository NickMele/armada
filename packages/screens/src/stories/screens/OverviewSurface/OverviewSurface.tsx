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
 * roster is a stand-in — #921 is what puts Overview in the rail — and the dock's questions are
 * `DockQuestions`'s own fixture, not this surface's arithmetic. No page head, since #1090.
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
        stats={{
          rows: [
            { id: "approval", label: "Awaiting approval", value: 1, tone: "warn" },
            { id: "review", label: "Needs review", value: 0 },
            { id: "escalated", label: "Escalated", value: 0 },
            { id: "jobs", label: "Jobs", value: 6 },
            { id: "drones", label: "Drones", value: "2 of 4" },
            { id: "manifest", label: "Manifest", value: "Current" },
          ],
          open: true,
          onOpenChange: noop,
        }}
        fleet={{ state: "running", label: "Running", detail: "pid 4417 · port 7411", open: true, onOpenChange: noop }}
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
