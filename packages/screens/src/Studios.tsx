// A repository's Studios: the list, and one Studio open on its whiteboard — #1287.
//
// **Reopened read-only.** A Studio is kept to be reread (`docs/concepts/studio.md`), so one opened
// from the list moves nothing and draws no act until Continue; one a person just started opens
// editable. **A person's acts live here and nowhere else**: accepting or rejecting a proposed
// relation, deleting a node, which confirms because its edges go with it, and every rung of
// promotion, which is `StudioPromotion.tsx`'s panel. What is drawn is what Fleet wrote — every act
// answers with the Studio whole, and main folds it into `studio`.
//
// **The whiteboard's selection is held here, not in `App`.** Clustering is of several nodes, and
// the one `App` keeps is what Helm's footer names — so this keeps the list and reports its first.

import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Dialog,
  StudioAddNode,
  StudioFrameSheet,
  StudioName,
  StudioWhiteboard,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeaderCell,
  TableRow,
  useStudioPlacement,
} from "@armada/components";
import type { StudioNodeByHand, StudioNodeByHandKind } from "@armada/components";
import type {
  JobSummary,
  Outcome,
  Studio,
  StudioPosition,
  StudioPromotion,
  StudioSummary,
} from "@armada/protocol";

import { said } from "./copy";
import { absoluteOf } from "./duration";
import {
  framesDrawn,
  nodeNamed,
  proposedRelations,
  UNTITLED_STUDIO,
  whiteboardEdges,
  whiteboardNodes,
} from "./studio";
import { useStudioFrames, type ReadStudioFrame } from "./studio-frames";
import { useAddNodeKeys } from "./studio-keys";
import type { StudioAnswer, StudioRead, StudiosRead } from "./studio-reads";
import { useStudioPromotion } from "./StudioPromotion";

/**
 * What a Note past `MOST_FRAMES_DRAWN` says on its plate. **One press away**:
 * selecting the Note is what asks for it, and the sentence names that press
 * rather than reporting a limit nobody set.
 */
const PAST_THE_BOUND = "Select this Note to draw it.";

/** Two selections that name the same nodes in the same order. */
const same = (held: readonly string[], ids: readonly string[]): boolean =>
  held.length === ids.length && held.every((id, at) => id === ids[at]);

/** Which Studio is open, and whether Continue has been pressed on it. */
export type OpenStudio = { id: string; editable: boolean };

export type StudiosProps = {
  /** The repository's list, as main holds it. */
  studios: StudiosRead;
  /** The open Studio, as main holds it. */
  studio: StudioRead;
  /** `null` is the list. */
  open: OpenStudio | null;
  /** The Board's rows, which a Job node reads its state and title off. */
  jobs: readonly JobSummary[];
  /** A live connection. Nothing is sent without one. */
  live: boolean;
  /** The node selected on the whiteboard, for the acts on it and for Helm. */
  selectedNode: string | null;
  onSelectNode: (nodeId: string | null) => void;
  onOpen: (studioId: string) => void;
  onBack: () => void;
  onContinue: () => void;
  /** Start an untitled Studio in this repository. */
  onCreate: () => Promise<StudioAnswer>;
  /** Name a Studio, or name it again. Reaches the list's rows and the open Studio alike. */
  onRename: (studioId: string, name: string) => Promise<Outcome>;
  /** Put a Note, a Link or a Sketch on the open Studio, where the person is looking. */
  onAddNode: (node: StudioNodeByHand, position: StudioPosition) => Promise<Outcome>;
  onMoveNode: (nodeId: string, position: { x: number; y: number }) => Promise<Outcome>;
  onRemoveNode: (nodeId: string) => Promise<Outcome>;
  onDecideEdge: (edgeId: string, accepted: boolean) => Promise<Outcome>;
  /** The picture one Note kept, as bytes. The screen mints the `blob:` and revokes it — #1352. */
  onReadFrame: ReadStudioFrame;
  /** One rung of promotion — cluster, outline, defer, write up, edit, settle, dispatch. */
  onPromote: (promotion: StudioPromotion) => Promise<Outcome>;
};

export function Studios(props: StudiosProps) {
  return props.open === null ? <StudioList {...props} /> : <OpenedStudio {...props} open={props.open} />;
}

function StudioList({ studios, live, onOpen, onCreate, onRename }: StudiosProps) {
  const [creating, setCreating] = useState(false);
  const [refused, setRefused] = useState<string | null>(null);
  const [naming, setNaming] = useState<string | null>(null);

  function create(): void {
    setCreating(true);
    setRefused(null);
    void onCreate().then((answer) => {
      setCreating(false);
      if (!answer.ok) setRefused(said(answer.outcome));
    });
  }

  function rename(studioId: string, name: string): void {
    setNaming(studioId);
    setRefused(null);
    void onRename(studioId, name).then((outcome) => {
      setNaming(null);
      if (!outcome.ok) setRefused(said(outcome));
    });
  }

  return (
    <div className="armada-screen__pane">
      <Card>
        <CardHeader className="armada-studio__head">
          <CardTitle>Studios</CardTitle>
          <Button variant="primary" size="sm" pending={creating} disabled={!live} onClick={create}>
            {creating ? "Starting a Studio" : "New Studio"}
          </Button>
        </CardHeader>
        <CardContent>
          {refused === null ? null : (
            <Alert tone="escalated" title="Fleet did not take that">
              {refused}
            </Alert>
          )}
          <ListBody studios={studios} live={live} naming={naming} onOpen={onOpen} onRename={rename} />
        </CardContent>
      </Card>
    </div>
  );
}

type ListBodyProps = {
  studios: StudiosRead;
  /** A live connection. A row draws its name and no rename without one. */
  live: boolean;
  /** The Studio whose rename is out to Fleet, or `null`. */
  naming: string | null;
  onOpen: (studioId: string) => void;
  onRename: (studioId: string, name: string) => void;
};

function ListBody({ studios, live, naming, onOpen, onRename }: ListBodyProps) {
  if (studios.state === "failed") {
    return (
      <Alert tone="escalated" title="This repository's Studios could not be read">
        {said(studios.outcome)}
      </Alert>
    );
  }
  // `none` is the frame before the read is asked for, and says what `reading` says.
  if (studios.state !== "read") return <p className="text-fg-muted">Reading this repository's Studios.</p>;
  if (studios.list.studios.length === 0) {
    return <p className="text-fg-muted">No Studios yet. Start one to keep what you work out before it is a Job.</p>;
  }
  return (
    <Table>
      <TableHead>
        <TableRow>
          <TableHeaderCell>Name</TableHeaderCell>
          <TableHeaderCell>Last touched</TableHeaderCell>
        </TableRow>
      </TableHead>
      <TableBody>
        {studios.list.studios.map((one) => (
          <Row
            key={one.id}
            studio={one}
            live={live}
            saving={naming === one.id}
            onOpen={onOpen}
            onRename={onRename}
          />
        ))}
      </TableBody>
    </Table>
  );
}

/**
 * One row. **The name opens the Studio and the rename sits beside it**: a row's
 * name is already the way in, so renaming cannot also take that press — #1364.
 */
function Row({
  studio,
  live,
  saving,
  onOpen,
  onRename,
}: {
  studio: StudioSummary;
  live: boolean;
  saving: boolean;
  onOpen: (studioId: string) => void;
  onRename: (studioId: string, name: string) => void;
}) {
  return (
    <TableRow>
      <TableCell>
        <StudioName
          name={studio.name ?? null}
          untitled={UNTITLED_STUDIO}
          editable={live}
          saving={saving}
          onOpen={() => onOpen(studio.id)}
          onRename={(name) => onRename(studio.id, name)}
        />
      </TableCell>
      <TableCell>{absoluteOf(studio.touched_at) ?? studio.touched_at}</TableCell>
    </TableRow>
  );
}

function OpenedStudio(props: StudiosProps & { open: OpenStudio }) {
  const { studio, open, onBack } = props;
  if (studio.state === "read" && studio.studio.id === open.id) {
    return <Board {...props} graph={studio.studio} />;
  }
  return (
    <div className="armada-screen__pane">
      <div>
        <Button variant="ghost" size="sm" onClick={onBack}>
          Back to Studios
        </Button>
      </div>
      {studio.state === "failed" ? (
        <Alert tone="escalated" title="This Studio could not be read">
          {said(studio.outcome)}
        </Alert>
      ) : studio.state === "gone" ? (
        <Alert tone="neutral" title="This Studio was deleted">
          Nothing of it is kept.
        </Alert>
      ) : (
        <p className="text-fg-muted">Reading this Studio.</p>
      )}
    </div>
  );
}

/**
 * The whiteboard. **Takes the Studio rather than the read**, because promotion is a hook and a
 * hook cannot sit under an early return: narrowing happens in `OpenedStudio`, which already only
 * draws this when the read is one.
 */
function Board(props: StudiosProps & { open: OpenStudio; graph: Studio }) {
  const { graph: studio, open, jobs, live, selectedNode, onSelectNode, onBack, onContinue } = props;
  const [refused, setRefused] = useState<string | null>(null);
  const [removing, setRemoving] = useState<string | null>(null);
  const [deciding, setDeciding] = useState<string | null>(null);
  /** The Note whose frame is open, full size. */
  const [opened, setOpened] = useState<string | null>(null);
  /** Every node picked on the whiteboard. A cluster is of several, and `App` keeps one. */
  const [picked, setPicked] = useState<readonly string[]>([]);
  const [naming, setNaming] = useState(false);
  // Which kind is being written, and whether it is out to Fleet — #1364. Held
  // here rather than in the control, because `N`, `V` and `S` open it too.
  const [adding, setAdding] = useState<StudioNodeByHandKind | null>(null);
  const [addingOut, setAddingOut] = useState(false);
  useAddNodeKeys(open.editable && live, setAdding);
  // The pictures the Notes kept, and the `blob:` each one becomes — #1352.
  const frames = useStudioFrames(props.onReadFrame, open.id);
  const drawn = framesDrawn(studio, selectedNode);
  // `want` sends nothing twice, so asking again on every render asks once.
  useEffect(() => void frames.want([...drawn]), [drawn, frames]);
  const frameOf = (nodeId: string) =>
    drawn.has(nodeId) ? (frames.of(nodeId) ?? {}) : { why: PAST_THE_BOUND };
  const openedNote = studio.nodes.find((node) => node.id === opened && node.kind === "note");
  const editable = open.editable && live;
  const proposed = proposedRelations(studio, jobs);
  const onBoard = picked.filter((id) => studio.nodes.some((node) => node.id === id));
  const selected = onBoard.length === 1 ? studio.nodes.find((node) => node.id === onBoard[0]) : undefined;

  function answered(outcome: Outcome): void {
    setRefused(outcome.ok ? null : said(outcome));
  }

  const promotion = useStudioPromotion({
    studio,
    selected: onBoard,
    onPromote: props.onPromote,
    onAnswered: answered,
  });

  function rename(name: string): void {
    setNaming(true);
    void props.onRename(studio.id, name).then((outcome) => {
      setNaming(false);
      answered(outcome);
    });
  }

  function add(node: StudioNodeByHand, position: StudioPosition): void {
    setAddingOut(true);
    void props.onAddNode(node, position).then((outcome) => {
      setAddingOut(false);
      answered(outcome);
      if (outcome.ok) setAdding(null);
    });
  }

  function decide(edgeId: string, accepted: boolean): void {
    setDeciding(edgeId);
    void props.onDecideEdge(edgeId, accepted).then((outcome) => {
      setDeciding(null);
      answered(outcome);
    });
  }

  return (
    <div className="armada-studio">
      <div className="armada-studio__head">
        <Button variant="ghost" size="sm" onClick={onBack}>
          Back to Studios
        </Button>
        <StudioName
          heading
          name={studio.name ?? null}
          untitled={UNTITLED_STUDIO}
          editable={editable}
          saving={naming}
          onRename={rename}
        />
        {open.editable ? null : (
          <>
            <span className="armada-studio__standing">Read-only</span>
            <Button variant="primary" size="sm" disabled={!live} onClick={onContinue}>
              Continue
            </Button>
          </>
        )}
      </div>
      {refused === null ? null : (
        <Alert tone="escalated" title="Fleet did not take that">
          {refused}
        </Alert>
      )}
      <div className="armada-studio__board">
        <StudioWhiteboard
          key={studio.id}
          nodes={whiteboardNodes(studio, jobs, frameOf)}
          edges={whiteboardEdges(studio)}
          readOnly={!editable}
          onNodeMoved={(nodeId, position) => {
            // The whiteboard refuses a move while read-only already. This is the second lock.
            if (!editable) return;
            void props.onMoveNode(nodeId, { x: Math.round(position.x), y: Math.round(position.y) }).then(answered);
          }}
          onSelectionChange={(ids) => {
            // **The same list keeps its identity.** React Flow re-subscribes whenever this handler
            // changes, and re-subscribing calls it — so a fresh array here is a state change that
            // re-renders, re-subscribes and calls it again, which is an update loop with no end.
            setPicked((held) => (same(held, ids) ? held : [...ids]));
            // Helm is told one node, the first: its footer names what a person is looking at, and
            // a cluster of four is not a place.
            onSelectNode(ids[0] ?? null);
          }}
        >
          {editable ? (
            <Card aria-label="Add a node">
              <CardContent>
                <AddNode
                  adding={adding}
                  onAdding={setAdding}
                  onAdd={add}
                  saving={addingOut}
                  disabled={!editable}
                />
              </CardContent>
            </Card>
          ) : null}
          {studio.nodes.length === 0 ? (
            <Card>
              <CardContent>Nothing on this Studio yet. Add a note, a link or a sketch to start it.</CardContent>
            </Card>
          ) : null}
          {onBoard.length === 0 ? null : (
            <Card aria-label="Selected node">
              <CardContent className="armada-studio__aside">
                <p>{onBoard.map((id) => nodeNamed(studio, id, jobs)).join("; ")}</p>
                {editable ? promotion.acts : null}
                {/* Opening the picture is reading, so it is offered read-only
                    too — and it is an act on the node, where the acts on a node
                    already are, rather than a press on a card the board drags. */}
                {selected?.kind === "note" && selected.capture?.frame !== undefined ? (
                  <Button size="sm" onClick={() => setOpened(selected.id)}>
                    Open frame
                  </Button>
                ) : null}
                {editable && selected !== undefined ? (
                  <Button variant="destructive" size="sm" onClick={() => setRemoving(selected.id)}>
                    Delete node
                  </Button>
                ) : null}
              </CardContent>
            </Card>
          )}
          {proposed.length === 0 ? null : (
            <Card aria-label="Proposed relations">
              <CardHeader>
                <CardTitle>Proposed</CardTitle>
              </CardHeader>
              <CardContent className="armada-studio__aside">
                {proposed.map((one) => (
                  <div key={one.id} className="armada-studio__proposed">
                    <p>
                      {one.from} <em>{one.relation}</em> {one.to}
                    </p>
                    {editable ? (
                      <div className="armada-studio__acts">
                        <Button
                          size="sm"
                          pending={deciding === one.id}
                          disabled={deciding !== null}
                          aria-label={`Accept: ${one.from} ${one.relation} ${one.to}`}
                          onClick={() => decide(one.id, true)}
                        >
                          Accept
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          disabled={deciding !== null}
                          aria-label={`Reject: ${one.from} ${one.relation} ${one.to}`}
                          onClick={() => decide(one.id, false)}
                        >
                          Reject
                        </Button>
                      </div>
                    ) : null}
                  </div>
                ))}
                {editable ? null : <p className="text-fg-muted">Continue to accept or reject.</p>}
              </CardContent>
            </Card>
          )}
        </StudioWhiteboard>
      </div>
      {/* Outside the whiteboard, both of these: React Flow paints its nodes over anything inside
          its own subtree, so a layer drawn in there is read through the Notes it is about. */}
      {editable ? promotion.dialog : null}
      {openedNote === undefined || openedNote.kind !== "note" ? null : (
        <StudioFrameSheet
          open
          said={openedNote.said}
          frame={frameOf(openedNote.id)}
          onClose={() => setOpened(null)}
        />
      )}
      <Dialog
        open={removing !== null}
        title="Delete this node"
        confirmLabel="Delete node"
        onCancel={() => setRemoving(null)}
        onConfirm={() => {
          const nodeId = removing;
          setRemoving(null);
          if (nodeId === null) return;
          onSelectNode(null);
          void props.onRemoveNode(nodeId).then(answered);
        }}
      >
        {removing === null ? null : (
          <p>
            {nodeNamed(studio, removing, jobs)} goes, and every edge on it. The rest of the Studio stays where
            it is.
          </p>
        )}
      </Dialog>
    </div>
  );
}

/**
 * The `+ Node` control, drawn on the board's own aside.
 *
 * **A component and not markup**, because `useStudioPlacement` reads the
 * viewport React Flow is holding and only a component rendered inside the
 * board is inside that provider. What it buys is the rule: a node lands where
 * the person is looking rather than at the origin.
 */
function AddNode(props: {
  adding: StudioNodeByHandKind | null;
  onAdding: (kind: StudioNodeByHandKind | null) => void;
  onAdd: (node: StudioNodeByHand, position: StudioPosition) => void;
  saving: boolean;
  disabled: boolean;
}) {
  const place = useStudioPlacement();
  return (
    <StudioAddNode
      adding={props.adding}
      onAdding={props.onAdding}
      onAdd={(node) => props.onAdd(node, place())}
      saving={props.saving}
      disabled={props.disabled}
    />
  );
}
