import type { CSSProperties, ReactNode } from "react";
import { Fragment, useCallback, useEffect, useId, useLayoutEffect, useRef, useState } from "react";

import { PhaseCard, phaseGlyph } from "../PhaseCard/PhaseCard";
import type { PhaseCardRow, PhaseStageKind, PhaseStageState } from "../PhaseCard/PhaseCard";

/**
 * The path this step took — its phases and its gate tiers as one graph:
 * Instructed, Working, Submitted, then its Checks, its Judge, and you, with an
 * edge for every hand-back that sent it round again.
 *
 * **They are one graph because they are one progression.** A step that has
 * been submitted and is waiting on a Check is not in two places; drawing the
 * phases as a position marker and the gates as a separate row of chips made a
 * reader hold two readings of the same fact.
 *
 * **A step worked three times used to draw what a step worked once draws.** Six
 * chips joined by five straight connectors can say what a step is doing and
 * cannot say that it has been here before — so the hand-backs, which are the
 * most important thing that happened to the step, were the one thing the
 * drawing discarded. A `loops` edge is where they go.
 *
 * **One rendering, not two.** A step with no loop is a straight chain of nodes,
 * which is what the strip always drew; a step with a loop draws the loop over
 * the same chain. There is no variant prop keeping a second vocabulary alive
 * beside the first.
 *
 * **`You` closes the graph, always.** It is the last thing that can hold a
 * step, and a graph that stopped at the Judge said a step could only ever be
 * waiting on a machine. Where the workflow asks for no person the stage is
 * still drawn and still last, in the `never` state and under its own label —
 * an absent tier is not a failed tier, and a tier that can never ask for you
 * is not one that has not reached you yet.
 *
 * **Every node is a control, and it opens on hover and pins on click.** The
 * drawing specifies click and hovering is what was asked for; a card that does
 * both satisfies each without inventing a third behaviour. Hovering away
 * closes an unpinned card and leaves a pinned one, `Escape` unpins, and
 * keyboard focus opens the same card a cursor does.
 *
 * **An absent tier is not a failed tier.** A step that declares no Check and
 * no Judge passes neither stage, and `note` is where it says what does advance
 * it. An empty gate drawn greyed out says the gate failed to render.
 */
export type { PhaseCardRow, PhaseStageKind, PhaseStageState };

/**
 * A row inside an opened stage. The card's row, under the strip's name for it
 * — a caller building a strip reaches for this and never has to know the card
 * exists, which is the whole point of the strip owning the composition.
 */
export type PhaseStageRow = PhaseCardRow;

export type PhaseStage = {
  id: string;
  /**
   * What the stage is called on the graph — `Instructed`, `build, test`,
   * `Judge · 2 criteria`, `You`. **The Checks tier names its commands rather
   * than counting them** while two fit; past three it counts, and the story's
   * Produced chapter lists them.
   */
  label: ReactNode;
  kind?: PhaseStageKind;
  state: PhaseStageState;
  /**
   * Where it stands, in the caller's words — what it is waiting on, or what it
   * came to. Drawn in the card's header.
   */
  stands?: ReactNode;
  /** The Checks and their exit codes, or the criteria and their verdicts. */
  rows?: PhaseCardRow[];
  /** What the tier is. Defaults to the standing sentence for its kind. */
  said?: ReactNode | null;
  /** What the rows do not say, on the card's own well. */
  cardNote?: ReactNode;
  /** The card's closing line. Defaults to the standing one for its kind. */
  detail?: ReactNode | null;
  /**
   * Whether the stage opens a card at all. A phase with nothing to say — most
   * of `Instructed`, `Working`, `Submitted` — draws as a marker rather than as
   * a control that opens an empty box.
   */
  opens?: boolean;
};

/**
 * An edge the chain has no room for: a ruling that sent the step backwards.
 *
 * **By stage id rather than by a `retried` flag**, because the two loops this
 * app has do not return to the same place. A failed Check hands the step back
 * to the Drone; a person's `request_changes` re-briefs it. Both are loops, and
 * a boolean could only ever have drawn one of them.
 *
 * **`says` is the whole label.** The count belongs in the words — *handed back
 * ×2* — rather than in a number beside them, because how a hand-back is worded
 * is the caller's (a Check hands back, a person returns) and a second spelling
 * of one fact is the drift this package deletes on sight.
 */
export type PhaseLoop = {
  /** The stage the ruling came from. */
  from: string;
  /** The stage it returned to. Earlier than `from`, or there is no loop. */
  to: string;
  /** What the ruling was, in the caller's words — `handed back ×2`. */
  says: ReactNode;
};

export type PhaseStripProps = {
  stages: PhaseStage[];
  /**
   * The hand-backs. Drawn above the chain, widest first, each in a lane of its
   * own — a loop crossing another is a graph a reader has to untangle, and the
   * order they nest in is the order they contain each other.
   *
   * A loop naming a stage that is not here, or returning forwards, is dropped:
   * the ids are the caller's own and there is nothing to draw between two
   * stages that do not exist.
   */
  loops?: PhaseLoop[];
  /**
   * The label over the graph. **Sentence case, plain text** — the build drew it
   * as an uppercase caption, and a caption announces a region where this
   * introduces a line.
   */
  label?: ReactNode;
  /**
   * The sentence beneath — where the step stands, in the panel's own voice.
   * *The Drone is working. Nothing has been submitted, so no gate has been
   * asked anything yet.*
   *
   * **One sentence describing the state, not a paragraph describing the
   * menu.** What each act does belongs on that act's tooltip with its binding,
   * and this is also where an ungated step says what advances it instead of
   * drawing an empty gate.
   */
  note?: ReactNode;
  /** Which stage is pinned on mount. After that the strip holds its own. */
  pinnedId?: string;
  /**
   * Which stage is pinned, held by the caller. **Present makes the strip
   * controlled**: it draws what this says and pins nothing itself, and `onPin`
   * is the only way the value moves — including `Escape`, which reports rather
   * than unpins. `null` is a strip with nothing pinned.
   *
   * This exists so a keyboard map can open a stage by id instead of finding the
   * node by the class this component happens to ship. Hover is untouched by it:
   * hovering is a reading of the pointer's position, not a held decision, and a
   * caller holding it would have to be told about every mouse crossing.
   */
  pinnedStage?: string | null;
  /** Told when a stage is pinned or unpinned, for a caller that records it. */
  onPin?: (stageId: string | null) => void;
  /**
   * Told when a row inside an open stage names an artifact and is pressed —
   * a Check's output, a panel's judgment. Handed straight to `PhaseCard`.
   *
   * **The graph is where a person finds out a tier went red**, so it is where
   * they should be able to go and read why. Without this they close the card
   * and hunt for the same Check in the story below.
   */
  onOpenArtifact?: (artifactId: string) => void;
};

/**
 * The custom property the card's arrow reads, written here and used in
 * `PhaseCard.css`. Named in one place because it crosses two components: the
 * strip is the only thing that knows where a node is, and the card is the only
 * thing that draws the arrow.
 */
const ARROW = "--armada-phase-arrow";

/** Strip glyphs are 12px at strokeWidth 2, as every mark on this screen is. */
const GLYPH = 12;
const STROKE = 2;

/**
 * The drawing's coordinates, in CSS pixels, and the one place they are written
 * as numbers.
 *
 * **Every `viewBox` below is 1:1 with the box CSS gives it**, so nothing is
 * scaled and no arrowhead is stretched: `EDGE` is `--phase-edge` and `NODE` is
 * `--phase-node`, both resolved in `PhaseStrip.css`. Change one and change the
 * other — a mismatch does not fail, it squashes the chevrons.
 */
const EDGE = 20;
const NODE = 24;
/** Where a loop turns: the corner's box, half a node tall. */
const TURN = NODE / 2;
/** The arrowhead's box. Small enough that the stem above it still reads. */
const HEAD = 8;

/**
 * Which grid column a thing sits in. Columns alternate edge, node, edge, node
 * …, starting and ending on an edge, so every node has a column on each side
 * for an arrow or for a loop's stem to come down in.
 *
 * **Hand-computed, and deliberately so.** Six nodes and at most a couple of
 * loops is a fixed shape rather than a graph needing a layout engine; a
 * function from an index to a column cannot surprise anybody at runtime, and
 * it adds no dependency.
 */
const edgeBefore = (at: number) => at * 2 + 1;
const nodeAt = (at: number) => at * 2 + 2;
const edgeAfter = (at: number) => at * 2 + 3;

/**
 * The node row, named from the end. Loops take a lane each above it, so
 * counting from the top would move every node whenever a hand-back arrived.
 */
const NODE_ROW = "-2 / -1";

export function PhaseStrip({
  stages,
  loops = [],
  label = "Where this step is",
  note,
  pinnedId,
  pinnedStage,
  onPin,
  onOpenArtifact,
}: PhaseStripProps) {
  const [held, setHeld] = useState<string | null>(pinnedId ?? null);
  const [hovered, setHovered] = useState<string | null>(null);
  const frameRef = useRef<HTMLDivElement>(null);
  const graphRef = useRef<HTMLDivElement>(null);
  const popRef = useRef<HTMLDivElement>(null);
  // Every node, by stage id — markers as well as controls, because where the
  // step is is as often a phase as a tier. Held rather than queried: a selector
  // into this component's own class names is the coupling `detail-keys.ts` had
  // to be talked out of.
  const nodes = useRef(new Map<string, HTMLElement>());
  // Controlled by presence, not by a flag: a caller either holds the value or
  // it does not, and a boolean beside it is a second answer that can disagree.
  const controlled = pinnedStage !== undefined;
  const pinned = controlled ? pinnedStage : held;
  // Two strips on one page is the gallery, every day. A fixed id would point
  // every stage on the second strip at the first strip's card.
  const panelId = useId();

  const open = pinned ?? hovered;
  const shown = stages.find((stage) => stage.id === open) ?? null;

  // `onPin` is called here rather than inside the updater. A state updater runs
  // twice under StrictMode and must be pure; a caller told twice that one node
  // was clicked is a caller that recorded it twice.
  const pin = useCallback(
    (stageId: string) => {
      const next = pinned === stageId ? null : stageId;
      if (!controlled) setHeld(next);
      onPin?.(next);
    },
    [controlled, onPin, pinned],
  );

  // Escape unpins. A card held open over the graph is covering the thing it
  // explains, and the way out of it should not be finding the same node again.
  // A controlled strip is told rather than unpinned: the caller holds the value
  // and this would otherwise be a second hand on it.
  useEffect(() => {
    if (pinned === null) return;
    function onKey(event: KeyboardEvent) {
      if (event.key !== "Escape") return;
      if (!controlled) setHeld(null);
      onPin?.(null);
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [controlled, pinned, onPin]);

  const drawn = drawnLoops(stages, loops);
  // The card hangs off the graph rather than off the node, because the graph
  // scrolls when it outgrows the panel and a scroll container clips what hangs
  // out of it. Which edge it takes is decided by position and never measured —
  // a stage past the halfway point opens leftward, so the card stays inside the
  // panel whatever the window does. Only its arrow is measured, below.
  const openAt = stages.findIndex((stage) => stage.id === open);
  const align = openAt * 2 >= stages.length ? "end" : "start";

  // Where the step actually is: the furthest stage it has reached, which is the
  // last one not still ahead of it and not one it can never reach.
  const live = stages.reduce<string | null>(
    (found, stage) => (stage.state === "ahead" || stage.state === "never" ? found : stage.id),
    null,
  );

  // The graph opens on that node rather than on its own beginning.
  //
  // **A graph that scrolls can hide the end of itself, and the end is the
  // answer.** Below the layout breakpoint the six standard tiers do not fit the
  // panel, and a strip that opened at Instructed would show only the part
  // already cleared — `Judge` and `You` off the edge, on a step that is waiting
  // on you.
  //
  // **It moves nothing that is already right.** A jump on mount that was not
  // needed is worse than never moving, so the node is measured first, and only
  // the graph's own scroll box is touched — `scrollIntoView` would take the
  // panel with it.
  useLayoutEffect(() => {
    const graph = graphRef.current;
    const node = live === null ? undefined : nodes.current.get(live);
    if (graph === null || node === undefined) return;

    const box = graph.getBoundingClientRect();
    const at = node.getBoundingClientRect();
    // The graph's padding is the focus ring's room. Scrolling a node flush to
    // the clip edge would put its ring under it.
    const lead = parseFloat(getComputedStyle(graph).paddingLeft) || 0;
    const trail = parseFloat(getComputedStyle(graph).paddingRight) || 0;

    if (at.right > box.right - trail) graph.scrollLeft += at.right - box.right + trail;
    else if (at.left < box.left + lead) graph.scrollLeft -= box.left - at.left + lead;
  }, [live]);

  // Where the card's arrow points. The card is anchored to the graph, so
  // nothing about its position says which node opened it — and the arrow sat
  // at a fixed inset, pointing at the first node on a leading card and the
  // last on a trailing one whatever was hovered. At 880px of card that is
  // plainly the wrong node.
  //
  // **Measured, because there is nothing else to measure it against.** A node
  // is as wide as its label, the graph scrolls, and the card is anchored to a
  // different box than the node is in; no arrangement of tokens gives the
  // distance between them. It is recomputed on scroll and on resize for the
  // same reason — an offset taken once is wrong the moment either moves.
  useLayoutEffect(() => {
    const graph = graphRef.current;
    const frame = frameRef.current;
    const pop = popRef.current;
    const node = open === null ? undefined : nodes.current.get(open);
    if (graph === null || frame === null || pop === null || node === undefined) return;

    function point() {
      const at = node!.getBoundingClientRect();
      const box = frame!.getBoundingClientRect();
      const middle = at.left + at.width / 2;
      pop!.style.setProperty(
        ARROW,
        `${align === "end" ? box.right - middle : middle - box.left}px`,
      );
    }

    point();
    graph.addEventListener("scroll", point, { passive: true });
    // The graph resizes when the panel does and when a label changes, and both
    // move every node after the one that changed.
    const watching = new ResizeObserver(point);
    watching.observe(graph);
    return () => {
      graph.removeEventListener("scroll", point);
      watching.disconnect();
    };
  }, [open, align]);

  return (
    <section className="armada-phases">
      {label === undefined ? null : <span className="armada-phases__label">{label}</span>}

      <div className="armada-phases__frame" ref={frameRef}>
        <div
          className="armada-phases__graph"
          ref={graphRef}
          style={{
            gridTemplateColumns: columnsOf(stages.length, drawn),
            gridTemplateRows: `repeat(${drawn.length}, var(--phase-loop)) var(--phase-node)`,
          }}
        >
          {/* An edge before every stage but the first, and one before the first
              where a loop returns to it — an arrowhead pointing at nothing is
              what a hand-back to the start would otherwise draw. */}
          {drawn.some((loop) => loop.to === 0) ? <ChainEdge column={edgeBefore(0)} /> : null}

          {stages.map((stage, at) => {
            const kind = stage.kind ?? "phase";
            const Mark = phaseGlyph(kind, stage.state);
            const opens = stage.opens ?? kind !== "phase";
            const isOpen = open === stage.id;
            // A loop is read out at both of its ends, because a shape is not
            // what a screen reader hears — the arc says nothing to it, and the
            // words are the whole of what it can be told.
            //
            // **Both ends, because only one of them is reliably a control.** A
            // hand-back returns to a phase, and a phase with nothing to say
            // draws as a marker rather than as a control that opens an empty
            // box; a description on it would be a description on a `span`,
            // which nothing announces. The tier that ruled is always a control,
            // so putting it there as well is what makes the loop audible at all.
            const returns = drawn
              .filter((loop) => loop.to === at || loop.from === at)
              .map((loop) => `${panelId}-loop-${loop.key}`);

            const node = (
              <>
                {Mark === undefined ? null : (
                  <Mark size={GLYPH} strokeWidth={STROKE} aria-hidden />
                )}
                {stage.label}
              </>
            );
            const place: CSSProperties = { gridColumn: nodeAt(at), gridRow: NODE_ROW };

            return (
              <Fragment key={stage.id}>
                {at === 0 ? null : <ChainEdge column={edgeBefore(at)} />}

                {opens ? (
                  <button
                    type="button"
                    className="armada-phases__node"
                    style={place}
                    ref={(held) => {
                      if (held === null) nodes.current.delete(stage.id);
                      else nodes.current.set(stage.id, held);
                    }}
                    data-state={stage.state}
                    data-kind={kind}
                    data-open={isOpen || undefined}
                    data-pinned={pinned === stage.id || undefined}
                    aria-expanded={isOpen}
                    aria-controls={panelId}
                    aria-describedby={returns.length === 0 ? undefined : returns.join(" ")}
                    onMouseEnter={() => setHovered(stage.id)}
                    onMouseLeave={() => setHovered((was) => (was === stage.id ? null : was))}
                    onFocus={() => setHovered(stage.id)}
                    onBlur={() => setHovered((was) => (was === stage.id ? null : was))}
                    onClick={() => pin(stage.id)}
                  >
                    {node}
                  </button>
                ) : (
                  <span
                    className="armada-phases__node"
                    style={place}
                    ref={(held) => {
                      if (held === null) nodes.current.delete(stage.id);
                      else nodes.current.set(stage.id, held);
                    }}
                    data-state={stage.state}
                    data-kind={kind}
                  >
                    {node}
                  </span>
                )}

                {/* Written into the document beside the node it returns to,
                    though the grid draws it above them all. Reading order is
                    the only order a screen reader has, and "Working, handed
                    back ×2, Submitted" is the path — the same loop written
                    after the last node would be a phrase about nothing. */}
                {drawn.map((loop, lane) =>
                  loop.to === at ? (
                    <Loop
                      key={loop.key}
                      loop={loop}
                      lane={lane}
                      says={`${panelId}-loop-${loop.key}`}
                    />
                  ) : null,
                )}
              </Fragment>
            );
          })}
        </div>

        {shown === null ? null : (
          <div
            className="armada-phases__pop"
            data-align={align}
            id={panelId}
            role="dialog"
            ref={popRef}
          >
            <PhaseCard
              floating
              align={align}
              kind={shown.kind ?? "phase"}
              name={shown.label}
              state={shown.state}
              stands={shown.stands}
              said={shown.said}
              rows={shown.rows}
              note={shown.cardNote}
              detail={shown.detail}
              onOpenArtifact={onOpenArtifact}
            />
          </div>
        )}
      </div>

      {note === undefined ? null : <p className="armada-phases__note">{note}</p>}
    </section>
  );
}

/** A loop placed on the graph: its ends as indices, and the lane it takes. */
type Drawn = { key: string; from: number; to: number; state: PhaseStageState; says: ReactNode };

/**
 * The loops that can be drawn, outermost first.
 *
 * **Widest first**, so a loop that contains another is drawn above it and the
 * two never cross. Nesting is the only arrangement two backwards edges over one
 * chain can have, so this is a sort and not a layout.
 */
function drawnLoops(stages: PhaseStage[], loops: PhaseLoop[]): Drawn[] {
  const at = (id: string) => stages.findIndex((stage) => stage.id === id);
  return loops
    .map((loop, i) => ({
      key: `${i}`,
      from: at(loop.from),
      to: at(loop.to),
      // The hue of the ruling that produced the edge, which is the state of the
      // tier that ruled. Anything `status.css` does not declare a hue for is
      // neutral, as every other mark below Job level is.
      state: stages[at(loop.from)]?.state ?? "ahead",
      says: loop.says,
    }))
    .filter((loop) => loop.to >= 0 && loop.from > loop.to)
    .sort((a, b) => b.from - b.to - (a.from - a.to));
}

/**
 * The track list. Node columns take their labels' width and edge columns are
 * fixed, so the whole layout follows from the number of stages.
 *
 * The two outermost edge columns collapse unless a loop needs them: a strip
 * with no hand-back would otherwise start and end with a 20px indent nothing
 * is drawn in.
 */
function columnsOf(count: number, loops: Drawn[]): string {
  const lead = loops.some((loop) => loop.to === 0);
  const trail = loops.some((loop) => loop.from === count - 1);
  const edge = "var(--phase-edge)";
  const columns = [lead ? edge : "0"];
  for (let at = 0; at < count; at += 1) {
    columns.push("max-content");
    columns.push(at === count - 1 && !trail ? "0" : edge);
  }
  return columns.join(" ");
}

/** One step of the chain: a line to the next node, and an arrowhead on it. */
function ChainEdge({ column }: { column: number }) {
  return (
    <svg
      className="armada-phases__chain"
      style={{ gridColumn: column, gridRow: NODE_ROW }}
      viewBox={`0 0 ${EDGE} ${NODE}`}
      aria-hidden
    >
      <path d={`M0 ${TURN + 0.5}H${EDGE - 1}`} />
      <path d={`M${EDGE - 5} ${TURN - 3.5}L${EDGE - 1} ${TURN + 0.5}L${EDGE - 5} ${TURN + 4.5}`} />
    </svg>
  );
}

/**
 * A hand-back, drawn from the node it left to the node it returned to.
 *
 * **Three pieces, because only two of them are curved.** The corners and the
 * arrowhead are fixed boxes with their `viewBox` 1:1 to their CSS size, and
 * the run between them is a straight rule that stretches. A single stretched
 * SVG would have to squash its arrowhead to fit, which is the one part of the
 * drawing a reader has to recognise at a glance.
 */
function Loop({ loop, lane, says }: { loop: Drawn; lane: number; says: string }) {
  return (
    <div
      className="armada-phases__loop"
      data-state={loop.state}
      style={{
        gridColumn: `${edgeBefore(loop.to)} / ${edgeAfter(loop.from) + 1}`,
        gridRow: `${lane + 1} / -1`,
      }}
    >
      <span className="armada-phases__cap">
        <svg className="armada-phases__turn" viewBox={`0 0 ${EDGE} ${TURN}`} aria-hidden>
          <path d={`M${EDGE} 0.5H${EDGE - 6}A4 4 0 0 0 ${EDGE - 10} 4.5V${TURN}`} />
        </svg>
        <span className="armada-phases__stem" />
        <svg className="armada-phases__head" viewBox={`0 0 ${EDGE} ${HEAD}`} aria-hidden>
          <path d={`M${EDGE / 2 - 4} ${HEAD - 4.5}L${EDGE / 2} ${HEAD}L${EDGE / 2 + 4} ${HEAD - 4.5}`} />
        </svg>
      </span>

      <span className="armada-phases__run">
        <span className="armada-phases__rule" />
        <span className="armada-phases__says" id={says}>
          {loop.says}
        </span>
      </span>

      <span className="armada-phases__cap">
        <svg className="armada-phases__turn" viewBox={`0 0 ${EDGE} ${TURN}`} aria-hidden>
          <path d={`M0 0.5H6A4 4 0 0 1 10 4.5V${TURN}`} />
        </svg>
        <span className="armada-phases__stem" />
      </span>
    </div>
  );
}
