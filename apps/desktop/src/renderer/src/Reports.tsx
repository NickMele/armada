// What people have said the machine got wrong, and the counts those sentences
// are read beside.
//
// # Why this is a surface and not a region of the board
//
// The board is scanned: a person opens it to find what needs them now, and
// every row on it is a job something can still be done to. A report is the
// opposite kind of reading — nothing here is waiting, nothing here can be
// acted on, and the question it answers is *has the judge been worth trusting*,
// which is asked deliberately and rarely. `docs/journeys/3-triage-queue.md`
// draws that line, and putting a calibration figure on the board would put a
// number nobody can act on beside rows that exist to be acted on.
//
// # The counts are four, and a rate is refused
//
// `crates/ipc/src/report.rs` is explicit and this only draws it. Dividing
// disputes by refusals produces a number whose denominator counts every job
// nobody read, and an unread job is not a pass — so the four counts stay four
// and the gap between them stays visible rather than divided away. A wrong
// refusal and a wrong pass are not two halves of one figure either: a refusal
// stops work loudly and somebody is there to notice, and a wrong pass is
// refused by nothing and surfaces only because a person said so.
//
// # The count reads the claim, and only a reader reads the sentence
//
// The first override in this repository carries the reason `probe`. A required
// text field took it, and no count can tell it from a considered one — which is
// why the claim is a closed set and why the sentences are on this page in full
// rather than summarised into the figures above them. The counts say how often;
// only reading says whether.
//
// # Nothing here changes anything
//
// There is no act on this surface. Filing is on the job that failed, where the
// record comes from, and nothing withdraws or edits a report — `job_events` is
// append-only and so is this. The one control is a copy, because armada opens
// nothing in an issue tracker and says so.

import { useEffect } from "react";
import {
  Alert,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeaderCell,
  TableRow,
} from "@armada/components";

import type { Reports as ReportsRead } from "../../shared/bridge";
import type { Calibration, Report } from "../../shared/protocol";
import { said } from "./copy";
import { absoluteOf } from "./duration";
import { claimed, issueOf } from "./Report";

/**
 * Ask main for the filed reports, or drop them.
 *
 * **Module scope, so it is stable**, for the reason `Stopped.tsx`'s reads are:
 * an effect depending on a lambda rebuilt every render would open and close the
 * read on a loop, and the read publishes state, so the loop would feed itself.
 */
function askForReports(want: boolean): void {
  void window.armada.readReports(want);
}

export type ReportsProps = {
  /** `GET /reports`, as main published it. */
  reports: ReportsRead;
  /** A clipboard write is silent, so the surface confirms it. */
  onCopied: (value: string) => void;
};

/**
 * Every filed report, newest first, under the counts they are read beside.
 *
 * **The read is opened by this surface and dropped when it closes.** The
 * rendered record travels with every row — `list_reports` says why, and says
 * what it costs — so a list nobody has open is bytes held for nothing.
 *
 * The rows are not virtualized, and that is a bound rather than an oversight: a
 * report is one per verdict somebody disagreed with, `list_reports` states that
 * nothing has hundreds, and the panel this mounts in is the one region that
 * scrolls. The day a store has hundreds, this is the surface that needs
 * `[list-virtualization]` answered before anything else does.
 */
export function Reports({ reports, onCopied }: ReportsProps) {
  useEffect(() => {
    askForReports(true);
    return () => askForReports(false);
  }, []);

  if (reports.state === "failed") {
    return (
      <Alert tone="escalated" title="The filed reports could not be read">
        {said(reports.outcome)}
      </Alert>
    );
  }
  // `none` is the moment before the effect above has run, which is a frame and
  // not a state anybody reads — it says the same thing as `reading` rather than
  // drawing an empty list that would read as nothing having been filed.
  if (reports.state !== "read") {
    // A muted line rather than an `Alert`, which requires the facts needed to
    // decide — `Panels.tsx` says a reading state the same way. There is nothing
    // to decide here and nothing yet to say.
    return <p className="text-fg-muted">Reading what has been reported.</p>;
  }

  return (
    <div className="flex flex-col gap-6">
      <Counts calibration={reports.list.calibration} />
      {reports.list.reports.length === 0 ? (
        <Nothing />
      ) : (
        reports.list.reports.map((report) => (
          <Filed key={report.id} report={report} onCopied={onCopied} />
        ))
      )}
    </div>
  );
}

/** What each count counts, in the order `Calibration` declares them. */
const COUNTED: { of: keyof Calibration; label: string; note: string }[] = [
  {
    of: "refusals_recorded",
    label: "Refusals the judge recorded",
    note: "Every criterion answered not met, over every job in the store and every attempt of every step.",
  },
  {
    of: "refusals_disputed",
    label: "Refusals a person disputed",
    note: "Reports saying the judge refused work that was right. Not a share of the row above: nothing says the rest were read.",
  },
  {
    of: "passes_disputed",
    label: "Passes a person disputed",
    note: "Reports saying something wrong got through. Nothing refused these, so there is no recorded population to count them against.",
  },
  {
    of: "reports_filed",
    label: "Reports filed",
    note: "Every report, including the ones claiming armada itself misbehaved — those dispute no verdict and are counted nowhere above.",
  },
];

/**
 * The four counts, and the two sentences that say what they are not.
 *
 * **A table and not a figure run.** Each count is unreadable without what it
 * counts, and three of the four are unreadable without what they *do not*
 * count — a row of large numbers over short labels would be the score this
 * deliberately is not.
 */
function Counts({ calibration }: { calibration: Calibration }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Whether the judge has been worth trusting</CardTitle>
      </CardHeader>
      <CardContent>
        <Table>
          <TableHead>
            <TableRow>
              <TableHeaderCell>What is counted</TableHeaderCell>
              <TableHeaderCell>How many</TableHeaderCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {COUNTED.map((count) => (
              <TableRow key={count.of}>
                <TableCell variant="primary">
                  {count.label}
                  {/* The caveat travels with the number rather than under the
                      table: read apart from what it does not count, three of
                      these four say something they do not mean. */}
                  <CardDescription>{count.note}</CardDescription>
                </TableCell>
                {/* Mono, because armada counted it. */}
                <TableCell variant="mono">{calibration[count.of]}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
        <p>
          Four counts, and deliberately not a rate. A rate&apos;s denominator would count every
          job nobody read, and an unread job is not a pass — so the gap between what the judge
          refused and what somebody who read the work disputed is left visible rather than
          divided away.
        </p>
        <p>
          What is counted is the claim each report chose, never the sentence beside it. A report
          filed after reading the whole diff and one filed to see whether the button worked count
          the same here, which is why the sentences are below in full: the counts say how often,
          and only reading them says whether.
        </p>
      </CardContent>
    </Card>
  );
}

/**
 * One filed report: what it was about, what the person said, and the record
 * they can carry somewhere else.
 *
 * **The sentence is the body of the card, and the record is not shown.** The
 * finding is the part that exists nowhere else; the record is everything armada
 * already knew, rendered, and putting a page of it under every row would bury
 * the one line that was written by a person. The copy is what it is for.
 */
function Filed({ report, onCopied }: { report: Report; onCopied: (value: string) => void }) {
  const filed = absoluteOf(report.filed_at);

  function copy(): void {
    void navigator.clipboard.writeText(issueOf(report)).then(
      () => onCopied("The report"),
      // A failed clipboard write is otherwise indistinguishable from a dead
      // control, so the surface is told either way.
      () => onCopied("The report"),
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{report.job_title}</CardTitle>
        {/* When, at the trailing edge where the header puts its badge. A date
            and not an age: reports are read weeks apart, and "6d ago" changes
            meaning while the page is open. */}
        {filed === null ? null : <span className="mono">{filed}</span>}
      </CardHeader>
      <CardContent>
        <CardDescription>
          {claimed(report.claim)}
          {" · "}
          <Scope report={report} />
        </CardDescription>
        {/* The finding, in the person's own words, drawn exactly as they wrote
            it. Nothing here summarises it: a summary of one sentence is the
            claim above, which the count already read. */}
        <p>{report.said}</p>
      </CardContent>
      <CardFooter>
        {/* The one control on this surface, and it changes nothing. Armada
            holds no credential for an issue tracker and opens nothing in one —
            what it can do is hand over the record whole. */}
        <Button variant="secondary" onClick={copy}>
          Copy the issue
        </Button>
      </CardFooter>
    </Card>
  );
}

/**
 * What the report is about: one criterion of one step, or the whole job.
 *
 * **The pair or neither.** A criterion id is unique inside a step, so one
 * without its step would name every attempt of every step at once — which is
 * the reason Fleet takes them together and the reason this draws them together.
 */
function Scope({ report }: { report: Report }) {
  if (report.step_id === undefined || report.criterion_id === undefined) {
    return <>the job as a whole</>;
  }
  return (
    <>
      <span className="mono">{report.step_id}</span>
      {" · "}
      <span className="mono">{report.criterion_id}</span>
    </>
  );
}

/**
 * Nothing filed, said as the fact it is.
 *
 * **Not "no data".** An empty list here means nobody has disagreed with a
 * verdict, which is a real reading of the machine and not a failure of this
 * page — and it says where filing happens, because a surface that shows only
 * what it cannot show is a dead end.
 */
function Nothing() {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Nothing has been reported</CardTitle>
      </CardHeader>
      <CardContent>
        <p>
          Nobody has said a job failed in error. That is a reading of the machine and not a gap
          in this page — though a wrong pass surfaces only because somebody says so, so it is
          also what a machine nobody has checked looks like.
        </p>
        <p>
          Reports are filed from the job they are about: open one that stopped and use{" "}
          <strong>Report this job</strong>. The job&apos;s own record is attached for you, and the
          report outlives the job — cleaning it up leaves the report whole.
        </p>
      </CardContent>
    </Card>
  );
}
