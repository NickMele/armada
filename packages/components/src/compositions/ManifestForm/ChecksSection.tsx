import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Input } from "../../primitives/Input/Input";
import { Switch } from "../../primitives/Switch/Switch";
import { Textarea } from "../../primitives/Textarea/Textarea";

import { AddEntry, Entry, Hinted, replaced, Section } from "./Entries";
import type { ManifestFormCheck, ManifestFormProps } from "./ManifestForm";

export function ChecksSection({ draft, onDraft, problems }: ManifestFormProps) {
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
            <Hinted hint="Empty is 0.">
              <Input
                label="Passes on exit code"
                mono
                inputMode="numeric"
                value={check.expectExitCode}
                invalid={problems[`checks.${check.name}.expect_exit_code`] !== undefined}
                message={problems[`checks.${check.name}.expect_exit_code`]}
                onChange={(event) => set(at, { ...check, expectExitCode: event.target.value })}
              />
            </Hinted>
            <Hinted hint="One path pattern a line. Empty runs it on every change.">
              <Textarea
                label="Runs when a change touches"
                value={check.when}
                rows={2}
                spellCheck={false}
                onChange={(event) => set(at, { ...check, when: event.target.value })}
              />
            </Hinted>
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
                <Hinted hint="{} is the path, as in -p {}.">
                  <Input
                    label="Each changed path becomes"
                    mono
                    value={narrow.each}
                    onChange={(event) => set(at, { ...check, narrow: { ...narrow, each: event.target.value } })}
                  />
                </Hinted>
                <Hinted hint="The directory whose entries a changed path is read as. Empty takes the path itself.">
                  <Input
                    label="Paths under"
                    mono
                    value={narrow.under}
                    onChange={(event) => set(at, { ...check, narrow: { ...narrow, under: event.target.value } })}
                  />
                </Hinted>
                <Hinted hint="One path pattern a line. Empty takes every changed path.">
                  <Textarea
                    label="Only paths matching"
                    value={narrow.from}
                    rows={2}
                    spellCheck={false}
                    onChange={(event) => set(at, { ...check, narrow: { ...narrow, from: event.target.value } })}
                  />
                </Hinted>
                <Hinted hint="One name a line, left out of the narrowed run.">
                  <Textarea
                    label="Never"
                    value={narrow.except}
                    rows={2}
                    spellCheck={false}
                    onChange={(event) => set(at, { ...check, narrow: { ...narrow, except: event.target.value } })}
                  />
                </Hinted>
              </div>
            )}
          </Entry>
        );
      })}
      <AddEntry
        noun="Check"
        taken={draft.checks.map((check) => check.name)}
        onAdd={(name) =>
          onDraft({ ...draft, checks: [...draft.checks, { name, run: "", expectExitCode: "", requires: [], when: "", narrow: null }] })
        }
      />
    </Section>
  );
}
