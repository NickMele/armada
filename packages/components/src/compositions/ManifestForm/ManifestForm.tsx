import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { AfterMergeSection } from "./AfterMergeSection";
import { BaseSection } from "./BaseSection";
import { BudgetSection } from "./BudgetSection";
import { ChecksSection } from "./ChecksSection";
import { CommandsSection } from "./CommandsSection";
import { DroneSection } from "./DroneSection";
import { EvidenceSection } from "./EvidenceSection";
import { FreezeSection } from "./FreezeSection";
import { PolicySection } from "./PolicySection";
import { PortsSection } from "./PortsSection";
import { SetupSection } from "./SetupSection";

/**
 * The Manifest as forms — Journey 9's *Editing*, the default editing view, with
 * the file behind the toggle named by its path.
 *
 * One section for each part of the schema, so the file view is never where a
 * key has to be reached. **Save writes the file and stops** — no staging, no commit
 * — and a removal takes the entry and the comment directly above it.
 */

/** `checks.<name>.narrow`, as typed. Lists are one entry a line. */
export type ManifestFormNarrow = {
  run: string;
  each: string;
  from: string;
  under: string;
  except: string;
};

export type ManifestFormCheck = {
  name: string;
  run: string;
  /** Empty is 0. */
  expectExitCode: string;
  /** Command names, in the order the file names them. */
  requires: string[];
  /** One path pattern a line. Empty runs the Check on every change. */
  when: string;
  narrow: ManifestFormNarrow | null;
};

export type ManifestFormCommand = {
  name: string;
  run: string;
  destructive: boolean;
  /** Non-empty makes it a server. */
  serve: string;
  ready: string;
  links: { url: string; name: string }[];
};

export type ManifestFormPort = { name: string; container: string; env: string };

/** `evidence:`, as typed. */
export type ManifestFormEvidence = {
  serve: string;
  ready: string;
  run: string;
  frames: string;
  /** One path a line. */
  never: string;
};

/** Everything the form holds, as typed. */
export type ManifestFormDraft = {
  checks: ManifestFormCheck[];
  commands: ManifestFormCommand[];
  ports: ManifestFormPort[];
  /** `freeze: true`. */
  freeze: boolean;
  autoMerge: string;
  reviewGate: string;
  /** Dollars. Empty defers to what Fleet runs with. */
  costCap: string;
  turnCap: string;
  /** Empty: Armada infers one. */
  base: string;
  /** `null` where the file declares none. */
  evidence: ManifestFormEvidence | null;
  /** Check names, in the order they run after a merge. */
  afterMerge: string[];
  /** Command names, in the order setup runs them. */
  setup: string[];
  /** Seconds. This and the two below are empty where the file defers to Fleet. */
  quietAfter: string;
  pokeLimit: string;
  /** One path a line. */
  excludePaths: string;
};

export type ManifestFormProps = {
  /** The file the form writes. Mono: a path is a fact the system reported. */
  path: string;
  draft: ManifestFormDraft;
  onDraft: (draft: ManifestFormDraft) => void;
  /** Every word each policy takes, as Fleet's registry spells them. */
  autoMergeWords: string[];
  reviewGateWords: string[];
  /** What keeps Save back, keyed as the file spells where — `checks.lint.run`. */
  problems: Record<string, string>;
  /** Where a cap is below what a past Job here cost. */
  budgetWarnings: string[];
  /** `id` and `version`, shown and never edited. Absent where Fleet did not say. */
  identity?: { id: string; version: number };
  /** Whether the draft would send any edit. Save is offered only then. */
  changed: boolean;
  saving?: boolean;
  onSave: () => void;
  onDiscard: () => void;
  /** The line beside the path: when the last save landed, or that one has not. */
  receipt?: string;
  /** What Fleet would not write, with its faults key by key. */
  refused?: { saying: string; faults: { key: string; fault: string }[] };
  /** The file changed after the form read it. Nothing was written. */
  moved?: { gone: boolean; onReadAgain: () => void; onSaveOver?: () => void };
};

export function ManifestForm(props: ManifestFormProps) {
  const { path, identity, changed, saving = false, onSave, onDiscard, receipt, refused, moved, problems } = props;
  const blocked = Object.keys(problems).length > 0;
  return (
    <div className="armada-manifest-form">
      {/* One sticky block, so a refusal is on screen wherever Save was pressed. */}
      <div className="armada-manifest-form__top">
      <div className="armada-manifest-form__head">
        <span className="armada-manifest-form__path">{path}</span>
        <span className="armada-manifest-form__receipt">{receipt}</span>
        <Button variant="secondary" size="sm" disabled={!changed || saving} onClick={onDiscard}>
          Discard changes
        </Button>
        <Button variant="primary" size="sm" disabled={!changed || saving || blocked} onClick={onSave}>
          {saving ? "Saving" : "Save"}
        </Button>
      </div>

      {refused === undefined ? null : (
        <Alert tone="escalated" title="Not saved">
          <p className="armada-manifest-form__said">{refused.saying}</p>
          {refused.faults.length === 0 ? null : (
            <ul className="armada-manifest-form__faults">
              {refused.faults.map((fault) => (
                <li key={`${fault.key}:${fault.fault}`}>
                  <span className="armada-manifest-form__key">{fault.key}</span> {fault.fault}
                </li>
              ))}
            </ul>
          )}
        </Alert>
      )}

      {moved === undefined ? null : (
        <Alert
          tone="caution"
          title={moved.gone ? "The file is no longer there" : "The file changed after you opened it"}
        >
          <p className="armada-manifest-form__said">
            Nothing was saved, and your changes are still here.
          </p>
          <div className="armada-manifest-form__acts">
            <Button variant="secondary" size="sm" disabled={saving} onClick={moved.onReadAgain}>
              Discard my changes and read it again
            </Button>
            {moved.onSaveOver === undefined ? null : (
              <Button variant="primary" size="sm" disabled={saving} onClick={moved.onSaveOver}>
                Make my changes to it as it is now
              </Button>
            )}
          </div>
        </Alert>
      )}
      </div>

      {identity === undefined ? null : (
        <section className="armada-manifest-form__identity" aria-label="This Manifest">
          <dl className="armada-manifest-form__facts">
            <div>
              <dt>Id</dt>
              <dd className="armada-manifest-form__key">{identity.id}</dd>
            </div>
            <div>
              <dt>Version</dt>
              <dd className="armada-manifest-form__key">{identity.version}</dd>
            </div>
          </dl>
          <p className="armada-manifest-form__hint">
            Past Jobs here are recorded against this id, so it is not changed from a form.
          </p>
        </section>
      )}

      <FreezeSection {...props} />
      <ChecksSection {...props} />
      <CommandsSection {...props} />
      <SetupSection {...props} />
      <PortsSection {...props} />
      <EvidenceSection {...props} />
      <AfterMergeSection {...props} />
      <BaseSection {...props} />
      <PolicySection {...props} />
      <BudgetSection {...props} />
      <DroneSection {...props} />
    </div>
  );
}
