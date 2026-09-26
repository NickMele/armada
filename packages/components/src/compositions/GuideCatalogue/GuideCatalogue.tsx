import { useRef, useState, type CSSProperties, type KeyboardEvent, type PointerEvent } from "react";

import { GUIDE_GROUPS, GUIDES } from "../../guides";
import type { Guide, GuideGroupId } from "../../guides/guide";
import { Sheet } from "../../primitives/Sheet/Sheet";

/**
 * Every guide: the list on the left, the one you chose open beside it.
 *
 * **The shape is the owner's**, 23 September 2026, against No Man's Sky and
 * Crimson Desert — a grouped list with a count per group, the chosen row
 * marked, and the entry open beside it.
 *
 * **It replaces the document #1607 drew**, which rendered all fourteen one
 * after another. A person comes here having met one word and wanting one
 * guide, and finding it in a scroll of fourteen is the cost the list removes.
 *
 * **Under `--layout-breakpoint` the guide is a sheet over the list** —
 * `narrow`, the same fold job detail's inspector takes. The choice survives
 * the sheet closing, so widening the window draws the guide last read.
 */
export type GuideCatalogueProps = {
  /** Every guide, in catalogue order. The real set by default. */
  guides?: readonly Guide[];
  /**
   * The guide open on arrival, by number. Absent opens the first, so the panel
   * is never empty.
   *
   * **Nothing in Bridge passes one yet.** It is the seam a deep link would
   * take — a card's `Read all guides` naming the guide it was showing — and
   * wiring it needs `GuidanceProvider.onReadAll` to carry the guide and `App`
   * to hold it. Under `narrow` a named guide arrives with its sheet up: that
   * press asked for a guide, not for the list.
   */
  arriveAt?: number;
  /** The window is under `--layout-breakpoint`: the guide folds to a sheet. */
  narrow?: boolean;
  /** The window is at `--window-floor`. The folded sheet goes flush. */
  floor?: boolean;
  /**
   * The list column's resting width in px. Absent draws `--w-guide-list`.
   * Ignored while `narrow`, where the list is the whole content and there is
   * no second column to take width from.
   */
  listWidth?: number;
  /**
   * Drags and arrow-key nudges the list's inner edge, clamped between
   * `--w-guide-list-min` and `--w-guide-list-max` — `clampGuideListWidth`.
   * **Absent draws no handle at all**, the shell's own reasoning: an edge that
   * looks grabbable and does nothing is worse than no edge. Drawn nowhere
   * under `narrow`, for the same reason `listWidth` is ignored there.
   * Persisting the result across a restart is the caller's, the same way the
   * shell's left column is.
   */
  onResizeList?: (width: number) => void;
};

export function GuideCatalogue({
  guides = GUIDES,
  arriveAt,
  narrow = false,
  floor = false,
  listWidth,
  onResizeList,
}: GuideCatalogueProps) {
  const sections = GUIDE_GROUPS.map((group) => ({
    group,
    entries: guides.filter((one) => one.group === group.id),
  })).filter((section) => section.entries.length > 0);

  const landed = guides.find((one) => one.number === arriveAt) ?? guides[0];
  const [chosen, setChosen] = useState<number | undefined>(landed?.number);
  // Narrow arrives on the list, which is what a rail press asked for. A deep
  // link asked for a guide, so that one is already up.
  const [reading, setReading] = useState(arriveAt !== undefined);

  // A second arrival naming a different guide — a card's `Read all guides`
  // pressed while the catalogue is mounted. Adjusted during render rather than
  // in an effect, which would draw the old guide for a frame first.
  const [arrived, setArrived] = useState(arriveAt);
  if (arriveAt !== arrived) {
    setArrived(arriveAt);
    if (arriveAt !== undefined) {
      setChosen(arriveAt);
      setReading(true);
    }
  }

  const open = guides.find((one) => one.number === chosen) ?? landed;

  // The width whether or not a caller passed one, clamped before anything
  // draws from it: the handle's `aria-valuenow`, its drag anchor and the track
  // itself all read this one figure, so a width remembered from a build with
  // different tokens cannot show a number the column on screen disagrees with.
  const width = clampGuideListWidth(listWidth ?? defaultGuideListWidth());

  return (
    <div
      className="armada-guides"
      data-narrow={narrow || undefined}
      // The track's own property rather than a whole `grid-template-columns`:
      // the stylesheet keeps the layout and the token stays its resting value,
      // and the one thing a drag changes is the one thing it moved.
      style={
        narrow || listWidth === undefined
          ? undefined
          : ({ "--armada-guide-list": `${width}px` } as CSSProperties)
      }
    >
      <div className="armada-guides__list">
        {sections.map(({ group, entries }) => (
          <section key={group.id} className="armada-guides__group" aria-label={group.title}>
            {/* The count is the group's own guides. Not a tally of what has
                been read: the reference's `11/23` is a game's collectibles. */}
            <h2 className="armada-guides__group-title">
              {group.title}
              <span className="armada-guides__count mono">{entries.length}</span>
            </h2>
            <ul className="armada-guides__entries">
              {entries.map((guide) => (
                <li key={guide.number}>
                  <button
                    type="button"
                    className="armada-guides__row"
                    // The card's own naming, so one guide reads as one name
                    // wherever it is pressed: the mark, the card and this row.
                    aria-label={`Guide ${guide.number}, ${guide.title}`}
                    aria-current={guide.number === open?.number ? "true" : undefined}
                    onClick={() => {
                      setChosen(guide.number);
                      setReading(true);
                    }}
                  >
                    <span className="armada-guides__number mono">{guide.number}</span>
                    <span className="armada-guides__row-title">{guide.title}</span>
                  </button>
                </li>
              ))}
            </ul>
          </section>
        ))}
      </div>

      {/* The list's inner edge. Nothing here under `narrow`: the list is the
          whole content there and the guide is a layer over it, so there is no
          second column for a drag to take width from. */}
      {narrow || onResizeList === undefined ? null : (
        <ListHandle width={width} onResize={onResizeList} />
      )}

      {/* Beside the list wherever the window can pay for both. */}
      {narrow || open === undefined ? null : (
        <article className="armada-guides__panel">
          <div className="armada-guides__panel-head">
            <p className="armada-guides__eyebrow">
              <span className="mono">Guide {open.number}</span>
              <span aria-hidden="true">·</span>
              <span>{groupTitle(open.group)}</span>
            </p>
            <h2 className="armada-guides__panel-title">{open.title}</h2>
          </div>
          <GuideBody guide={open} />
        </article>
      )}

      {/* Folded, the guide is the screen's layer and not the window's, so the
          shell's rail stays out from under it. The sheet's own head carries
          the title and the eyebrow, so the panel head is not drawn twice. */}
      {narrow && open !== undefined ? (
        <Sheet
          open={reading}
          contained
          floor={floor}
          title={open.title}
          subtitle={`Guide ${open.number} · ${groupTitle(open.group)}`}
          closeLabel="Close"
          closeBinding="Esc"
          bleed
          onClose={() => setReading(false)}
        >
          <div className="armada-guides__folded">
            <GuideBody guide={open} />
          </div>
        </Sheet>
      ) : null}
    </div>
  );
}

// Fallbacks only for a caller with no stylesheet loaded (a bare unit test);
// the tokens are the real source and are read fresh on every drag. The shell's
// own handles carry the same pair for the same reason.
const LIST_WIDTH_MIN_FALLBACK = 220;
const LIST_WIDTH_MAX_FALLBACK = 400;
const LIST_WIDTH_DEFAULT_FALLBACK = 286;

/**
 * The list's drag range — `--w-guide-list-min` and `--w-guide-list-max`, which
 * carry the measurements for both bounds. **No window-relative ceiling here**,
 * unlike Helm's dock: the guide beside the list has a bounded measure of its
 * own, so the widest the index may be is a fact about the two columns rather
 * than about the window.
 */
function listWidthBounds(): { min: number; max: number } {
  if (typeof document === "undefined") {
    return { min: LIST_WIDTH_MIN_FALLBACK, max: LIST_WIDTH_MAX_FALLBACK };
  }
  const style = getComputedStyle(document.documentElement);
  const min = parseFloat(style.getPropertyValue("--w-guide-list-min"));
  const max = parseFloat(style.getPropertyValue("--w-guide-list-max"));
  return {
    min: Number.isFinite(min) ? min : LIST_WIDTH_MIN_FALLBACK,
    max: Number.isFinite(max) ? max : LIST_WIDTH_MAX_FALLBACK,
  };
}

/** The list's own resting width, for a caller with none of its own to remember yet. */
export function defaultGuideListWidth(): number {
  if (typeof document === "undefined") return LIST_WIDTH_DEFAULT_FALLBACK;
  const value = parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--w-guide-list"));
  return Number.isFinite(value) ? value : LIST_WIDTH_DEFAULT_FALLBACK;
}

/** Clamped to `listWidthBounds`, the same reasoning `clampLeftWidth` uses for the shell's column. */
export function clampGuideListWidth(width: number): number {
  const { min, max } = listWidthBounds();
  return Math.min(max, Math.max(min, width));
}

/** One `--space-4` per arrow press, the step the shell's two handles already take. */
const LIST_WIDTH_STEP = 16;

/**
 * The list's trailing-edge handle, the shape `TheShell`'s `LeftHandle` set: a
 * drag and the arrow keys move it, and both read the same clamp so neither can
 * put the edge somewhere the other could not reach.
 *
 * **Right widens the list**, since it sits on the leading side of the pair and
 * dragging toward the guide is dragging the edge that grows it. Home and End
 * take a plain slider's own sense, the left column's rather than the dock's
 * inverted one: nothing here is mirrored to the opposite edge.
 */
function ListHandle({ width, onResize }: { width: number; onResize: (width: number) => void }) {
  const drag = useRef<{ pointerId: number; startX: number; startWidth: number } | null>(null);
  // Only for the grip's intensified colour while dragging — `:hover` drops the
  // moment the cursor leaves the hit area, which a fast drag does almost at
  // once, and the grip going dim mid-drag would read as let go.
  const [dragging, setDragging] = useState(false);

  function pointerDown(event: PointerEvent<HTMLDivElement>): void {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = { pointerId: event.pointerId, startX: event.clientX, startWidth: width };
    setDragging(true);
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    // Suppressing the drag's own text selection also suppresses the focus a
    // click would grant — put back by hand, so the keyboard works right after
    // a press finds the handle.
    event.currentTarget.focus();
    event.preventDefault();
  }

  function pointerMove(event: PointerEvent<HTMLDivElement>): void {
    if (drag.current === null || drag.current.pointerId !== event.pointerId) return;
    const delta = event.clientX - drag.current.startX;
    onResize(clampGuideListWidth(drag.current.startWidth + delta));
  }

  function endDrag(event: PointerEvent<HTMLDivElement>): void {
    if (drag.current?.pointerId !== event.pointerId) return;
    drag.current = null;
    setDragging(false);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }

  function keyDown(event: KeyboardEvent<HTMLDivElement>): void {
    const { min, max } = listWidthBounds();
    if (event.key === "ArrowRight") onResize(clampGuideListWidth(width + LIST_WIDTH_STEP));
    else if (event.key === "ArrowLeft") onResize(clampGuideListWidth(width - LIST_WIDTH_STEP));
    else if (event.key === "Home") onResize(min);
    else if (event.key === "End") onResize(max);
    else return;
    event.preventDefault();
  }

  const { min, max } = listWidthBounds();
  return (
    <div
      className="armada-guides__handle"
      role="separator"
      aria-orientation="vertical"
      aria-label="Resize the list of guides"
      aria-valuenow={Math.round(width)}
      aria-valuemin={Math.round(min)}
      aria-valuemax={Math.round(max)}
      data-dragging={dragging || undefined}
      tabIndex={0}
      onPointerDown={pointerDown}
      onPointerMove={pointerMove}
      onPointerUp={endDrag}
      onPointerCancel={endDrag}
      onKeyDown={keyDown}
    />
  );
}

/**
 * What a guide says, drawn the same beside the list or over it.
 *
 * **Two facts the card never draws**: the group it is filed under, on the
 * eyebrow above, and the `docs/concepts/` page holding the same knowledge for
 * a reader of the repository. A guide read on purpose can afford both; a card
 * that interrupted you carries the guide and the way out.
 */
function GuideBody({ guide }: { guide: Guide }) {
  return (
    <>
      {/* No frame is held for a picture that does not exist — the guide data's
          own rule, and none of the fourteen carries one yet. */}
      {guide.picture === undefined ? null : (
        <img className="armada-guides__picture" src={guide.picture.src} alt={guide.picture.alt} />
      )}
      {guide.body.map((paragraph) => (
        <p key={paragraph} className="armada-guides__paragraph">
          {paragraph}
        </p>
      ))}
      {/* A machine value carries a word naming it. Not a link: it names a file
          in the repository rather than an address, and no surface navigates. */}
      {guide.concept === undefined ? null : (
        <p className="armada-guides__concept">
          <span className="armada-guides__concept-label">In the repository</span>
          <span className="mono">{guide.concept}</span>
        </p>
      )}
    </>
  );
}

/** A group's own title, from the one table that declares them. */
function groupTitle(id: GuideGroupId): string {
  return GUIDE_GROUPS.find((group) => group.id === id)?.title ?? id;
}
