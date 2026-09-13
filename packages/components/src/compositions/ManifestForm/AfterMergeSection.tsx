import { Choices, Section } from "./Entries";
import type { ManifestFormProps } from "./ManifestForm";

export function AfterMergeSection({ draft, onDraft }: ManifestFormProps) {
  // A Check that runs a Command first would write in the checkout a person works in.
  const prepares = draft.checks.filter((check) => check.requires.length > 0).map((check) => check.name);
  // A name no longer declared stays, ticked, so it can be cleared; Fleet refuses it otherwise.
  const declared = draft.checks.map((check) => check.name);
  const offered = [
    ...declared.filter((name) => !prepares.includes(name) || draft.afterMerge.includes(name)),
    ...draft.afterMerge.filter((name) => !declared.includes(name)),
  ];
  return (
    <Section
      title="Proved after merge"
      says="Checks run once against what a merge left on the base. None run unless named here, and nothing waits on the answer."
    >
      <Choices
        label="Runs after a merge"
        offered={offered}
        chosen={draft.afterMerge}
        none="This file declares no Check that can run here."
        onChosen={(afterMerge) => onDraft({ ...draft, afterMerge })}
      />
      {prepares.length === 0 ? null : (
        <p className="armada-manifest-form__hint">
          {prepares.length === 1
            ? `${prepares[0]} runs a Command first, which would write in the checkout a person works in, so it is not offered.`
            : `${prepares.join(", ")} run a Command first, which would write in the checkout a person works in, so they are not offered.`}
        </p>
      )}
    </Section>
  );
}
