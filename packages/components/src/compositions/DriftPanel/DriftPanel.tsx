import { useState, type ReactNode } from "react";
import { Button } from "../../primitives/Button/Button";

/**
 * Drift — whether the repository still has what `armada.yml` names. Journey 9,
 * *Verify*: a free read, run on opening the Manifest surface.
 *
 * **Two verdicts, as words, amber throughout.** `gone` and `current`, never
 * red: a drifted file is behind, not broken, and the dry-run beside this panel
 * is what would prove broken.
 *
 * **It reports and never fixes.** No row carries an act — no Apply, no Accept
 * all — because acting on one means editing the file, where the consequence is
 * stated. Whatever a row names, the Manifest's own `id` included, it offers
 * nothing to press.
 *
 * **Its own panel, never half of Verify's**, and it says what it says nothing
 * about: a clean list must not read as "all of this is still right".
 *
 * **Data in.** No protocol type: a caller reads the wire into rows.
 */

export type DriftPanelRow = {
  id: string;
  /** Where the line is in the file — `checks.lint.run`. Mono. */
  where: string;
  /** The line, as the repository wrote it. Mono. */
  run: string;
  verdict: "gone" | "current";
  /** What the line names that the checkout does not have. Present on `gone`. */
  missing?: string[];
  /** What the read did not follow, and why, whatever the verdict. */
  unfollowed?: { word: string; why: ReactNode }[];
};

export type DriftPanelProps = {
  /** One row per line the file declares, in the order it writes them. */
  rows?: DriftPanelRow[];
  /** Said instead of the rows while there are none: reading, or why not. */
  note?: ReactNode;
};

export function DriftPanel({ rows, note }: DriftPanelProps) {
  // Current lines fold away: the rows a person came for are the gone ones,
  // and fourteen current lines above the run page would push it off screen.
  const [showingCurrent, setShowingCurrent] = useState(false);
  const gone = (rows ?? []).filter((row) => row.verdict === "gone");
  const current = (rows ?? []).filter((row) => row.verdict === "current");

  return (
    <section className="armada-drift-panel" aria-label="Drift">
      <div className="armada-drift-panel__head">
        <span className="armada-drift-panel__title">Drift</span>
        <p className="armada-drift-panel__says">
          {rows === undefined ? note : summaryOf(gone.length, rows.length)}
        </p>
      </div>

      {gone.length === 0 ? null : (
        <ul className="armada-drift-panel__rows">
          {gone.map((row) => (
            <DriftRow key={row.id} row={row} />
          ))}
        </ul>
      )}

      {current.length === 0 ? null : (
        <>
          <span className="armada-drift-panel__fold">
            <Button
              variant="ghost"
              size="sm"
              aria-expanded={showingCurrent}
              onClick={() => setShowingCurrent(!showingCurrent)}
            >
              {`${showingCurrent ? "Hide" : "Show"} the ${linesOf(current.length)} still current`}
            </Button>
          </span>
          {showingCurrent ? (
            <ul className="armada-drift-panel__rows">
              {current.map((row) => (
                <DriftRow key={row.id} row={row} />
              ))}
            </ul>
          ) : null}
        </>
      )}

      <p className="armada-drift-panel__scope">
        Drift reads whether what each run, serve and ready line names is still in this
        checkout. It says nothing about policy, permissions or budgets, which name nothing
        runnable, or about whether a command still does the right thing.
      </p>
    </section>
  );
}

function DriftRow({ row }: { row: DriftPanelRow }) {
  const missing = row.missing ?? [];
  return (
    <li className="armada-drift-panel__row">
      <span className="armada-drift-panel__where">{row.where}</span>
      <span className="armada-drift-panel__verdict">{row.verdict}</span>
      <code className="armada-drift-panel__run">{row.run}</code>
      {missing.length === 0 ? null : (
        <p className="armada-drift-panel__detail">
          {"Not in this checkout: "}
          <span className="armada-drift-panel__named">{missing.join(", ")}</span>
        </p>
      )}
      {(row.unfollowed ?? []).map((one) => (
        <p className="armada-drift-panel__detail" key={one.word}>
          {"Not followed: "}
          <span className="armada-drift-panel__named">{one.word}</span>
          {" — "}
          {one.why}
        </p>
      ))}
    </li>
  );
}

function linesOf(count: number): string {
  return count === 1 ? "1 line" : `${count} lines`;
}

function summaryOf(gone: number, total: number): string {
  if (gone === 0) return `None of the ${linesOf(total)} names something this checkout no longer has.`;
  return `${gone} of ${linesOf(total)} ${gone === 1 ? "names" : "name"} something this checkout no longer has.`;
}
