import { ChevronRight, ChevronUp } from "lucide-react";
import type { LeftOutWorkflow, WorkflowSummary } from "@armada/protocol";

import { Select } from "../../primitives/Select/Select";
import { DroneCap } from "./DroneCap";
import { sourceOf } from "./source";
import { TierModels } from "./TierModels";
import type { TierChoice } from "./TierModels";

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
  /**
   * The Kit and carried definitions Fleet runs without, each with why — #425.
   * Named here because this is where a workflow is picked.
   */
  leftOut?: readonly LeftOutWorkflow[];
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

/** The four, each absent until somebody sets it. */
export type DispatchSettingsValue = {
  workflowId?: string;
  tiers?: TierChoice;
  /** How many Drones this Job may run at once, inside the machine's own cap. */
  droneCap?: number;
  lands?: "auto" | "you_at_review";
};

export function DispatchSettings({
  open,
  onOpenChange,
  settings,
  onSettings,
  workflows,
  leftOut = [],
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

          <div className="armada-dispatch-settings__workflow">
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
                  {`${workflow.name} — ${workflow.steps.length} steps${sourceOf(workflow.source)}`}
                </option>
              ))}
            </Select>
            {/* A definition Fleet left out is named where a workflow is picked,
                so a person who customised `bug` in Kit is not running Armada's
                without knowing — #425. This is that place now: the form that
                used to carry it is gone. */}
            {leftOut.length === 0 ? null : (
              <ul className="armada-dispatch-settings__left-out" aria-label="Workflows left out">
                {leftOut.map((left) => (
                  <li key={left.file}>{left.said}</li>
                ))}
              </ul>
            )}
          </div>

          <TierModels
            tiers={tiers}
            onTiers={(chosen) => moved({ tiers: chosen })}
            models={models}
            said="A planning Drone gives each task a tier, and the tier picks the model it runs on."
            disabled={disabled}
          />

          <DroneCap
            {...(settings.droneCap === undefined ? {} : { cap: settings.droneCap })}
            onCap={(cap) => moved({ droneCap: cap })}
            machineCap={machineCap}
            disabled={disabled}
          />

          {/* The three read as three answers to one question — does anybody
              get asked — rather than as a decider beside a mode. `The workflow
              decides` against `Auto` was two different kinds of answer in one
              list, and the owner asked what the difference was: the first
              names who, the second names what, and nothing said that leaving
              it alone may land without asking anyway. */}
          <div className="armada-dispatch-settings__lands">
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
              <option value="">Leave it to the workflow</option>
              <option value="auto">Land it without asking me</option>
              <option value="you_at_review">Stop and ask me at review</option>
            </Select>
            <p className="armada-dispatch-settings__said">
              Left alone, the workflow&rsquo;s delivering step decides, and some workflows ask
              you and some do not.
            </p>
          </div>
        </div>
      )}
    </section>
  );
}
