import { useState, type ReactNode } from "react";

import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { PortAddForm, type PortAddition } from "../PortAddForm/PortAddForm";
import { Provenance, type ProvenanceProps } from "../Provenance/Provenance";

/**
 * One workspace's proposal, over the picker — Journey 3's *The proposal sheet*.
 *
 * **Ports first**, so a variable is met before a command reading it. **Checks and Commands are
 * two registries**, and every row moves between them, because which one Scan put a script in
 * is a guess. **The values are the fields**: a value is text until pressed, then an input at
 * the same height. Edits land in Fleet's draft at once; Write puts the file down, once.
 *
 * **No Open in Helm.** Nothing serves a Helm session scoped to a proposal, and a control that
 * reaches nothing is worse than none. Row actions are labelled ghosts: the icon registry has
 * no glyph for move or remove. **Data in, callbacks out** — no protocol type.
 */

export type ProposalBand = "checks" | "commands";

export type ProposalPort = { name: string; container?: number; env?: string; cited: ProvenanceProps };

export type ProposalEntry = {
  name: string;
  run: string;
  /** Command names, in the order they run first. */
  requires?: string[];
  destructive?: boolean;
  cited: ProvenanceProps;
};

export type ProposalPolicy = {
  key: string;
  label: string;
  value: string;
  /** Every value it takes, each with the consequence in plain words. */
  options: { value: string; says: string }[];
  cited: ProvenanceProps;
};

export type ProposalSheetProps = {
  open: boolean;
  dir: string;
  /** Where Write would put the file, relative to the checkout. */
  file: string;
  id: { value: string; cited: ProvenanceProps };
  ports: ProposalPort[];
  checks: ProposalEntry[];
  commands: ProposalEntry[];
  setup?: { requires: string[]; cited: ProvenanceProps };
  policy: ProposalPolicy[];
  /** The budget caps, stated in one line and never offered as controls. */
  caps: string;
  /** Port variables other workspaces in the batch map, for the add form's warning. */
  elsewhere?: { env: string; dir: string }[];
  /** An edit Fleet would not apply, in its words. */
  problem?: string;
  /** Faults the file would not load for; each is also drawn under the row it names. */
  refused?: { saying: string; faults: { key: string; fault: string }[] };
  /** A file was already at the path, so nothing was written. */
  appeared?: { onDisk: string | null };
  /** The receipt once Write landed. The sheet takes no edits after it. */
  written?: ReactNode;
  /** Verify, mounted on the sheet that wrote the file. */
  verify?: ReactNode;
  busy?: boolean;
  onEditId: (id: string) => void;
  onEditRun: (band: ProposalBand, name: string, run: string) => void;
  onMove: (band: ProposalBand, name: string) => void;
  onRemove: (band: ProposalBand | "ports", name: string) => void;
  onAdd: (band: ProposalBand, name: string, run: string) => void;
  onAddPort: (port: PortAddition) => void;
  onPolicy: (key: string, value: string) => void;
  onWrite: () => void;
  onClose: () => void;
  contained?: boolean;
  floor?: boolean;
};

const NOUN: Record<ProposalBand, string> = { checks: "Check", commands: "Command" };
const OTHER: Record<ProposalBand, ProposalBand> = { checks: "commands", commands: "checks" };
const TITLE: Record<ProposalBand, string> = { checks: "Checks", commands: "Commands" };

export function ProposalSheet(props: ProposalSheetProps) {
  const { open, dir, file, id, written, busy = false, refused } = props;
  const locked = written !== undefined || busy;
  const faultsAt = (prefix: string) =>
    (refused?.faults ?? []).filter((one) => one.key === prefix || one.key.startsWith(`${prefix}.`));

  return (
    <Sheet
      open={open}
      title={`Proposal for ${dir}`}
      subtitle={<span className="armada-proposal-sheet__file">{file}</span>}
      size="wide"
      contained={props.contained}
      floor={props.floor}
      closeLabel="Back to the workspaces"
      onClose={props.onClose}
      footer={<Foot {...props} />}
    >
      <div className="armada-proposal-sheet">
        {written === undefined ? null : (
          <div className="armada-proposal-sheet__landed">
            <p className="armada-proposal-sheet__said">{written}</p>
            {props.verify}
          </div>
        )}
        {props.problem === undefined ? null : (
          <Alert tone="caution" title="That edit was not applied">
            {props.problem}
          </Alert>
        )}

        <div className="armada-proposal-sheet__identity">
          <span className="armada-proposal-sheet__label">Id</span>
          <InPlace label="Id" value={id.value} locked={locked} onCommit={props.onEditId} />
          <Provenance {...id.cited} />
        </div>

        <Band title="Ports" says="Declared once, and read by every Check and Command below as a variable.">
          {props.ports.length === 0 ? (
            <p className="armada-proposal-sheet__says">
              A declared port is leased per worktree and handed in, so two Jobs can run the same Check at
              once without colliding. No file here declares one.
            </p>
          ) : (
            <ul className="armada-proposal-sheet__rows" aria-label="Ports">
              {props.ports.map((port) => (
                <Row key={port.name} name={port.name} cited={port.cited} faults={faultsAt(`ports.${port.name}`)}>
                  <span className="armada-proposal-sheet__mono">
                    {port.env === undefined
                      ? `${port.container ?? "?"}, rewritten in the compose file`
                      : `${port.container ?? "?"} → $${port.env}`}
                  </span>
                  {locked ? null : (
                    <Button variant="ghost" size="sm" onClick={() => props.onRemove("ports", port.name)}>
                      Remove
                    </Button>
                  )}
                </Row>
              ))}
            </ul>
          )}
          {locked ? null : (
            <PortAddForm
              taken={props.ports.map((port) => port.name)}
              elsewhere={props.elsewhere}
              onAdd={props.onAddPort}
            />
          )}
        </Band>

        {(["checks", "commands"] as const).map((band) => (
          <Registry key={band} band={band} {...props} locked={locked} faultsAt={faultsAt} />
        ))}

        {props.setup === undefined ? null : (
          <Band title="Setup" says="Commands run once per worktree, before any Check. Not a Check or a Command, so it does not move.">
            <ul className="armada-proposal-sheet__rows" aria-label="Setup">
              <Row name="requires" cited={props.setup.cited} faults={faultsAt("setup")}>
                <span className="armada-proposal-sheet__mono">{props.setup.requires.join(", ")}</span>
              </Row>
            </ul>
          </Band>
        )}

        <Band title="Policy" says="Nothing in a repository says either, so each reads default until you choose. A default is written as no key.">
          {props.policy.map((row) => (
            <div key={row.key} className="armada-proposal-sheet__policy">
              <RadioGroup label={row.label}>
                {row.options.map((option) => (
                  <Radio
                    key={option.value}
                    name={`${dir}:${row.key}`}
                    value={option.value}
                    checked={row.value === option.value}
                    disabled={locked}
                    onChange={() => props.onPolicy(row.key, option.value)}
                  >
                    <span className="armada-proposal-sheet__mono">{option.value}</span>
                  </Radio>
                ))}
              </RadioGroup>
              <p className="armada-proposal-sheet__says">
                {row.options.find((option) => option.value === row.value)?.says}
              </p>
              <Provenance {...row.cited} />
            </div>
          ))}
          <p className="armada-proposal-sheet__says">{props.caps}</p>
        </Band>
      </div>
    </Sheet>
  );
}

function Foot({ file, refused, appeared, written, busy = false, onWrite }: ProposalSheetProps) {
  return (
    <div className="armada-proposal-sheet__foot">
      {refused === undefined ? null : (
        <Alert tone="escalated" title="Not written">
          <p className="armada-proposal-sheet__said">{refused.saying}</p>
          <ul className="armada-proposal-sheet__faults">
            {refused.faults.map((fault) => (
              <li key={`${fault.key}:${fault.fault}`}>
                <span className="armada-proposal-sheet__mono">{fault.key}</span> {fault.fault}
              </li>
            ))}
          </ul>
        </Alert>
      )}
      {appeared === undefined ? null : (
        <Alert tone="caution" title={`There is already a file at ${file}`}>
          <p className="armada-proposal-sheet__said">Nothing was written over it. It reads:</p>
          <pre className="armada-proposal-sheet__disk">{appeared.onDisk ?? "(unreadable)"}</pre>
        </Alert>
      )}
      <Button variant="primary" disabled={written !== undefined || busy} onClick={onWrite}>
        {busy ? "Writing" : `Write ${file}`}
      </Button>
    </div>
  );
}

function Registry({
  band,
  locked,
  faultsAt,
  ...props
}: ProposalSheetProps & {
  band: ProposalBand;
  locked: boolean;
  faultsAt: (prefix: string) => { key: string; fault: string }[];
}) {
  const entries = props[band];
  const says =
    band === "checks"
      ? "A Check verifies the work and gates it. Where Scan placed a script here, the row reads convention."
      : "A Command is a script the workspace uses and gates nothing. A Check may run one first.";
  return (
    <Band title={TITLE[band]} says={says}>
      {entries.length === 0 ? (
        <p className="armada-proposal-sheet__says">{`No ${TITLE[band]} proposed.`}</p>
      ) : (
        <ul className="armada-proposal-sheet__rows" aria-label={TITLE[band]}>
          {entries.map((entry) => (
            <Row key={entry.name} name={entry.name} cited={entry.cited} faults={faultsAt(`${band}.${entry.name}`)}>
              <span className="armada-proposal-sheet__run">
                {entry.requires === undefined || entry.requires.length === 0 ? null : (
                  <span className="armada-proposal-sheet__mono">{`${entry.requires.join(", ")} →`}</span>
                )}
                <InPlace
                  label={`${entry.name} command`}
                  value={entry.run}
                  locked={locked}
                  onCommit={(run) => props.onEditRun(band, entry.name, run)}
                />
                {entry.destructive ? <span className="armada-proposal-sheet__says">destructive</span> : null}
              </span>
              {locked ? null : (
                <span className="armada-proposal-sheet__acts">
                  <Button variant="ghost" size="sm" onClick={() => props.onMove(band, entry.name)}>
                    {`Move to ${TITLE[OTHER[band]]}`}
                  </Button>
                  <Button variant="ghost" size="sm" onClick={() => props.onRemove(band, entry.name)}>
                    Remove
                  </Button>
                </span>
              )}
            </Row>
          ))}
        </ul>
      )}
      {locked ? null : (
        <AddLine noun={NOUN[band]} taken={entries.map((entry) => entry.name)} onAdd={(name, run) => props.onAdd(band, name, run)} />
      )}
    </Band>
  );
}

function Band({ title, says, children }: { title: string; says: string; children: ReactNode }) {
  return (
    <section className="armada-proposal-sheet__band" aria-label={title}>
      <h3 className="armada-proposal-sheet__title">{title}</h3>
      <p className="armada-proposal-sheet__says">{says}</p>
      {children}
    </section>
  );
}

function Row({
  name,
  cited,
  faults,
  children,
}: {
  name: string;
  cited: ProvenanceProps;
  faults: { key: string; fault: string }[];
  children: ReactNode;
}) {
  return (
    <li className="armada-proposal-sheet__row" aria-label={name} data-refused={faults.length > 0 || undefined}>
      <span className="armada-proposal-sheet__name">{name}</span>
      {children}
      <Provenance {...cited} />
      {faults.length === 0 ? null : (
        <span className="armada-proposal-sheet__fault">{faults.map((one) => one.fault).join(" ")}</span>
      )}
    </li>
  );
}

/** A value that is text until pressed, then an input at the same height. Enter or leaving it commits. */
function InPlace({
  label,
  value,
  locked,
  onCommit,
}: {
  label: string;
  value: string;
  locked: boolean;
  onCommit: (value: string) => void;
}) {
  const [typed, setTyped] = useState<string | null>(null);
  if (locked) return <span className="armada-proposal-sheet__mono">{value}</span>;
  if (typed === null) {
    return (
      <button type="button" className="armada-proposal-sheet__value" aria-label={`Edit ${label}`} onClick={() => setTyped(value)}>
        {value}
      </button>
    );
  }
  const commit = () => {
    const next = typed.trim();
    setTyped(null);
    if (next !== "" && next !== value) onCommit(next);
  };
  return (
    <Input
      aria-label={label}
      mono
      autoFocus
      value={typed}
      onChange={(event) => setTyped(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => {
        if (event.key === "Enter") commit();
      }}
    />
  );
}

function AddLine({ noun, taken, onAdd }: { noun: string; taken: string[]; onAdd: (name: string, run: string) => void }) {
  const [name, setName] = useState("");
  const [run, setRun] = useState("");
  const clash = taken.includes(name.trim());
  return (
    <div className="armada-proposal-sheet__add" role="group" aria-label={`Add a ${noun}`}>
      <Input
        label={`New ${noun} name`}
        mono
        value={name}
        invalid={clash}
        message={clash ? `A ${noun} named ${name.trim()} is already here.` : undefined}
        onChange={(event) => setName(event.target.value)}
      />
      <Input label="Command" mono value={run} onChange={(event) => setRun(event.target.value)} />
      <Button
        variant="secondary"
        size="sm"
        disabled={name.trim() === "" || run.trim() === "" || clash}
        onClick={() => {
          onAdd(name.trim(), run.trim());
          setName("");
          setRun("");
        }}
      >
        {`Add a ${noun}`}
      </Button>
    </div>
  );
}
