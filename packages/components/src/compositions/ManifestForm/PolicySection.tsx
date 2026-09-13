import { ADVANCE_GATE, AUTO_MERGE, type Rendering } from "../../generated/vocabulary";
import { Select } from "../../primitives/Select/Select";

import { Section } from "./Entries";
import type { ManifestFormProps } from "./ManifestForm";

/** The registry's words for a policy value, with the file's word beside them. */
function reads(vocabulary: Readonly<Record<string, Rendering | undefined>>, word: string): string {
  const verb = vocabulary[word]?.verb;
  return verb === null || verb === undefined ? word : `${verb.charAt(0).toUpperCase()}${verb.slice(1)} · ${word}`;
}

export function PolicySection({ draft, onDraft, autoMergeWords, reviewGateWords }: ManifestFormProps) {
  const consequence = AUTO_MERGE[draft.autoMerge]?.hint;
  return (
    <Section title="Policy" says="Who decides a landing and a review here. Across several Manifests, the most cautious wins.">
      <div className="armada-manifest-form__hinted">
        <Select
          label="Auto merge"
          value={draft.autoMerge}
          onChange={(event) => onDraft({ ...draft, autoMerge: event.target.value })}
        >
          {autoMergeWords.map((word) => (
            <option key={word} value={word}>
              {reads(AUTO_MERGE, word)}
            </option>
          ))}
        </Select>
        {consequence === null || consequence === undefined ? null : (
          <p className="armada-manifest-form__hint">{consequence}</p>
        )}
      </div>
      <Select
        label="Review gate"
        value={draft.reviewGate}
        onChange={(event) => onDraft({ ...draft, reviewGate: event.target.value })}
      >
        {reviewGateWords.map((word) => (
          <option key={word} value={word}>
            {reads(ADVANCE_GATE, word)}
          </option>
        ))}
      </Select>
    </Section>
  );
}
