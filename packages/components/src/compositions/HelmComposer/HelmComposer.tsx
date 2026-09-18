import type { FormEvent, KeyboardEvent } from "react";
import { AttachmentChip } from "../../primitives/AttachmentChip/AttachmentChip";
import { Button } from "../../primitives/Button/Button";
import { Select } from "../../primitives/Select/Select";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { SendKbd, sendsOn } from "../../send-message";

/** One repository the switch may point Helm at — id is the Manifest id. */
export type HelmRepositoryOption = { id: string; label: string };

/** The open Job chipped above the message box — `#1075`. */
export type HelmComposerChip = { jobHandle: string; title: string };

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
  /** The open Job, while its chip stands. Absent off a Job, or once its `×` has been pressed. */
  chip?: HelmComposerChip;
  /** The chip's own `×`. Omitted with no `chip` draws nothing to remove. */
  onRemoveChip?: () => void;
  /**
   * The footer's one sentence — where the person is, in the caller's own
   * words. **Built off the same context #1075 sends with every ask**, never a
   * second reading of the screen: a footer that could disagree with what
   * Helm was actually told would be worse than none. No lead-in — never
   * "Helm reads where you are: …". `#1094`. Absent draws no footer.
   */
  location?: string;
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
  chip,
  onRemoveChip,
  location,
  value,
  onChange,
  onSend,
  disabled = false,
}: HelmComposerProps) {
  const blank = value.trim() === "";
  const available = !blank && !disabled;
  const label = repositories.find((one) => one.id === current)?.label;

  function submit(event: FormEvent): void {
    event.preventDefault();
    if (available) onSend();
  }

  // ⌘Enter asks exactly what Send asks — the drone message box's binding, one
  // treatment. Plain Enter is still a new line: an ask is prose too.
  function keyed(event: KeyboardEvent<HTMLTextAreaElement>): void {
    if (!sendsOn(event)) return;
    event.preventDefault();
    if (available) onSend();
  }

  return (
    <form className="armada-helm-composer" onSubmit={submit}>
      {chip === undefined ? null : (
        <AttachmentChip filename={`Job ${chip.jobHandle} · ${chip.title}`} onRemove={onRemoveChip} />
      )}
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
      {/* Send sits inside the field's own frame, at its trailing foot — the
          same treatment as the drone message box, decided together on
          17 Sep 2026. The dock is narrow, so Send sits closer to the text
          here than it does there; the room it takes is still reserved as the
          field's bottom padding, so a typed line never runs under it. */}
      <div className="armada-helm-composer__field">
        <Textarea
          aria-label="Ask Helm"
          // Three rows, as the drone message box takes: an ask is a sentence
          // or two, and in a dock this narrow one of them wraps.
          rows={3}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          onKeyDown={keyed}
          disabled={disabled}
          placeholder="Ask Helm about this repository"
          // Reopening the dock remounts this field — #1094 — and that is the
          // only time this fires: nothing else in the tree changes its key.
          autoFocus
        />
        {/* Tinted Helm, not the accent: Send is not the view's one Primary
            (design-system.md, Component → token mapping → Helm dock). The
            tint is HelmComposer.css's, over a secondary. */}
        <Button type="submit" variant="secondary" ground="card" size="sm" disabled={disabled || blank}>
          Send
          <SendKbd available={available} />
        </Button>
      </div>
      {/* Where the person is stays under the field, not inside it: it is a
          sentence to read, and the dock has no width to put it beside Send. */}
      {location === undefined ? null : (
        <span className="armada-helm-composer__location">{location}</span>
      )}
    </form>
  );
}
