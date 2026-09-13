import { Input } from "../../primitives/Input/Input";
import { Textarea } from "../../primitives/Textarea/Textarea";

import { Hinted, Section } from "./Entries";
import type { ManifestFormProps } from "./ManifestForm";

export function DroneSection({ draft, onDraft, problems }: ManifestFormProps) {
  return (
    <Section
      title="Drone"
      says="How patient Fleet is with a Drone working here, and what its work stays out of. Empty takes what Fleet runs with."
    >
      <Hinted hint="How long a Drone may say nothing before Fleet pokes it.">
        <Input
          label="Silence threshold, in seconds"
          mono
          inputMode="numeric"
          value={draft.quietAfter}
          invalid={problems["drone.quiet_after_seconds"] !== undefined}
          message={problems["drone.quiet_after_seconds"]}
          onChange={(event) => onDraft({ ...draft, quietAfter: event.target.value })}
        />
      </Hinted>
      <Hinted hint="How many nudges a silent Drone gets before the Job escalates as stalled. 0 escalates at the first silence.">
        <Input
          label="Poke limit"
          mono
          inputMode="numeric"
          value={draft.pokeLimit}
          invalid={problems["drone.poke_limit"] !== undefined}
          message={problems["drone.poke_limit"]}
          onChange={(event) => onDraft({ ...draft, pokeLimit: event.target.value })}
        />
      </Hinted>
      <Hinted hint="One path a line. What a step's work is meant to stay out of; a Judge may lift one when the fix needs it.">
        <Textarea
          label="Scope exclusions"
          value={draft.excludePaths}
          rows={2}
          spellCheck={false}
          onChange={(event) => onDraft({ ...draft, excludePaths: event.target.value })}
        />
      </Hinted>
    </Section>
  );
}
