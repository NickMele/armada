import { Switch } from "../../primitives/Switch/Switch";

import { Section } from "./Entries";
import type { ManifestFormProps } from "./ManifestForm";

/** `freeze`. First, because it is the one key a person reaches for in a hurry and must find again to undo. */
export function FreezeSection({ draft, onDraft }: ManifestFormProps) {
  return (
    <Section title="Freeze" says="Nothing lands in this repository while it is frozen. Work waits where it is, and none of it is lost.">
      <Switch
        checked={draft.freeze}
        description="New work waits, a running Job waits before its next step, and nothing merges or is sent out. What you approve, restart or merge is taken and carries on when the freeze lifts. A merge pressed while frozen is pressed again if Fleet restarts first."
        onChange={(event) => onDraft({ ...draft, freeze: event.target.checked })}
      >
        Freeze this repository
      </Switch>
    </Section>
  );
}
