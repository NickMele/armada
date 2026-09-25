import { useState, type CSSProperties } from "react";

import type { GuideFigureId } from "../../guides/guide";

/**
 * A guide's drawing. **One drawing per relation, scaled — never redrawn**, per
 * `docs/contracts/design-system.md`, *Teaching borrows a game's motion*: both
 * shapes mount this at two scales rather than each carrying a drawing.
 *
 * **No points, no streaks, no badges, no progress bar** — the contract's own
 * words. A mark's number is the member's order, which is the fact taught.
 *
 * **Tokens and a `<div>` tree, not an asset.** No file, no external image, no
 * new dependency, so the drawing rethemes with the tokens.
 */
export type GuideFigureProps = {
  figure: GuideFigureId;
  /**
   * `inline` sits under one step in the `steps` shape; `lead` is the `figure`
   * shape's own, above its captions and at the measure's full width.
   */
  scale: "inline" | "lead";
};

/** The three members, in the order they land, each labelled with the link it carries. */
const MEMBERS: readonly { at: number; link: string }[] = [
  { at: 1, link: "Stacked" },
  { at: 2, link: "Parked" },
  { at: 3, link: "Waiting on a release" },
];

/** The four landing rules, in the order guide 1 names them. */
const RULES: readonly string[] = [
  "The pull request merged",
  "The pull request opened",
  "Every member landed",
  "The delivering step delivered",
];

/** What the drawing says for somebody who cannot see it. A sentence, never "diagram". */
const READING: Record<GuideFigureId, string> = {
  "members-landing":
    "One branch with three members landing onto it in order: 1 stacked, then 2 parked, " +
    "then 3 waiting on a release.",
  "completion-rules":
    "The four landing rules, one of which a job carries: the pull request merged, the pull " +
    "request opened, every member landed, the delivering step delivered. A job whose rule is " +
    "met completes; a job killed, rejected or escalated ended without it.",
};

export function GuideFigure({ figure, scale }: GuideFigureProps) {
  // Read at first render rather than in an effect, so the first painted frame is
  // already still — `travel.ts` and `useHold.ts` take the preference the same
  // way. The tokens zero --duration-travel under the same query as well.
  const [still] = useState(
    () => typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches,
  );

  return (
    <div
      className="armada-guide-figure"
      data-figure={figure}
      data-scale={scale}
      data-still={still || undefined}
      role="img"
      aria-label={READING[figure]}
    >
      {figure === "members-landing" ? <MembersLanding /> : <CompletionRules />}
    </div>
  );
}

/**
 * Three members landing onto one branch in order — one of the three relations
 * the contract names as worth drawing.
 *
 * **It animates once on open and never loops**: a loop says *still working*,
 * and a guide is not working. Each member's segment of the branch draws and its
 * mark arrives, staggered by `--duration-travel`.
 *
 * **Nothing is carried by motion alone.** The order is on the marks and in
 * their places, and held still the drawing is the finished branch — which is
 * where the motion was going.
 */
function MembersLanding() {
  // Divs and not a list: `role="img"` above takes the whole subtree out of the
  // accessibility tree, so list semantics here would be markup nothing reads.
  return (
    <div className="armada-guide-figure__landings">
      {MEMBERS.map((member, index) => (
        <div
          key={member.at}
          className="armada-guide-figure__landing"
          // Its place in the queue, which is what staggers it. Not a duration —
          // --duration-travel is the duration and this multiplies it.
          style={{ "--armada-landing-order": index } as CSSProperties}
        >
          <span className="armada-guide-figure__rung">
            <span className="armada-guide-figure__rail" />
            <span className="armada-guide-figure__mark mono">{member.at}</span>
          </span>
          <span className="armada-guide-figure__link">{member.link}</span>
        </div>
      ))}
      <div className="armada-guide-figure__base">
        <span className="armada-guide-figure__base-rail" />
        <span className="armada-guide-figure__base-label mono">main</span>
      </div>
    </div>
  );
}

/**
 * The four landing rules, and the two ways a job stops.
 *
 * **This drawing is the words in boxes, and that is the finding.** Guide 1 is a
 * rule, not a relation: nothing relates to anything, so nothing animates and no
 * picture carries a fact the sentence does not. It is here because the `figure`
 * shape makes every guide lead with a drawing, and that cost is what is being
 * decided — special-casing this guide would have wasted the comparison.
 */
function CompletionRules() {
  return (
    <div className="armada-guide-figure__rules">
      <div className="armada-guide-figure__rule-list">
        {RULES.map((rule) => (
          <span key={rule} className="armada-guide-figure__rule">
            {rule}
          </span>
        ))}
      </div>
      <div className="armada-guide-figure__ends">
        <span className="armada-guide-figure__end" data-end="completed">
          Its rule met — completed
        </span>
        <span className="armada-guide-figure__end" data-end="ended">
          Killed, rejected, escalated — ended
        </span>
      </div>
    </div>
  );
}
