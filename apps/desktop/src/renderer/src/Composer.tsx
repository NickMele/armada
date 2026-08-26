// The create form. A Job drafted onto the approval gate, and nothing else —
// what comes back is a Job at `awaiting_approval`, never a running one.
//
// **An empty title and an empty brief are both refused before the Job is
// created**, here and again in the main process. The check here is what makes
// the refusal legible; the one behind it is the rule.
//
// A title is typed for now. The Job proposer will generate one from the
// description, which is the same call that decides the workflow and the write
// targets — hand entry stays the override rather than the path.
//
// # The two id fields were text boxes, and that was half a bug
//
// Nothing served the workflows or the Manifests, so this form offered whatever
// ids happened to be on the board and a typed one otherwise. A pasted id was
// then accepted by Fleet unchecked, which is how a Job ended up on the board
// claiming a workflow that does not exist. Both halves are closed: Fleet
// refuses an id it does not hold, and `list_workflows` and `list_manifests` say
// what those are — so these are pickers over what Fleet will accept, and the
// value shown is the name rather than the ULID.
//
// # And the model is a picker now
//
// `job-fields.toml` says "At M1 nothing varies it — no create-form field, no
// picker; the value comes from configuration". That was a scope note written
// before any of this existed and the owner has decided otherwise. It starts on
// the configured default, so the common path is still one click, and a
// proposal that names nothing still gets that default at the Fleet boundary —
// the picker is the convenience, not the rule.

import { useEffect, useState } from "react";
import { Button, Card, CardContent, CardFooter, CardHeader, CardTitle, Checkbox, Input, Select, Textarea } from "@armada/components";

import { URGENCIES } from "../../shared/generated/vocabulary";
import type { Draft } from "../../shared/bridge";
import type { ManifestSummary, ModelChoices, WorkflowSummary } from "../../shared/protocol";

/**
 * A person filling in this form by hand is the `manual` origin. The other three
 * top-level origins are written by something else — the Job proposer, Helm, or
 * a finished workflow — so none of them is a value this form may claim, and
 * `sub_dispatched` does not deserialise at all.
 */
const ORIGIN = "manual";

export type ComposerProps = {
  /** The workflows Fleet holds. **The only ones it will accept.** */
  workflows: readonly WorkflowSummary[];
  manifests: readonly ManifestSummary[];
  /** The models, and the configured default the field starts on. */
  models: ModelChoices | null;
  /** Nothing may be proposed while the connection is not live. */
  disabled: boolean;
  onPropose: (draft: Draft) => void;
};

export function Composer({ workflows, manifests, models, disabled, onPropose }: ComposerProps) {
  const [title, setTitle] = useState("");
  const [brief, setBrief] = useState("");
  const [workflowId, setWorkflowId] = useState("");
  const [manifestId, setManifestId] = useState("");
  const [model, setModel] = useState("");
  const [urgency, setUrgency] = useState(URGENCIES[0] ?? "");
  const [atomic, setAtomic] = useState(false);
  const [tried, setTried] = useState(false);

  // Selected as soon as the connection answers, and only while nothing has been
  // chosen — a person who picked something keeps it when the roster is re-read.
  // With one of each, which is M1, this means the form is complete on arrival.
  useEffect(() => {
    if (workflowId === "" && workflows.length > 0) setWorkflowId(workflows[0]!.id);
  }, [workflows, workflowId]);
  useEffect(() => {
    if (manifestId === "" && manifests.length > 0) setManifestId(manifests[0]!.id);
  }, [manifests, manifestId]);
  useEffect(() => {
    if (model === "" && models !== null) setModel(models.default);
  }, [models, model]);

  const emptyTitle = title.trim() === "";
  const emptyBrief = brief.trim() === "";
  const emptyWorkflow = workflowId === "";
  const emptyManifest = manifestId === "";
  // The field cannot be left empty: a Job cannot reach dispatch with no model,
  // and the refusal is the picker's job as well as Fleet's.
  const emptyModel = model === "";
  const refused = emptyTitle || emptyBrief || emptyWorkflow || emptyManifest || emptyModel;

  function propose(): void {
    setTried(true);
    if (refused) return;
    onPropose({
      title: title.trim(),
      workflowId,
      manifestId,
      origin: ORIGIN,
      urgency,
      model,
      atomic,
      brief,
    });
    setTitle("");
    setBrief("");
    setTried(false);
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Propose a job</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex flex-col gap-4">
          <Input
            label="Title"
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            invalid={tried && emptyTitle}
            message="A job needs a title. It is what names the row in the list."
          />
          <Textarea
            label="Brief"
            value={brief}
            onChange={(event) => setBrief(event.target.value)}
            invalid={tried && emptyBrief}
            message="A job needs a brief. Write what the work is."
          />
          <div className="flex flex-wrap gap-4">
            <Select
              label="Workflow"
              value={workflowId}
              onChange={(event) => setWorkflowId(event.target.value)}
              invalid={tried && emptyWorkflow}
              message={held(workflows.length, "workflow")}
            >
              <option value="" />
              {workflows.map((workflow) => (
                <option key={workflow.id} value={workflow.id}>
                  {`${workflow.name} — ${workflow.steps.length} steps`}
                </option>
              ))}
            </Select>
            <Select
              label="Manifest"
              value={manifestId}
              onChange={(event) => setManifestId(event.target.value)}
              invalid={tried && emptyManifest}
              message={held(manifests.length, "manifest")}
            >
              <option value="" />
              {manifests.map((manifest) => (
                <option key={manifest.id} value={manifest.id}>
                  {/* The repository, because `armada.yml` declares no name. */}
                  {manifest.repository}
                </option>
              ))}
            </Select>
            <Select
              label="Model"
              value={model}
              onChange={(event) => setModel(event.target.value)}
              invalid={tried && emptyModel}
              message="A job needs a model. It is what the drone is spawned as."
            >
              <option value="" />
              {(models?.models ?? []).map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
            </Select>
            <Select
              label="Urgency"
              value={urgency}
              onChange={(event) => setUrgency(event.target.value)}
            >
              {URGENCIES.map((value) => (
                <option key={value} value={value}>
                  {value}
                </option>
              ))}
            </Select>
          </div>
          <Checkbox checked={atomic} onChange={(event) => setAtomic(event.target.checked)}>
            The write targets land as one unit
          </Checkbox>
        </div>
      </CardContent>
      <CardFooter>
        <Button variant="primary" onClick={propose} disabled={disabled}>
          Propose
        </Button>
      </CardFooter>
    </Card>
  );
}

/**
 * What an empty picker says. **A form that cannot be completed says why**, and
 * the why is a fact about the connection rather than about the person: Fleet
 * holds one of each and Bridge has not read them yet, or Fleet is not there.
 */
function held(count: number, what: string): string {
  return count === 0
    ? `Fleet has named no ${what}. Nothing can be proposed until it does.`
    : `The ${what}s Fleet holds. It refuses any other.`;
}
