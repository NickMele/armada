import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Switch } from "../../primitives/Switch/Switch";

import { AddEntry, Entry, replaced, Section } from "./Entries";
import type { ManifestFormCommand, ManifestFormProps } from "./ManifestForm";

export function CommandsSection({ draft, onDraft, problems }: ManifestFormProps) {
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
