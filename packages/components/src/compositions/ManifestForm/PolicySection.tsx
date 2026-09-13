import { ADVANCE_GATE } from "../../generated/vocabulary";
import { Select } from "../../primitives/Select/Select";

import { Hinted, Section } from "./Entries";
import type { ManifestFormProps } from "./ManifestForm";

/** The registry's words for a gate, with the file's word beside them. */
function gateReads(word: string): string {
  const verb = ADVANCE_GATE[word]?.verb;
  return verb === null || verb === undefined ? word : `${verb.charAt(0).toUpperCase()}${verb.slice(1)} · ${word}`;
}

export function PolicySection({ draft, onDraft, autoMergeWords, reviewGateWords }: ManifestFormProps) {
  return (
    <Section title="Policy" says="Who decides a landing and a review here. Across several Manifests, the most cautious wins.">
      {/* No registry row says `auto_merge`'s values in words yet, so the file's word is shown. */}
      <Hinted hint="checks-pass is the forge's checks, not Armada's, and always means always.">
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
      </Hinted>
      <Select
        label="Review gate"
        value={draft.reviewGate}
        onChange={(event) => onDraft({ ...draft, reviewGate: event.target.value })}
      >
        {reviewGateWords.map((word) => (
          <option key={word} value={word}>
            {gateReads(word)}
          </option>
        ))}
      </Select>
    </Section>
  );
}
