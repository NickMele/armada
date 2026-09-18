// The Studios surface, as the window mounts it — #1287. Out of `App.tsx`, which is near the length
// the gate refuses: this holds the two reads open and answers the screen's presses.
//
// **A Studio belongs to one repository**, so on All repositories the surface asks for one first,
// the Manifest surface's own ask, and a repository with no Manifest keeps none.
//
// **A question nobody can answer is not asked.** The ask lists only a repository that has a
// Manifest, so where none of them does it draws a picker with nothing pickable in it. That state
// says what is true instead, and points at the Manifest surface, which is where a repository is
// picked and set up.

import { useEffect } from "react";
import type { RepositorySummary } from "@armada/protocol";
import { BoardEmptyState, Button } from "@armada/components";
import { AskRepository, Studios, type OpenStudio } from "@armada/screens";
import { Boundary } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import {
  addStudioNode,
  createStudio,
  decideStudioEdge,
  moveStudioNode,
  openCaptureWindow,
  openServerLink,
  openStudioNode,
  promoteOnStudio,
  readStudioFrame,
  removeStudioNodes,
  renameStudio,
  startStudioRun,
  startStudioServer,
  stopServer,
  watchCheckoutRunSheet,
  watchStudio,
  watchStudios,
} from "./commands";

export type StudiosSurfaceProps = {
  state: BridgeState;
  live: boolean;
  repositories: readonly RepositorySummary[];
  /** The picked repository's Manifest id, or `undefined` where none is picked or it has none. */
  manifestId: string | undefined;
  /** All repositories, where Fleet serves any. */
  all: boolean;
  onPick: (root: string) => void;
  /**
   * The way out of All repositories with nothing set up: the Manifest surface,
   * whose own ask picks a repository and opens Setup on one with no Manifest.
   * The route is Bridge's existing one, not a second one cut from here.
   */
  onSetUp: () => void;
  open: OpenStudio | null;
  onOpenChange: (open: OpenStudio | null) => void;
  selectedNode: string | null;
  onSelectNode: (nodeId: string | null) => void;
  /** Open one Job whole — the Board's own press, reached from a Job node (#1379). */
  onOpenJob: (jobId: string) => void;
  /** The clock the window ticks on, for how long a server node has been up — #1345. */
  now: number;
  onCopied: (value: string) => void;
};

export function StudiosSurface(props: StudiosSurfaceProps) {
  const { state, live, repositories, manifestId, all, onPick, open, onOpenChange, selectedNode, onSelectNode } = props;
  const openId = open?.id ?? null;

  useEffect(() => {
    watchStudios(manifestId ?? null);
    return () => watchStudios(null);
  }, [manifestId]);

  useEffect(() => {
    watchStudio(openId);
    return () => watchStudio(null);
  }, [openId]);

  // What this repository's checkout declares, so a Studio can start one of it —
  // #1345. **The Manifest surface's own read**, held open only while a Studio
  // is open: one surface is mounted at a time, so the two never want it at once.
  useEffect(() => {
    watchCheckoutRunSheet(openId !== null);
    return () => watchCheckoutRunSheet(false);
  }, [openId]);

  if (all) {
    // Nothing to pick from, so nothing is asked: one line saying what is true, and the one act
    // that changes it. No card and no glyph — `docs/contracts/design-system.md` on empty states.
    if (!repositories.some((one) => one.manifest !== undefined)) {
      return (
        <BoardEmptyState
          quiet
          // **Not the sentence Helm's dock opens with**, though the fact is the same one: the dock
          // says "Nothing is set up yet for Helm to answer about." a hand's width from this line,
          // and two lines that read alike are a surface a person has to disentangle. The dock took
          // this line's vocabulary in `HelmDock.tsx` and kept its own opening for that reason.
          lead="No repository is set up yet, so none of them keeps Studios."
          action={
            <Button variant="primary" onClick={props.onSetUp}>
              Set up a repository
            </Button>
          }
        >
          A Studio is kept against a repository's Manifest. Set one up, and its Studios open here.
        </BoardEmptyState>
      );
    }
    return (
      <AskRepository
        repositories={repositories}
        title="Pick a repository to open its Studios"
        next="A Studio belongs to one repository, and picking it focuses the Board there too."
        onPick={onPick}
        onlySetUp
      />
    );
  }
  if (manifestId === undefined) {
    return <p className="text-fg-muted">This repository has no Manifest yet, so it keeps no Studios.</p>;
  }

  return (
    <Boundary region="Studios" bridge={state.bridge} onCopied={props.onCopied}>
      <Studios
        studios={state.studios}
        studio={state.studio}
        open={open}
        jobs={state.jobs}
        live={live}
        selectedNode={selectedNode}
        onSelectNode={onSelectNode}
        onOpen={(studioId) => {
          // Reopened read-only: rereading is not editing, and Continue is what makes it editable.
          onSelectNode(null);
          onOpenChange({ id: studioId, editable: false });
        }}
        onBack={() => {
          onSelectNode(null);
          onOpenChange(null);
        }}
        onContinue={() => open !== null && onOpenChange({ ...open, editable: true })}
        onCreate={async () => {
          const answer = await createStudio(manifestId);
          // A Studio a person has just started is theirs to work in.
          if (answer.ok) onOpenChange({ id: answer.studio.id, editable: true });
          return answer;
        }}
        onRename={(studioId, name) => renameStudio(studioId, name)}
        onAddNode={(node, position) => addStudioNode(openId ?? "", node, position)}
        onMoveNode={(nodeId, position) => moveStudioNode(openId ?? "", nodeId, position)}
        onRemoveNodes={(nodeIds) => removeStudioNodes(openId ?? "", nodeIds)}
        onDecideEdge={(edgeId, accepted) => decideStudioEdge(openId ?? "", edgeId, accepted)}
        onReadFrame={readStudioFrame}
        onPromote={(promotion) => promoteOnStudio(openId ?? "", promotion)}
        // A node's own address, to the system browser through main — #1406.
        // Nothing opens inside Bridge: no surface navigates.
        onOpenAddress={openStudioNode}
        // The Studio stays standing under the Job, so closing it comes back to
        // the whiteboard with the node reading whatever the Job is doing now.
        onOpenJob={props.onOpenJob}
        // What the checkout declares, what Fleet is holding, and the clock.
        // **The live holder, never a copy**: a Run node holding a server reads
        // its state off this list, so a Studio left open stays right — #1345.
        runSheet={state.checkoutRunSheet}
        servers={state.servers.servers}
        now={props.now}
        onStartRun={(name, position) => startStudioRun(openId ?? "", name, position)}
        onStartServer={(name, position) => startStudioServer(openId ?? "", name, position)}
        onStopServer={stopServer}
        // The address is main's to find, off the server it published — the same
        // rule a node's own address is opened under.
        onOpenServerLink={openServerLink}
        // Capture on that server's page, in a window pinned to its origin — #1294.
        onCaptureOn={openCaptureWindow}
      />
    </Boundary>
  );
}
