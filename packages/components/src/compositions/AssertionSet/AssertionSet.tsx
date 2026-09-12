import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

/**
 * The assertion set — what a Check's suite actually asserted, one row each.
 * `exit 0 · 315 passed` is not auditable: a suite can go green by deleting
 * the failing assertion, and a summary line cannot tell that from a fix.
 * The row list is the difference; the sentence is the row, the identifier
 * only stands in where the harness gave none, and `against` marks a row
 * worth reading rather than its position.
 *
 * `absent` is not `failed`: a failed assertion ran and came out wrong, an
 * absent one did not run here but did at the parent commit — both take
 * `--verdict-not-met`, and `against` tells them apart, in words. Nothing on
 * the wire serves this: a Check reports an `outcome` and an `output_path`,
 * the assertions inside are unparsed, and the comparison is not recorded
 * anywhere. Fixtures only.
 */

/**
 * What the assertion came to. Spelled as a Check's own result is spelled, so
 * nothing here is a second vocabulary — `FactChip` takes the same two words.
 *
 * `absent` is the third, and nothing on the wire says it yet: it is a
 * comparison between this run and the parent commit's, which the check emits
 * as output rather than as a field. Reported.
 */
export type AssertionNamed = "passed" | "failed" | "absent";

export type Assertion = {
  /**
   * What the assertion asserts, in the harness's own words — a description, a
   * docstring. Sans, because it is a sentence somebody wrote. Absent where the
   * harness supplied none, and then `identifier` stands in and says so.
   */
  says?: ReactNode;
  /**
   * The test's identifier, in mono. **Always carried, drawn only as the
   * fallback** — a citation names the identifier, so a row that dropped it
   * could not be pointed at.
   */
  identifier: string;
  named: AssertionNamed;
  /**
   * The glyph. **Absent by default and the registry has no member for this
   * row**: `shield-*` is reserved to a Check's own result and an assertion is
   * one line inside one, `circle-*` is the Judge's. Reported. The prop stands
   * so a caller can supply one the day the registry declares it.
   */
  icon?: LucideIcon;
  /**
   * How this assertion stood at the parent commit — `identical at HEAD and
   * HEAD~1`, `passed at HEAD~1 · absent at HEAD`. Mono, because it is a
   * comparison the check measured rather than anything it was told.
   *
   * **This is what makes the set worth drawing.** Without it every row reads
   * `ok` and the page is a longer way of saying 315 passed.
   */
  against?: ReactNode;
};

export type AssertionSetProps = {
  rows: Assertion[];
  /**
   * The assertions not drawn, as one row. A suite of 315 is not a list to
   * scan, and a set that showed five with no account of the rest would read as
   * a suite of five.
   */
  rest?: { says: ReactNode; against?: ReactNode };
  /** The label over the set, where it stands on its own rather than in a viewer. */
  label?: ReactNode;
};

/** Row marks are 12px at strokeWidth 2, like every mark below Job level. */
const ROW_ICON = 12;
const ROW_STROKE = 2;

/**
 * Why a row is showing an identifier instead of a sentence.
 *
 * Standing copy rather than a value: it is true of every such row on every
 * Job, and a screen that retyped it would be the second place the rule that
 * the identifier is a fallback is stated.
 */
const NO_DESCRIPTION =
  "the harness supplied no description — the identifier is the fallback, not the default";

export function AssertionSet({ rows, rest, label }: AssertionSetProps) {
  return (
    <div className="armada-assertions">
      {label ? <span className="armada-assertions__label">{label}</span> : null}
      <ol className="armada-assertions__list">
        {rows.map((row) => (
          <li className="armada-assertions__row" key={row.identifier} data-named={row.named}>
            <span className="armada-assertions__mark">
              {row.icon ? <row.icon size={ROW_ICON} strokeWidth={ROW_STROKE} aria-hidden /> : null}
            </span>
            <span className="armada-assertions__what">
              {row.says === undefined ? (
                <>
                  {/* The identifier drawn as itself, with the reason beneath
                      it. Two lines rather than one, because a bare mono row in
                      a column of sentences reads as a different kind of thing
                      and does not say why it is one. */}
                  <span className="armada-assertions__id">{row.identifier}</span>
                  <span className="armada-assertions__gloss">{NO_DESCRIPTION}</span>
                </>
              ) : (
                <span className="armada-assertions__says">{row.says}</span>
              )}
            </span>
            {row.against === undefined ? null : (
              <span className="armada-assertions__against">{row.against}</span>
            )}
          </li>
        ))}
        {rest === undefined ? null : (
          <li className="armada-assertions__row" data-rest="true">
            <span className="armada-assertions__mark" aria-hidden />
            <span className="armada-assertions__what">
              <span className="armada-assertions__says">{rest.says}</span>
            </span>
            {rest.against === undefined ? null : (
              <span className="armada-assertions__against">{rest.against}</span>
            )}
          </li>
        )}
      </ol>
    </div>
  );
}
