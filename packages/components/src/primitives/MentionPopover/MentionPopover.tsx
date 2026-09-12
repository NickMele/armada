export { useMention } from "./useMention";

/**
 * The `@` mention popup — a person typing a request or a brief types `@` and
 * narrows this as they keep typing, the way Claude Code's own `@` file
 * reference works. `useMention` in this module decides when it is open and
 * what it holds; this only draws what it is handed.
 *
 * **Anchored under the field, not the caret.** `CommandPalette` is the nearest
 * relative and is a centred, floating layer with nothing to anchor to; a
 * caret-tracked popup needs either a second, invisible copy of the field's
 * text to measure against or a positioning library, and this ships the
 * simpler anchor rather than carry either for a first cut. Flagged as the
 * thing a caret-position version would change, not silently accepted as
 * final.
 */
export type MentionPopoverProps = {
  /** What has been typed since the `@`, shown in the empty state. */
  query: string;
  /** Capped by `crates/fleet/src/files.rs`; never re-cut here. */
  results: readonly string[];
  /** The row `Enter` or `Tab` would choose. */
  active: number;
  onHover: (index: number) => void;
  onChoose: (path: string) => void;
};

export function MentionPopover({ query, results, active, onHover, onChoose }: MentionPopoverProps) {
  return (
    <div className="armada-mention" role="listbox" aria-label="Files matching the mention">
      {results.length === 0 ? (
        <p className="armada-mention__empty">{`No file matches “${query}”.`}</p>
      ) : (
        results.map((path, index) => (
          <div
            key={path}
            role="option"
            aria-selected={index === active}
            className={
              index === active ? "armada-mention__row armada-mention__row--active" : "armada-mention__row"
            }
            onMouseEnter={() => onHover(index)}
            // `onMouseDown`, not `onClick`: a click fires after the field's own
            // blur, which would have already closed this popover — the state
            // `onChoose` reads would already be gone. Prevented so the field
            // never loses focus at all.
            onMouseDown={(event) => {
              event.preventDefault();
              onChoose(path);
            }}
          >
            {path}
          </div>
        ))
      )}
    </div>
  );
}
