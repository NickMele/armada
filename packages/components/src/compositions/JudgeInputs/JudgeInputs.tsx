import type { ReactNode } from "react";
import { useState } from "react";
import { Tabs } from "../../primitives/Tabs/Tabs";

/**
 * Judge inputs — what the panel was shown, and the evidence that it was one
 * panel.
 *
 * **This is the view a log file could never give you.** A panel is only a panel
 * if the judges ran independently on identical inputs. The digest is the
 * evidence for that guarantee, and it is the first thing to doubt when a
 * unanimous verdict looks too easy.
 *
 * **You can open any judge's own object, not just be told they agreed.** It
 * carried one summary and a sentence saying whether they matched, which asks a
 * reader to take the most important claim on the screen on trust — and on the
 * Job where they *did* differ, being told *j2 was shown something else* without
 * being able to see what j2 was shown is the least useful place to stop. The
 * segmented control is the whole point: `Compare` is the reading, and each
 * judge is the object it was actually handed.
 *
 * **`Compare` leads and is the default.** Whether the panel was a panel is the
 * question the view exists to answer; which paths j2 got is what you ask once
 * the answer is no.
 *
 * **A row that differs is marked in the per-judge views too.** Reading j2's
 * object alone, nothing about a digest says it is the odd one — the mark is
 * what carries the comparison into the view that has lost it.
 *
 * **The assurance is never silent.** It says every judge received the same
 * object, or it says they did not; a rows-with-no-verdict rendering is a set of
 * facts with the one question about them unanswered.
 *
 * **Every row is machine-derived, so every value is mono.** Nothing here was
 * written by anybody — it is what Fleet handed the panel, read back.
 *
 * **Nothing on the wire serves this.** No scope digest and no per-judge input
 * object exists on `Judged`; the guarantee this view checks is one the wire
 * cannot yet be asked about. Fixtures only.
 */

export type JudgeInputRow = {
  /** What it is — `scope digest`, `context_paths`, `yardstick`. Mono: a field name. */
  name: ReactNode;
  /** What it was. Mono, and it wraps — a digest clipped from the right is gone. */
  value: ReactNode;
  /**
   * Whether this field is one the judges were not handed alike. **Marked in
   * every view**, including the per-judge ones, because a digest read on its
   * own says nothing about whether it matches anybody else's.
   */
  differs?: boolean;
};

/** One judge, and the object it was actually handed. */
export type JudgeInputsFor = {
  /** What a selection names. */
  id: string;
  /** The judge, as the panel numbers it — `j1`. */
  judge: string;
  rows: JudgeInputRow[];
};

export type JudgeInputsProps = {
  /**
   * The comparison — what every judge got, and any field they did not get
   * alike. This is the default view.
   */
  rows: JudgeInputRow[];
  /**
   * Each judge's own object. **Absent draws no segmented control**, which is
   * the honest rendering where only the comparison was recorded: a segment per
   * judge that opened the same summary would promise a reading nobody kept.
   */
  each?: JudgeInputsFor[];
  /**
   * That every judge received the identical object. **Rendered as met**,
   * because it is the guarantee the panel rests on.
   */
  identical?: ReactNode;
  /**
   * That they did not, and how they differed. **Rendered as a refusal**, and
   * it displaces `identical` rather than sitting beside it: the two are
   * answers to one question.
   */
  differed?: ReactNode;
  /** The label over the block, where it stands on its own. */
  label?: ReactNode;
  /** Which view is open, held by the caller. Absent holds its own. */
  view?: string;
  onView?: (viewId: string) => void;
};

/** What the comparison view is called, and the id it answers to. */
const COMPARE = "compare";

export function JudgeInputs({
  rows,
  each,
  identical,
  differed,
  label,
  view,
  onView,
}: JudgeInputsProps) {
  const [held, setHeld] = useState(COMPARE);
  const controlled = view !== undefined;
  const open = controlled ? view : held;

  function press(id: string) {
    if (!controlled) setHeld(id);
    onView?.(id);
  }

  const showing = each?.find((one) => one.id === open);
  const shown = showing?.rows ?? rows;

  return (
    <div className="armada-judge-inputs">
      {label ? <span className="armada-judge-inputs__label">{label}</span> : null}
      {each === undefined || each.length === 0 ? null : (
        <Tabs
          items={[
            { id: COMPARE, label: "Compare" },
            ...each.map((one) => ({ id: one.id, label: one.judge })),
          ]}
          value={open}
          onChange={press}
        />
      )}
      {/* The assurance belongs to the comparison. Repeating it over one judge's
          object would attach a claim about the panel to a reading of one
          member of it. */}
      {showing !== undefined ? null : differed === undefined ? (
        identical === undefined ? null : (
          <p className="armada-judge-inputs__assurance" data-named="met">
            {identical}
          </p>
        )
      ) : (
        <p className="armada-judge-inputs__assurance" data-named="not_met">
          {differed}
        </p>
      )}
      <dl className="armada-judge-inputs__rows">
        {shown.map((row, at) => (
          // Both halves are direct children of one grid, so every value starts
          // on one edge and the block reads down. A wrapper per pair would give
          // each its own grid and align nothing.
          <div
            className="armada-judge-inputs__pair"
            key={at}
            data-differs={row.differs ? "true" : undefined}
          >
            <dt className="armada-judge-inputs__name">{row.name}</dt>
            <dd className="armada-judge-inputs__value">
              {row.value}
              {row.differs ? (
                <span className="armada-judge-inputs__differs">
                  not what every judge was handed
                </span>
              ) : null}
            </dd>
          </div>
        ))}
      </dl>
    </div>
  );
}
