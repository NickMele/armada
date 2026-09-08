import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

/**
 * The assertion set — what a Check's suite actually asserted, one row each.
 *
 * **This exists because `exit 0 · 315 passed` is not auditable.** A suite can
 * go green by deleting the assertion that was failing, and a person reading a
 * summary line cannot tell that from a suite that was fixed. The list of what
 * ran is the difference, so it is drawn as rows rather than folded into a
 * count.
 *
 * **The row is the sentence and the identifier is not on it.** Sans names work
 * and mono names machinery, so an assertion a person scans reads as what it
 * asserts. `parses_loose_trailing_whitespace` survives only where a machine
 * reference is the point — in the output itself, and in a citation.
 *
 * **Where the harness supplied no sentence the identifier stands in, and the
 * row says why.** A de-snake-cased guess would read like a description the
 * harness wrote and be one nobody can check against the test. The gloss is
 * standing copy written once here rather than at each call site, because it is
 * true of every such row on every Job.
 *
 * **Rows keep the order they ran in.** This is a transcript's index, not a
 * ledger — refusals sort first on `CriterionVerdicts` because a verdict is an
 * answer to a question somebody asked, and an assertion is a thing that
 * happened at a moment. `against` is what marks the row worth reading, not its
 * position.
 *
 * **`absent` is not `failed`, and that is the whole finding.** A failed
 * assertion ran and came out wrong. An absent one did not run here and did run
 * at the parent commit — the suite is green because a case stopped existing.
 * Both take `--verdict-not-met`, because both are the work not being what was
 * asked; what tells them apart is `against`, in words.
 *
 * **Nothing on the wire serves this.** A Check reports an `outcome` and an
 * `output_path`; the assertions inside that output are unparsed, and the
 * comparison against the parent commit is not recorded anywhere. Fixtures
 * only.
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
