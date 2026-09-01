// Saying a job failed in error, and what comes back when you do.
//
// **The one act on this screen that changes nothing about the job.** The acts
// beside it end a job, replace one, or move one past a verdict; this one records
// what a person concluded and leaves the job exactly where it was. That is why
// it sits in the Job header's menu rather than on its face, and why it is
// offered on every stopped job rather than only on the ones something can still
// be done to — a job nothing can be done to is the one most likely to have
// failed wrongly and been left.
//
// **The trigger is not here.** `Acts.tsx` renders the menu entry and holds
// whether this dialog is up; `b` is the binding, from `actions.toml`.
//
// # The sentence is the point, and the bundle is not
//
// Everything armada attaches to a report was already written down and already
// served — the transitions, the verdicts, the flags, the checks, the claims.
// What does not exist until somebody types it is why they think the machine was
// wrong. So the dialog asks for that in the person's own words, refuses to send
// without it, and never offers a "just attach everything" path: a report that
// bundles the record and says nothing is the terminal-paste this replaces,
// automated.
//
// # The claim is a choice and not a sentence
//
// The first override in this repository carries the reason `probe`. A required
// text field accepted it, and nothing counting a rate can read it. So what is
// counted is the value chosen here, out of three — and the sentence stays what
// a person reads rather than what a number is made of.
//
// # Armada does not file it anywhere
//
// Fleet holds no credential for the issue tracker and nothing on the wire names
// the repository's remote, so filing produces a record and an issue body.
// The dialog says so in those words and puts the body on the clipboard, because
// a control that says "file the issue" and files nothing is worse than one that
// says what it did.

import { useState } from "react";
import { Dialog, Radio, RadioGroup, Select, Textarea } from "@armada/components";

import type { Outcome } from "@armada/protocol";
import type { FileReport, JobDetail as JobWhole, Report } from "@armada/protocol";

/**
 * The three things a person can be saying, with the words the dialog offers
 * them in.
 *
 * **The one closed set Bridge writes rather than renders**, which is why the
 * wire spellings appear here at all — every other roster on this side is left
 * as `string` and drawn as it arrives. `crates/ipc/src/report.rs` is the
 * authority and this is the picker.
 *
 * A wrong refusal and a wrong pass are not one value. The first stops work that
 * was right, loudly, and somebody is there to notice; the second lets wrong
 * work through and nothing surfaces it at all. Counting them together would
 * average the two.
 */
export const CLAIMS: { value: string; label: string; note: string }[] = [
  {
    value: "wrongly_refused",
    label: "The judge refused work that was right",
    note: "The verdict is on the record and this does not lift it — overruling is the act that does.",
  },
  {
    value: "wrongly_passed",
    label: "Something wrong got through",
    note: "Nothing in armada surfaces this on its own, which is why saying it is the whole of the record.",
  },
  {
    value: "armada_misbehaved",
    label: "Armada itself did the wrong thing",
    note: "No verdict is in question — the machinery did something other than what it said it did.",
  },
];

/**
 * One claim in the words the picker offered it in. **Never a word chosen at the
 * call site** — the same wire value renders the same sentence wherever it is
 * read, which is what keeps the dialog that files a report and the list that
 * reads them back from naming the same claim two ways. An unknown spelling
 * renders as itself, the fallback every other closed set here takes.
 */
export function claimed(claim: string): string {
  return CLAIMS.find((option) => option.value === claim)?.label ?? claim;
}

/** Whether the claim is about a verdict, and so whether a criterion applies. */
function disputesAVerdict(claim: string): boolean {
  return claim === "wrongly_refused" || claim === "wrongly_passed";
}

/** One thing a person can be disagreeing with, as the picker offers it. */
type Disputable = {
  stepId: string;
  /** Absent where the scope is the step itself rather than a verdict inside it. */
  criterionId?: string;
  label: string;
};

/**
 * What a report can be aimed at on this job: each step that has run, and every
 * criterion the judge answered inside it.
 *
 * **The step itself is offered and not only its criteria.** A step that
 * escalated on `gate_undecided` has no judged criterion at all — the judge's
 * answer would not read — so a picker holding only criteria offers nothing for
 * the one case a person most wants to report, and the report then goes against
 * the whole job carrying no scope at all.
 *
 * **Both verdicts are offered**, not only the refusals: a person saying
 * something wrong got through is disagreeing with a `met`, and a picker holding
 * only refusals would make that claim unscopeable.
 *
 * A step that never started is left out. Nothing happened in it to disagree
 * with, and offering one would put most of the workflow in the list.
 */
function disputable(whole: JobWhole | null): Disputable[] {
  if (whole === null) return [];
  return whole.steps
    .filter((step) => step.state !== "not_started")
    .flatMap((step) => [
      { stepId: step.step_id, label: `${step.label} · the step itself` },
      ...step.judged.map((judged) => ({
        stepId: step.step_id,
        criterionId: judged.criterion_id,
        label: `${step.label} · ${judged.criterion_id} · ${judged.verdict}`,
      })),
    ]);
}

/**
 * The value the select holds. Two ids in one string, since a select holds one —
 * and the step id alone where there is no criterion.
 *
 * **Never parsed back.** The picked entry is found by comparing keys, so this
 * only has to be unique.
 */
function keyOf(verdict: Disputable): string {
  return verdict.criterionId === undefined
    ? verdict.stepId
    : `${verdict.stepId} ${verdict.criterionId}`;
}

/** The whole job, which is what a report with no criterion is about. */
const WHOLE_JOB = "";

export type ReportControlProps = {
  jobId: string;
  /** `GET /jobs/:job_id`, for the verdicts a criterion scope picks between. */
  whole: JobWhole | null;
  /**
   * Whether the dialog is up. **Held by the caller**, because the control that
   * opens it is the Job header's split-button menu and a menu item is not this
   * file's to render — see `Acts.tsx`.
   */
  open: boolean;
  /** Closed, however it closed: cancelled, filed and dismissed, or refused. */
  onClose: () => void;
  /**
   * File it. **Answers with the outcome**, because the filed record is what the
   * dialog shows next — putting it in app state would leave a report on screen
   * after the dialog that filed it closed.
   */
  onReport: (jobId: string, filing: FileReport) => Promise<Outcome>;
  /** A clipboard write is silent, so the surface confirms it. */
  onCopied: (value: string) => void;
};

/**
 * The dialog a report is filed in, and what it becomes once one is.
 *
 * **One dialog and not two.** Filing and reading back what was filed are one
 * continuous act for the person doing it: they press, and the thing they need
 * next is the record they are about to carry somewhere else. A second dialog
 * would put a modal in front of a modal to say what the first one produced.
 *
 * **It no longer carries its own button.** `Report this job` is a menu entry on
 * the Job header's split button now, so the caller owns whether the dialog is
 * up — and this file keeps everything the dialog asks for and everything it
 * shows back. What a person filled in is still dropped on close, which is why
 * every field is reset in `close` rather than remembered for the next press.
 */
export function ReportControl({
  jobId,
  whole,
  open,
  onClose,
  onReport,
  onCopied,
}: ReportControlProps) {
  const [claim, setClaim] = useState(CLAIMS[0]!.value);
  const [verdict, setVerdict] = useState(WHOLE_JOB);
  const [said, setSaid] = useState("");
  const [filing, setFiling] = useState(false);
  /** The report that came back. While it is here, the dialog is showing it. */
  const [filed, setFiled] = useState<Report | null>(null);
  /** Why the send did not land, in Fleet's words or Bridge's. */
  const [refused, setRefused] = useState<string | null>(null);

  const verdicts = disputable(whole);
  const scoped = disputesAVerdict(claim) && verdicts.length > 0;

  function close(): void {
    onClose();
    setClaim(CLAIMS[0]!.value);
    setVerdict(WHOLE_JOB);
    setSaid("");
    setFiled(null);
    setRefused(null);
  }

  async function file(): Promise<void> {
    const picked = verdicts.find((one) => keyOf(one) === verdict);
    setFiling(true);
    setRefused(null);
    try {
      const answer = await onReport(jobId, {
        claim,
        said,
        // The step is sent alone or with its criterion, and a criterion is
        // never sent without its step: a criterion id is unique inside a step,
        // and one on its own names every attempt of every step at once.
        ...(scoped && picked !== undefined
          ? {
              step_id: picked.stepId,
              ...(picked.criterionId === undefined
                ? {}
                : { criterion_id: picked.criterionId }),
            }
          : {}),
      });
      if (answer.ok && answer.report !== undefined) {
        setFiled(answer.report);
        return;
      }
      if (answer.ok) {
        // Fleet took it and answered something Bridge could not read as a
        // report. The filing happened; saying otherwise sends somebody to do it
        // twice.
        setFiled(null);
        close();
        return;
      }
      setRefused(
        answer.why === "refused" ? answer.error.message : `The report was not filed: ${answer.why}.`,
      );
    } finally {
      setFiling(false);
    }
  }

  function copy(): void {
    if (filed === null) return;
    void navigator.clipboard.writeText(issueOf(filed)).then(
      () => onCopied("The report"),
      // A failed clipboard write is otherwise indistinguishable from a dead
      // control, so the surface is told either way.
      () => onCopied("The report"),
    );
  }

  return (
    <>
      <Dialog
        open={open}
        tone="neutral"
        title={filed === null ? "Report this job as failed in error?" : "Filed"}
        confirmLabel={filed === null ? "File the report" : "Copy the issue"}
        confirmDisabled={filed === null && (said.trim() === "" || filing)}
        onCancel={close}
        onConfirm={() => {
          if (filed === null) {
            void file();
            return;
          }
          copy();
          close();
        }}
      >
        {filed === null ? (
          <>
            {/* What this is, before what it asks for. The record is already
                written down; the sentence is the part that is not. */}
            <p>
              Everything armada knows about this job is attached for you — every move it made,
              what each gate said, what the drone claimed, and what it changed. What it does not
              have is why you think it was wrong.
            </p>
            <RadioGroup label="What was wrong">
              {CLAIMS.map((option) => (
                <Radio
                  key={option.value}
                  name="report-claim"
                  value={option.value}
                  checked={claim === option.value}
                  onChange={() => setClaim(option.value)}
                >
                  {option.label}
                </Radio>
              ))}
            </RadioGroup>
            <p>{CLAIMS.find((option) => option.value === claim)?.note}</p>
            {scoped ? (
              <Select
                label="What it is about, if it is not the whole job"
                value={verdict}
                onChange={(event) => setVerdict(event.target.value)}
              >
                <option value={WHOLE_JOB}>The whole job</option>
                {verdicts.map((one) => (
                  <option key={keyOf(one)} value={keyOf(one)}>
                    {one.label}
                  </option>
                ))}
              </Select>
            ) : null}
            {/* No `autoFocus`: the dialog's own contract puts initial focus on
                Cancel, and a second claim on it would only lose to it. */}
            <Textarea
              label="What you know went wrong"
              rows={5}
              value={said}
              onChange={(event) => setSaid(event.target.value)}
            />
            {refused === null ? null : <p>{refused}</p>}
          </>
        ) : (
          <>
            {/* What was filed, and what was not done with it. Said plainly:
                a control that claimed to have opened an issue would be lying
                about the one step still left to a person. */}
            <p>
              The report is on this machine and outlives the job — cleaning the job up leaves it
              whole. <strong>Armada does not open anything in the tracker</strong>; copying puts the
              issue below on your clipboard.
            </p>
            <Textarea label="The issue" rows={12} readOnly value={issueOf(filed)} />
          </>
        )}
      </Dialog>
    </>
  );
}

/**
 * The filed report as an issue body: the person's sentence first, the record
 * under it, and the claim said in words.
 *
 * **The sentence leads.** It is the finding, and a reader who stops after the
 * first paragraph has the part that could not be reconstructed from the
 * database. It is also a claim rather than a verified fact, and it says so —
 * a button that turns a hunch into an issue makes filing a wrong one cheap, and
 * the record beneath is what a reader checks it against.
 */
/**
 * What the report is about, in one phrase.
 *
 * **Three widths.** A step with no criterion is not the whole job: it is what a
 * report about a step the gate judged nothing on looks like, and rendering it as
 * "the job as a whole" would throw away the scope on the way to the reader.
 */
function scopeOf(report: Report): string {
  if (report.step_id === undefined) return "the job as a whole";
  if (report.criterion_id === undefined) return `${report.step_id}, the step itself`;
  return `${report.step_id} · ${report.criterion_id}`;
}

export function issueOf(report: Report): string {
  const scope = scopeOf(report);
  return [
    `# ${report.job_title}`,
    "",
    `**${claimed(report.claim)}** — ${scope}.`,
    "",
    "## What I know went wrong",
    "",
    report.said,
    "",
    "This is what a person concluded after reading the work, not a verified finding.",
    "The record below is what it can be checked against.",
    "",
    `Job \`${report.job_id}\`, reported ${report.filed_at}.`,
    "",
    "---",
    "",
    report.record,
  ].join("\n");
}
