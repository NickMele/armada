import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { BudgetSection } from "./BudgetSection";
import { ChecksSection } from "./ChecksSection";
import { CommandsSection } from "./CommandsSection";
import { PolicySection } from "./PolicySection";
import { PortsSection } from "./PortsSection";

/**
 * The Manifest as forms — Journey 9's *Editing*, the default editing view, with
 * the file behind the toggle named by its path.
 *
 * One section each for what a form edit can reach: Checks, Commands, ports,
 * policy and budget. **Save writes the file and stops** — no staging, no commit
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

/** Everything the form holds, as typed. */
export type ManifestFormDraft = {
  checks: ManifestFormCheck[];
  commands: ManifestFormCommand[];
  ports: ManifestFormPort[];
  autoMerge: string;
  reviewGate: string;
  /** Dollars. Empty defers to what Fleet runs with. */
  costCap: string;
  turnCap: string;
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
  const { path, changed, saving = false, onSave, onDiscard, receipt, refused, moved, problems } = props;
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

      <ChecksSection {...props} />
      <CommandsSection {...props} />
      <PortsSection {...props} />
      <PolicySection {...props} />
      <BudgetSection {...props} />
    </div>
  );
}
