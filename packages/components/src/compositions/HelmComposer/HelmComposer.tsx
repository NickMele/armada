import type { FormEvent } from "react";
import { Button } from "../../primitives/Button/Button";
import { Select } from "../../primitives/Select/Select";
import { Textarea } from "../../primitives/Textarea/Textarea";

/** One repository the switch may point Helm at — id is the Manifest id. */
export type HelmRepositoryOption = { id: string; label: string };

/**
 * The composer under Helm's thread — #944. **Typing never answers a pending
 * question**; that is a card's own button, above this, in the dock's
 * questions zone. This only ever sends a fresh message.
 */
export type HelmComposerProps = {
  /** The repository Helm answers for right now, by its Manifest id. Absent where nothing is servable yet. */
  current?: string;
  /** Every repository the switch may choose. Fewer than two draws no control — there is nothing to switch to. */
  repositories?: HelmRepositoryOption[];
  /** The dock's own switch, on All repositories. The rail's pick never moves for it. */
  onSwitch?: (manifestId: string) => void;
  onStartFresh?: () => void;
  /** Refused while a reply is being written — Fleet's own rule, not a guess drawn here. */
  startFreshDisabled?: boolean;
  value: string;
  onChange: (value: string) => void;
  onSend: () => void;
  /** Nothing is servable, or the connection is down. */
  disabled?: boolean;
};

export function HelmComposer({
  current,
  repositories = [],
  onSwitch,
  onStartFresh,
  startFreshDisabled = false,
  value,
  onChange,
  onSend,
  disabled = false,
}: HelmComposerProps) {
  const blank = value.trim() === "";
  const label = repositories.find((one) => one.id === current)?.label;

  function submit(event: FormEvent): void {
    event.preventDefault();
    if (!blank && !disabled) onSend();
  }

  return (
    <form className="armada-helm-composer" onSubmit={submit}>
      <div className="armada-helm-composer__head">
        <span className="armada-helm-composer__repository">
          {label ?? "No repository to ask yet"}
        </span>
        {onSwitch === undefined || repositories.length < 2 ? null : (
          <Select
            aria-label="Point Helm at a different repository"
            value={current ?? ""}
            onChange={(event) => onSwitch(event.target.value)}
          >
            {repositories.map((one) => (
              <option key={one.id} value={one.id}>
                {one.label}
              </option>
            ))}
          </Select>
        )}
        {onStartFresh === undefined ? null : (
          <Button variant="ghost" size="sm" onClick={onStartFresh} disabled={startFreshDisabled}>
            Start fresh
          </Button>
        )}
      </div>
      <Textarea
        aria-label="Ask Helm"
        rows={2}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        disabled={disabled}
        placeholder="Ask Helm about this repository"
      />
      <div className="armada-helm-composer__actions">
        <Button type="submit" variant="primary" size="sm" disabled={disabled || blank}>
          Send
        </Button>
      </div>
    </form>
  );
}
