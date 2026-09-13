import type { ReactNode } from "react";

import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Popover } from "../../primitives/Popover/Popover";
import { Switch } from "../../primitives/Switch/Switch";

/**
 * A value a cell cannot hold, opened from the value where it sits — Journey 3's *Three sets, one
 * popover*. Prerequisites are an ordered set; the destructive flag is the set of one, a single
 * switch. **Data in, callbacks out**, and nothing is saved here: each change is the caller's.
 *
 * **No grip.** The registry has no glyph for dragging, so order is the order ticked, numbered.
 */
export type ValuePopoverProps = {
  /** Names the value: the trigger is `Edit <label>` and the layer is `<label>`. */
  label: string;
  /** The value as its row reads it. */
  value: string;
  /** The value is a placeholder for nothing set, and reads quieter than a fact. */
  empty?: boolean;
  defaultOpen?: boolean;
  children: ReactNode;
};

export function ValuePopover({ label, value, empty = false, defaultOpen, children }: ValuePopoverProps) {
  return (
    <Popover
      label={label}
      defaultOpen={defaultOpen}
      trigger={
        <button
          type="button"
          className="armada-value-popover__value"
          data-empty={empty || undefined}
          aria-label={`Edit ${label}`}
        >
          {value}
        </button>
      }
    >
      <div className="armada-value-popover">{children}</div>
    </Popover>
  );
}

export type OrderedPick = {
  name: string;
  /** Why this one cannot be ticked. A name already ticked can still be unticked. */
  unavailable?: string;
};

export type OrderedPicksProps = {
  label: string;
  /** One line on what ticking does. */
  says: string;
  options: OrderedPick[];
  /** Names in order. */
  picked: string[];
  disabled?: boolean;
  onPicked: (picked: string[]) => void;
};

/** Ticks for which, and a number for in what order. A new tick goes last. */
export function OrderedPicks({ label, says, options, picked, disabled = false, onPicked }: OrderedPicksProps) {
  return (
    <div className="armada-value-popover__picks" role="group" aria-label={label}>
      <p className="armada-value-popover__says">{says}</p>
      <ul className="armada-value-popover__list">
        {options.map((option) => {
          const at = picked.indexOf(option.name);
          const ticked = at >= 0;
          return (
            <li key={option.name} className="armada-value-popover__pick">
              <Checkbox
                checked={ticked}
                disabled={disabled || (!ticked && option.unavailable !== undefined)}
                onChange={() =>
                  onPicked(ticked ? picked.filter((one) => one !== option.name) : [...picked, option.name])
                }
              >
                <span className="armada-value-popover__name">{option.name}</span>
              </Checkbox>
              {ticked ? <span className="armada-value-popover__order">{at + 1}</span> : null}
              {option.unavailable === undefined ? null : (
                <span className="armada-value-popover__says">{option.unavailable}</span>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}

export type OneFlagProps = {
  /** The switch's own label. */
  children: ReactNode;
  checked: boolean;
  /** What on does, and what off does. */
  description: string;
  /** Who decides it, in a line. */
  judgement: string;
  /** Why it cannot be switched on here. */
  unavailable?: string;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
};

/** The degenerate set: one switch, the sentence naming what it changes, and who decides it. */
export function OneFlag({ children, checked, description, judgement, unavailable, disabled = false, onChange }: OneFlagProps) {
  return (
    <div className="armada-value-popover__flag">
      <Switch
        checked={checked}
        description={description}
        disabled={disabled || (!checked && unavailable !== undefined)}
        onChange={(event) => onChange(event.target.checked)}
      >
        {children}
      </Switch>
      <p className="armada-value-popover__says">{unavailable ?? judgement}</p>
    </div>
  );
}
