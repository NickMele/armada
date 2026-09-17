// The Studios surface, as the window mounts it — #1287. Out of `App.tsx`, which is near the length
// the gate refuses: this holds the two reads open and answers the screen's presses.
//
// **A Studio belongs to one repository**, so on All repositories the surface asks for one first,
// the Manifest surface's own ask, and a repository with no Manifest keeps none.

import { useEffect } from "react";
import type { RepositorySummary } from "@armada/protocol";
import { AskRepository, Studios, type OpenStudio } from "@armada/screens";
import { Boundary } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { createStudio, decideStudioEdge, moveStudioNode, removeStudioNode, watchStudio, watchStudios } from "./commands";

export type StudiosSurfaceProps = {
  state: BridgeState;
  live: boolean;
  repositories: readonly RepositorySummary[];
  /** The picked repository's Manifest id, or `undefined` where none is picked or it has none. */
  manifestId: string | undefined;
  /** All repositories, where Fleet serves any. */
  all: boolean;
  onPick: (root: string) => void;
  open: OpenStudio | null;
  onOpenChange: (open: OpenStudio | null) => void;
  selectedNode: string | null;
  onSelectNode: (nodeId: string | null) => void;
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

  if (all) {
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
        onMoveNode={(nodeId, position) => moveStudioNode(openId ?? "", nodeId, position)}
        onRemoveNode={(nodeId) => removeStudioNode(openId ?? "", nodeId)}
        onDecideEdge={(edgeId, accepted) => decideStudioEdge(openId ?? "", edgeId, accepted)}
      />
    </Boundary>
  );
}
