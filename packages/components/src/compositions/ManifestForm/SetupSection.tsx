import { Choices, Section } from "./Entries";
import type { ManifestFormProps } from "./ManifestForm";

export function SetupSection({ draft, onDraft }: ManifestFormProps) {
  // A server never finishes, so nothing could start after it.
  const runnable = draft.commands.filter((command) => command.serve.trim() === "").map((command) => command.name);
  return (
    <Section
      title="Setup"
      says="Commands run once where a worktree is cut, before any step starts. A Drone is never given them."
    >
      <Choices
        label="Runs before any step"
        offered={[...runnable, ...draft.setup.filter((name) => !runnable.includes(name))]}
        chosen={draft.setup}
        none="This file declares no Command that runs and exits."
        onChosen={(setup) => onDraft({ ...draft, setup })}
      />
    </Section>
  );
}
