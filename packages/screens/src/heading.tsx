// The Job header, built — and the Job that has no header, so cannot be drawn.
//
// The badge, the headline, the Job's own facts, the acts that end or replace
// it, and the one way out of the app. **Four things about the Job rather than
// about any step**, which is the line between this file and `step.tsx`: what is
// here changes when the Job does, and what is there changes when the selection
// does.
//
// **This is where the next thing lands.** Pilot's slot is left of the kill
// group — `#250` — and the pull request link arrived here in `#422`. Both were
// written into `JobDetail.tsx` because that was where the header was assembled;
// the header has a file now, and the screen has one line.
//
// **A Job the registry has no glyph or verb for has no header at all**, so
// `headingOf` answers `null` rather than a half-built one, and `Unrenderable`
// is what the screen draws instead. The two are here together because they are
// one decision: the badge is the header, so there is no partial render to fall
// back to.

import type { JobDetailHeading } from "@armada/components";
import type {
  FileReport,
  JobDetail as JobWhole,
  JobSummary,
  Outcome,
  WorkflowSummary,
} from "@armada/protocol";
import { Acts, type ConfirmableAct } from "./Acts";
import { factsOf } from "./facts";
import { openPullRequest, type OpenPullRequest } from "./opening";
import { leading, readingOf } from "./reading";
import type { Render } from "./render";

/** What the header is built from. The Job's, never a step's. */
export type Heading = {
  job: JobSummary;
  /** `GET /jobs/:job_id`, or `null` while it has not arrived. */
  whole: JobWhole | null;
  workflow: WorkflowSummary | undefined;
  /** Now, injected. A whole-Job elapsed is read, so it has to move. */
  now: number;
  render: Render;
  stale: boolean;
  acting: boolean;
  approving: boolean;
  /** Whether the report dialog is up. Held by the screen; `b` opens it too. */
  reporting: boolean;
  onReporting: (reporting: boolean) => void;
  onAct: (act: ConfirmableAct, jobId: string) => void;
  onApprove: (jobId: string) => void;
  onReport: (jobId: string, filing: FileReport) => Promise<Outcome>;
  onOpenPullRequest: OpenPullRequest;
  onCopied: (value: string) => void;
  /** Say a sentence to the person. Only ever a failure — see `opening.ts`. */
  onSaid: (sentence: string) => void;
};

/**
 * The Job header. **`null` is a Job that cannot be drawn**, not a header that
 * is missing a field: the badge is the header, and the registry carries no
 * sanctioned glyph or verb for this status.
 */
export function headingOf({
  job,
  whole,
  workflow,
  now,
  render,
  stale,
  acting,
  approving,
  reporting,
  onReporting,
  onAct,
  onApprove,
  onReport,
  onOpenPullRequest,
  onCopied,
  onSaid,
}: Heading): JobDetailHeading | null {
  const reading = readingOf(job);
  if (reading.as !== "badge") return null;
  return {
    status: reading.status,
    statusIcon: reading.icon,
    // The registry's own verb, opening a line. `enum-verbs.toml` spells it
    // lowercase because most of its readings are mid-sentence; here it is the
    // first word in the badge, and the badge is the header.
    statusLabel: leading(reading.verb),
    headline: job.title,
    jobId: job.id,
    fields: factsOf(job, whole, workflow, now),
    // The acts that end or replace the Job. **Pilot's slot is this one**, left
    // of the kill group — #250, and it lands without this line changing.
    actions: (
      <Acts
        job={job}
        whole={whole}
        render={render}
        acting={acting}
        approving={approving}
        stale={stale}
        onAct={onAct}
        onApprove={onApprove}
        onReport={onReport}
        reporting={reporting}
        onReporting={onReporting}
        onCopied={onCopied}
      />
    ),
    // The pull request fact, followed. The address the link carries is what the
    // fact drew from; what is sent is the Job id, so the string never decides
    // what opens. `opening.ts` says why.
    onFollowed: () => {
      void openPullRequest(onOpenPullRequest, job.id).then((because) => {
        if (because !== null) onSaid(because);
      });
    },
  };
}

/**
 * A Job the registry has no sanctioned glyph, verb or hue for. The badge is
 * the header, so there is no partial render to fall back to — and no glyph is
 * invented for it here any more than in the list.
 */
export function Unrenderable({ job }: { job: JobSummary }) {
  const reading = readingOf(job);
  const missing = reading.as === "badge" ? ["variant"] : reading.missing;
  return (
    <p className="text-fg-muted">
      {`${job.title} — `}
      <span className="mono">{job.status}</span>
      {`. The registry carries no ${missing.join(" and no ")} for it, so this Job has no detail to draw.`}
    </p>
  );
}
