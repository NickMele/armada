// The create form. A Job drafted onto the approval gate, and nothing else —
// what comes back is a Job at `awaiting_approval`, never a running one.
//
// **An empty title and an empty brief are refused here and again in the main
// process.** The check here makes the refusal legible; the one behind it is the
// rule. A title is typed for now; the Job proposer will generate one, from the
// same call that decides the workflow and the write targets.
//
// # The id fields were text boxes, and that was half a bug
//
// Nothing served the workflows or the Manifests, so this offered whatever ids
// were on the board and a typed one otherwise — and a pasted id was accepted by
// Fleet unchecked, which is how a Job claimed a workflow that does not exist.
// Both halves are closed: Fleet refuses an id it does not hold, and
// `list_workflows` says what the workflows are, so that field is a picker over
// what Fleet will accept, showing the name rather than the ULID.
//
// **The project is not a picker.** The drawn composer makes it read-only:
// Bridge dispatches into the workspace it is pointed at, and a disabled select
// is a control that looks choosable and is not. The rail names that workspace.
//
// **The model is a picker, against `job-fields.toml`.** That file says M1 has
// no create-form field for it; it is a scope note written before any of this
// existed and the owner decided otherwise. A proposal naming no model still
// gets the configured default at the Fleet boundary.
//
// # The workflow picker said how many steps and not what they do
//
// `bug — 4 steps` is a count, and this is the surface where a dispatch is
// agreed to: after the fact the rail says what happened, and here it says what
// is being agreed to. A workflow that will halt and wait for a person, and will
// spend two Judge calls before it gets there, previewed as neither.
//
// So the picked workflow draws the rail it will become, from `preview.ts`. It
// is the running rail's own component and its own words — a second treatment
// for the same declarations would be two vocabularies for one sentence read at
// two moments.

import { useEffect, useRef, useState } from "react";
import type { ChangeEvent, ClipboardEvent } from "react";
import {
  AttachmentChip,
  Button,
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
  Checkbox,
  Input,
  Select,
  Textarea,
  WorkflowRail,
} from "@armada/components";

import { URGENCIES } from "../../shared/generated/vocabulary";
import type { Draft } from "../../shared/bridge";
import type { ManifestSummary, ModelChoices, WorkflowSummary } from "../../shared/setup";
import { previewOf } from "./preview";

/** One staged file, as the composer holds it before `propose()` sends it on. */
type StagedAttachment = { path: string; filename: string; mimeType: string };

/**
 * What the preview is, above it.
 *
 * **A promise and not a record**, which is the one thing this rail has to say
 * that the running one does not: every row is what the workflow declares it
 * will do, and no Job exists to have done any of it yet.
 */
const PREVIEW = "What this workflow will do, step by step";

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
  /** The Manifest the rail names. A fact here, not a choice. */
  manifest: ManifestSummary | undefined;
  /** The models, and the configured default the field starts on. */
  models: ModelChoices | null;
  /** Nothing may be proposed while the connection is not live. */
  disabled: boolean;
  onPropose: (draft: Draft) => void;
};

export function Composer({ workflows, manifest, models, disabled, onPropose }: ComposerProps) {
  const [title, setTitle] = useState("");
  const [brief, setBrief] = useState("");
  const [workflowId, setWorkflowId] = useState("");
  const [model, setModel] = useState("");
  const [urgency, setUrgency] = useState(URGENCIES[0] ?? "");
  const [atomic, setAtomic] = useState(false);
  const [tried, setTried] = useState(false);
  const [attachments, setAttachments] = useState<StagedAttachment[]>([]);
  // The hidden file input the "Attach" button clicks through. A ref rather
  // than state because nothing here reads its value; `onChange` does.
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Selected as soon as the connection answers, and only while nothing has been
  // chosen — a person who picked something keeps it when the roster is re-read.
  // With one of each, which is M1, this means the form is complete on arrival.
  useEffect(() => {
    if (workflowId === "" && workflows.length > 0) setWorkflowId(workflows[0]!.id);
  }, [workflows, workflowId]);
  useEffect(() => {
    if (model === "" && models !== null) setModel(models.default);
  }, [models, model]);

  // The picked workflow, drawn as the rail it will become. Empty until one is
  // picked, and empty for a roster that has not arrived.
  const preview = previewOf(workflows.find((held) => held.id === workflowId));

  const manifestId = manifest?.id ?? "";
  const emptyTitle = title.trim() === "";
  const emptyBrief = brief.trim() === "";
  const emptyWorkflow = workflowId === "";
  const emptyManifest = manifestId === "";
  // The field cannot be left empty: a Job cannot reach dispatch with no model,
  // and the refusal is the picker's job as well as Fleet's.
  const emptyModel = model === "";
  const refused = emptyTitle || emptyBrief || emptyWorkflow || emptyManifest || emptyModel;

  /**
   * One file staged and appended to `attachments`. Shared by the picker and
   * a pasted screenshot — both hand this the same three facts and differ
   * only in where the bytes came from. Staged before the Job exists, because
   * there is no Job id yet to key storage on; `propose()` carries the path.
   */
  async function stage(file: File): Promise<void> {
    const bytes = await file.arrayBuffer();
    const { path } = await window.armada.stageAttachment(bytes, file.name, file.type);
    setAttachments((current) => [...current, { path, filename: file.name, mimeType: file.type }]);
  }

  function onFilesPicked(event: ChangeEvent<HTMLInputElement>): void {
    const files = event.target.files;
    if (files !== null) for (const file of Array.from(files)) void stage(file);
    // Cleared so picking the same file again still fires `onChange`.
    event.target.value = "";
  }

  /**
   * A screenshot pasted straight into the Brief field, without a trip to the
   * file picker — the case a bug brief most needs. `clipboardData.items`
   * carries every kind a paste can hold; only image entries are staged here,
   * and plain text still falls through to the field as text.
   */
  function onBriefPaste(event: ClipboardEvent<HTMLTextAreaElement>): void {
    for (const item of Array.from(event.clipboardData.items)) {
      if (!item.type.startsWith("image/")) continue;
      const file = item.getAsFile();
      if (file !== null) void stage(file);
    }
  }

  function removeAttachment(path: string): void {
    setAttachments((current) => current.filter((attachment) => attachment.path !== path));
  }

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
      attachments,
    });
    setTitle("");
    setBrief("");
    setAttachments([]);
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
            onPaste={onBriefPaste}
            invalid={tried && emptyBrief}
            message="A job needs a brief. Write what the work is."
          />
          {/* Hidden behind the "Attach" button — no file input is ever drawn
              directly, the platform's own picker chrome is not this app's to
              style. */}
          <input
            ref={fileInputRef}
            type="file"
            multiple
            className="hidden"
            onChange={onFilesPicked}
          />
          <div className="flex flex-col gap-2">
            <Button
              variant="secondary"
              size="sm"
              onClick={() => fileInputRef.current?.click()}
            >
              Attach
            </Button>
            {attachments.length > 0 && (
              <div className="flex flex-wrap gap-2">
                {attachments.map((attachment) => (
                  <AttachmentChip
                    key={attachment.path}
                    filename={attachment.filename}
                    onRemove={() => removeAttachment(attachment.path)}
                  />
                ))}
              </div>
            )}
          </div>
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
            {/* Read-only, so it is a labelled value rather than a field —
                the drawing's own treatment. The repository, because
                `armada.yml` declares no name. */}
            <div className="flex flex-col gap-1">
              <span className="text-2xs text-fg-muted">Project</span>
              {manifest === undefined ? null : (
                <span className="mono text-fg-default">{manifest.repository}</span>
              )}
              <span className="text-2xs text-fg-muted">
                {held(manifest === undefined ? 0 : 1, "manifest")}
              </span>
            </div>
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
          {/* What the picked workflow will do, before it does any of it. The
              rail a running Job draws, one moment earlier — see `preview.ts`
              for why no row here carries a result. */}
          {preview.length === 0 ? null : (
            <div className="flex flex-col gap-1">
              <span className="text-2xs text-fg-muted">{PREVIEW}</span>
              <WorkflowRail steps={preview} />
            </div>
          )}
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
