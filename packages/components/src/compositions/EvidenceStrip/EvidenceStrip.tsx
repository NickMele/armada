import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * The evidence strip — everything else this step produced, one chip each.
 *
 * **Every chip is a selector into the viewer.** A measurement, a patch, a
 * document, a test result: they are all artifacts with a kind and a file, so
 * they enter one renderer set and one strip rather than four regions that each
 * hold one sort of thing.
 *
 * **A chip states its kind.** `bench 1.42 → 1.19µs` and `−318 +94 · 5 files`
 * are both values, and what tells a reader which viewer is behind them is the
 * word under the value — not the shape of the chip, which is the same for all
 * of them on purpose.
 *
 * **The selected chip is where the viewer's current artifact lives.** Opening
 * a check's output does not destroy what was showing: that artifact moves into
 * the strip, marked, so one press puts it back. Without that, three presses in
 * and the page has lost the opinion it was composed with.
 *
 * **Not `FactChip`.** A fact chip is a value being read — the run tree is full
 * of them and none of them is a target. These are controls, and they are drawn
 * as controls: the accent on the edge, a hover state, a focus ring.
 */

export type EvidenceChip = {
  /** What a selection names. */
  id: string;
  /** The value, in mono — `public symbols 41 → 41`, `MIGRATION.md`. */
  says: ReactNode;
  /**
   * What this artifact *is*, in a reader's words — `time to parse one date`,
   * `symbols the crate exports`, `the patch this step wrote`.
   *
   * **Not a category.** It said `measurement` and `code diff`, which name the
   * renderer rather than the thing: a reviewer asked what `bench 1.42 →
   * 1.19µs · measurement` meant and was right to, because the value is jargon
   * and the word under it repeated that the jargon was a number. The value is
   * the measurement; this is what was measured. Where the two together still
   * assume something, the caller adds a hover — the chip is a chip and cannot
   * hold a sentence.
   *
   * **Not optional**: a strip of five values with no kinds is five things a
   * person has to press to identify.
   */
  kind: ReactNode;
  /**
   * Which verdict this artifact is, where it is one. Absent on every chip that
   * is only a measurement — which is most of them, and why neutral is the
   * default. Spelled as the wire spells it.
   */
  named?: "passed" | "failed" | "met" | "not_met" | "refused";
  /** The glyph, where the registry has one for this kind of artifact. */
  icon?: LucideIcon;
};

export type EvidenceStripProps = {
  chips: EvidenceChip[];
  /** Which artifact the viewer is showing, or `null` for none. */
  openId?: string | null;
  onOpen?: (artifactId: string) => void;
  /**
   * What the list is — `Everything this step produced`, `Produced so far` on a
   * running one.
   *
   * **A phrase, not a word.** It read `also produced`, and *also* is relative
   * to something a reader has to work out: the artifact currently in the
   * viewer, which on a judgment view is not one of these at all. Say what the
   * list is and the relation stops needing to be inferred.
   *
   * **The tense is the caller's**, because only the caller knows whether the
   * list is finished.
   */
  label?: ReactNode;
};

/** Chip glyphs are 12px at strokeWidth 2, like every mark below Job level. */
const CHIP_ICON = 12;
const CHIP_STROKE = 2;

export function EvidenceStrip({ chips, openId = null, onOpen, label }: EvidenceStripProps) {
  return (
    <div className="armada-evidence-strip">
      {label ? <span className="armada-evidence-strip__label">{label}</span> : null}
      {chips.map((chip) => (
        // The hover says where it opens, not what it is — the kind is already
        // written under the value, and *in the viewer* is the half a chip
        // cannot show: pressing one replaces what the viewer is holding, and
        // the artifact it displaces comes back here marked.
        //
        // **One sentence, not one built from the kind.** It read
        // `Click to open this ${kind} in the viewer`, which needed `kind` to be
        // a category noun — and `kind` now says what the artifact *is*, in a
        // reader's words, so the template produced "Click to open this the
        // patch this step wrote in the viewer". A sentence assembled from a
        // field somebody else is free to word is a sentence waiting to break.
        <Tooltip asChild key={chip.id} label="Click to open this in the viewer">
          {/* `aria-pressed` carries which artifact is open. It used to be a
              border colour and nothing else, which told nobody who was not
              looking at it. */}
          <button
            type="button"
            className="armada-evidence-strip__chip"
            data-named={chip.named}
            aria-pressed={chip.id === openId}
            onClick={() => onOpen?.(chip.id)}
          >
            {chip.icon ? (
              <chip.icon size={CHIP_ICON} strokeWidth={CHIP_STROKE} aria-hidden />
            ) : null}
            <span className="armada-evidence-strip__says">{chip.says}</span>
            <span className="armada-evidence-strip__kind">{chip.kind}</span>
          </button>
        </Tooltip>
      ))}
    </div>
  );
}
