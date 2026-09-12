import { ChevronRight, ChevronUp, type LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { Fragment, useId, useState } from "react";
import { conceptSaid } from "../../concepts";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeaderCell,
  TableRow,
} from "../../primitives/Table/Table";

/**
 * Judge verdicts — the panel's answer, as criteria against judges. A
 * panel's output is not a stream, so it is not drawn as one: under the
 * veto-only contract a Judge that meets a criterion writes nothing, so
 * the output is a grid with its refusals opening out of it, not three
 * opinions to read.
 *
 * A refusal opens under the row it refuses. It sat in a block below the
 * whole grid until 2026-09-08, which read acceptably with one refusal and
 * fell apart with two: each block restated *Refused — 0N* and the reader
 * mapped it back to a row scrolled past, twice over — and everything that
 * block said about criterion 02 was a second description of 02 nowhere
 * near 02. There is one place a criterion is discussed now: its own row.
 */

/**
 * One open at a time, `StepStory`'s rule and holds here for the same
 * reason: three refusals open at once is a wall of prose with a table
 * lost at the top of it, and a collapsed refusal still carries its split
 * so three can be triaged without opening any.
 *
 * The acts are not in here: Overrule, retry and redispatch act on the
 * step rather than on a criterion — one Job is killed once however many
 * criteria were refused — so they belong to the decision after the
 * story. A set of acts per refusal would have offered to kill the same
 * Job three times.
 */

/**
 * The measured rows are the veto-only contract, drawn: a criterion a
 * Check settled never reached the panel, and rendering that as one band
 * across the judge columns says which parts of the verdict rest on a
 * machine and which on a model, without a sentence.
 *
 * The split is shown and never becomes the verdict: one veto is a
 * refusal whatever its size, so hue here is per criterion and binary. But
 * `2 of 3` and `1 of 3` are different situations for the person deciding
 * whether to rewrite the brief or take the Job over — the count is a
 * confidence signal, not a vote, so it is a figure and never a second
 * colour.
 */

/**
 * Rows are in the criteria's own frozen order and never sorted: this is
 * a table with a numbered column, and a numbered column that runs 02,
 * 01, 03, 04 reads as a bug rather than an opinion. `CriterionVerdicts`
 * sorts its refusals to the top and is right to — it is an unnumbered
 * list of one Judge's answers, where nothing but order says which row
 * matters. Here the refused row is marked in three other ways, so order
 * is spent on the one job only it can do.
 */

/** How one judge answered one criterion. Spelled as the wire spells it. */
export type JudgeMark = "met" | "not_met" | "gate_undecided";

export type JudgeVerdictRow = {
  /**
   * The criterion's frozen position in `acceptance_criteria[]`, 1-based. What
   * a citation names, and never the row's place on screen.
   */
  ordinal?: number;
  criterionId: string;
  /** The criterion's short name — `Behaviour unchanged`. Sans. */
  name: ReactNode;
  /** The requester's own words, under the name. */
  text?: ReactNode;
  /**
   * One mark per judge, in panel order. **Absent where a Check settled the
   * criterion** — and then `measured` says which, across the whole span.
   */
  marks?: JudgeMark[];
  /**
   * Which Check settled it, where the panel never saw it — `measured —
   * check:public_api`. Mono, because it names a Check.
   */
  measured?: ReactNode;
  /**
   * The refusal in full — a `JudgeRefusal`. **Present makes the row a
   * disclosure**, opening beneath itself.
   *
   * Absent on a refused row leaves the marks and no way in, which is the honest
   * rendering of a panel that recorded a verdict and no grounds: a chevron
   * opening an empty region would promise a reading nobody wrote.
   */
  refusal?: ReactNode;
  /**
   * How large the refusal is, on the row itself — `2 of 3`. **So three
   * collapsed refusals can be triaged without opening one**: a lone dissent and
   * a unanimous refusal are the same verdict and different situations, and that
   * is the whole of what a closed row can usefully say.
   */
  split?: ReactNode;
};

export type JudgeVerdictsProps = {
  rows: JudgeVerdictRow[];
  /**
   * The judges, in panel order — `j1`, `j2`, `j3`. **The header is the panel's
   * shape**, so a panel of two and a panel of three are told apart before a
   * single mark is read.
   */
  judges: ReactNode[];
  /*
   * The panel is on the wire: `Judged.member` arrived in protocol 7.7, one
   * row per criterion per member, absent at `panel_size: 1`, so the
   * columns, marks and split all have a producer, grouped by `screens`'
   * `gates.ts`. This block said otherwise until 2026-09-09.
   *
   * What each judge read is on the wire too, since 8.3: `Judged.cited`
   * places every quotation a member made in the brief shown, and
   * `Judged.given` digests what that member was handed — `JudgeCitations`
   * and `JudgeInputs` both draw served data now. What still has none: the
   * per-judge citation sets `JudgeRefusal.overlap` draws; `cited` carries
   * each member's list and nothing computes the intersection, so the
   * overlap is a reading nobody has written rather than a record nobody
   * keeps.
   */

  /** The glyphs, from the `circle-*` family the Judge owns. */
  glyphs?: Partial<Record<JudgeMark, LucideIcon>>;
  /** What a mark means, for the reader who is not looking at the glyph. */
  meanings?: Partial<Record<JudgeMark, string>>;
  /** The label over the grid, where it stands on its own. */
  label?: ReactNode;
  /**
   * Which refusal is open on mount. Absent opens the first one there is — a
   * grid whose only refusal is shut makes a reader press to reach the thing the
   * screen was opened for.
   */
  openId?: string;
  /**
   * Which refusal is open, held by the caller. **Present makes the grid
   * controlled**: it draws what this says and changes nothing itself. `null` is
   * every refusal collapsed.
   */
  openCriterionId?: string | null;
  onOpen?: (criterionId: string | null) => void;
};

/** Verdict glyphs are 12px at strokeWidth 2, like every mark below Job level. */
const MARK_ICON = 12;
const MARK_STROKE = 2;
/** The disclosure runs at 16px, which is the registry's size for the pair. */
const CHEVRON = 16;

/**
 * What each mark is called, for the accessible name. The wire's own words in
 * sentence case, written once here — a screen that retyped them would be the
 * second place a verdict is named.
 */
const MEANS: Record<JudgeMark, string> = {
  met: "met",
  not_met: "refused",
  gate_undecided: "could not read the artifact",
};

/**
 * The criteria in the order they were asked in.
 *
 * **Sorted here rather than trusted from the caller.** The ordinal is the
 * frozen position a citation names, and a grid that drew the rows in whatever
 * order they arrived would put the numbers out of sequence for a reason no
 * reader could see. A row with no ordinal keeps its place at the end, because
 * a criterion the Job carries no position for cannot be given one.
 */
function inFrozenOrder(rows: JudgeVerdictRow[]): JudgeVerdictRow[] {
  return [...rows].sort((a, b) => (a.ordinal ?? Infinity) - (b.ordinal ?? Infinity));
}

/** Whether any judge refused this criterion. */
function refusedIn(row: JudgeVerdictRow): boolean {
  return row.marks?.some((mark) => mark === "not_met") ?? false;
}

export function JudgeVerdicts({
  rows,
  judges,
  glyphs,
  meanings,
  label,
  openId,
  openCriterionId,
  onOpen,
}: JudgeVerdictsProps) {
  // Each chevron is named by the criterion it opens, through the name cell
  // rather than through a string: `name` is a ReactNode and cannot be
  // interpolated into a label, and three buttons all reading "Open this
  // refusal" name nothing a person could act on.
  const named = useId();
  const ordered = inFrozenOrder(rows);
  // The first refusal with something to open. A grid that opened nothing makes
  // a reader press to reach the row the screen exists for; one that opened all
  // three is the wall this arrangement replaced.
  const first = ordered.find((row) => refusedIn(row) && row.refusal !== undefined);
  const [held, setHeld] = useState<string | null>(openId ?? first?.criterionId ?? null);
  const controlled = openCriterionId !== undefined;
  const open = controlled ? openCriterionId : held;

  function press(criterionId: string) {
    const next = open === criterionId ? null : criterionId;
    if (!controlled) setHeld(next);
    onOpen?.(next);
  }

  return (
    <div className="armada-judge-verdicts">
      {label ? <span className="armada-judge-verdicts__label">{label}</span> : null}
      <Table className="armada-judge-verdicts__grid">
        <TableHead>
          <TableRow>
            {/* The disclosure's own column, and it is unlabelled: a header over
                a gutter of chevrons would be naming the furniture. */}
            <TableHeaderCell scope="col" className="armada-judge-verdicts__gutter" />
            {/* `asChild` throughout this grid. A `td` and a `th` are the only
                things a table row may contain, so a wrapper span would put the
                header outside its own column. */}
            <Tooltip asChild label={conceptSaid("#")}>
              <TableHeaderCell scope="col">#</TableHeaderCell>
            </Tooltip>
            <Tooltip asChild label={conceptSaid("criterion")}>
              <TableHeaderCell scope="col">Criterion</TableHeaderCell>
            </Tooltip>
            {judges.map((judge, at) => (
              <TableHeaderCell scope="col" key={at} className="armada-judge-verdicts__judge">
                {judge}
              </TableHeaderCell>
            ))}
          </TableRow>
        </TableHead>
        <TableBody>
          {ordered.map((row) => {
            const refused = refusedIn(row);
            const opens = refused && row.refusal !== undefined;
            const isOpen = opens && open === row.criterionId;
            const nameId = `${named}-${row.criterionId}`;
            const said = (
              <>
                <span className="armada-judge-verdicts__name" id={nameId}>
                  {row.name}
                </span>
                {row.text === undefined ? null : (
                  <span className="armada-judge-verdicts__text">{row.text}</span>
                )}
              </>
            );
            return (
              <Fragment key={row.criterionId}>
                <TableRow
                  data-refused={refused ? "true" : undefined}
                  data-opens={opens ? "true" : undefined}
                  // The whole row opens it. The chevron is the affordance, not
                  // the target: a 16px glyph in a gutter is a smaller thing to
                  // hit than the sentence a reader is already looking at, and
                  // both mean the same act.
                  onClick={opens ? () => press(row.criterionId) : undefined}
                >
                  {/* The disclosure sits in its own gutter, left of the number.
                      Inline before the criterion it indented every refused
                      title by 16px and left the unrefused ones where they
                      were, so a column of criteria no longer shared a left
                      edge — the control was setting the alignment of the
                      content beside it. `RunTree` puts its chevron in a gutter
                      for the same reason and this now matches it.

                      A row with nothing to open draws the column and leaves it
                      empty, so every criterion still starts on one edge. */}
                  <TableCell className="armada-judge-verdicts__gutter">
                    {opens ? (
                      <button
                        type="button"
                        className="armada-judge-verdicts__caret"
                        aria-expanded={isOpen}
                        aria-labelledby={nameId}
                        // The row handles the press. Without stopping here the
                        // chevron's click also bubbles to the row, the state
                        // toggles twice, and the control reads as dead.
                        onClick={(event) => {
                          event.stopPropagation();
                          press(row.criterionId);
                        }}
                      >
                        {isOpen ? (
                          <ChevronUp size={CHEVRON} strokeWidth={MARK_STROKE} aria-hidden />
                        ) : (
                          <ChevronRight size={CHEVRON} strokeWidth={MARK_STROKE} aria-hidden />
                        )}
                      </button>
                    ) : null}
                  </TableCell>
                  <TableCell variant="mono">
                    {row.ordinal === undefined ? null : String(row.ordinal).padStart(2, "0")}
                  </TableCell>
                  <TableCell>
                    {said}
                    {row.split === undefined ? null : (
                      // On the closed row rather than only inside, so three
                      // refusals can be told apart without opening one — and
                      // what `2 of 3` is for is the thing a bare figure cannot
                      // say, so it is hovered.
                      <Tooltip asChild label={conceptSaid("the split")}>
                        <span className="armada-judge-verdicts__split">{row.split}</span>
                      </Tooltip>
                    )}
                  </TableCell>
                  {row.marks === undefined ? (
                    // One band across every judge column. A criterion a Check
                    // settled has no per-judge answer, and drawing an empty
                    // cell under each judge would read as three judges who said
                    // nothing rather than three who were never asked.
                    <TableCell
                      colSpan={judges.length}
                      variant="mono"
                      className="armada-judge-verdicts__measured"
                    >
                      {row.measured}
                    </TableCell>
                  ) : (
                    row.marks.map((mark, at) => {
                      const Glyph = glyphs?.[mark];
                      const means = meanings?.[mark] ?? MEANS[mark];
                      return (
                        <TableCell
                          key={at}
                          className="armada-judge-verdicts__mark"
                          data-mark={mark}
                        >
                          {Glyph ? (
                            <Glyph size={MARK_ICON} strokeWidth={MARK_STROKE} aria-hidden />
                          ) : null}
                          {/* The glyph is the whole cell, so the word behind it
                              is what a reader who is not looking at it gets. */}
                          <span className="armada-judge-verdicts__sr">{means}</span>
                        </TableCell>
                      );
                    })
                  )}
                </TableRow>
                {/* Kept in the table while collapsed and hidden rather than
                    unmounted, which is what `Chapter` does one level out: a
                    refusal that remounted would lose the scroll position of
                    whatever the reader had opened inside it. */}
                {opens ? (
                  <TableRow className="armada-judge-verdicts__refusal" hidden={!isOpen}>
                    <TableCell colSpan={judges.length + 3}>{row.refusal}</TableCell>
                  </TableRow>
                ) : null}
              </Fragment>
            );
          })}
        </TableBody>
      </Table>
    </div>
  );
}
