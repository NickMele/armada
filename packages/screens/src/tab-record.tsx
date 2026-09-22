// Record — what this Job has produced so far, read after the fact.
//
// **One strip on this screen, and it is the destination strip.** `JobRecord`
// folds the same readings behind a strip of its own, and drawn here that is two
// segmented controls stacked, four pixels apart, in the same treatment — looked
// at, it reads as one broken control rather than two working ones. So the
// sections are regions in a column instead, which is the grammar the run column
// already uses, and `JobRecord` keeps its one consumer until `#1537` deletes
// it. That issue's own words are "replace `JobRecord` rather than nesting it".
//
// `#1537` replaces this with one ledger: when, step, kind, what, outcome, eight
// filters, and a row that opens in the inspector.

import { JobBrief, RunTree, type JobBriefProps, type RunTreeStep } from "@armada/components";

import { TAB_LABEL } from "./detail-tabs";
import { Eyebrow } from "./InsideAJob";

export type RecordTabProps = {
  /** The run, exactly as Overview's tree draws it. */
  run: RunTreeStep[];
  /** The frozen brief, where Fleet has answered with one. */
  brief?: JobBriefProps;
  onCopied?: (value: string) => void;
};

export function RecordTab({ run, brief, onCopied }: RecordTabProps) {
  const nothing = run.length === 0 && brief === undefined;
  return (
    <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.record}>
      {nothing ? (
        <p className="armada-inside__absent" role="note">
          Fleet has not answered for this job, so there is no record to fold.
        </p>
      ) : null}
      {run.length === 0 ? null : (
        <section className="armada-detail-tab__region" aria-label="Steps and checks">
          <Eyebrow>Steps and checks</Eyebrow>
          {/* Nothing pulses here. A record is read after the fact, and a mark
              that loops would claim this page is the live one. */}
          <RunTree steps={run} pulsing={false} onCopied={onCopied} />
        </section>
      )}
      {brief === undefined ? null : (
        <section className="armada-detail-tab__region" aria-label="What it was told">
          <Eyebrow>What it was told</Eyebrow>
          <JobBrief {...brief} />
        </section>
      )}
    </div>
  );
}
