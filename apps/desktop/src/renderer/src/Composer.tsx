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
// Two fields are ids Bridge cannot mint and cannot list. Fleet is the sole
// authority for the ids that name records, and the served operations are
// `list_jobs`, `propose_job`, `approve_dispatch` and the two kills — none of
// which enumerates workflows or Manifests. So the field offers the ids already
// on the board and takes a typed one otherwise, which is carrying an id rather
// than inventing one. Reported as a gap, not designed around.

import { useState } from "react";
import { Button, Card, CardContent, CardFooter, CardHeader, CardTitle, Checkbox, Input, Select } from "@armada/components";

import { URGENCIES } from "../../shared/generated/vocabulary";
import type { Draft } from "../../shared/bridge";

/**
 * A person filling in this form by hand is the `manual` origin. The other three
 * top-level origins are written by something else — the Job proposer, Helm, or
 * a finished workflow — so none of them is a value this form may claim, and
 * `sub_dispatched` does not deserialise at all.
 */
const ORIGIN = "manual";

export type ComposerProps = {
  /** Workflow ids already on the board. The only ones Bridge can offer. */
  workflows: readonly string[];
  manifests: readonly string[];
  /** Nothing may be proposed while the connection is not live. */
  disabled: boolean;
  onPropose: (draft: Draft) => void;
};

export function Composer({ workflows, manifests, disabled, onPropose }: ComposerProps) {
  const [title, setTitle] = useState("");
  const [brief, setBrief] = useState("");
  const [workflowId, setWorkflowId] = useState("");
  const [manifestId, setManifestId] = useState("");
  const [urgency, setUrgency] = useState(URGENCIES[0] ?? "");
  const [atomic, setAtomic] = useState(false);
  const [tried, setTried] = useState(false);

  const emptyTitle = title.trim() === "";
  const emptyBrief = brief.trim() === "";
  const emptyWorkflow = workflowId.trim() === "";
  const emptyManifest = manifestId.trim() === "";
  const refused = emptyTitle || emptyBrief || emptyWorkflow || emptyManifest;

  function propose(): void {
    setTried(true);
    if (refused) return;
    onPropose({
      title: title.trim(),
      workflowId: workflowId.trim(),
      manifestId: manifestId.trim(),
      origin: ORIGIN,
      urgency,
      // The registry's `model` row says M1 varies nothing and takes the value
      // from configuration, with no create-form field and no picker. Bridge
      // cannot read that configuration through any served operation, so it
      // claims nothing rather than inventing a model name.
      model: "",
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
          <Input
            label="Brief"
            value={brief}
            onChange={(event) => setBrief(event.target.value)}
            invalid={tried && emptyBrief}
            message="A job needs a brief. Write what the work is."
          />
          <div className="flex flex-wrap gap-4">
            <IdField
              label="Workflow"
              known={workflows}
              value={workflowId}
              onChange={setWorkflowId}
              invalid={tried && emptyWorkflow}
              message="Fleet mints workflow ids. Pick one on the board, or paste one."
            />
            <IdField
              label="Manifest"
              known={manifests}
              value={manifestId}
              onChange={setManifestId}
              invalid={tried && emptyManifest}
              message="Fleet mints Manifest ids. Pick one on the board, or paste one."
            />
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
 * An id Fleet minted. A select over what is on the board, and a mono field
 * where the board has none — the first Job on a machine has no id to pick from,
 * and a form that cannot be completed at all is worse than one that says where
 * the value comes from.
 */
function IdField(props: {
  label: string;
  known: readonly string[];
  value: string;
  invalid: boolean;
  message: string;
  onChange: (value: string) => void;
}) {
  if (props.known.length === 0) {
    return (
      <Input
        label={props.label}
        mono
        value={props.value}
        onChange={(event) => props.onChange(event.target.value)}
        invalid={props.invalid}
        message={props.message}
      />
    );
  }
  return (
    <Select
      label={props.label}
      value={props.value}
      onChange={(event) => props.onChange(event.target.value)}
      invalid={props.invalid}
      message={props.message}
    >
      <option value="" />
      {props.known.map((id) => (
        <option key={id} value={id}>
          {id}
        </option>
      ))}
    </Select>
  );
}
