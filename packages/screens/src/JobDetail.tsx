// One Job, read whole, at five destinations: Overview, Workflow, Plan, Record,
// Pulse. Overview is the arrangement this screen has always had — the run as a
// tree, the selected step in the inspector, its story in the order it happened
// — and the other four are where the readings that used to compete for that one
// panel go instead. `#1534`.
//
// **This file is the screen, not a tab.** It reads the Job whole, draws the
// header, the standing callout and the strip, and hands each tab what it needs.
// What a tab holds is that tab's own module: `tab-overview.tsx`,
// `tab-record.tsx`, `tab-pulse.tsx`, `tab-awaited.tsx`.

import { JobDetailHeaderActions, type JobResourcesProps } from "@armada/components";
import { useReducer, useState } from "react";
import { useAtFloor, useNarrow } from "@armada/shell";

import { countsOf, FIRST_TAB, JobTabs, type DetailTab } from "./detail-tabs";
import { headingOf, Unrenderable } from "./heading";
import { detailOf } from "./mine";
import { renderFor } from "./render";
import { replacedCallout } from "./replaced";
import { holdingOf, lookOf } from "./mine";
import { span } from "./duration";
import {
  LOOK_FAILED,
  nothingToAsk,
  PULSE_REFRESHES,
  pulseFiguresOf,
  pulseReadingOf,
  whyNoReading,
} from "./resources";
import { pulseViewOf } from "./draft/pulse";
import { NO_SHEET, sheetMoved } from "./Sheets";
import type { JobDetail as JobWhole } from "@armada/protocol";
import { AwaitedTab } from "./tab-awaited";
import { OverviewTab } from "./tab-overview";
import { PlanTab } from "./tab-plan";
import { PulseTab } from "./tab-pulse";
import { RecordTab } from "./tab-record";
import { ledgerOf } from "./draft/ledger";

export type { ConfirmableAct, HeldAct, JobAct } from "./Acts";
export type { FoldedReads } from "./mine";
export { renderFor } from "./render";
export type { Render } from "./render";
export type { DetailTab } from "./detail-tabs";
export { DETAIL_TABS, TAB_LABEL } from "./detail-tabs";

/**
 * What a caller hands this screen. It lives in `detail-props.ts`, moved there
 * when this file crossed the 900-line refusal for the third time.
 *
 * **Re-exported here on purpose.** A caller names the screen it is
 * configuring, not the file the type sits in.
 */
export type { JobDetailProps } from "./detail-props";
import type { JobDetailProps } from "./detail-props";

/**
 * The screen, **remounted for every Job it is handed.** Everything below holds
 * the open state of one reading, and none of it is the next Job's — so the
 * reset is the key, rather than an effect per piece that lands a frame late and
 * a piece nobody wrote one for.
 *
 * **Where things are is the one exception**, and it is not held here at all:
 * `whereOpen` is Fleet's own preference, handed in and saved by the caller —
 * this package stays free of Electron. That is what survives a Job switch
 * and a relaunch alike, on its own.
 */
export function JobDetail(props: JobDetailProps) {
  return <OneJob key={props.job.id} {...props} />;
}

function OneJob(props: JobDetailProps) {
  // Which destination is open. **Not held across Jobs**: a reader who opened
  // Pulse on a wedged Job is not asking for Pulse on the next one, and the key
  // above resets it with everything else.
  const [tab, setTab] = useState<DetailTab>(FIRST_TAB);

  // Whether the report dialog is up. Two controls open it — the Job header's
  // menu entry and `b` — and the keyboard is bound on the tab that draws the run.
  const [reporting, setReporting] = useState(false);
  // Whether the raise dialog is up, on `reporting`'s terms: the header's
  // button and `B` both open it.
  const [raising, setRaising] = useState(false);
  // Whether the turn-cap dialog is up. Its own state beside the cost cap's:
  // `budget_hold` offers one control or the other, never both.
  const [raisingTurns, setRaisingTurns] = useState(false);

  // Which sheet is up and what it is reading — `Sheets.tsx`'s `SheetReading`.
  // **Held here and not on the tab**, because the header opens one: Settings
  // is a menu entry on a header that no destination owns. Overview is what
  // draws the layer, so opening one from the header lands there.
  const [onSheet, move] = useReducer(sheetMoved, NO_SHEET);

  // The Job whole, read for the id the prop carries — `mine.ts`'s own check,
  // taken this early because what it answers is which Job this binding reads.
  const whole = detailOf(props.watched, props.job.id);
  // **The one binding every tab reads.** `whole.job` is Fleet's own answer to
  // `GET /jobs/:job_id`, fetched and re-read for this exact Job, so once it has
  // arrived it stands in for the board row the prop carries, which can lag a
  // `job.state_changed` event that missed or has not yet applied.
  const job = whole?.job ?? props.job;
  const render = renderFor(job);

  // Whether the inspector has a column of its own, and whether a folded sheet
  // goes flush. Both are tokens read off the document — a media feature value
  // cannot be a custom property, and `floor.ts` carries the whole of why.
  const narrow = useNarrow();
  const floor = useAtFloor();

  // The Job header, and everything that goes in it. `heading.tsx` holds what
  // it is made of — the badge, the facts, the acts that end or replace the Job,
  // and the way out to the pull request — which is where the next thing added
  // to this header goes rather than here.
  const heading = headingOf({
    job,
    whole,
    now: props.now,
    render,
    stale: props.stale,
    acting: props.acting,
    actingAct: props.actingAct,
    answered: props.answered,
    approving: props.approving,
    reporting,
    onReporting: setReporting,
    onAct: props.onAct,
    onActHeld: props.onActHeld,
    onApprove: props.onApprove,
    onReport: props.onReport,
    onRaiseCap: props.onRaiseCap,
    raising,
    onRaising: setRaising,
    onRaiseTurnCap: props.onRaiseTurnCap,
    raisingTurns,
    onRaisingTurns: setRaisingTurns,
    onOpenSettings: () => {
      setTab("overview");
      move({ move: "open", which: "settings" });
    },
    onOpenPullRequest: props.onOpenPullRequest,
    onOpenJob: props.onOpenJob,
    onCopied: props.onCopied,
    onSaid: props.onSaid,
  });

  // The badge is the header, so a Job the registry has no glyph or verb for
  // cannot be drawn at all — which is the `null` above. Named rather than
  // half-drawn.
  if (heading === null || render === "unrenderable") {
    return <Unrenderable job={job} />;
  }

  return (
    <div className="armada-screen__detail">
      <JobDetailHeaderActions {...heading} onCopied={props.onCopied} />
      {/* Under the header and above the strip, because a job that was replaced
          is where a person lands and no one destination can say so. #1439. */}
      {replacedCallout(whole?.replaced_by, props.onOpenJob)}
      <JobTabs value={tab} onChange={setTab} counts={countsOf(whole)} />

      {tab === "overview" ? (
        <OverviewTab
          {...props}
          job={job}
          whole={whole}
          render={render}
          narrow={narrow}
          floor={floor}
          onSheet={onSheet}
          onMove={move}
          onReporting={setReporting}
          onRaising={setRaising}
          onRaisingTurns={setRaisingTurns}
        />
      ) : tab === "plan" ? (
        <PlanTab
          job={job}
          whole={whole}
          floor={floor}
          stale={props.stale}
          acting={props.acting}
          deciding={props.deciding}
          onApproveReview={props.onApproveReview}
          onRedirect={props.onRedirect}
          {...(props.draft === undefined ? {} : { draft: props.draft })}
        />
      ) : tab === "record" ? (
        <RecordTab
          {...recordOf(props, whole)}
          jobId={job.id}
          narrow={narrow}
          floor={floor}
          onReadCheckOutput={props.onReadCheckOutput}
          onSaid={props.onSaid}
        />
      ) : tab === "pulse" ? (
        <PulseTab holds={pulseOf(props, whole, job.id)} />
      ) : (
        <AwaitedTab tab={tab} />
      )}
    </div>
  );
}

/**
 * What the Record tab reads.
 *
 * **Composed here rather than fetched**, because no operation answers a ledger
 * yet — `draft/ledger.ts` reads the Job whole, its evidence, its footprint and
 * its history into one list, and every one of those is a read this screen is
 * already holding for another region.
 */
function recordOf(props: JobDetailProps, whole: JobWhole | null) {
  if (whole === null) return { detail: null, rows: [] };
  const jobId = props.job.id;
  const handed =
    props.recorded.handed.state === "heard" && props.recorded.handed.jobId === jobId
      ? props.recorded.handed.moment
      : undefined;
  const footprint =
    props.recorded.footprint.state === "read" && props.recorded.footprint.jobId === jobId
      ? props.recorded.footprint.reading
      : undefined;
  const evidence =
    props.recorded.evidence.state === "read" && props.recorded.evidence.jobId === jobId
      ? props.recorded.evidence.steps
      : undefined;
  const history =
    props.history?.state === "read" && props.history.jobId === jobId
      ? props.history.moves
      : undefined;
  return {
    detail: whole,
    rows: ledgerOf({
      detail: whole,
      ...(history === undefined ? {} : { history }),
      ...(evidence === undefined ? {} : { evidence }),
      ...(footprint === undefined ? {} : { footprint }),
      ...(handed === undefined ? {} : { handed }),
    }),
  };
}

/**
 * The Pulse board: the machine reading, the figures over it, and the look.
 *
 * **`pulseViewOf` is the one derivation.** The board is built on the draft
 * shape (`draft/pulse.ts`), filled from what Fleet serves today — so the same
 * board draws the real Fleet and whatever `#1545` promotes onto the wire.
 */
function pulseOf(props: JobDetailProps, whole: JobWhole | null, jobId: string): JobResourcesProps {
  const holding = holdingOf(props.resources, jobId);
  const view = holding === null ? null : pulseViewOf(holding);
  const looked = lookOf(props.examination, jobId);
  const examined = looked?.state === "found" ? looked.examined : null;
  const nothing = nothingToAsk(props.resources);
  return {
    reading: view === null ? null : pulseReadingOf(view, examined),
    figures: pulseFiguresOf(view, whole),
    note: whyNoReading(props.resources),
    ...(view === null ? {} : { age: span(view.read_at, props.now) ?? undefined }),
    refreshed: PULSE_REFRESHES,
    examined,
    looking: looked?.state === "looking",
    ...(looked?.state === "failed" ? { lookFailed: LOOK_FAILED } : {}),
    ...(nothing === undefined ? {} : { nothingToAsk: nothing }),
    onExamine: () => props.onExamine(jobId),
  };
}
