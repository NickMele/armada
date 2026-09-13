import { Input } from "../../primitives/Input/Input";

import { Section } from "./Entries";
import type { ManifestFormProps } from "./ManifestForm";

export function BaseSection({ draft, onDraft }: ManifestFormProps) {
  return (
    <Section
      title="Base"
      says="The branch a Job's work merges into. Empty, Armada infers one from git and says so in the pull request."
    >
      <Input
        label="Base branch"
        mono
        value={draft.base}
        spellCheck={false}
        onChange={(event) => onDraft({ ...draft, base: event.target.value })}
      />
    </Section>
  );
}
