// The composer, as the window mounts it: out of `App.tsx`, which is at the length the gate
// refuses. On All repositories it asks which repository first, since a Job belongs to one.

import type { ManifestSummary, RepositorySummary } from "@armada/protocol";
import { AskRepository, Composer, DispatchJob, watchOf } from "@armada/screens";
import { Boundary } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { searchFiles, stageAttachment, type useCommands } from "./commands";

export function Composing({
  state,
  commands,
  now,
  live,
  all,
  repositories,
  scoped,
  onPick,
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
  onPick: (root: string) => void;
  onOpen: (jobId: string) => void;
  onClose: () => void;
  onCopied: (value: string) => void;
}) {
  if (all) {
    return (
      <AskRepository
        repositories={repositories}
        title="Pick the repository this Job is for"
        next="A Job belongs to one repository. Picking it focuses the Board there, where the Job is listed."
        onPick={onPick}
      />
    );
  }
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
        // handed over at the press rather than held by the command.
        onPropose={(request, attachments) =>
          commands.proposeFrom(request, attachments, {
            workflows: state.holds.workflows,
            bridge: state.bridge,
          })
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
            leftOut={state.holds.leftOut}
            onStage={stageAttachment}
            onSearchFiles={searchFiles}
            manifest={scoped}
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
