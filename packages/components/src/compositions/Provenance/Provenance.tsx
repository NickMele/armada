/**
 * Where one line of a proposal came from — Journey 3's *Provenance*. **The typeface is the
 * citation**: mono for a file Scan read, sans for a guess, a default, an edit or an
 * addition. Plain `--fg-muted`, never a chip: provenance is not a Job state.
 *
 * `convention` carries both, because the command was read from a file and only its
 * placement is a guess. A record, not a value, so nothing here is editable.
 */
export type ProvenanceProps = {
  /** `read`, `convention`, `default`, `edited_during_setup` or `added_during_setup`. */
  source: string;
  file?: string;
  /** Where in the file — `scripts.test`. */
  at?: string;
};

const WORDS: Record<string, string> = {
  convention: "convention",
  default: "default",
  edited_during_setup: "edited during setup",
  added_during_setup: "added during setup",
};

export function Provenance({ source, file, at }: ProvenanceProps) {
  const cites = file !== undefined && (source === "read" || source === "convention");
  return (
    <span className="armada-provenance">
      {cites ? (
        <span className="armada-provenance__file">{at === undefined ? file : `${file} ${at}`}</span>
      ) : null}
      {source === "read" ? null : (
        <span className="armada-provenance__word">{WORDS[source] ?? source.replaceAll("_", " ")}</span>
      )}
    </span>
  );
}
