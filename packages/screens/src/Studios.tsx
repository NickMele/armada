// A repository's Studios: the list, and one Studio open on its whiteboard — #1287.
//
// **Reopened read-only.** A Studio is kept to be reread (`docs/concepts/studio.md`), so one opened
// from the list moves nothing and draws no act until Continue; one a person just started opens
// editable. **A person's acts live here and nowhere else**: accepting or rejecting a proposed
// relation, deleting what is picked, which confirms because its edges go with it, and every rung
// of promotion, which is `StudioPromotion.tsx`'s panel. What is drawn is what Fleet wrote — every act
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
  DropdownMenu,
  StudioAddNode,
  StudioFrameSheet,
  StudioName,
  StudioPicked,
  StudioWhiteboard,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeaderCell,
  TableRow,
  useStudioPlacement,
} from "@armada/components";
import type { StudioNodeByHand, StudioNodeByHandKind, StudioPickedAct } from "@armada/components";
import type {
  CheckoutRunSheetRead,
  JobSummary,
  Outcome,
  ServerState,
  Studio,
  StudioPosition,
  StudioPromotion,
  StudioSummary,
} from "@armada/protocol";

import { said } from "./copy";
import { openServerLink, openStudioNode, type OpenServerLink, type OpenStudioNode } from "./opening";
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
import { clearingLabel, clearingOf, clearingSaid } from "./studio-clearing";
import { keepsAnAddress } from "./studio-promotion";
import { useAddNodeKeys } from "./studio-keys";
import { studioStartEntries, studioStarts, type StudioStart } from "./studio-starting";
import type { StudioAnswer, StudioRead, StudiosRead } from "./studio-reads";
import { useStudioPromotion } from "./StudioPromotion";

/**
 * What a Note past `MOST_FRAMES_DRAWN` says on its plate. **One press away**:
 * selecting the Note is what asks for it, and the sentence names that press
 * rather than reporting a limit nobody set.
 */
const PAST_THE_BOUND = "Select this Note to draw it.";

/**
 * Why *Read it in* is refused on the paste offer — #1378. **The choice is
 * drawn and says it is not built**, rather than left off: a person deciding
 * what to do with an address is owed both halves of the offer. It goes when
 * #1293 lands and this screen calls its operation instead.
 */
const READING_IN_UNBUILT = "Reading an address in is not built yet. Keep the link, and read it in when it is.";

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
  /**
   * Delete everything picked, as one write — #1411. **The only delete**, one
   * node or eighteen, and **all of them or none**: Fleet takes the whole
   * selection in one transaction, so a refusal leaves every node standing and
   * there is no half a board to reconcile.
   */
  onRemoveNodes: (nodeIds: readonly string[]) => Promise<Outcome>;
  onDecideEdge: (edgeId: string, accepted: boolean) => Promise<Outcome>;
  /** The picture one Note kept, as bytes. The screen mints the `blob:` and revokes it — #1352. */
  onReadFrame: ReadStudioFrame;
  /** One rung of promotion — cluster, outline, defer, write up, edit, settle, dispatch. */
  onPromote: (promotion: StudioPromotion) => Promise<Outcome>;
  /**
   * Read one Job whole, which is what a Board row's press does — #1379. The
   * Studio stays open underneath it, so closing the Job comes back here.
   */
  onOpenJob: (jobId: string) => void;
  /**
   * Hand one node's own address to the system browser — #1406. **A Studio and a
   * node, never an address**: main reads it off the record it published, so no
   * string composed here reaches the shell.
   */
  onOpenAddress: OpenStudioNode;
  /**
   * What this repository's checkout declares, as the Manifest surface reads it
   * — the Checks, the Commands and the servers a Studio can start (#1345).
   */
  runSheet: CheckoutRunSheetRead;
  /**
   * Every server Fleet holds. **A Run node holding one reads this rather than a
   * copy**, so a Studio left open says what the server is doing now.
   */
  servers: readonly ServerState[];
  /** The clock the window ticks on, for how long a server has been up. */
  now: number;
  /** Run one Check or Command in the checkout, as a Run node on this Studio. */
  onStartRun: (name: string, position: StudioPosition) => Promise<Outcome>;
  /** Start one server in the checkout, as a Run node holding the instance. */
  onStartServer: (name: string, position: StudioPosition) => Promise<Outcome>;
  /** End the instance a picked Run node holds. */
  onStopServer: (serverId: string) => Promise<Outcome>;
  /**
   * Hand one of a server's links to the system browser — main checks it against
   * the links it is already holding for that server, so no string composed here
   * reaches the shell. `Followed` rather than `Outcome`: opening an address has
   * its own reasons for failing, and they are the same ones a forge link's are.
   */
  onOpenServerLink: OpenServerLink;
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
  /** Whether the confirmation for deleting what is picked is up — #1411. */
  const [clearing, setClearing] = useState(false);
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
  /** A start is out to Fleet: the menu does not send a second — #1345. */
  const [starting, setStarting] = useState(false);
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
  const board = { servers: props.servers, now: props.now };
  // The instance a picked Run node holds, while Fleet still holds it — #1345.
  // **The live holder, never the node**: what a server is doing is Fleet's, and
  // a node that kept a result is one whose server is already gone.
  const serving =
    selected?.kind === "run" && selected.held === "server"
      ? props.servers.find((one) => one.id === selected.run_id && one.phase !== "exited")
      : undefined;
  const starts = studioStarts(props.runSheet);
  // **The row, not the node, is what opens.** A Job node holds a reference and
  // no status, so a Job the Board no longer carries is one there is nothing to
  // read — and the node draws its id, which is what says so.
  const openable = selected?.kind === "job" ? jobs.find((job) => job.id === selected.job_id) : undefined;

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

  /**
   * Start one entry, and let the node it makes land where the person is
   * looking. **A server goes to its own operation** — it is held rather than
   * run, and `start_studio_run` refuses a name carrying `serve` — #1345.
   */
  function start(started: StudioStart, position: StudioPosition): void {
    setStarting(true);
    const out = started.server
      ? props.onStartServer(started.name, position)
      : props.onStartRun(started.name, position);
    void out.then((outcome) => {
      setStarting(false);
      answered(outcome);
    });
  }

  /**
   * Bridge's own acts on what is picked — no rung, and nothing Fleet holds.
   * **Acts on the node rather than presses on its card**: the board is a drag
   * surface, and a control inside a node is a press fighting a drag. Opening an
   * address, a picture or a Job is reading, so a read-only Studio offers all
   * three.
   */
  const own: (StudioPickedAct & { press: () => void })[] = [
    ...(selected !== undefined && keepsAnAddress(selected)
      ? [{ id: "open", label: "Open", press: () => openAddress(selected.id) }]
      : []),
    ...(selected?.kind === "note" && selected.capture?.frame !== undefined
      ? [{ id: "frame", label: "Open frame", press: () => setOpened(selected.id) }]
      : []),
    ...(openable === undefined
      ? []
      : [{ id: "job", label: "Open Job", press: () => props.onOpenJob(openable.id) }]),
    // **A server's links and its Stop are acts on the node**, for the reason
    // every act here is: the board is a drag surface, and a button inside a card
    // is a press fighting a drag — #1345, and `StudioNode`'s own rule.
    ...(serving === undefined
      ? []
      : serving.links.map((link, at) => ({
          id: `link-${at}`,
          label: `Open ${link.name ?? link.url}`,
          press: () => void openServerLink(props.onOpenServerLink, serving.id, link.url).then(setRefused),
        }))),
    ...(serving === undefined
      ? []
      : [
          {
            id: "stop-server",
            label: "Stop the server",
            press: () => void props.onStopServer(serving.id).then(answered),
          },
        ]),
    // **One delete, counted rather than named** — #1411. Eighteen titles is the
    // panel that overflowed the window, and a second act for the one-node case
    // is a second path to keep in step with this one: they had already drifted
    // over what happens to a Note's picture.
    ...(editable && onBoard.length > 0
      ? [
          {
            id: "remove-picked",
            label: clearingLabel(onBoard.length),
            danger: true,
            press: () => setClearing(true),
          },
        ]
      : []),
  ];

  /** Hand the node's address to whatever browses the web here — #1406. */
  function openAddress(nodeId: string): void {
    void openStudioNode(props.onOpenAddress, studio.id, nodeId).then(setRefused);
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
          nodes={whiteboardNodes(studio, jobs, frameOf, board)}
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
          {editable && starts.length > 0 ? (
            <Card aria-label="Run">
              <CardContent>
                <StartRun starts={starts} saving={starting} onStart={start} />
              </CardContent>
            </Card>
          ) : null}
          {studio.nodes.length === 0 ? (
            <Card>
              <CardContent>Nothing on this Studio yet. Add a note, a link or a sketch to start it.</CardContent>
            </Card>
          ) : null}
          <StudioPicked
            picked={onBoard.map((id) => nodeNamed(studio, id, jobs, board))}
            acts={[...(editable ? promotion.acts : []), ...own]}
            onAct={(id) => {
              const mine = own.find((act) => act.id === id);
              return mine === undefined ? promotion.onAct(id) : mine.press();
            }}
          />
          {proposed.length === 0 ? null : (
            <Card className="armada-studio__proposals">
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
      {/* Everything picked, as one write — #1411. **The count is the name**, in the act, in the
          title and on the confirm, and what goes with them is said before the press. */}
      <Dialog
        open={clearing && onBoard.length > 0}
        tone="destructive"
        title={clearingLabel(onBoard.length)}
        confirmLabel={clearingLabel(onBoard.length)}
        onCancel={() => setClearing(false)}
        onConfirm={() => {
          const going = [...onBoard];
          setClearing(false);
          onSelectNode(null);
          void props.onRemoveNodes(going).then(answered);
        }}
      >
        {clearingSaid(clearingOf(studio, onBoard)).map((line) => (
          <p key={line}>{line}</p>
        ))}
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
/**
 * The `Run` control, on the board's own aside — #1345.
 *
 * **`AddNode`'s shape for `AddNode`'s reason**: `useStudioPlacement` reads the
 * viewport React Flow holds, and only a component drawn inside the board is
 * inside that provider. What it buys is the same rule — the Run node lands
 * where the person is looking rather than at the origin.
 */
function StartRun(props: {
  starts: readonly StudioStart[];
  saving: boolean;
  onStart: (start: StudioStart, position: StudioPosition) => void;
}) {
  const place = useStudioPlacement();
  const entries = studioStartEntries(props.starts);
  if (entries === undefined) return null;
  return (
    <DropdownMenu
      triggerLabel="Run"
      disabled={props.saving}
      entries={entries}
      onSelect={(id) => {
        const started = props.starts.find((one) => one.id === id);
        if (started !== undefined) props.onStart(started, place());
      }}
    />
  );
}

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
      readIn={READING_IN_UNBUILT}
      saving={props.saving}
      disabled={props.disabled}
    />
  );
}
