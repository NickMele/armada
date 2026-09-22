import { Select } from "../../primitives/Select/Select";

/**
 * Which model each tier runs on.
 *
 * **One control, drawn at two moments.** A person sets the map while they are
 * still typing the request and again on the proposal Armada came back with,
 * and the two are the same decision one moment apart — so they are the same
 * component rather than two that drift. `DispatchSettings` is the first,
 * `JobProposal` the second.
 */
export type TierChoice = {
  difficult: string | null;
  medium: string | null;
  easy: string | null;
};

/** What a tier reads while nobody has named a model for it. */
export const AUTO = "Auto";

/** The tiers in the order the planner hands work out, hardest first. */
export const TIERS: [keyof TierChoice, string][] = [
  ["difficult", "Difficult"],
  ["medium", "Medium"],
  ["easy", "Easy"],
];

export type TierModelsProps = {
  /** The map as it stands. Every tier `null` is nobody having chosen one. */
  tiers: TierChoice;
  onTiers: (tiers: TierChoice) => void;
  /** The models a tier may name. Empty until the connection answers. */
  models: readonly string[];
  /** What is said under the label. The two surfaces say it differently. */
  said: string;
  disabled?: boolean;
};

export function TierModels({ tiers, onTiers, models, said, disabled = false }: TierModelsProps) {
  return (
    <div className="armada-dispatch-settings__tiers">
      <span className="armada-dispatch-settings__label">Models by tier</span>
      <p className="armada-dispatch-settings__said">{said}</p>
      <div className="armada-dispatch-settings__tier-row">
        {TIERS.map(([tier, label]) => (
          <Select
            key={tier}
            label={label}
            value={tiers[tier] ?? ""}
            disabled={disabled}
            onChange={(event) =>
              onTiers({ ...tiers, [tier]: event.target.value === "" ? null : event.target.value })
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
  );
}
