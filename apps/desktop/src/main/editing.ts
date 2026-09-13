// The Manifest file, read and saved — Journey 9's *Editing*.
//
// Beside `checkout-runs.ts` rather than inside it: both answer routes under
// `/manifest`, but a run is a rehearsal of the file and this is the file.
//
// **Fleet names the file, and nothing here carries a path.** A Fleet serves
// one repository and already holds the file it was started against, so a
// caller names the act and Fleet resolves where it lands.

import type { ManifestFile, ManifestSaved, SaveManifestFile } from "@armada/protocol";
import type { ManifestFileRead, ManifestSaveAnswer } from "@armada/screens/src/editing";

import { ask, type Answer } from "./request";

/**
 * The refusal Fleet raises where the file changed after the edit read it.
 * Declared beside the variant in `crates/fleet/src/refusing.rs`; **matched,
 * never minted** — the one code this seam has to tell apart, because what a
 * person does about it is reconcile rather than retry.
 */
const MANIFEST_MOVED = "fleet.manifest_moved_under_the_edit";

/**
 * A save's answer, as the file view folds it.
 *
 * **`on_disk` rides the refusal's `fields`**, absent where the file is gone —
 * which is a different thing to do about, so it is carried as `null` rather
 * than as an empty text a person could save over.
 */
export function saveAnswerOf(answer: Answer): ManifestSaveAnswer {
  if (answer.ok === true) return { state: "saved", saved: answer.body as ManifestSaved };
  const outcome = answer.outcome;
  if (!outcome.ok && outcome.why === "refused" && outcome.error.code === MANIFEST_MOVED) {
    const onDisk = outcome.error.fields["on_disk"];
    return { state: "moved", onDisk: typeof onDisk === "string" ? onDisk : null };
  }
  return { state: "failed", outcome };
}

/** `get_manifest_file` and `save_manifest_file`. */
export class ManifestFileCommands {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  /** `armada.yml` as it is on disk, whole and unparsed. */
  async readFile(): Promise<ManifestFileRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const answer = await ask(port, "GET", "/manifest/file");
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, file: answer.body as ManifestFile };
  }

  /**
   * Put a corrected Manifest on disk. **Writes and stops** — no staging, no
   * commit. What Fleet made of it arrives later as `manifest.reread`.
   */
  async saveFile(body: SaveManifestFile): Promise<ManifestSaveAnswer> {
    const port = this.port();
    if (port === null) return { state: "failed", outcome: { ok: false, why: "not_connected" } };
    return saveAnswerOf(await ask(port, "POST", "/manifest/save_file", body));
  }
}
