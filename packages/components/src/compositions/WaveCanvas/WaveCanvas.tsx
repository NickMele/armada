import {
  BaseEdge,
  Handle,
  MarkerType,
  getBezierPath,
  getNodesBounds,
  useReactFlow,
  useStore,
  type Edge,
  type EdgeProps,
  type FitViewOptions,
  type Node,
  type NodeProps,
} from "@xyflow/react";
import { useEffect, useMemo } from "react";

import { Badge } from "../../primitives/Badge/Badge";
import { JOB_STATUS } from "../../generated/vocabulary";
import { FactChip } from "../FactChip/FactChip";
import { GRAPH_CANVAS_SIDES, GraphCanvas, facingSides } from "../GraphCanvas/GraphCanvas";

/**
 * A wave of Jobs under one plan, drawn as the graph the plan recorded — one
 * card per Job, and an edge wherever one waits on another. `#1544`.
 *
 * **An edge runs from the Job waited on to the Job waiting**, so the one that
 * waits is drawn behind it. Reversed, the graph says the later work feeds the
 * earlier, which is the defect this surface was drawn to make visible.
 *
 * **Placement is the caller's**, computed from the waiting order — the same
 * rule `WorkflowCanvas` keeps, and the whole difference between this and a
 * Studio's whiteboard.
 */

/** One Job on the wave, as the card draws it. */
export type WaveCanvasCard = {
  /** The Job's id. Identity and what a press opens, never drawn. */
  job: string;
  title: string;
  /** A registered `job_status` wire value. The verb and glyph come from it. */
  status: string;
  /** What a person calls it, in mono. */
  handle?: string;
  /** Short values under the title — `merged`, `waits on 2`. */
  facts?: readonly string[];
  /** Open the Job. Absent draws a card that is not a control. */
  onOpen?: () => void;
};

export type WaveCanvasNode = {
  id: string;
  /** Where it sits, in the canvas's own coordinates. Computed from the order. */
  position: { x: number; y: number };
  card: WaveCanvasCard;
};

export type WaveCanvasEdge = {
  id: string;
  /** The Job waited on. */
  source: string;
  /** The Job that waits, drawn behind it. */
  target: string;
};

export type WaveCanvasProps = {
  nodes: readonly WaveCanvasNode[];
  edges: readonly WaveCanvasEdge[];
  /** What the wave is, read to somebody who cannot see it. */
  label: string;
  /**
   * Where to open when the whole wave cannot be drawn and still be read. A
   * wave fitted below the legible floor draws cards nobody can read, which is
   * the v1 complaint `docs/practices/bridge.md` names.
   */
  opensOn?: readonly string[];
};

type WaveNode = Node<{ card: WaveCanvasCard }, "wave">;
type WaveEdge = Edge<Record<string, never>, "wave">;

/** What the status reads as, off the registry. A wire value with no row shows itself. */
function JobCard({ card }: { card: WaveCanvasCard }) {
  const rendering = JOB_STATUS[card.status];
  const drawable = rendering?.verb != null && rendering.badgeStatus != null && rendering.icon != null;
  const said = drawable ? rendering.verb : card.status;
  const body = (
    <>
      <span className="armada-wave-card__head">
        {drawable && rendering?.badgeStatus != null && rendering.icon != null ? (
          <Badge
            status={rendering.badgeStatus}
            icon={rendering.icon}
            pulsing={card.status === "running"}
          >
            {rendering.verb}
          </Badge>
        ) : (
          <span
            className="armada-wave-card__state"
            title={`No verb in the registry for ${card.status}`}
          >
            {card.status}
          </span>
        )}
        {card.handle === undefined ? null : (
          <span className="armada-wave-card__handle mono" title={card.handle}>
            {card.handle}
          </span>
        )}
      </span>
      <span className="armada-wave-card__title">{card.title}</span>
      {card.facts === undefined || card.facts.length === 0 ? null : (
        <span className="armada-wave-card__facts">
          {card.facts.map((fact) => (
            <FactChip key={fact}>{fact}</FactChip>
          ))}
        </span>
      )}
    </>
  );
  const attributes = { className: "armada-wave-card", "data-status": card.status };
  const name = `${card.title}, ${said ?? card.status}`;
  return card.onOpen === undefined ? (
    <span {...attributes} role="group" aria-label={name}>
      {body}
    </span>
  ) : (
    <button {...attributes} type="button" aria-label={name} onClick={card.onOpen}>
      {body}
    </button>
  );
}

function NodeView({ data }: NodeProps<WaveNode>) {
  return (
    <>
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`t-${side}`} id={`t-${side}`} type="target" position={side} isConnectable={false} />
      ))}
      <JobCard card={data.card} />
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`s-${side}`} id={`s-${side}`} type="source" position={side} isConnectable={false} />
      ))}
    </>
  );
}

function EdgeView(props: EdgeProps<WaveEdge>) {
  const [path] = getBezierPath(props);
  return <BaseEdge id={props.id} path={path} markerEnd={props.markerEnd} />;
}

const NODE_TYPES = { wave: NodeView };
const EDGE_TYPES = { wave: EdgeView };

/** How far out a person may take the wave by hand, to read its shape. */
const FURTHEST_OUT = 0.2;

/** Below this a fit draws cards nobody can read, so it opens on part of the wave instead. */
const SMALLEST_READABLE = 0.6;

/** What a fit leaves around the wave, as a factor on the scale. */
const ROOM_TO_BREATHE = 0.9;

/**
 * Fit the wave again whenever the frame changes size. React Flow fits once, at
 * the size it was first measured at, and Helm's dock takes 380px out of the
 * content after that first measure.
 */
function FitsTheFrame({
  options,
  opensOn,
}: {
  options: FitViewOptions;
  opensOn: readonly string[] | undefined;
}) {
  const flow = useReactFlow();
  const width = useStore((state) => state.width);
  const height = useStore((state) => state.height);
  useEffect(() => {
    if (width === 0 || height === 0) return;
    const whole = getNodesBounds(flow.getNodes());
    if (whole.width === 0 || whole.height === 0) return;
    const scale = Math.min(width / whole.width, height / whole.height) * ROOM_TO_BREATHE;
    const readable = scale >= SMALLEST_READABLE || opensOn === undefined || opensOn.length === 0;
    void flow.fitView(readable ? options : { ...options, nodes: opensOn.map((id) => ({ id })) });
  }, [flow, options, opensOn, width, height]);
  return null;
}

export function WaveCanvas({ nodes: given, edges: givenEdges, label, opensOn }: WaveCanvasProps) {
  const nodes = useMemo<WaveNode[]>(
    () =>
      given.map((entry) => ({
        id: entry.id,
        position: entry.position,
        type: "wave" as const,
        draggable: false,
        // Selectable is what gives the node pointer events at all — React Flow
        // sets `pointer-events: none` on one nothing can select, and the pane
        // behind it then swallows every press on the card.
        selectable: true,
        focusable: false,
        data: { card: entry.card },
      })),
    [given],
  );

  const edges = useMemo<WaveEdge[]>(() => {
    const placed = new Map(nodes.map((node) => [node.id, node]));
    const named = new Map(given.map((entry) => [entry.id, entry.card.title]));
    return givenEdges.map((edge) => {
      const from = named.get(edge.source) ?? edge.source;
      const to = named.get(edge.target) ?? edge.target;
      return {
        id: edge.id,
        source: edge.source,
        target: edge.target,
        ...facingSides(placed.get(edge.source), placed.get(edge.target)),
        type: "wave" as const,
        ariaLabel: `${to} waits on ${from}`,
        markerEnd: { type: MarkerType.ArrowClosed },
        data: {},
      };
    });
  }, [given, givenEdges, nodes]);

  const fitViewOptions = useMemo(() => ({ maxZoom: 1, minZoom: SMALLEST_READABLE }), []);

  return (
    <GraphCanvas<WaveNode, WaveEdge>
      surface="armada-wave-canvas"
      label={label}
      nodes={nodes}
      edges={edges}
      nodeTypes={NODE_TYPES}
      edgeTypes={EDGE_TYPES}
      controls="signs"
      minZoom={FURTHEST_OUT}
      fitViewOptions={fitViewOptions}
    >
      <FitsTheFrame options={fitViewOptions} opensOn={opensOn} />
    </GraphCanvas>
  );
}
