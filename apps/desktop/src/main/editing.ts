// The Manifest file, read and saved — Journey 9's *Editing*.
//
// Beside `checkout-runs.ts` rather than inside it: both answer routes under
// `/manifest`, but a run is a rehearsal of the file and this is the file.
//
// **Fleet names the file, and nothing here carries a path.** A Fleet serves
// one repository and already holds the file it was started against, so a
// caller names the act and Fleet resolves where it lands.

import type {
  EditManifest,
  ManifestEdited,
  ManifestFile,
  ManifestSaved,
  ManifestSpend,
  SaveManifestFile,
} from "@armada/protocol";
import type {
  ManifestEditAnswer,
  ManifestFileRead,
  ManifestSaveAnswer,
  ManifestSpendRead,
} from "@armada/screens/src/editing";

import { ask, type Answer } from "./request";
import { SetupCommands } from "./setting-up";

/**
 * The refusal Fleet raises where the file changed after the edit read it.
 * Declared beside the variant in `crates/fleet/src/refusing.rs`; **matched,
 * never minted** — the one code this seam has to tell apart, because what a
 * person does about it is reconcile rather than retry.
 */
const MANIFEST_MOVED = "fleet.manifest_moved_under_the_edit";

/** What Fleet would not write for a form, from `crates/fleet/src/amending.rs`. Matched, never minted. */
const EDIT_REFUSALS = [
  "fleet.manifest_edit_refused",
  "fleet.manifest_edit_misnamed",
  "fleet.manifest_edit_unplaceable",
];

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

/**
 * An edit's answer, as the form folds it. `faults` rides as `[key, fault]`
 * pairs; a pair in any other shape is dropped rather than drawn half-read.
 */
export function editAnswerOf(answer: Answer): ManifestEditAnswer {
  if (answer.ok === true) return { state: "edited", edited: answer.body as ManifestEdited };
  const outcome = answer.outcome;
  if (outcome.ok || outcome.why !== "refused") return { state: "failed", outcome };
  const { code, message, fields } = outcome.error;
  if (code === MANIFEST_MOVED) {
    const onDisk = fields["on_disk"];
    return { state: "moved", onDisk: typeof onDisk === "string" ? onDisk : null };
  }
  if (!EDIT_REFUSALS.includes(code)) return { state: "failed", outcome };
  const pairs = Array.isArray(fields["faults"]) ? (fields["faults"] as unknown[]) : [];
  const faults = pairs.flatMap((pair) =>
    Array.isArray(pair) && typeof pair[0] === "string" && typeof pair[1] === "string"
      ? [{ key: pair[0], fault: pair[1] }]
      : [],
  );
  return { state: "refused", saying: message, faults };
}

/** `get_manifest_file`, `save_manifest_file`, `edit_manifest` and `get_manifest_spend`. */
export class ManifestFileCommands {
  private readonly port: () => number | null;
  /** Setup's Scan, proposals, edits and Write: the other writer of `armada.yml`. */
  readonly setup: SetupCommands;

  constructor(port: () => number | null) {
    this.port = port;
    this.setup = new SetupCommands(port);
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

  /** A form's edits, by key. **Writes and stops**, and answers with the file as written. */
  async edit(body: EditManifest): Promise<ManifestEditAnswer> {
    const port = this.port();
    if (port === null) return { state: "failed", outcome: { ok: false, why: "not_connected" } };
    return editAnswerOf(await ask(port, "POST", "/manifest/edit", body));
  }

  /** The costliest and the longest past Job against this Manifest. */
  async readSpend(): Promise<ManifestSpendRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const answer = await ask(port, "GET", "/manifest/spend");
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, spend: answer.body as ManifestSpend };
  }
}
