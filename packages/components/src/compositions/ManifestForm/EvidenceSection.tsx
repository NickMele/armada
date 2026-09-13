import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Textarea } from "../../primitives/Textarea/Textarea";

import { Entry, Hinted, Section } from "./Entries";
import type { ManifestFormEvidence, ManifestFormProps } from "./ManifestForm";

const UNDECLARED: ManifestFormEvidence = { serve: "", ready: "", run: "", frames: "", never: "" };

export function EvidenceSection({ draft, onDraft, problems }: ManifestFormProps) {
  const evidence = draft.evidence;
  const set = (next: ManifestFormEvidence | null) => onDraft({ ...draft, evidence: next });
  return (
    <Section
      title="Evidence"
      says="How this repository shows what a change looks like. A step that captures runs one spec with it, and Fleet keeps the frames."
    >
      {evidence === null ? (
        <div>
          <Button variant="secondary" size="sm" onClick={() => set(UNDECLARED)}>
            Declare evidence
          </Button>
        </div>
      ) : (
        <Entry name="evidence" onRemove={() => set(null)}>
          <Hinted hint="{} is where the spec's path goes. There is no shell: to chain commands, name a script.">
            <Input
              label="Runs one spec"
              mono
              value={evidence.run}
              invalid={problems["evidence.run"] !== undefined}
              message={problems["evidence.run"]}
              onChange={(event) => set({ ...evidence, run: event.target.value })}
            />
          </Hinted>
          <Hinted hint="A directory relative to the repository root. Fleet keeps what lands there.">
            <Input
              label="Frames land in"
              mono
              value={evidence.frames}
              invalid={problems["evidence.frames"] !== undefined}
              message={problems["evidence.frames"]}
              onChange={(event) => set({ ...evidence, frames: event.target.value })}
            />
          </Hinted>
          <Hinted hint="Optional: what starts the thing being shown. It goes with Ready when.">
            <Input
              label="Serves"
              mono
              value={evidence.serve}
              onChange={(event) => set({ ...evidence, serve: event.target.value })}
            />
          </Hinted>
          <Hinted hint="A command that exits 0 once it is serving.">
            <Input
              label="Ready when"
              mono
              value={evidence.ready}
              invalid={problems["evidence.ready"] !== undefined}
              message={problems["evidence.ready"]}
              onChange={(event) => set({ ...evidence, ready: event.target.value })}
            />
          </Hinted>
          <Hinted hint="One path a line. Armada does not enforce it: whoever writes the spec reads it, and so does whoever reviews it.">
            <Textarea
              label="Never visits"
              value={evidence.never}
              rows={2}
              spellCheck={false}
              onChange={(event) => set({ ...evidence, never: event.target.value })}
            />
          </Hinted>
        </Entry>
      )}
    </Section>
  );
}
