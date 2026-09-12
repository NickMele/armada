import type { ReactNode } from "react";
import { Button } from "../../primitives/Button/Button";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Tabs } from "../../primitives/Tabs/Tabs";

/**
 * The evidence viewer — one artifact, at full size, on the layer that can
 * hold it. One viewer, and everything on screen points into it: a check's
 * output, a patch, a measurement, a panel's judgment are all artifacts
 * with a kind and a file, so they get one renderer set and one layer. The
 * check row's output control, a criterion's proof chip and a judge's
 * citation are three selectors that all land here.
 */

/**
 * It is a sheet for the reason the activity log is one: 2,180 lines of
 * console output is not a longer version of the four rows the chapter
 * previews, so the artifact leaves the panel and the chapter line is the
 * way back — `Sheets.tsx` holds the one-sheet-at-a-time rule.
 *
 * Three sizes, and this is the middle one: the chapter's preview composes
 * the page, this holds the whole artifact with its tools, and *Open full
 * page* leaves for a route, for the genuinely huge. Nothing at any size
 * covers the criteria — the yardstick stays on the screen behind the
 * layer.
 */

/**
 * Views are alternate renderings of one artifact, never other artifacts:
 * Output, Assertions and Timing are three ways of drawing one check run;
 * Verdicts, Citations and Inputs are three ways of drawing one judgment.
 * Those sit in the header, since they are this artifact. A different
 * artifact is a different selection, from the strip on this layer — under
 * the reading, not over it.
 */

/**
 * There is no separate control for going back to what the page opened
 * with, and there was: `Back to the default view` sat beside `Close`, and
 * a reviewer could not tell the two apart — both are the shape of an
 * escape, and neither says what it leaves. The way back was always in the
 * strip: the artifact the page composed with is a chip like any other,
 * marked as the one it opens on, and pressing it restores. A second
 * control saying that in different words was a second vocabulary for one
 * act.
 */

export type EvidenceView = { id: string; label: string };

export type EvidenceSheetProps = {
  open: boolean;
  /**
   * What kind of artifact this is — `Console output`, `Test results`, `Judge
   * verdicts`, `Code diff`. Sentence case, and it is the sheet's title because
   * it says which renderer is below before the name is read.
   */
  kind: string;
  /** Which artifact — `check:test_suite — output`. Mono, in the subtitle. */
  name?: ReactNode;
  /** The step it came from. Restated here: the tree is under the layer. */
  step?: ReactNode;
  /** The Job, in mono. Absent at the floor, where the width is not there. */
  jobId?: ReactNode;
  /** What it weighs — `2,180 lines · 4.1MB`. Mono, and last in the subtitle. */
  extent?: ReactNode;
  /** Alternate renderings of this one artifact. Absent where there is only one. */
  views?: EvidenceView[];
  view?: string;
  onView?: (viewId: string) => void;
  /**
   * The other artifacts this step produced, as selectors — an `EvidenceStrip`.
   *
   * **Under the artifact and shut, not a band above it.** It was a band, which
   * meant opening the Judge's verdicts showed a list of what the step produced
   * before showing the verdict: the reader pressed one thing and met another.
   * It is where to go next rather than what to read now, so it sits after the
   * reading and behind a disclosure.
   */
  strip?: ReactNode;
  /**
   * What the disclosure says while it is shut — `5 others from this step`.
   *
   * **A count, not a claim about completeness.** `Everything this step
   * produced` collided with the Produced chapter on the panel behind, which is
   * the same list under a different name; this says what is behind the control
   * and leaves the chapter to be the record.
   */
  stripLabel?: ReactNode;
  /** Leaves for a route. Absent on an artifact a sheet can hold whole. */
  onOpenFull?: () => void;
  children: ReactNode;
  /** The window is at `--window-floor`. */
  floor?: boolean;
  onClose?: () => void;
};

export function EvidenceSheet({
  open,
  kind,
  name,
  step,
  jobId,
  extent,
  views,
  view,
  onView,
  strip,
  stripLabel = "Other evidence from this step",
  onOpenFull,
  children,
  floor = false,
  onClose,
}: EvidenceSheetProps) {
  const tabs =
    views === undefined || views.length < 2 ? undefined : (
      <Tabs items={views} value={view ?? views[0]!.id} onChange={(id) => onView?.(id)} />
    );

  return (
    <Sheet
      open={open}
      contained
      size="wide"
      floor={floor}
      title={kind}
      subtitle={
        <>
          {name === undefined ? null : <span className="armada-evidence-sheet__mono">{name}</span>}
          {step === undefined ? null : <>{" · "}{step}</>}
          {jobId === undefined || floor ? null : (
            <>
              {" · "}
              <span className="armada-evidence-sheet__mono">{jobId}</span>
            </>
          )}
          {extent === undefined ? null : (
            <>
              {" · "}
              <span className="armada-evidence-sheet__mono">{extent}</span>
            </>
          )}
        </>
      }
      // At the floor the views drop into the strip band, for the reason the
      // log's filters do: a title, a subtitle and a close in 768px is already
      // the line that breaks.
      controls={floor ? undefined : tabs}
      closeLabel="Close"
      closeBinding="Esc"
      bleed
      // Only the views live in a band. The strip used to as well, which put a
      // list of other artifacts above the one the reader had just asked for:
      // press Verdicts, and the first thing the layer shows is what the step
      // produced. It is below the artifact now — see the disclosure under the
      // body.
      bands={
        floor && tabs !== undefined ? (
          <div className="armada-evidence-sheet__strip">{tabs}</div>
        ) : undefined
      }
      onClose={onClose}
    >
      <div className="armada-evidence-sheet__body">{children}</div>
      {strip === undefined ? null : (
        // Under the artifact, and shut.
        //
        // **You came here for one thing.** The strip is where to go next, not
        // what to read now, and a reader who opened the panel to see a verdict
        // should meet the verdict. Closed by default for the same reason: it
        // costs a line until it is wanted, and the count on the summary is
        // enough to say something is behind it.
        <details className="armada-evidence-sheet__more">
          <summary className="armada-evidence-sheet__more-head">{stripLabel}</summary>
          <div className="armada-evidence-sheet__more-body">{strip}</div>
        </details>
      )}
      {onOpenFull === undefined ? null : (
        // Last, under the artifact, rather than beside the close. Leaving the
        // screen is what you reach for after the sheet was not enough — never
        // the first control your eye lands on.
        <div className="armada-evidence-sheet__leave">
          <Button variant="secondary" size="sm" ground="sunken" onClick={onOpenFull}>
            Open full page
          </Button>
        </div>
      )}
    </Sheet>
  );
}
