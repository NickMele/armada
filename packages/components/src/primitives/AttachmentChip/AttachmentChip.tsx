/**
 * A file staged onto a Job brief, before the Job exists — one filename and
 * one removal control. Nothing here reads a byte or a path aloud: a person
 * composing the brief needs to see what they attached and be able to take it
 * back, not a preview of its contents.
 *
 * **No icon.** The `file-*` glyph family is reserved to Evidence throughout
 * `packages/icons/icons.toml` — an attachment staged on a draft brief is not
 * evidence, so this does not reach for it. The remove control is a bare `×`
 * character rather than lucide's `x`, whose registry entry reserves it to
 * system failure and never a human decision — a chip's dismiss is exactly
 * the decision that glyph may not carry. Flagged as a gap: the registry has
 * no glyph proposed for "remove this," and this component does not invent
 * one on the spot.
 */
export type AttachmentChipProps = {
  /** Shown as typed at attach time. Never the staged path. */
  filename: string;
  /** Omitted renders a read-only chip — nothing to take back. */
  onRemove?: () => void;
};

export function AttachmentChip({ filename, onRemove }: AttachmentChipProps) {
  return (
    <span className="armada-attachment-chip">
      <span className="armada-attachment-chip__name">{filename}</span>
      {onRemove !== undefined && (
        <button
          type="button"
          className="armada-attachment-chip__remove"
          onClick={onRemove}
          aria-label={`Remove ${filename}`}
        >
          ×
        </button>
      )}
    </span>
  );
}
