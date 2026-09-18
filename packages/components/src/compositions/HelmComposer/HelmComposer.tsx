import type { FormEvent, KeyboardEvent } from "react";
import { AttachmentChip } from "../../primitives/AttachmentChip/AttachmentChip";
import { Button } from "../../primitives/Button/Button";
import { Select } from "../../primitives/Select/Select";
import { SplitButton } from "../../primitives/SplitButton/SplitButton";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { SendKbd, sendsOn } from "../../send-message";
import { COPY_DEBUG_INFO } from "../../errors/ErrorNotice/payload";

/** One repository the switch may point Helm at — id is the Manifest id. */
export type HelmRepositoryOption = { id: string; label: string };

/** No Manifest id is empty, so this value is never a repository. `AskRepository` reads the same way. */
const UNPOINTED = "";

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
  /**
   * Every repository the switch may choose. **Pointed at one of them, fewer
   * than two draws no control** — there is nothing to switch to. Pointed at
   * nothing, one is enough: it is somewhere to go. None is still nothing.
   */
  repositories?: HelmRepositoryOption[];
  /** The dock's own switch, on All repositories. The rail's pick never moves for it. */
  onSwitch?: (manifestId: string) => void;
  /**
   * Puts the session record on the clipboard in one press — `#1367`. **The
   * banner form of the error treatment**: the dock is a standing surface, and
   * the moment this exists for is a bad answer somebody wants to carry now.
   * That is why it is the split button's face and *Details* is behind the
   * caret. Absent draws no control.
   */
  onCopyRecord?: () => void;
  /**
   * Opens the same record to read. **The banner's *Details*, in that word** —
   * the sheet it opens is titled *Session record*, and that sheet carries a
   * copy of its own, so nothing is lost by it sitting under the caret. Absent
   * draws no control.
   */
  onOpenRecord?: () => void;
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
  onCopyRecord,
  onOpenRecord,
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
  /**
   * Somewhere to point Helm that it is not pointed at already. **Pointed at a
   * repository, one repository is nothing to switch between**, which is all
   * this used to say. Pointed at nothing, that one repository is somewhere to
   * go, and the dock says so in words — *Helm is not pointed at a repository.
   * Pick armada to ask about it.* — so the control it names has to be here to
   * pick with. With none set up there is still nothing to draw either way, and
   * the dock's own sentence carries that moment on its own.
   */
  const somewhere = current === undefined ? repositories.length > 0 : repositories.length > 1;
  const switching = onSwitch !== undefined && somewhere;
  /**
   * What Helm is pointed at, said once on this line. Pointed at nothing with
   * the switch drawn, the switch's own entry is what says it — and says what to
   * do about it — so this stays empty rather than repeating it: at the dock's
   * width the two together cut each other down to "No repo…" and "Choose a
   * rep…". With no switch to draw there is nowhere else for it to be said.
   */
  const naming = label ?? (switching ? undefined : "No repository to ask yet");

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
        {naming === undefined ? null : (
          <span className="armada-helm-composer__repository">{naming}</span>
        )}
        {onSwitch === undefined || !switching ? null : (
          <Select
            // Named for what it does from where Helm is standing. *A different
            // repository* is a lie while it is pointed at none, and since this
            // change that is the switch's commonest moment — every unpointed
            // dock with anything set up draws it, one repository included.
            aria-label={current === undefined ? "Point Helm at a repository" : "Point Helm at a different repository"}
            value={current ?? UNPOINTED}
            onChange={(event) => {
              if (event.target.value !== UNPOINTED) onSwitch(event.target.value);
            }}
          >
            {/* Pointed at nothing, the switch needs an entry of its own to
                stand at — the same disabled placeholder the repository asks
                elsewhere draw (`AskRepository`), in the same words. Without
                one, a `value` matching no `<option>` left Chromium displaying
                the first repository: the switch disagreed with the chip beside
                it, and that first repository could not be chosen at all,
                because picking the entry already displayed fires no `change`.
                Once Helm is pointed at one, there is nothing to stand in for
                and the entry is gone. */}
            {current === undefined ? (
              <option value={UNPOINTED} disabled>
                Choose a repository
              </option>
            ) : null}
            {repositories.map((one) => (
              <option key={one.id} value={one.id}>
                {one.label}
              </option>
            ))}
          </Select>
        )}
        {/* The record's two acts as one control — the owner's note of
            18 Sep 2026: three controls beside the repository wrapped this row
            onto a second line at the dock's own width. *Copy debug info*
            takes the face because it is the act this pair exists for — a bad
            answer somebody wants to carry now — and the reading it hides is
            still one press away behind the caret, in a sheet that carries a
            copy of its own.

            **Both handlers or nothing to split.** With one, there is no menu
            to put behind a caret, and a caret over an empty menu is a control
            that does not answer — so that case draws the plain button it
            already was, in the same weight as the split one. */}
        {onCopyRecord !== undefined && onOpenRecord !== undefined ? (
          <SplitButton
            variant="secondary"
            ground="card"
            size="sm"
            items={[{ label: "Details", onSelect: onOpenRecord }]}
            menuLabel="More ways to report this answer"
            onAction={onCopyRecord}
          >
            {COPY_DEBUG_INFO}
          </SplitButton>
        ) : onCopyRecord !== undefined ? (
          <Button variant="secondary" ground="card" size="sm" onClick={onCopyRecord}>
            {COPY_DEBUG_INFO}
          </Button>
        ) : onOpenRecord !== undefined ? (
          <Button variant="secondary" ground="card" size="sm" onClick={onOpenRecord}>
            Details
          </Button>
        ) : null}
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
