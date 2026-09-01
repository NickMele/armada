import { ChevronDown, ChevronRight } from "lucide-react";
import type { ReactNode } from "react";

/**
 * One chapter of a step's story — a numbered header line, and a body that
 * opens beneath it.
 *
 * **A chapter collapses to its header, never to nothing.** That is the whole
 * mechanism: the chapter you open grows in place and the others fall back to
 * one line each, so the story stays on screen, the order never changes, and
 * one thing is long at a time. A collapsed chapter still carries its number,
 * its name and its meta, so what happened in the step is readable at a glance
 * even while you are deep in one part of it.
 *
 * **One open at a time is the caller's rule, not this component's.** A chapter
 * knows whether it is open; what closes the others is the region holding them.
 * That constraint is what makes this different from a stack of accordions,
 * which allowed all six at once and became unreadable — but a component cannot
 * enforce a rule about its siblings.
 *
 * **No tab strip, and no second surface.** The four-tab region job detail grew
 * is what put the activity log behind a click; the chapters replace it because
 * a tab hides the order these things happened in, which is the one thing the
 * story is for.
 *
 * **The header meta is a fact about the chapter, and it is always drawn.**
 * `47 entries`, `3 files · +94 −31`, a timestamp. It is what makes a collapsed
 * chapter worth leaving collapsed.
 */

/**
 * What the chapter is.
 *
 * `neutral` is every chapter that reports. `waiting` is the one that asks —
 * the decision chapter on a step stopped at a human gate, which takes the
 * amber a person-waited-on carries everywhere else, on the header alone. The
 * body stays neutral: the chapter is not an alert and its contents are not
 * warnings.
 */
export type ChapterTone = "neutral" | "waiting";

export type ChapterProps = {
  /**
   * The chapter's position in the story — 1, 2, 3. Mono and quiet, in
   * `--border-strong`: it orders the chapters and is not read for itself.
   */
  ordinal: number;
  /** `Drone instructions`, `Activity log`, `Produced`, `Your decision`. */
  name: ReactNode;
  /**
   * The fact that makes a collapsed chapter worth leaving collapsed —
   * `14:22:07`, `47 entries`, `3 files · +94 −31 · all inside the plan`.
   */
  meta?: ReactNode;
  /**
   * Whether the chapter is streaming. Draws the running dot before the meta,
   * which is what says the activity log is live rather than a snapshot — the
   * one claim a count cannot make.
   */
  live?: boolean;
  tone?: ChapterTone;
  /** Whether the body is shown. Closed collapses to the header line. */
  open?: boolean;
  /** Open or close. Absent draws the header as a label, not a control. */
  onToggle?: () => void;
  /** The chapter's contents. */
  children?: ReactNode;
  /**
   * The control at the foot of the body — `Open the log — all 47 entries`,
   * `Close`. The accent, because it is the one thing in a chapter that leads
   * somewhere, and it sits at the start of its line rather than filling it.
   */
  moreLabel?: ReactNode;
  onMore?: () => void;
  /**
   * Whether `moreLabel` closes rather than opens. Picks the glyph, out of the
   * registry's expand-and-collapse pair: `chevron-down` on a chapter already
   * open, `chevron-right` on one that goes further.
   *
   * The drawing puts a chevron pointing **up** on Close. The registry carries
   * no `chevron-up` and pairs down with right for exactly this, so the pair
   * wins over the drawing. Reported.
   */
  moreCloses?: boolean;
  /** For a caller that needs to point at the body. */
  bodyId?: string;
};

/** Chapter glyphs are 12px at strokeWidth 2, as every mark on this screen is. */
const GLYPH = 12;
const STROKE = 2;

export function Chapter({
  ordinal,
  name,
  meta,
  live,
  tone = "neutral",
  open = true,
  onToggle,
  children,
  moreLabel,
  onMore,
  moreCloses,
  bodyId,
}: ChapterProps) {
  const head = (
    <>
      <span className="armada-chapter__n" aria-hidden>
        {ordinal}
      </span>
      <span className="armada-chapter__name">{name}</span>
      {meta === undefined && !live ? null : (
        <span className="armada-chapter__meta">
          {live ? <span className="armada-chapter__live" aria-hidden /> : null}
          {meta}
        </span>
      )}
    </>
  );

  return (
    <section className="armada-chapter" data-tone={tone} data-open={open || undefined}>
      {onToggle === undefined ? (
        <div className="armada-chapter__head">{head}</div>
      ) : (
        <button
          type="button"
          className="armada-chapter__head"
          aria-expanded={open}
          aria-controls={bodyId}
          onClick={onToggle}
        >
          {head}
        </button>
      )}

      {/* Kept in the document while collapsed so the header's `aria-controls`
          names something. Hidden, not unmounted — and the activity log is
          still streaming into it while the chapter is shut. */}
      <div className="armada-chapter__body" id={bodyId} hidden={!open}>
        {children}
        {moreLabel === undefined || onMore === undefined ? null : (
          <button type="button" className="armada-chapter__more" onClick={onMore}>
            {moreLabel}
            {moreCloses ? (
              <ChevronDown size={GLYPH} strokeWidth={STROKE} aria-hidden />
            ) : (
              <ChevronRight size={GLYPH} strokeWidth={STROKE} aria-hidden />
            )}
          </button>
        )}
      </div>
    </section>
  );
}
