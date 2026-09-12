import { ChevronRight, ChevronUp } from "lucide-react";
import type { ReactNode } from "react";
import { conceptSaid } from "../../concepts";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * One chapter of a step's story — a numbered header line, and a body that
 * opens beneath it. A chapter collapses to its header, never to nothing:
 * number, name and meta stay readable at a glance, and each chapter opens
 * and closes on its own — pressing one header never closes another.
 *
 * No tab strip, no second surface: the four-tab region job detail grew put
 * the activity log behind a click, and chapters replace it because a tab
 * hides the order things happened in, which is the one thing the story is
 * for. The header meta — `47 entries`, `3 files · +94 −31`, a timestamp —
 * is always drawn, since it is what makes a collapsed chapter worth
 * leaving collapsed.
 */

/**
 * What the chapter is.
 *
 * `neutral` is every chapter that reports. `waiting` is the one that asks —
 * the decision chapter on a step stopped at a human gate, which takes the
 * amber a person-waited-on carries everywhere else, on the header alone. The
 * body stays neutral: the chapter is not an alert and its contents are not
 * warnings.
 *
 * `muted` is the one that says the same thing on every Job — the Drone
 * brief's standing instructions, folded by default because they carry no fact
 * specific to this Job. **Dimming is a token, not an alpha**, per
 * `docs/contracts/design-system.md`: the name steps down to `--fg-subtle`
 * rather than losing opacity, which would muddy the tone hues beside it.
 */
export type ChapterTone = "neutral" | "waiting" | "muted";

export type ChapterProps = {
  /**
   * The chapter's position in the story — 1, 2, 3. Mono and quiet, in
   * `--border-strong`: it orders the chapters and is not read for itself.
   *
   * **Absent draws a fold arrow in its place instead of a number.** A brief's
   * own sections are not a numbered story of their own — they are one
   * chapter's contents — so a caller with nothing to count passes nothing,
   * and the slot that would have held a digit holds the open/closed state
   * instead. `ChevronRight`/`ChevronUp` are the same pair `moreLabel` below
   * already draws, read the same way: closed points at more to see, open
   * points at closing it again.
   */
  ordinal?: number;
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
  /**
   * What the header says on hover, where what it does is the interesting half —
   * `Click to view the files`.
   *
   * **Written at the control, because it is an act.** Where it is absent the
   * chapter's own name is looked up in the vocabulary instead: `Drone
   * instructions` and `Activity log` name Armada concepts and have one
   * explanation between every surface that draws them. A name that is neither
   * draws no tooltip.
   */
  says?: ReactNode;
  /** Whether the body is shown. Closed collapses to the header line. */
  open?: boolean;
  /** Open or close. Absent draws the header as a label, not a control. */
  onToggle?: () => void;
  /**
   * The control on the header line — `Open the log`, `Open the diff`.
   *
   * **The affordance is on the header rather than at the foot of the body**,
   * because what these two chapters open is a trailing sheet and not a body: a
   * chapter whose content has no end leaves the panel, so there is no body for
   * the control to sit under. It is a sibling of the header rather than inside
   * it — a control nested in a control is one target a pointer cannot tell
   * apart, and invalid besides.
   */
  act?: ReactNode;
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
   * Whether `moreLabel` closes rather than opens. Picks the glyph:
   * `chevron-up` on a chapter already open, `chevron-right` on one that goes
   * further.
   *
   * **It drew `chevron-down` here until 2026-09-07** — a caret pointing into
   * the thing it was about to shut — because the registry carried only the
   * down-and-right pair and the contract wins over a drawing. The report was
   * answered rather than overruled: `chevron-up` is registered now, reserved
   * to a control whose whole label is Close, and down keeps disclosure on a
   * closed record.
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
  says,
  open = true,
  onToggle,
  act,
  children,
  moreLabel,
  onMore,
  moreCloses,
  bodyId,
}: ChapterProps) {
  const head = (
    <>
      {ordinal === undefined ? (
        <span className="armada-chapter__fold" aria-hidden>
          {open ? (
            <ChevronUp size={GLYPH} strokeWidth={STROKE} />
          ) : (
            <ChevronRight size={GLYPH} strokeWidth={STROKE} />
          )}
        </span>
      ) : (
        <span className="armada-chapter__n" aria-hidden>
          {ordinal}
        </span>
      )}
      <span className="armada-chapter__name">{name}</span>
      {meta === undefined && !live ? null : (
        <span className="armada-chapter__meta">
          {live ? <span className="armada-chapter__live" aria-hidden /> : null}
          {meta}
        </span>
      )}
    </>
  );

  const hover = says ?? conceptSaid(name);
  const line =
    onToggle === undefined ? (
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
    );

  return (
    <section className="armada-chapter" data-tone={tone} data-open={open || undefined}>
      <div className="armada-chapter__line">
        {/* `asChild`, because the line is a grid and the header takes its first
            track — a wrapper here would push the act out of the row. */}
        {hover === undefined ? (
          line
        ) : (
          <Tooltip asChild label={hover}>
            {line}
          </Tooltip>
        )}
        {act === undefined ? null : <div className="armada-chapter__act">{act}</div>}
      </div>

      {/* Kept in the document while collapsed so the header's `aria-controls`
          names something. Hidden, not unmounted — and the activity log is
          still streaming into it while the chapter is shut. */}
      <div className="armada-chapter__body" id={bodyId} hidden={!open}>
        {children}
        {moreLabel === undefined || onMore === undefined ? null : (
          <button type="button" className="armada-chapter__more" onClick={onMore}>
            {moreLabel}
            {moreCloses ? (
              <ChevronUp size={GLYPH} strokeWidth={STROKE} aria-hidden />
            ) : (
              <ChevronRight size={GLYPH} strokeWidth={STROKE} aria-hidden />
            )}
          </button>
        )}
      </div>
    </section>
  );
}
