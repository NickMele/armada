import type { ReactNode } from "react";
import { useCallback, useEffect, useId, useState } from "react";

import { PhaseCard, phaseGlyph } from "../PhaseCard/PhaseCard";
import type { PhaseCardRow, PhaseStageKind, PhaseStageState } from "../PhaseCard/PhaseCard";

/**
 * Where this step is — a step's phases and its gate tiers, drawn as one
 * progression: Instructed, Working, Submitted, then its Checks, its Judge, and
 * you.
 *
 * **They are one strip because they are one progression.** A step that has
 * been submitted and is waiting on a Check is not in two places; drawing the
 * phases as a position marker and the gates as a separate row of chips made a
 * reader hold two readings of the same fact.
 *
 * **`You` closes the strip, always.** It is the last thing that can hold a
 * step, and a strip that stopped at the Judge said a step could only ever be
 * waiting on a machine. Where the workflow asks for no person the stage is
 * still drawn, still ahead and never lit — an absent tier is not a failed
 * tier.
 *
 * **The connectors are not decoration.** Six chips in a row with a gap between
 * them is a set; six chips joined by a line is an order, and the order is the
 * whole claim the strip makes.
 *
 * **Every stage is a control, and it opens on hover and pins on click.** The
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
   * What the stage is called on the strip — `Instructed`, `build, test`,
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

export type PhaseStripProps = {
  stages: PhaseStage[];
  /**
   * The label over the strip. **Sentence case, plain text** — the build drew it
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
   * chip by the class this component happens to ship. Hover is untouched by it:
   * hovering is a reading of the pointer's position, not a held decision, and a
   * caller holding it would have to be told about every mouse crossing.
   */
  pinnedStage?: string | null;
  /** Told when a stage is pinned or unpinned, for a caller that records it. */
  onPin?: (stageId: string | null) => void;
};

/** Strip glyphs are 12px at strokeWidth 2, as every mark on this screen is. */
const GLYPH = 12;
const STROKE = 2;

export function PhaseStrip({
  stages,
  label = "Where this step is",
  note,
  pinnedId,
  pinnedStage,
  onPin,
}: PhaseStripProps) {
  const [held, setHeld] = useState<string | null>(pinnedId ?? null);
  const [hovered, setHovered] = useState<string | null>(null);
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
  // twice under StrictMode and must be pure; a caller told twice that one chip
  // was clicked is a caller that recorded it twice.
  const pin = useCallback(
    (stageId: string) => {
      const next = pinned === stageId ? null : stageId;
      if (!controlled) setHeld(next);
      onPin?.(next);
    },
    [controlled, onPin, pinned],
  );

  // Escape unpins. A card held open over the strip is covering the thing it
  // explains, and the way out of it should not be finding the same chip again.
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

  return (
    <section className="armada-phases">
      {label === undefined ? null : <span className="armada-phases__label">{label}</span>}

      <ol className="armada-phases__strip">
        {stages.map((stage, at) => {
          const kind = stage.kind ?? "phase";
          const Mark = phaseGlyph(kind, stage.state);
          const opens = stage.opens ?? kind !== "phase";
          const isOpen = open === stage.id;
          // Which edge the card hangs off. Decided by position rather than by
          // measurement: a stage past the halfway point opens leftward, so the
          // card stays inside the panel without anything having to be measured
          // on a resize.
          const align = at * 2 >= stages.length ? "end" : "start";

          const chip = (
            <>
              {Mark === undefined ? null : (
                <Mark size={GLYPH} strokeWidth={STROKE} aria-hidden />
              )}
              {stage.label}
            </>
          );

          return (
            <li className="armada-phases__stage" key={stage.id}>
              {/* A connector before every stage but the first. Six chips with a
                  gap between them is a set; six joined by a line is an order. */}
              {at === 0 ? null : <span className="armada-phases__conn" aria-hidden />}

              {opens ? (
                <button
                  type="button"
                  className="armada-phases__control"
                  data-state={stage.state}
                  data-kind={kind}
                  data-open={isOpen || undefined}
                  data-pinned={pinned === stage.id || undefined}
                  aria-expanded={isOpen}
                  aria-controls={panelId}
                  onMouseEnter={() => setHovered(stage.id)}
                  onMouseLeave={() => setHovered((was) => (was === stage.id ? null : was))}
                  onFocus={() => setHovered(stage.id)}
                  onBlur={() => setHovered((was) => (was === stage.id ? null : was))}
                  onClick={() => pin(stage.id)}
                >
                  {chip}
                </button>
              ) : (
                <span className="armada-phases__control" data-state={stage.state} data-kind={kind}>
                  {chip}
                </span>
              )}

              {isOpen && shown !== null ? (
                <div className="armada-phases__pop" data-align={align} id={panelId} role="dialog">
                  <PhaseCard
                    floating
                    align={align}
                    kind={kind}
                    name={shown.label}
                    state={shown.state}
                    stands={shown.stands}
                    said={shown.said}
                    rows={shown.rows}
                    note={shown.cardNote}
                    detail={shown.detail}
                  />
                </div>
              ) : null}
            </li>
          );
        })}
      </ol>

      {note === undefined ? null : <p className="armada-phases__note">{note}</p>}
    </section>
  );
}
