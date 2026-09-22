import { ChevronRight, ChevronUp } from "lucide-react";
import type { WorkflowSummary } from "@armada/protocol";

import { Input } from "../../primitives/Input/Input";
import { Select } from "../../primitives/Select/Select";

/**
 * What a person may set while they are still typing the request, and does not
 * have to.
 *
 * **Every control here is an override of a decision somebody else makes.** The
 * proposer picks the workflow, the planner's tiers resolve to the configured
 * model, the machine's own cap holds, and the workflow's delivering step
 * decides who is asked before it lands. So each field opens on an answer that
 * names who decides rather than on a value nobody chose, and the head says how
 * many a person has moved — a closed block that cannot say whether anything is
 * inside it is one people open to check.
 */
export type DispatchSettingsProps = {
  /** Whether the block is open. **Controlled** — nothing here opens itself. */
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** What is set. Every field absent is the ordinary case. */
  settings: DispatchSettingsValue;
  onSettings: (settings: DispatchSettingsValue) => void;
  /** The workflows Fleet holds. **The only ones it will accept.** */
  workflows: readonly WorkflowSummary[];
  /** The models a tier may name. Empty until the connection answers. */
  models: readonly string[];
  /**
   * How many Drones this machine runs across every Job — `LimitValues.concurrency`.
   * `null` before Fleet has said, which is not the same as none.
   */
  machineCap: number | null;
  /** Nothing may be set while the connection is not live. */
  disabled?: boolean;
};

/** Which model each tier runs on. `null` is Auto — the harness chooses. */
export type TierChoice = {
  difficult: string | null;
  medium: string | null;
  easy: string | null;
};

/** The four, each absent until somebody sets it. */
export type DispatchSettingsValue = {
  workflowId?: string;
  tiers?: TierChoice;
  /** How many Drones this Job may run at once, inside the machine's own cap. */
  droneCap?: number;
  lands?: "auto" | "you_at_review";
};

/** The tiers in the order the planner hands work out, hardest first. */
const TIERS: [keyof TierChoice, string][] = [
  ["difficult", "Difficult"],
  ["medium", "Medium"],
  ["easy", "Easy"],
];

/** What a tier reads while nobody has named a model for it. */
const AUTO = "Auto";

export function DispatchSettings({
  open,
  onOpenChange,
  settings,
  onSettings,
  workflows,
  models,
  machineCap,
  disabled = false,
}: DispatchSettingsProps) {
  const set = [settings.workflowId, settings.tiers, settings.droneCap, settings.lands].filter(
    (one) => one !== undefined,
  ).length;
  const tiers = settings.tiers ?? { difficult: null, medium: null, easy: null };

  /** One field moved, with the rest of them carried through untouched. */
  function moved(change: DispatchSettingsValue): void {
    onSettings({ ...settings, ...change });
  }

  return (
    <section className="armada-dispatch-settings">
      <button
        type="button"
        className="armada-dispatch-settings__head"
        aria-expanded={open}
        onClick={() => onOpenChange(!open)}
      >
        {open ? (
          <ChevronUp className="armada-dispatch-settings__mark" size={16} strokeWidth={2} aria-hidden />
        ) : (
          <ChevronRight className="armada-dispatch-settings__mark" size={16} strokeWidth={2} aria-hidden />
        )}
        <span className="armada-dispatch-settings__name">Settings</span>
        <span className="armada-dispatch-settings__count">
          {set === 0 ? "nothing set" : `${set} set`}
        </span>
      </button>

      {!open ? null : (
        <div className="armada-dispatch-settings__body">
          <p className="armada-dispatch-settings__lede">
            Set any of these, or none. What you leave alone is decided when the Job is classified.
          </p>

          <Select
            label="Workflow"
            value={settings.workflowId ?? ""}
            disabled={disabled}
            onChange={(event) =>
              moved(event.target.value === "" ? { workflowId: undefined } : { workflowId: event.target.value })
            }
          >
            <option value="">Armada picks it from the request</option>
            {workflows.map((workflow) => (
              <option key={workflow.id} value={workflow.id}>
                {`${workflow.name} — ${workflow.steps.length} steps`}
              </option>
            ))}
          </Select>

          <div className="armada-dispatch-settings__tiers">
            <span className="armada-dispatch-settings__label">Models by tier</span>
            <p className="armada-dispatch-settings__said">
              A planning Drone gives each task a tier, and the tier picks the model it runs on.
            </p>
            <div className="armada-dispatch-settings__tier-row">
              {TIERS.map(([tier, label]) => (
                <Select
                  key={tier}
                  label={label}
                  value={tiers[tier] ?? ""}
                  disabled={disabled}
                  onChange={(event) =>
                    moved({
                      tiers: { ...tiers, [tier]: event.target.value === "" ? null : event.target.value },
                    })
                  }
                >
                  <option value="">{AUTO}</option>
                  {models.map((model) => (
                    <option key={model} value={model}>
                      {model}
                    </option>
                  ))}
                </Select>
              ))}
            </div>
          </div>

          <div className="armada-dispatch-settings__cap">
            <Input
              label="Drones at once"
              type="number"
              min={1}
              value={settings.droneCap === undefined ? "" : String(settings.droneCap)}
              placeholder={AUTO}
              disabled={disabled}
              onChange={(event) =>
                moved(
                  event.target.value === ""
                    ? { droneCap: undefined }
                    : { droneCap: Number(event.target.value) },
                )
              }
            />
            {/* The machine's own cap, beside the Job's and never merged into
                it: one is how many Drones this Job may run, the other is how
                many run here at all. */}
            <p className="armada-dispatch-settings__said">
              {machineCap === null
                ? "Fleet has not said how many this machine runs at once."
                : `This machine runs ${machineCap} at once, across every Job.`}
            </p>
          </div>

          <Select
            label="How it lands"
            value={settings.lands ?? ""}
            disabled={disabled}
            onChange={(event) =>
              moved(
                event.target.value === ""
                  ? { lands: undefined }
                  : { lands: event.target.value as "auto" | "you_at_review" },
              )
            }
          >
            <option value="">The workflow decides</option>
            <option value="auto">Auto</option>
            <option value="you_at_review">Ask me at review only</option>
          </Select>
        </div>
      )}
    </section>
  );
}
