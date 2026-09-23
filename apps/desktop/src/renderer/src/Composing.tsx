// The composer, as the window mounts it: out of `App.tsx`, which is at the length the gate
// refuses. On All repositories it asks which repository first, since a Job belongs to one.
//
// **The answer is held here, apart from the rail's pick — #959.** Before this, the ask answered
// by picking in the rail, which is what narrowed the Board behind the composer to one repository.
// The rail's own pick stays exactly where it was; this component reads it only to skip the ask
// once one is already picked, and never writes it.

import { useCallback, useEffect, useState } from "react";
import type { LeftOutWorkflow, ManifestReading, ManifestSummary, RepositorySummary } from "@armada/protocol";
import { Button, Dialog, KbdBinding } from "@armada/components";
import { AskRepository, DispatchJob, watchOf } from "@armada/screens";
import { Boundary } from "@armada/shell";

import { dispatchSettingsOf } from "@armada/screens/src/draft/dispatch";
import { landingRuleOf } from "@armada/screens/src/draft/landing";

import type { BridgeState } from "../../shared/bridge";
import { readComposing, searchFiles, stageAttachment, type useCommands } from "./commands";
import { useDrafted } from "./drafted";

/**
 * The way out, on the head of the thing it closes — the owner's call of
 * 2026-09-17, against a `Cancel` that sat on its own line above the card
 * belonging to nothing. Same control and same word as the Job settings sheet
 * and Helm's dock, key included: one act, one vocabulary.
 *
 * **`ground` is the surface it lands on**, and the two are not
 * interchangeable: a secondary is filled one step from its ground, so a card
 * takes `card` and the ask's sunken alert takes `sunken`.
 */
function WayOut({ ground, onClose }: { ground: "card" | "sunken"; onClose: () => void }) {
  return (
    <Button variant="secondary" size="sm" ground={ground} onClick={onClose} title="Close — Esc">
      Close
      <KbdBinding binding="Esc" />
    </Button>
  );
}

/**
 * What the ask says before it throws a draft away — the owner's call of
 * 2026-09-17: an untouched composer closes at once, one carrying anything
 * typed asks first, the way killing a Job does.
 *
 * It states what happens and what survives, and what survives is the point:
 * nothing was sent, so there is no job to lose.
 */
const DISCARD = {
  title: "Discard what you typed?",
  body:
    "What you have typed here, and anything attached to it, is dropped and the composer closes. " +
    "Nothing has been sent to Fleet, so no job exists yet and the board is unchanged. " +
    "Dispatch opens on an empty composer next time.",
  act: "Discard",
};

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
  // already carries the right one, so nothing here is asked. It reaches the
  // Settings block's Workflow field, which is where a workflow is picked now.
  const [composingFor, setComposingFor] = useState(UNREAD);
  // What closing would throw away: the request, anything staged or linked
  // against it, and any setting moved. One surface reports it, since the hand
  // form that used to hold the other half is gone.
  const [typed, setTyped] = useState(false);
  // Whether the discard ask is up. While it is, the key below is the dialog's.
  const [asking, setAsking] = useState(false);
  // What a mock moment already holds — the request half typed, the branches,
  // the two refs. `{}` in the app, where nothing serves any of it yet:
  // `drafted.tsx` is the whole of that seam.
  const drafted = useDrafted();
  /** The one act the control and the key share: leave, or ask first. */
  const leave = useCallback(() => {
    if (typed) setAsking(true);
    else onClose();
  }, [typed, onClose]);
  // Escape leaves the composer, which is what the control on its head says it
  // does. Here rather than in `App.tsx` because the ask depends on what has
  // been typed, and this is where that is known. A press a layer above already
  // answered is not a second exit — the `@` mention popup is one of them.
  useEffect(() => {
    if (asking) return;
    const pressed = (event: KeyboardEvent): void => {
      if (event.key !== "Escape" || event.defaultPrevented) return;
      leave();
    };
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, [asking, leave]);
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
      <div className="armada-screen__pane">
        <AskRepository
          repositories={repositories}
          title="Pick the repository this Job is for"
          next="A Job belongs to one repository. The Board stays on All; the new Job is listed under the repository you pick."
          onPick={setAnswered}
          onlySetUp
          // The ask is an alert rather than a card, and the alert's own slot
          // centres what it holds — which put this control level with the
          // Repository label, two lines under the title. `actionOn="title"`
          // draws it on the title's line instead, the way the two card
          // states below draw the same control.
          action={<WayOut ground="sunken" onClose={leave} />}
        />
      </div>
    );
  }
  // Off All, the repository already picked — unchanged. On All, the one the
  // ask answered, read by root rather than by the pick, which stays on All.
  const manifest = all ? repositories.find((one) => one.root === answered)?.manifest : scoped;
  const guarded = { bridge: state.bridge, onCopied };
  return (
    <div className="armada-screen__pane">
      {/* Describing the work is the only way in since the owner took the hand
          form out on 2026-09-23. What Fleet holds is read over the one
          connection and not scraped off the Jobs already on the board, which
          is what this offered before `list_workflows` and `list_manifests`
          existed. */}
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
        {...(manifest === undefined ? {} : { repository: manifest.repository })}
        // Where the work starts and where it lands. **`landingRuleOf` with no
        // Manifest is two null refs**, which draw as empty fields — Bridge
        // holds no `base` outside the Manifest editor's own read, and a branch
        // name invented here would be a value nobody chose.
        landing={drafted.landing ?? landingRuleOf()}
        // `null` is nothing having listed this repository's branches, which is
        // every dispatch against a real Fleet: no operation asks for a
        // repository's refs, and the two reads `branchesOf` derives from — the
        // Manifest's declared base and the worktrees Fleet holds — are neither
        // of them read here. The two fields draw as the plain ones they were.
        branches={drafted.branches ?? null}
        workflows={state.holds.workflows}
        // What Fleet ran without, named under the Workflow field — #425. On
        // All it is the answered repository's own read, since the pick stays
        // on All; off All the pick already scopes it.
        leftOut={all ? composingFor.leftOut : state.holds.leftOut}
        models={state.holds.models?.models ?? []}
        // The machine's own cap. A moment carries its own, because `limits` is
        // a read and a scenario publishes none.
        machineCap={drafted.proposal?.machine_cap ?? state.limits?.concurrency ?? null}
        {...(drafted.proposal === undefined
          ? {}
          : { settings: dispatchSettingsOf(drafted.proposal) })}
        {...(drafted.prompt === undefined ? {} : { opensOn: drafted.prompt })}
        {...(drafted.sketch === undefined ? {} : { sketch: drafted.sketch })}
        // On the head of each card this surface draws, since each is its own
        // way out of the same composer.
        close={<WayOut ground="card" onClose={leave} />}
        // What the request field and its attachments would lose.
        onTyped={setTyped}
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
      />
      </Boundary>
      {/* The ask, and only where something would be lost. Every destructive
          act confirms through this one dialog, which owns `Cancel` holding
          focus; this supplies the words. Cancelling leaves the composer
          exactly as it was, since nothing here unmounts it. */}
      {!asking ? null : (
        <Dialog
          open
          title={DISCARD.title}
          confirmLabel={DISCARD.act}
          onCancel={() => setAsking(false)}
          onConfirm={() => {
            setAsking(false);
            onClose();
          }}
        >
          {DISCARD.body}
        </Dialog>
      )}
    </div>
  );
}
