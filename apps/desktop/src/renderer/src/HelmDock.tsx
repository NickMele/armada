// Helm's conversation zone, under the dock's questions — #944. `App.tsx` only
// wires this in; which repository Helm answers for is main's own decision,
// made in `main/helm.ts`, and this draws whatever it publishes.
//
// It carries Helm's own permission asks too, since #1519: a Drone's question
// and a Judge's stay in the block above, and an ask that stopped this
// conversation is drawn in it.

import { useState } from "react";
import { copyHelmRecord, DockQuestions, HelmComposer, HelmRecord, HelmThread } from "@armada/components";
import type {
  DockQuestion,
  HelmApprovalCard,
  HelmApprovalCardState,
  HelmComposerChip,
  HelmRepositoryOption,
  HelmThreadRow,
} from "@armada/components";
import type {
  HelmContext,
  HelmDebugInfo,
  HelmDebugRead,
  JobSummary,
  RepositorySummary,
  WorkflowSummary,
} from "@armada/protocol";
import { helmRowsOf, type HelmApprovalAsk, type HelmFoldedRow } from "@armada/screens/src/helm-thread";
import type { BridgeState } from "../../shared/bridge";
import { locationOf, type StudioNamed } from "./helm-context";

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
  /** The open Studio and its selected node, by name, for the footer. #1287. */
  studio?: StudioNamed;
  onSwitch: (manifestId: string) => void;
  /** The session as one record, read once when a person opens it — #1367. */
  onReadRecord: () => Promise<HelmDebugRead>;
  /** Told after the record's clipboard write, either way. */
  onCopied: (what: string) => void;
  /** Told why the record could not be read, where nothing could be copied. */
  onSaid: (sentence: string) => void;
  /** Approve, answered: the card waits on this, and a refusal puts it back to ready. #1117. */
  onApprove: (jobId: string) => Promise<{ ok: boolean }>;
  /**
   * Every permission ask this session is held inside, already worded by
   * `dockQuestionsOf` — #1519. Drawn under the thread and over the message
   * box, which is where the reply it stopped is and where the person is
   * looking. Oldest first, and empty draws nothing.
   */
  asks?: readonly DockQuestion[];
};

/**
 * A card's own press, held for this dock's lifetime alone — a reload starts
 * over. #1041. `"pending"` is Approve pressed and Fleet not yet answered. #1117.
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
  studio,
  onSwitch,
  onReadRecord,
  onCopied,
  onSaid,
  onApprove,
  asks = [],
}: HelmDockProps) {
  const [draft, setDraft] = useState("");
  const [record, setRecord] = useState<Read>({ open: false });
  const [pressed, setPressed] = useState<Record<string, Pressed>>({});
  const current = helm.state === "none" ? undefined : helm.manifestId;
  const options = repositories
    .filter((one): one is RepositorySummary & { manifest: NonNullable<RepositorySummary["manifest"]> } =>
      one.manifest !== undefined,
    )
    .map((one) => ({ id: one.manifest.id, label: one.manifest.repository }));
  const folded = helm.state === "open" || helm.state === "failed" ? helmRowsOf(helm.items) : [];
  const rows = folded.map((row) => withCards(row, jobs, workflows, pressed, onApprove, setPressed));
  const replying = helmReplying(helm);

  const notice = !live
    ? "Fleet is not connected. What Helm already said is still here."
    : helm.state === "failed"
      ? helm.detail
      : undefined;
  const emptyNote =
    helm.state === "cleared"
      ? "The conversation is cleared. Ask Helm something to start again."
      : current === undefined
        ? unpointedNote(options)
        : undefined;

  /**
   * Read the record, then draw it. **Asked each time**, never held: it is one
   * artifact taken at a moment, and a stale one quoted into an issue is worse
   * than none.
   */
  function openRecord(): void {
    setRecord({ open: true });
    void onReadRecord().then((read) =>
      setRecord(read.ok ? { open: true, record: read.record } : { open: true, failed: whyRecord(read) }),
    );
  }

  /**
   * One press, and the record is on the clipboard — the banner form: this is
   * reached when Helm has just answered badly and somebody wants to carry it
   * now, and the reading is what they want second.
   *
   * **The read is between the press and the write**, so a Fleet that would not
   * answer must say so rather than leave a silent control: a failed clipboard
   * write and a dead control look the same, and so does a copy that never had
   * anything to copy.
   */
  function copyRecord(): void {
    void onReadRecord().then((read) =>
      read.ok ? copyHelmRecord(read.record, onCopied) : onSaid(whyRecord(read)),
    );
  }

  function send(): void {
    const text = draft.trim();
    if (text === "") return;
    onAsk(text, context);
    setDraft("");
  }

  return (
    <div className="armada-helm-dock">
      <HelmThread rows={rows} replying={replying} notice={notice} emptyNote={emptyNote} />
      {/* Between the thread and the message box: the reply is held open above
          it and the person is typing below it, so the ask is in the one place
          neither of them has to be left to find it — #1519. The same
          `DockQuestions` the dock draws, so there is one card and not two. */}
      <DockQuestions questions={asks} />
      <HelmComposer
        current={current}
        repositories={options}
        chip={chip}
        onRemoveChip={onRemoveChip}
        location={locationOf(context, jobs, studio)}
        // Always passed: **the composer alone decides when a switch is worth
        // drawing**, off `current` and this list. Counting the repositories
        // here as well is how one set up and Helm pointed at nothing kept
        // drawing no control under a sentence naming one. A specific pick
        // does not hide it either — Discuss, or the switch itself, points
        // Helm away from the picked repository without moving the rail.
        onSwitch={onSwitch}
        onCopyRecord={current === undefined ? undefined : copyRecord}
        onOpenRecord={current === undefined ? undefined : openRecord}
        value={draft}
        onChange={setDraft}
        onSend={send}
        disabled={!live || current === undefined}
      />
      <HelmRecord
        open={record.open}
        record={record.record}
        reading={record.record === undefined && record.failed === undefined}
        failed={record.failed}
        onCopied={onCopied}
        onClose={() => setRecord({ open: false })}
      />
    </div>
  );
}

/**
 * A reply is being written, or a message is waiting for one — the one rule
 * *Start fresh* is refused by, and Fleet's rather than a guess drawn here.
 * **Exported because the act left this component**: the head of the dock now
 * carries it (`App.tsx`), and the thread below still reads the same fact, so
 * the two must not be able to disagree about it.
 */
export function helmReplying(helm: BridgeState["helm"]): boolean {
  return helm.state === "open" && helm.replying;
}

/** The record sheet's own state, held for as long as it is open and no longer. */
type Read = { open: boolean; record?: HelmDebugInfo; failed?: string };

/**
 * Why the record could not be read, in one sentence. **Fleet's own refusal
 * where it gave one**, and the connection's where the request never left.
 */
function whyRecord(read: Extract<HelmDebugRead, { ok: false }>): string {
  const outcome = read.outcome;
  if (outcome.ok) return "The session could not be read.";
  if (outcome.why === "not_connected") return "Fleet is not connected, so the session cannot be read.";
  if (outcome.why === "refused") return outcome.error.message;
  return "The session could not be read.";
}

/**
 * What the thread says while Helm is pointed at no repository.
 *
 * **That is `helm.state === "none"` alone, and it says nothing about the
 * repositories Fleet serves** — `main/helm.ts` publishes it whenever its
 * target resolves to nothing, which includes a rail sitting on a repository
 * that is not set up while others are. One sentence stood for all of it and
 * read "No repository has a Manifest yet for Helm to answer about.", so a
 * person with two set up was told a false fact about their own machine.
 *
 * `options` is the fact the sentence was missing: every repository with a
 * Manifest, which the rail's *Not set up* group, Setup and the Studios empty
 * state all call **set up**.
 */
function unpointedNote(options: readonly HelmRepositoryOption[]): string {
  if (options.length === 0) {
    // Genuinely none, and now the only case that says so. **Not the Studios
    // empty state's own opening** — "No repository is set up yet, so none of
    // them keeps Studios." draws a hand's width from this one on the same
    // screen, so this takes that line's vocabulary and not its sentence.
    return "Nothing is set up yet for Helm to answer about. Set up a repository, and Helm answers for it.";
  }
  // The composer below holds the switch that does this, so the act is named
  // and its control is not described twice. With one set up the sentence
  // names that repository rather than counting it, and the switch under it
  // offers exactly that one to pick — pointed at nothing, one is somewhere
  // to go.
  const only = options.length === 1 ? options[0] : undefined;
  if (only !== undefined) return `Helm is not pointed at a repository. Pick ${only.label} to ask about it.`;
  return `Helm is not pointed at a repository. ${options.length} are set up, so pick one to ask about it.`;
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
  onApprove: (jobId: string) => Promise<{ ok: boolean }>,
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
  onApprove: (jobId: string) => Promise<{ ok: boolean }>,
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
            // Refused: drop the press, so the card reads the Board again and can be pressed again.
            void onApprove(ask.jobId).then((answer) =>
              setPressed(({ [ask.id]: _pending, ...rest }) =>
                answer.ok ? { ...rest, [ask.id]: "approved" } : rest,
              ),
            );
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
 * this dock approved reads "Approved." rather than snapping straight to
 * `elsewhere` the instant the Job leaves `awaiting_approval` because the
 * press worked, and one still waiting on Fleet reads `pending`.
 *
 * Absent any press, the Board's own status decides: a Job no longer there, or
 * one this card never moved, reads `elsewhere`; a Job this dock has never
 * heard of reads `unknown`, in words rather than a broken card.
 */
function stateOf(id: string, job: JobSummary | undefined, pressed: Record<string, Pressed>): HelmApprovalCardState {
  const local = pressed[id];
  if (local !== undefined) return local;
  if (job === undefined) return "unknown";
  return job.status === "awaiting_approval" ? "ready" : "elsewhere";
}
