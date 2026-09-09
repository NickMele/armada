import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { conceptSaid } from "../../concepts";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * Check runs — what each Check came to, and the output behind it.
 *
 * **A Check result is an evidence record.** It has a kind and a file, exactly
 * like a Drone's evidence, and the only difference is who produced it — which
 * is a field, not a screen. So a row here is a selector: pressing it puts that
 * Check's output in the viewer, the way a produced file puts a diff there.
 *
 * **This is the answer to "we show that checks pass or fail but we can't see
 * the output".** A suite can pass by deleting the assertion that was failing,
 * and `exit 0 · 315 passed` cannot be audited. The rows carry what the output
 * cost and how much of it there is — `2,180 lines` — so it is visibly *there*
 * before anyone presses anything.
 *
 * **Not `WorkflowRail`'s gate row.** That row draws a Check's id, its command
 * and the path its output was written to, for copying into a shell. This one
 * draws what the Check *came to*, as a sentence, and opens the output in
 * Bridge. The path stays where it is; a person who wants a terminal still has
 * it.
 *
 * **The sentence is the row and the identifier is under it.** Sans names work,
 * mono names machinery — so `All 315 tests passed` is what a person scans and
 * `check:test_suite` is what a citation resolves against.
 *
 * **A queued Check keeps its row and has no output affordance.** The shape of
 * what is still coming is part of reading a running Job, and hiding the rows
 * until they run would make a Job look like it has fewer gates than it has.
 */

/**
 * What the Check came to, for the hue. Spelled as the wire spells it, so
 * nothing here is a second vocabulary — `FactChip` takes the same words.
 */
export type CheckRunNamed = "passed" | "failed" | "refused" | "running" | "queued";

export type CheckRun = {
  /** What a selection names. The Check's own id. */
  id: string;
  /**
   * What the Check came to, in one sentence — `All 315 tests passed`,
   * `“Behaviour unchanged” refused by 2 of 3 judges`. Sans, because it is a
   * finding rather than a command.
   */
  says: ReactNode;
  /**
   * The Check, in mono — `check:test_suite`, `check:bench`. **Always
   * drawn**, under the sentence: a citation names the identifier, and a row
   * carrying only prose could not be pointed at.
   */
  identifier: ReactNode;
  /**
   * Whether `identifier` is a name a person wrote rather than a machine's own,
   * so it renders in sans and claims nothing about being joinable.
   *
   * **The Judge's row is why this exists.** A Check's second line is its
   * declared id — `check:test_suite`, out of the repository's own Manifest,
   * and a citation resolves against it. A panel has no such id: it is one
   * `judge_checks[]` entry on a step, so the row drew the *enum* — literally
   * `judge_check` — in mono, which told a reader the surface had leaked its
   * schema at them and offered a string that joins to nothing.
   *
   * Same convention as `RunTree`'s `labelIsAnIdentifier`, in the other
   * direction: there the name is the default and the identifier is the
   * exception, and here it is the reverse, because most rows on this list are
   * commands.
   */
  identifierIsAName?: boolean;
  /** The exit code or the state, in mono — `exit 0`, `refused`, `running`, `queued`. */
  result?: ReactNode;
  named?: CheckRunNamed;
  /**
   * The glyph, from the `shield-*` family that means gates and Checks. Absent
   * on a Judge's row, where `circle-*` is the family and the caller supplies
   * it.
   */
  icon?: LucideIcon;
  /**
   * What this Check produced and how much of it — `output · 2,180 lines`,
   * `verdicts`. **The size is not decoration**: it is what says the output is
   * a reading rather than a line, before anyone opens it.
   *
   * Absent on a Check that has not run, which is what makes a queued row a row
   * with nothing to press rather than a row with a dead control on it.
   */
  output?: ReactNode;
};

export type CheckRunsProps = {
  rows: CheckRun[];
  /**
   * Which Check's output the viewer is showing, or `null` for none. Held by
   * the surface, because the viewer is the surface's and one selection cannot
   * live in two places.
   */
  openId?: string | null;
  /** Told when a row's output is asked for. */
  onOpen?: (checkId: string) => void;
  /** The label over the list, where it stands on its own. */
  label?: ReactNode;
  /** Where the output is read from, said once — `tailed from the run log`. */
  note?: ReactNode;
  /**
   * What pressing an output does, on hover.
   *
   * **A prop because where it opens is the surface's answer, not this
   * component's.** The default names the viewer, which is what a surface that
   * has one shows; Bridge has none — it hands the path to the OS — so a fixed
   * sentence here would have the app promise a panel nobody built.
   */
  openSaid?: ReactNode;
};

/** Row marks are 12px at strokeWidth 2, like every mark below Job level. */
const ROW_ICON = 12;
const ROW_STROKE = 2;

export function CheckRuns({
  rows,
  openId = null,
  onOpen,
  label,
  note,
  openSaid = "Click to read this output in the viewer",
}: CheckRunsProps) {
  // The list's own name is the vocabulary's word, so what a Check is comes from
  // where that is written and not from here.
  const named = label === undefined ? undefined : conceptSaid(label);
  const heading = <span className="armada-check-runs__label">{label}</span>;
  return (
    <div className="armada-check-runs">
      {label === undefined && note === undefined ? null : (
        <div className="armada-check-runs__head">
          {label === undefined ? null : named === undefined ? (
            heading
          ) : (
            <Tooltip asChild label={named}>
              {heading}
            </Tooltip>
          )}
          {note ? <span className="armada-check-runs__note">{note}</span> : null}
        </div>
      )}
      <ul className="armada-check-runs__list">
        {rows.map((row) => {
          const open = row.id === openId;
          return (
            <li
              className="armada-check-runs__row"
              key={row.id}
              data-named={row.named}
              data-open={open ? "true" : undefined}
            >
              <span className="armada-check-runs__mark">
                {row.icon ? <row.icon size={ROW_ICON} strokeWidth={ROW_STROKE} aria-hidden /> : null}
              </span>
              <span className="armada-check-runs__what">
                <span className="armada-check-runs__says">{row.says}</span>
                <span
                  className="armada-check-runs__id"
                  data-name={row.identifierIsAName ? "true" : undefined}
                >
                  {row.identifier}
                </span>
              </span>
              {row.output === undefined ? (
                // A Check with nothing to show keeps the column and leaves it
                // empty. A disabled control here would be a target that
                // refuses, which is worse than no target.
                <span className="armada-check-runs__no-output" aria-hidden />
              ) : (
                // `aria-pressed` is what carries the selection. It used to live
                // only in the stylesheet, which told nobody who was not looking
                // at it which output the viewer was showing.
                <Tooltip asChild label={openSaid}>
                  <button
                    type="button"
                    className="armada-check-runs__output"
                    aria-pressed={open}
                    onClick={() => onOpen?.(row.id)}
                  >
                    {row.output}
                  </button>
                </Tooltip>
              )}
              {row.result === undefined ? null : (
                <span className="armada-check-runs__result">{row.result}</span>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
