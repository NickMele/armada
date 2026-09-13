import { Select } from "../../primitives/Select/Select";

import { Section } from "./Entries";
import type { ManifestFormProps } from "./ManifestForm";

export function PolicySection({ draft, onDraft, autoMergeWords, reviewGateWords }: ManifestFormProps) {
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
