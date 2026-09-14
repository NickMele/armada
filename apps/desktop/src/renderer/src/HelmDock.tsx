// Helm's conversation zone, under the dock's questions — #944. `App.tsx` only
// wires this in; which repository Helm answers for is main's own decision,
// made in `main/helm.ts`, and this draws whatever it publishes.

import { useState } from "react";
import { HelmComposer, HelmThread } from "@armada/components";
import type {
  HelmApprovalCard,
  HelmApprovalCardState,
  HelmComposerChip,
  HelmThreadRow,
} from "@armada/components";
import type { HelmContext, JobSummary, RepositorySummary, WorkflowSummary } from "@armada/protocol";
import { helmRowsOf, type HelmApprovalAsk, type HelmFoldedRow } from "@armada/screens/src/helm-thread";
import type { BridgeState } from "../../shared/bridge";

export type HelmDockProps = {
  helm: BridgeState["helm"];
  repositories: readonly RepositorySummary[];
  /** Resolved into a card's Job facts — never anything the model itself said. #1041. */
  jobs: readonly JobSummary[];
  workflows: readonly WorkflowSummary[];
  live: boolean;
  /** The open Job's chip, while it stands — `App.tsx`'s own state. #1075. */
  chip?: HelmComposerChip;
  onRemoveChip?: () => void;
  onAsk: (text: string, context: HelmContext) => void;
  /** Where the person is, assembled by `App.tsx`. Sent with every ask, unchanged here. #1075. */
  context: HelmContext;
  onStartFresh: () => void;
  onSwitch: (manifestId: string) => void;
  onApprove: (jobId: string) => void;
};

/**
 * A card's own press, held for this dock's lifetime alone — a reload starts
 * over. #1041.
 *
 * **`"pending"` is Approve pressed and Fleet not yet answered.** `onApprove`
 * is fire-and-forget from here — App.tsx voids the promise `commands.approve`
 * returns — so this has no answer of its own to await; `stateOf` below reads
 * the Job leaving `awaiting_approval` as the signal that it landed. #1117.
 */
type Pressed = "pending" | "approved" | "dismissed";

export function HelmDock({
  helm,
  repositories,
  jobs,
  workflows,
  live,
  chip,
  onRemoveChip,
  onAsk,
  context,
  onStartFresh,
  onSwitch,
  onApprove,
}: HelmDockProps) {
  const [draft, setDraft] = useState("");
  const [pressed, setPressed] = useState<Record<string, Pressed>>({});
  const current = helm.state === "none" ? undefined : helm.manifestId;
  const options = repositories
    .filter((one): one is RepositorySummary & { manifest: NonNullable<RepositorySummary["manifest"]> } =>
      one.manifest !== undefined,
    )
    .map((one) => ({ id: one.manifest.id, label: one.manifest.repository }));
  const folded = helm.state === "open" || helm.state === "failed" ? helmRowsOf(helm.items) : [];
  const rows = folded.map((row) => withCards(row, jobs, workflows, pressed, onApprove, setPressed));
  const replying = helm.state === "open" && helm.replying;

  const notice = !live
    ? "Fleet is not connected. What Helm already said is still here."
    : helm.state === "failed"
      ? helm.detail
      : undefined;
  const emptyNote =
    helm.state === "cleared"
      ? "The conversation is cleared. Ask Helm something to start again."
      : current === undefined
        ? "No repository has a Manifest yet for Helm to answer about."
        : undefined;

  function send(): void {
    const text = draft.trim();
    if (text === "") return;
    onAsk(text, context);
    setDraft("");
  }

  return (
    <div className="armada-helm-dock">
      <HelmThread rows={rows} replying={replying} notice={notice} emptyNote={emptyNote} />
      <HelmComposer
        current={current}
        repositories={options}
        chip={chip}
        onRemoveChip={onRemoveChip}
        // Drawn whenever there is somewhere else to point Helm — a specific
        // pick does not hide it, because Discuss or the switch itself is
        // what points Helm away from the picked repository without moving
        // the picker.
        onSwitch={options.length > 1 ? onSwitch : undefined}
        onStartFresh={onStartFresh}
        startFreshDisabled={replying}
        value={draft}
        onChange={setDraft}
        onSend={send}
        disabled={!live || current === undefined}
      />
    </div>
  );
}

/**
 * One folded row's bare `asks` resolved into drawable cards, off `jobs` and
 * `workflows` — the app's own published state, never anything Helm's tool
 * call said. `#1041`.
 */
function withCards(
  row: HelmFoldedRow,
  jobs: readonly JobSummary[],
  workflows: readonly WorkflowSummary[],
  pressed: Record<string, Pressed>,
  onApprove: (jobId: string) => void,
  setPressed: (update: (was: Record<string, Pressed>) => Record<string, Pressed>) => void,
): HelmThreadRow {
  const { asks, ...rest } = row;
  if (asks === undefined || asks.length === 0) return rest;
  return {
    ...rest,
    cards: asks.map((ask) => cardOf(ask, jobs, workflows, pressed, onApprove, setPressed)),
  };
}

function cardOf(
  ask: HelmApprovalAsk,
  jobs: readonly JobSummary[],
  workflows: readonly WorkflowSummary[],
  pressed: Record<string, Pressed>,
  onApprove: (jobId: string) => void,
  setPressed: (update: (was: Record<string, Pressed>) => Record<string, Pressed>) => void,
): HelmApprovalCard {
  const job = jobs.find((one) => one.id === ask.jobId);
  const workflow = job === undefined ? undefined : workflows.find(matching(job));
  const state = stateOf(ask.id, job, pressed);
  return {
    id: ask.id,
    jobHandle: job?.handle ?? ask.jobId,
    workflow: workflow?.name,
    stepCount: workflow?.steps.length,
    state,
    ...(state === "ready" || state === "pending"
      ? {
          onApprove: () => {
            setPressed((was) => ({ ...was, [ask.id]: "pending" }));
            onApprove(ask.jobId);
          },
          onDismiss: () => setPressed((was) => ({ ...was, [ask.id]: "dismissed" })),
        }
      : {}),
  };
}

function matching(job: JobSummary): (workflow: WorkflowSummary) => boolean {
  return (workflow) => workflow.id === job.workflow_id && workflow.manifest_id === job.owner_manifest_id;
}

/**
 * **A local press wins over the Board once it has been made** — so a card
 * this dock dismissed reads "Dismissed." rather than snapping back to `ready`.
 * `"pending"` is read against the Job rather than returned outright: Approve
 * is fire-and-forget from this dock, so **the Job leaving `awaiting_approval`
 * is what tells this card its press landed**, and until then it keeps waiting
 * rather than claiming "Approved." early. #1117.
 *
 * Absent any press, the Board's own status decides: a Job no longer there, or
 * one this card never moved, reads `elsewhere`; a Job this dock has never
 * heard of reads `unknown`, in words rather than a broken card.
 */
function stateOf(id: string, job: JobSummary | undefined, pressed: Record<string, Pressed>): HelmApprovalCardState {
  const local = pressed[id];
  if (local === "dismissed") return "dismissed";
  if (local === "pending") return job !== undefined && job.status !== "awaiting_approval" ? "approved" : "pending";
  if (job === undefined) return "unknown";
  return job.status === "awaiting_approval" ? "ready" : "elsewhere";
}
