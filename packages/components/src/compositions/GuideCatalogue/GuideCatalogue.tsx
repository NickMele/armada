import { useState } from "react";

import { GUIDE_GROUPS, GUIDES } from "../../guides";
import type { Guide, GuideGroupId } from "../../guides/guide";
import { useGuideShape } from "../../guide-shape";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { GuideShaped } from "../GuideShaped/GuideShaped";

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
};

export function GuideCatalogue({
  guides = GUIDES,
  arriveAt,
  narrow = false,
  floor = false,
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

  return (
    <div className="armada-guides" data-narrow={narrow || undefined}>
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

      {/* Beside the list wherever the window can pay for both. */}
      {narrow || open === undefined ? null : (
        <article
          className="armada-guides__panel"
          // Named by the guide it holds, the way the sheet it folds into is:
          // without it the region beside the list is one nothing can name.
          aria-label={open.title}
        >
          <div className="armada-guides__panel-head">
            <p className="armada-guides__eyebrow">
              <span className="mono">Guide {open.number}</span>
              <span aria-hidden="true">·</span>
              <span>{groupTitle(open.group)}</span>
            </p>
            <h2 className="armada-guides__panel-title">{open.title}</h2>
          </div>
          <GuideBody key={open.number} guide={open} />
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
            <GuideBody key={open.number} guide={open} />
          </div>
        </Sheet>
      ) : null}
    </div>
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
  // The mock's `?guides=`, for the shape comparison. Bridge provides no shape,
  // so this is `prose` in the app and the rendering below is untouched.
  const shape = useGuideShape();
  const shaped = shape !== "prose" && guide.shapes !== undefined ? shape : undefined;
  return (
    <>
      {/* No frame is held for a picture that does not exist — the guide data's
          own rule, and none of the fourteen carries one yet. */}
      {guide.picture === undefined ? null : (
        <img className="armada-guides__picture" src={guide.picture.src} alt={guide.picture.alt} />
      )}
      {shaped !== undefined ? <GuideShaped guide={guide} shape={shaped} where="panel" /> : null}
      {shaped !== undefined
        ? null
        : guide.body.map((paragraph) => (
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
