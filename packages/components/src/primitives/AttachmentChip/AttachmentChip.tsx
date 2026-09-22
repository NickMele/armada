/**
 * A thing attached to a request before the Job exists — one name and one
 * removal control. Nothing here reads a byte or a path aloud: a person
 * composing the request needs to see what they attached and be able to take it
 * back, not a preview of its contents.
 *
 * **Three kinds, told apart by a word and not by a glyph.** A file, a link and
 * a Studio node are three different things to a Drone, and a filename does not
 * always carry which one it is.
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
  /**
   * Where it came from, set back **before** the name — `From a Studio ·
   * sketch 1` (#1547).
   *
   * **Leading rather than trailing, because it is not the kind.** A kind says
   * what a thing is and reads after the name; provenance says where it was
   * made and is the first thing that matters about a picture, since a sketch
   * with no source is a person drawing from nothing and that is a different
   * answer. Absent draws nothing rather than "from nowhere".
   */
  from?: string;
  /**
   * What was attached. **`file` draws no word**: a filename already says which
   * of the three it is, and `screenshot.png · file` spends a column on nothing.
   */
  kind?: AttachmentKind;
  /** Omitted renders a read-only chip — nothing to take back. */
  onRemove?: () => void;
};

/** A staged file, an address, or one node of a Studio. */
export type AttachmentKind = "file" | "link" | "node";

/** What each kind is called. `Studio node`, because the word alone names nothing. */
const KIND: Record<AttachmentKind, string | null> = {
  file: null,
  link: "link",
  node: "Studio node",
};

export function AttachmentChip({ filename, from, kind = "file", onRemove }: AttachmentChipProps) {
  const said = KIND[kind];
  return (
    <span className="armada-attachment-chip">
      {from === undefined ? null : (
        <span className="armada-attachment-chip__from">{from}</span>
      )}
      <span className="armada-attachment-chip__name">{filename}</span>
      {said === null ? null : <span className="armada-attachment-chip__kind">{said}</span>}
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
