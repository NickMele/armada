import { useId, useState, type ReactNode } from "react";

import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Input } from "../../primitives/Input/Input";
import { Select } from "../../primitives/Select/Select";
import { Switch } from "../../primitives/Switch/Switch";
import { Textarea } from "../../primitives/Textarea/Textarea";

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

      <ChecksSection {...props} />
      <CommandsSection {...props} />
      <PortsSection {...props} />
      <PolicySection {...props} />
      <BudgetSection {...props} />
    </div>
  );
}

function Section({ title, says, children }: { title: string; says: string; children: ReactNode }) {
  return (
    <section className="armada-manifest-form__section" aria-label={title}>
      <h2 className="armada-manifest-form__title">{title}</h2>
      <p className="armada-manifest-form__says">{says}</p>
      {children}
    </section>
  );
}

/**
 * One named entry, grouped under its name, with Remove beside it. A removal
 * takes the entry's lines and the comment directly above it — the writer's.
 */
function Entry({ name, onRemove, children }: { name: string; onRemove: () => void; children: ReactNode }) {
  const id = useId();
  return (
    <fieldset className="armada-manifest-form__entry" aria-labelledby={id}>
      <div className="armada-manifest-form__entry-head">
        <span id={id} className="armada-manifest-form__name">
          {name}
        </span>
        <Button variant="ghost" size="sm" onClick={onRemove}>
          Remove
        </Button>
      </div>
      {children}
    </fieldset>
  );
}

/** A name, and the press that declares an entry by it. Taken names are refused here. */
function AddEntry({ noun, taken, onAdd }: { noun: string; taken: string[]; onAdd: (name: string) => void }) {
  const [typed, setTyped] = useState("");
  const name = typed.trim();
  const clash = taken.includes(name);
  return (
    <div className="armada-manifest-form__add">
      <Input
        label={`New ${noun} name`}
        mono
        value={typed}
        invalid={clash}
        message={clash ? `This file already declares ${name}.` : undefined}
        onChange={(event) => setTyped(event.target.value)}
      />
      <Button
        variant="secondary"
        size="sm"
        disabled={name === "" || clash}
        onClick={() => {
          onAdd(name);
          setTyped("");
        }}
      >
        {`Add a ${noun}`}
      </Button>
    </div>
  );
}

function replaced<T>(list: readonly T[], at: number, next: T): T[] {
  return list.map((one, index) => (index === at ? next : one));
}

function ChecksSection({ draft, onDraft, problems }: ManifestFormProps) {
  const set = (at: number, next: ManifestFormCheck) => onDraft({ ...draft, checks: replaced(draft.checks, at, next) });
  // What `requires` may name: a Command that runs and exits, never a server.
  const runnable = draft.commands.filter((command) => command.serve.trim() === "").map((command) => command.name);
  return (
    <Section title="Checks" says="What a change must pass. The gate starts them in the order the file writes them.">
      {draft.checks.map((check, at) => {
        const named = [...runnable, ...check.requires.filter((name) => !runnable.includes(name))];
        const narrow = check.narrow;
        return (
          <Entry
            key={check.name}
            name={check.name}
            onRemove={() => onDraft({ ...draft, checks: draft.checks.filter((_, index) => index !== at) })}
          >
            <Input
              label="Command"
              mono
              value={check.run}
              invalid={problems[`checks.${check.name}.run`] !== undefined}
              message={problems[`checks.${check.name}.run`]}
              onChange={(event) => set(at, { ...check, run: event.target.value })}
            />
            <Textarea
              label="Runs when a change touches"
              message="One path pattern a line. Empty runs it on every change."
              value={check.when}
              spellCheck={false}
              onChange={(event) => set(at, { ...check, when: event.target.value })}
            />
            {named.length === 0 ? null : (
              <div className="armada-manifest-form__choices" role="group" aria-label="Runs first">
                <span className="armada-manifest-form__label">Runs first</span>
                {named.map((name) => (
                  <Checkbox
                    key={name}
                    checked={check.requires.includes(name)}
                    onChange={(event) =>
                      set(at, {
                        ...check,
                        requires: event.target.checked
                          ? [...check.requires, name]
                          : check.requires.filter((one) => one !== name),
                      })
                    }
                  >
                    {name}
                  </Checkbox>
                ))}
              </div>
            )}
            <Switch
              checked={narrow !== null}
              description="What a Drone asking about its own change runs instead. The gate still runs the whole Check."
              onChange={(event) =>
                set(at, {
                  ...check,
                  narrow: event.target.checked ? { run: "", each: "", from: "", under: "", except: "" } : null,
                })
              }
            >
              Narrow to what changed
            </Switch>
            {narrow === null ? null : (
              <div className="armada-manifest-form__nested">
                {problems[`checks.${check.name}.narrow`] === undefined ? null : (
                  <p className="armada-manifest-form__problem">{problems[`checks.${check.name}.narrow`]}</p>
                )}
                <Input
                  label="Narrowed command"
                  mono
                  value={narrow.run}
                  onChange={(event) => set(at, { ...check, narrow: { ...narrow, run: event.target.value } })}
                />
                <Input
                  label="Each changed path becomes"
                  mono
                  message="{} is the path, as in -p {}."
                  value={narrow.each}
                  onChange={(event) => set(at, { ...check, narrow: { ...narrow, each: event.target.value } })}
                />
                <Input
                  label="Paths under"
                  mono
                  message="The directory whose entries a changed path is read as. Empty takes the path itself."
                  value={narrow.under}
                  onChange={(event) => set(at, { ...check, narrow: { ...narrow, under: event.target.value } })}
                />
                <Textarea
                  label="Only paths matching"
                  message="One path pattern a line. Empty takes every changed path."
                  value={narrow.from}
                  spellCheck={false}
                  onChange={(event) => set(at, { ...check, narrow: { ...narrow, from: event.target.value } })}
                />
                <Textarea
                  label="Never"
                  message="One name a line, left out of the narrowed run."
                  value={narrow.except}
                  spellCheck={false}
                  onChange={(event) => set(at, { ...check, narrow: { ...narrow, except: event.target.value } })}
                />
              </div>
            )}
          </Entry>
        );
      })}
      <AddEntry
        noun="Check"
        taken={draft.checks.map((check) => check.name)}
        onAdd={(name) =>
          onDraft({ ...draft, checks: [...draft.checks, { name, run: "", requires: [], when: "", narrow: null }] })
        }
      />
    </Section>
  );
}

function CommandsSection({ draft, onDraft, problems }: ManifestFormProps) {
  const set = (at: number, next: ManifestFormCommand) =>
    onDraft({ ...draft, commands: replaced(draft.commands, at, next) });
  return (
    <Section title="Commands" says="What a Drone may be given to run. They gate nothing.">
      {draft.commands.map((command, at) => {
        const server = command.serve.trim() !== "";
        return (
          <Entry
            key={command.name}
            name={command.name}
            onRemove={() => onDraft({ ...draft, commands: draft.commands.filter((_, index) => index !== at) })}
          >
            <Input
              label={server ? "Runs first" : "Command"}
              mono
              value={command.run}
              invalid={problems[`commands.${command.name}.run`] !== undefined}
              message={problems[`commands.${command.name}.run`]}
              onChange={(event) => set(at, { ...command, run: event.target.value })}
            />
            <Switch
              checked={command.destructive}
              description="A Drone asks before running it. This is the only place it is set: nothing Armada reads can tell."
              onChange={(event) => set(at, { ...command, destructive: event.target.checked })}
            >
              Destructive
            </Switch>
            <Input
              label="Serves"
              mono
              message="A command that keeps running makes this a server."
              value={command.serve}
              onChange={(event) => set(at, { ...command, serve: event.target.value })}
            />
            {!server ? null : (
              <div className="armada-manifest-form__nested">
                <Input
                  label="Ready when"
                  mono
                  message="A command that exits 0 once the server answers."
                  value={command.ready}
                  onChange={(event) => set(at, { ...command, ready: event.target.value })}
                />
                {problems[`commands.${command.name}.links`] === undefined ? null : (
                  <p className="armada-manifest-form__problem">{problems[`commands.${command.name}.links`]}</p>
                )}
                {command.links.map((link, index) => (
                  <div key={index} className="armada-manifest-form__link" role="group" aria-label={`Link ${index + 1}`}>
                    <Input
                      label="Address"
                      mono
                      value={link.url}
                      onChange={(event) =>
                        set(at, { ...command, links: replaced(command.links, index, { ...link, url: event.target.value }) })
                      }
                    />
                    <Input
                      label="Called"
                      value={link.name}
                      onChange={(event) =>
                        set(at, { ...command, links: replaced(command.links, index, { ...link, name: event.target.value }) })
                      }
                    />
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => set(at, { ...command, links: command.links.filter((_, one) => one !== index) })}
                    >
                      Remove link
                    </Button>
                  </div>
                ))}
                <div>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => set(at, { ...command, links: [...command.links, { url: "", name: "" }] })}
                  >
                    Add a link
                  </Button>
                </div>
              </div>
            )}
          </Entry>
        );
      })}
      <AddEntry
        noun="Command"
        taken={draft.commands.map((command) => command.name)}
        onAdd={(name) =>
          onDraft({
            ...draft,
            commands: [...draft.commands, { name, run: "", destructive: false, serve: "", ready: "", links: [] }],
          })
        }
      />
    </Section>
  );
}

function PortsSection({ draft, onDraft, problems }: ManifestFormProps) {
  const set = (at: number, next: ManifestFormPort) => onDraft({ ...draft, ports: replaced(draft.ports, at, next) });
  return (
    <Section title="Ports" says="Ports this repository's stack expects. Fleet places the number; nothing here picks one.">
      {draft.ports.map((port, at) => (
        <Entry
          key={port.name}
          name={port.name}
          onRemove={() => onDraft({ ...draft, ports: draft.ports.filter((_, index) => index !== at) })}
        >
          <Input
            label="Container port"
            mono
            inputMode="numeric"
            value={port.container}
            invalid={problems[`ports.${port.name}.container`] !== undefined}
            message={problems[`ports.${port.name}.container`]}
            onChange={(event) => set(at, { ...port, container: event.target.value })}
          />
          <Input
            label="Variable"
            mono
            message="The name the stack already reads the port from."
            value={port.env}
            onChange={(event) => set(at, { ...port, env: event.target.value })}
          />
        </Entry>
      ))}
      <AddEntry
        noun="port"
        taken={draft.ports.map((port) => port.name)}
        onAdd={(name) => onDraft({ ...draft, ports: [...draft.ports, { name, container: "", env: "" }] })}
      />
    </Section>
  );
}

function PolicySection({ draft, onDraft, autoMergeWords, reviewGateWords }: ManifestFormProps) {
  return (
    <Section title="Policy" says="Who decides a landing and a review here. Across several Manifests, the most cautious wins.">
      <Select
        label="Auto merge"
        value={draft.autoMerge}
        onChange={(event) => onDraft({ ...draft, autoMerge: event.target.value })}
      >
        {autoMergeWords.map((word) => (
          <option key={word} value={word}>
            {word}
          </option>
        ))}
      </Select>
      <Select
        label="Review gate"
        value={draft.reviewGate}
        onChange={(event) => onDraft({ ...draft, reviewGate: event.target.value })}
      >
        {reviewGateWords.map((word) => (
          <option key={word} value={word}>
            {word}
          </option>
        ))}
      </Select>
    </Section>
  );
}

function BudgetSection({ draft, onDraft, problems, budgetWarnings }: ManifestFormProps) {
  return (
    <Section title="Budget" says="What one Job here may spend. Empty takes what Fleet runs with.">
      <Input
        label="Cost cap per Job, in dollars"
        mono
        inputMode="decimal"
        value={draft.costCap}
        invalid={problems["budget.cost"] !== undefined}
        message={problems["budget.cost"]}
        onChange={(event) => onDraft({ ...draft, costCap: event.target.value })}
      />
      <Input
        label="Turn cap per Job"
        mono
        inputMode="numeric"
        value={draft.turnCap}
        invalid={problems["budget.turns"] !== undefined}
        message={problems["budget.turns"]}
        onChange={(event) => onDraft({ ...draft, turnCap: event.target.value })}
      />
      {budgetWarnings.map((warning) => (
        <Alert key={warning} tone="caution" title="Below what a past Job cost">
          {warning}
        </Alert>
      ))}
    </Section>
  );
}
