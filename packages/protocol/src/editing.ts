// The Manifest file itself: its text, and a corrected text on its way back.
// `crates/ipc/src/editing.rs`.
//
// **The half a person acts with**, where `reading.ts` is the half that reports.
// Correcting a Check used to mean leaving Bridge for an editor and then
// watching the daemon's console to find out whether the correction parsed.
//
// **Fleet resolves the path and nothing here carries one.** A Fleet serves one
// repository and already holds the file it was started against, so a caller
// names the act and Fleet names the file.
//
// The header rules in `protocol.ts` hold here: these are hand-written, they
// drift the day a field moves, and every closed set is left as `string`.

/**
 * `armada.yml` as it is on disk, for the view that draws it.
 *
 * **The bytes, not a document.** Nothing is parsed, so a file that does not
 * parse reads back as well as one that does — which is the case a person
 * opening this is most likely to be in. What the parse came to is
 * `ManifestReading`, asked for separately.
 */
export type ManifestFile = {
  /**
   * The file, as Fleet resolved it — **the same string `ManifestReading.path`
   * carries**, so a surface holding both can see they are about one file. It is
   * what names the toggle that reveals this view; the lexicon bans naming a
   * Manifest by its format.
   */
  path: string;
  /**
   * The whole file. **Not windowed**, where a Check's output serves a tail and
   * says so: a partial text handed to an editor would be saved back over the
   * rest.
   */
  text: string;
};

/**
 * A corrected Manifest, on its way to disk.
 *
 * **Text and nothing else.** No path, and no flag that would make this a
 * staging or a commit — Save writes the file and stops there, and Armada
 * committing to git on your behalf would be a surprise in the one place a
 * person is most sensitive to one.
 *
 * **A write that does not parse is still a write.** Refusing invalid YAML would
 * mean work in progress cannot be saved at all. The bytes go to disk and
 * `ManifestReading` reports what Fleet could not adopt, with the previous
 * values still in force.
 */
export type SaveManifestFile = {
  text: string;
};

/**
 * What a save left on disk, and when.
 *
 * **Not a reading.** Fleet's watch settles before it re-reads, so what a save
 * moved is not known when the write returns — `worthSaying` over the
 * `manifest.reread` that follows is still the judgement about whether it is
 * news.
 */
export type ManifestSaved = {
  path: string;
  /**
   * When the bytes landed — **the write's instant, not a read's**. The reading
   * that follows carries its own `at`, later by up to the settle window, which
   * is what tells this save's answer from a reading already on screen.
   */
  at: string;
};
