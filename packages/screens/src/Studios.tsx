// A repository's Studios: the list, and one Studio open on its whiteboard — #1287.
//
// **Reopened read-only.** A Studio is kept to be reread (`docs/concepts/studio.md`), so one opened
// from the list moves nothing and draws no act until Continue; one a person just started opens
// editable. **A person's acts live here and nowhere else**: accepting or rejecting a proposed
// relation, and deleting a node, which confirms because its edges go with it. What is drawn is
// what Fleet wrote — every act answers with the Studio whole, and main folds it into `studio`.

import { useState } from "react";
import {
  Alert,
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Dialog,
  StudioWhiteboard,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeaderCell,
  TableRow,
} from "@armada/components";
import type { JobSummary, Outcome, StudioSummary } from "@armada/protocol";

import { said } from "./copy";
import { absoluteOf } from "./duration";
import { nodeNamed, proposedRelations, studioName, whiteboardEdges, whiteboardNodes } from "./studio";
import type { StudioAnswer, StudioRead, StudiosRead } from "./studio-reads";

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
  onMoveNode: (nodeId: string, position: { x: number; y: number }) => Promise<Outcome>;
  onRemoveNode: (nodeId: string) => Promise<Outcome>;
  onDecideEdge: (edgeId: string, accepted: boolean) => Promise<Outcome>;
};

export function Studios(props: StudiosProps) {
  return props.open === null ? <StudioList {...props} /> : <OpenedStudio {...props} open={props.open} />;
}

function StudioList({ studios, live, onOpen, onCreate }: StudiosProps) {
  const [creating, setCreating] = useState(false);
  const [refused, setRefused] = useState<string | null>(null);

  function create(): void {
    setCreating(true);
    setRefused(null);
    void onCreate().then((answer) => {
      setCreating(false);
      if (!answer.ok) setRefused(said(answer.outcome));
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
            <Alert tone="escalated" title="The Studio was not started">
              {refused}
            </Alert>
          )}
          <ListBody studios={studios} onOpen={onOpen} />
        </CardContent>
      </Card>
    </div>
  );
}

function ListBody({ studios, onOpen }: { studios: StudiosRead; onOpen: (studioId: string) => void }) {
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
          <Row key={one.id} studio={one} onOpen={onOpen} />
        ))}
      </TableBody>
    </Table>
  );
}

function Row({ studio, onOpen }: { studio: StudioSummary; onOpen: (studioId: string) => void }) {
  return (
    <TableRow>
      <TableCell>
        <Button variant="ghost" size="sm" onClick={() => onOpen(studio.id)}>
          {studioName(studio)}
        </Button>
      </TableCell>
      <TableCell>{absoluteOf(studio.touched_at) ?? studio.touched_at}</TableCell>
    </TableRow>
  );
}

function OpenedStudio(props: StudiosProps & { open: OpenStudio }) {
  const { studio, open, onBack } = props;
  if (studio.state === "read" && studio.studio.id === open.id) return <Board {...props} />;
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

function Board(props: StudiosProps & { open: OpenStudio }) {
  const { studio: read, open, jobs, live, selectedNode, onSelectNode, onBack, onContinue } = props;
  const [refused, setRefused] = useState<string | null>(null);
  const [removing, setRemoving] = useState<string | null>(null);
  const [deciding, setDeciding] = useState<string | null>(null);
  if (read.state !== "read") return null;
  const studio = read.studio;
  const editable = open.editable && live;
  const proposed = proposedRelations(studio, jobs);
  const selected = selectedNode === null ? undefined : studio.nodes.find((node) => node.id === selectedNode);

  function answered(outcome: Outcome): void {
    setRefused(outcome.ok ? null : said(outcome));
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
        <h2 className="armada-studio__name">{studioName(studio)}</h2>
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
          nodes={whiteboardNodes(studio, jobs)}
          edges={whiteboardEdges(studio)}
          readOnly={!editable}
          onNodeMoved={(nodeId, position) => {
            // The whiteboard refuses a move while read-only already. This is the second lock.
            if (!editable) return;
            void props.onMoveNode(nodeId, { x: Math.round(position.x), y: Math.round(position.y) }).then(answered);
          }}
          onSelectionChange={(ids) => onSelectNode(ids[0] ?? null)}
        >
          {studio.nodes.length === 0 ? (
            <Card className="armada-glass">
              <CardContent>Nothing on this Studio yet.</CardContent>
            </Card>
          ) : null}
          {selected === undefined ? null : (
            <Card className="armada-glass" aria-label="Selected node">
              <CardContent className="armada-studio__aside">
                <p>{nodeNamed(studio, selected.id, jobs)}</p>
                {editable ? (
                  <Button variant="destructive" size="sm" onClick={() => setRemoving(selected.id)}>
                    Delete node
                  </Button>
                ) : null}
              </CardContent>
            </Card>
          )}
          {proposed.length === 0 ? null : (
            <Card className="armada-glass" aria-label="Proposed relations">
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
