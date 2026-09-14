import { ClipboardList, HardDrive, Settings as SettingsIcon } from "lucide-react";
import type { FleetLimits, Outcome } from "@armada/protocol";
import { DockQuestions, TheShell } from "@armada/components";
import { BridgeSettings } from "../../../BridgeSettings";
import type { HealthRead } from "../../../overview-reads";

const noop = () => {};

const LIMITS: FleetLimits = {
  concurrency: 2,
  memory_spare_percent: 15,
  disk_floor_gib: 10,
  checks_at_once: 4,
  shipped: { concurrency: 2, memory_spare_percent: 15, disk_floor_gib: 10, checks_at_once: 4 },
};

const HEALTH: HealthRead = {
  state: "read",
  health: {
    probes: [{ module: "Fleet", outcome: "pass", detail: "answering" }],
    not_probed: [],
    helm_action_authority: "acting",
  },
};

/**
 * Settings, the rail's own last surface — #1089. Fleet's four limits and
 * this machine's own settings, inside the shell with Helm's dock open, the
 * same arrangement `OverviewSurfaceFrom` stands Overview in.
 */
export function SettingsSurfaceFrom() {
  return (
    <div style={{ height: "100vh", display: "flex", flexDirection: "column" }}>
      <TheShell
        surfaces={[
          { id: "board", label: "Job Board", icon: ClipboardList, count: 6 },
          { id: "worktrees", label: "Cleanup", icon: HardDrive },
          { id: "settings", label: "Settings", icon: SettingsIcon },
        ]}
        activeId="settings"
        stats={{
          rows: [
            { id: "approval", label: "Awaiting approval", value: 1, tone: "warn" },
            { id: "jobs", label: "Jobs", value: 6 },
            { id: "drones", label: "Drones", value: "2 of 4" },
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
                  asked: "Two callers outside settings read the reducer directly. Move them in?",
                  waiting: "2m",
                  answers: [{ id: "move", label: "Move them in" }],
                  note: "Open job 31 to answer.",
                },
              ]}
            />
          ),
        }}
      >
        <div className="armada-screen__mounted">
          <BridgeSettings
            limits={LIMITS}
            live
            health={HEALTH}
            onSave={(): Promise<Outcome> => Promise.resolve({ ok: true })}
          />
        </div>
      </TheShell>
    </div>
  );
}
