// The composer, as the window mounts it: out of `App.tsx`, which is at the length the gate
// refuses. On All repositories it asks which repository first, since a Job belongs to one.
//
// **The answer is held here, apart from the rail's pick — #959.** Before this, the ask answered
// by picking in the rail, which is what narrowed the Board behind the composer to one repository.
// The rail's own pick stays exactly where it was; this component reads it only to skip the ask
// once one is already picked, and never writes it.

import { useEffect, useState } from "react";
import type { LeftOutWorkflow, ManifestReading, ManifestSummary, RepositorySummary } from "@armada/protocol";
import { AskRepository, Composer, DispatchJob, watchOf } from "@armada/screens";
import { Boundary } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { readComposing, searchFiles, stageAttachment, type useCommands } from "./commands";

/** What the answered repository's own reads are, before they have come back. */
const UNREAD: { leftOut: readonly LeftOutWorkflow[]; reading: ManifestReading | null } = {
  leftOut: [],
  reading: null,
};

export function Composing({
  state,
  commands,
  now,
  live,
  all,
  repositories,
  scoped,
  onOpen,
  onClose,
  onCopied,
}: {
  state: BridgeState;
  commands: ReturnType<typeof useCommands>;
  now: number;
  live: boolean;
  /** On All repositories, so no Manifest is picked for the Job to belong to. */
  all: boolean;
  repositories: readonly RepositorySummary[];
  /** The picked repository's Manifest, absent until it has one. */
  scoped: ManifestSummary | undefined;
  /**
   * The rail's own pick, unused here since #959: the ask no longer answers by
   * picking, so nothing in this component calls it. Still part of the type
   * because `App.tsx` — the other side of this seam, out of this change's
   * scope — still passes it.
   */
  onPick: (root: string) => void;
  onOpen: (jobId: string) => void;
  onClose: () => void;
  onCopied: (value: string) => void;
}) {
  // The repository the ask answered, held apart from the rail's pick so
  // answering it never narrows the Board — #959. `null` until answered; this
  // component is unmounted with the composer, so the next one opens unanswered.
  const [answered, setAnswered] = useState<string | null>(null);
  // `leftOut` and the Manifest reading for the repository the ask answered —
  // #959. Read once the answer is in, since `state.holds.leftOut` is scoped
  // to the pick, which stays on All throughout. Off All, `state.holds`
  // already carries the right one, so nothing here is asked.
  const [composingFor, setComposingFor] = useState(UNREAD);
  useEffect(() => {
    if (!all || answered === null) return;
    let current = true;
    void readComposing(answered).then((read) => {
      if (!current) return;
      setComposingFor(read.ok ? { leftOut: read.leftOut, reading: read.reading } : UNREAD);
    });
    return () => {
      current = false;
    };
  }, [all, answered]);
  if (all && answered === null) {
    return (
      <AskRepository
        repositories={repositories}
        title="Pick the repository this Job is for"
        next="A Job belongs to one repository. The Board stays on All; the new Job is listed under the repository you pick."
        onPick={setAnswered}
        onlySetUp
      />
    );
  }
  // Off All, the repository already picked — unchanged. On All, the one the
  // ask answered, read by root rather than by the pick, which stays on All.
  const manifest = all ? repositories.find((one) => one.root === answered)?.manifest : scoped;
  const guarded = { bridge: state.bridge, onCopied };
  return (
    /* Describing the work is the path and the form is the override, so
       the composer is what `Enter by hand` swaps to rather than what
       opens. What Fleet holds is read over the one connection and not
       scraped off the Jobs already on the board, which is what this
       offered before `list_workflows` and `list_manifests` existed. */
    <Boundary region="the job composer" {...guarded}>
      <DispatchJob
        // What the reading is read against is published state, so it is
        // handed over at the press rather than held by the command. On All,
        // the request names the answered repository rather than the pick,
        // which #959 keeps on All — `null` off All, where it already did.
        onPropose={(request, attachments) =>
          commands.proposeFrom(
            request,
            attachments,
            {
              workflows: state.holds.workflows,
              bridge: state.bridge,
            },
            all ? answered : null,
          )
        }
        onStage={stageAttachment}
        onSearchFiles={searchFiles}
        // What Fleet says the call is doing, against the same `now`
        // every other elapsed figure on screen is drawn from.
        watching={watchOf(state.proposing, now)}
        onStop={() => void commands.stopProposal()}
        // A proposed Job is opened where somebody wants to read it
        // first, which is the same signpost the Board's own
        // `awaiting_approval` row carries.
        onOpen={(jobId) => {
          onClose();
          onOpen(jobId);
        }}
        // And released without leaving, on the head of the proposal.
        // The same command the detail's own gate calls, so a second
        // approval is refused by the one guard rather than by two.
        onApprove={(jobId) => void commands.approve(jobId)}
        approving={state.approving}
        // What the board says each proposed Job is at now. The fold
        // `approveDispatch` does is what moves the row off its gate.
        statusOf={(jobId) => state.jobs.find((job) => job.id === jobId)?.status}
        disabled={!live}
        onCopied={onCopied}
        byHand={
          <Composer
            workflows={state.holds.workflows}
            leftOut={all ? composingFor.leftOut : state.holds.leftOut}
            onStage={stageAttachment}
            onSearchFiles={searchFiles}
            manifest={manifest}
            models={state.holds.models}
            disabled={!live}
            onPropose={(draft) => {
              void commands.propose(draft);
              onClose();
            }}
          />
        }
      />
    </Boundary>
  );
}
