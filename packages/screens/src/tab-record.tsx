// Record — what this Job produced, folded.
//
// **Today's `JobRecord`, given the destination it was built for.** The
// composition has existed since the fold was designed and nothing had a place
// to draw it; the Record tab is that place. Its sections are the caller's, as
// they always were, and they are drawn from readings this screen already holds
// — nothing here asks Fleet for anything Overview did not.
//
// `#1537` replaces it with one ledger: when, step, kind, what, outcome, eight
// filters and a row that opens in the inspector. It replaces `JobRecord`
// rather than nesting inside it, which is why nothing below grows a section.

import { JobBrief, JobRecord, RunTree, type JobBriefProps, type JobLogReferenceRow, type RunTreeStep } from "@armada/components";

import { TAB_LABEL } from "./detail-tabs";
import { WhereRegion } from "./InsideAJob";

export type RecordTabProps = {
  /** The run, exactly as Overview's tree draws it. */
  run: RunTreeStep[];
  /** The frozen brief, where Fleet has answered with one. */
  brief?: JobBriefProps;
  /** Where things are, in the rows the surface already builds. */
  where?: JobLogReferenceRow[];
  onCopied?: (value: string) => void;
};

export function RecordTab({ run, brief, where, onCopied }: RecordTabProps) {
  const sections = [
    ...(run.length === 0
      ? []
      : [
          {
            id: "steps",
            label: "Steps and checks",
            panel: <RunTree steps={run} pulsing={false} onCopied={onCopied} />,
          },
        ]),
    ...(brief === undefined
      ? []
      : [{ id: "told", label: "What it was told", panel: <JobBrief {...brief} /> }]),
    ...(where === undefined || where.length === 0
      ? []
      : [
          {
            id: "paths",
            label: "Where the work is",
            panel: <WhereRegion rows={where} onCopied={onCopied} />,
          },
        ]),
  ];

  return (
    <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.record}>
      <JobRecord
        sections={sections}
        emptyNote="Fleet has not answered for this job, so there is no record to fold."
      />
    </div>
  );
}
