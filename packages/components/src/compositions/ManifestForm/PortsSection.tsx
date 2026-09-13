import { Input } from "../../primitives/Input/Input";

import { AddEntry, Entry, Hinted, replaced, Section } from "./Entries";
import type { ManifestFormPort, ManifestFormProps } from "./ManifestForm";

export function PortsSection({ draft, onDraft, problems }: ManifestFormProps) {
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
          <Hinted hint="The name the stack already reads the port from.">
            <Input
              label="Variable"
              mono
              value={port.env}
              onChange={(event) => set(at, { ...port, env: event.target.value })}
            />
          </Hinted>
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
