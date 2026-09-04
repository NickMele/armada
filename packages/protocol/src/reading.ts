// What Fleet's last read of `armada.yml` came to. `crates/ipc/src/reading.rs`.
//
// **The one shape here about the fleet and not about a Job**, beside
// `FleetCapacity`. Every other read on this seam answers about a Job, a step or
// a Drone; a Manifest reload belongs to none of them, which is why the only
// place it used to be said was the daemon's console — a terminal window a
// person running Bridge does not have open.
//
// It arrives two ways and they carry the same type: `get_manifest_reading`
// answers it, and `manifest.reread` pushes it. That is deliberate — a refusal
// is a standing condition rather than an instant, so a Bridge opened a minute
// later has to be able to ask.
//
// The header rules in `protocol.ts` hold here: these are hand-written, they
// drift the day a field moves, and every closed set is left as `string`.

/**
 * Fleet's last reading of the Manifest, and what it did about it.
 *
 * **Refusal is the absent field, not a flag.** `refused` present is the read
 * that did not take; absent is the read that did. `JobJudging` and
 * `ProposalMoved` carry an absence the same way, rather than a boolean beside a
 * reason that means nothing when it is false.
 */
export type ManifestReading = {
  /** The file, as Fleet resolved it. A Fleet may hold more than one repository. */
  path: string;
  /**
   * When Fleet **read** it, not when the file was saved. The two differ by up
   * to the settle window, and the filesystem is where the save's instant lives.
   */
  at: string;
  /**
   * The live keys this read moved, already in force. **Absent is the ordinary
   * answer**: most saves edit something Fleet does not read live.
   */
  moved?: ManifestMoved[];
  /**
   * Sections that changed and were **not** adopted, spelled as `armada.yml`
   * spells them — `checks`, `commands`, `setup`, `id`, `version`, `base`.
   * Rendered, never matched on, so a section added later draws as itself.
   */
  at_restart?: string[];
  /** Why the read did not take, or **absent because it did**. */
  refused?: ManifestRefused;
};

/** One live key that changed, carrying both ends. */
export type ManifestMoved = {
  /** Its path in `armada.yml` — `drone.poke_limit` — which is what a person
   * would search the file for. */
  key: string;
  /**
   * **Absent is a real value, not a missing one**: the key was not in the file,
   * and the repository was deferring to what Fleet runs with. A surface spells
   * that rather than leaving a blank, which would read as a number that failed
   * to load.
   */
  before?: number;
  after?: number;
};

/**
 * Why a read was refused, and what is running instead.
 *
 * **The previous values stay in force.** One mistyped number is not grounds for
 * stopping every Job, so Fleet carries on with the last good configuration —
 * and that is the second thing a person needs to know, right after why.
 */
export type ManifestRefused = {
  /**
   * The whole refusal in one sentence, as Fleet renders it.
   *
   * **The only place a line number can appear.** A file that is not YAML at all
   * has no keys to attribute a fault to, and the parser's own error is what
   * carries the line and column.
   */
  summary: string;
  /**
   * What was wrong, key by key — **every fault, not the first**. Absent where
   * the document never became a document, in which case `summary` is the whole
   * answer.
   */
  faults?: ManifestFault[];
};

/** One key `armada.yml` was refused for. */
export type ManifestFault = {
  /** The dotted path inside the document — `checks.build.run`. Indices are the
   * array position, so it points at a line rather than at a name somebody
   * would have to count to find. */
  key: string;
  /** What is wrong with it, in the words Fleet used to refuse it. */
  fault: string;
};

/**
 * Whether a reading is worth putting in front of somebody.
 *
 * **A save that moved nothing is not news.** Editing a comment, or an editor
 * writing the same bytes back, is a reading like any other, and drawing it
 * would train a person to dismiss the surface that also carries the refusal.
 * `ManifestReading::worth_saying` is the same judgement in Rust; it is spelled
 * here too because the surface that draws it is here, and the alternative was
 * Fleet dropping the fact rather than reporting it.
 */
export function worthSaying(reading: ManifestReading): boolean {
  return (
    reading.refused !== undefined ||
    (reading.moved ?? []).length > 0 ||
    (reading.at_restart ?? []).length > 0
  );
}
