// What fleet is holding disk for, and giving back the ones you choose.
//
// # Why this is a surface and not a region of the board
//
// The board is scanned for what needs you now, and every row on it is a job
// something can still be done to. This is read the other way round: it is asked
// once, deliberately, when disk is the question — and the answer is about a set
// rather than about any one job. *Which of these do I give back* cannot be asked
// of a row, which is why the read that serves it is scoped to nothing.
//
// # The other half of the rule is on this page too
//
// Fleet reclaims what passes all five tests on its own and never asks. That
// half is invisible by design, so the worktrees it is about to take are drawn
// here in their own group rather than filtered out — a person who came looking
// for one and does not find it cannot tell "already given back" from "held and
// not said".
//
// # Per item, never all-or-nothing
//
// There is one bulk act in armada, `armada clean --everything`, and it is the
// one nobody should reach for from a screen. So the control here is up to
// three independent checkboxes per row — a checkout, a branch, a record — and
// a confirmation that reads out what each chosen row costs; there is no
// select-all, and adding one would be adding the act this surface exists to
// replace.
//
// # The confirmation says what is lost, not how much disk comes back
//
// Bytes are not the decision. Which commits go, whether anything else has them,
// which uncommitted files exist nowhere but the checkout, and which records are
// forgotten — reclaiming a checkout is still never a force, but deleting the
// branch that survives it is a separate, explicit choice a person confirms row
// by row. `held.ts` computes it and is unit-tested, because every sentence in
// it is read immediately before something is destroyed.

import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Dialog,
  HeldWorktree,
} from "@armada/components";
import { patternFor, useHaptics } from "@armada/components";
import type { ButtonAnswer, RowChoice } from "@armada/components";

import type { BranchDeleted, HeldWorktrees, JobSummary, Outcome, WorktreeReclaimed } from "@armada/protocol";
import { said } from "./copy";
import {
  choiceOf,
  chosenRows,
  confirmOpening,
  confirmTitle,
  divided,
  filesDestroyed,
  namedByHandle,
  NOTHING_IS_LOST,
  offeredOn,
  planned,
  sitting,
  unmergedOf,
} from "./held";
import type { Planned } from "./held";

export type WorktreesProps = {
  /**
   * Ask the host to open or close the read.
   *
   * **It has to be stable**, for `Reports`'s reason: this is depended on by an
   * effect, and a lambda rebuilt every render would open and close the read on
   * a loop — the read publishes state, so the loop would feed itself.
   */
  onWant: (want: boolean) => void;
  /** `GET /worktrees`, as main published it. */
  held: HeldWorktrees;
  /**
   * The board's own Jobs, read for their handles.
   *
   * **Only for naming a `depended_on` reason.** Every other read on this
   * screen comes off `held` alone; this is the one place a row on this
   * surface names a *different* job, so it is the one place this screen
   * reaches into the board's own list. Defaults to none, which falls back to
   * the id — a screen with no board read yet still has something to show.
   */
  jobs?: readonly JobSummary[];
  /**
   * Give one worktree back, and answer with what the two halves did.
   *
   * **One id at a time, and a promise rather than a callback.** There is no
   * bulk route on the wire and there should not be: each is independent, one
   * refusing does not stop the rest, and the receipt belongs to the press that
   * asked for it — publishing it as app state would make one person's gesture
   * part of what every surface re-renders on.
   */
  onReclaim: (jobId: string) => Promise<Outcome>;
  /**
   * Delete one row's branch, sending the tip the person confirmed.
   *
   * **The tip is what was confirmed, not what fleet answers with next.** Fleet
   * refuses with 409 where the tip has moved since — the safety net a stale
   * confirmation needs, not a fact this screen has to keep current itself.
   */
  onDeleteBranch: (jobId: string, tip: string) => Promise<Outcome>;
  /**
   * Delete one row's whole record. **There is no undo.** Only ever sent once
   * the checkout and the branch are gone or chosen in the same act — `held.ts`'s
   * `offeredOn` is what keeps that true before this is ever called.
   */
  onForget: (jobId: string) => Promise<Outcome>;
  /**
   * Back to the Board. **At the top of every state**, not only the read
   * one — the page head this used to live in carried it whether or not the
   * read had come back yet.
   */
  onClose: () => void;
  /**
   * The clock every elapsed figure on this surface is drawn from.
   *
   * **The app's one `now`, not a `Date.now()` per row.** Two clocks on one
   * screen drift, and a test that could not fix the instant would be asserting
   * against the wall.
   */
  now: number;
  /** A clipboard write is silent, so the surface confirms it. */
  onCopied: (value: string) => void;
};

/**
 * Every worktree fleet is holding, grouped by what can be done about it.
 *
 * The rows are not virtualized, and that is a bound rather than an oversight: a
 * worktree is one per job that has run, and fleet sweeps the safe ones away on
 * its own — the list a person meets is the residue. The day a store holds
 * hundreds of held worktrees, that is a fleet not sweeping rather than a list
 * needing windowing.
 */
export function Worktrees({
  onWant,
  held,
  jobs = [],
  onReclaim,
  onDeleteBranch,
  onForget,
  now,
  onClose,
  onCopied,
}: WorktreesProps) {
  useEffect(() => {
    onWant(true);
    return () => onWant(false);
  }, []);

  /** What is chosen for each of a row's three acts, by job id. */
  const [choices, setChoices] = useState<Record<string, RowChoice>>({});
  /** Whether the confirmation is up. */
  const [confirming, setConfirming] = useState(false);
  /** What each reclaim answered, by job id. Kept until the list is re-read. */
  const [receipts, setReceipts] = useState<Record<string, WorktreeReclaimed>>({});
  /** What each branch delete answered, by job id. Kept until re-read. */
  const [branchDeletions, setBranchDeletions] = useState<Record<string, BranchDeleted>>({});
  /** Refusals, named by row and by which of the three acts they were about. */
  const [refused, setRefused] = useState<RowFailure[]>([]);
  /** One act at a time, so a second press does not send the set twice. */
  const [sending, setSending] = useState(false);
  /** What Fleet said to the last clean-up, drawn on the control until the next press. */
  const [answer, setAnswer] = useState<ButtonAnswer>();
  const tap = useHaptics();

  /** The way out, at the top of every state — #1090 moved it here from the
   *  page head that used to carry it. */
  const back = (
    <div>
      <Button variant="ghost" size="sm" onClick={onClose}>
        Back to the list
      </Button>
    </div>
  );

  if (held.state === "failed") {
    return (
      <div className="armada-screen__pane">
        {back}
        <Alert tone="escalated" title="What fleet is holding could not be read">
          {said(held.outcome)}
        </Alert>
      </div>
    );
  }
  // `none` is the frame before the effect above has run. It says the same thing
  // as `reading` rather than drawing an empty list, which here would claim
  // fleet is holding nothing — the one answer on this page nobody should be
  // given by accident.
  if (held.state !== "read") {
    return (
      <div className="armada-screen__pane">
        {back}
        <p className="text-fg-muted">Reading what fleet is holding.</p>
      </div>
    );
  }

  const groups = divided(held.held.worktrees);
  const picked = chosenRows(groups.deciding, choices);
  const plan = planned(picked, choices);

  function choose(jobId: string, choice: RowChoice): void {
    setChoices((was) => ({ ...was, [jobId]: choice }));
  }

  /**
   * Run every chosen row's acts, one row at a time, checkout then branch then
   * record.
   *
   * **A failed checkout or branch cancels that row's forget.** `offeredOn`
   * only offers forget once both read as already gone — but a status can
   * still move between the press and this call, and sending it anyway would
   * be `worktrees_held`'s exact bug: a record forgotten while its disk stands.
   * Rows are independent of each other throughout, the way `reclaim` already
   * sent them.
   */
  async function cleanUp(): Promise<void> {
    setConfirming(false);
    setAnswer(undefined);
    setSending(true);
    const gaveBack: Record<string, WorktreeReclaimed> = {};
    const deletedBranches: Record<string, BranchDeleted> = {};
    const failed: RowFailure[] = [];

    for (const row of picked) {
      const choice = choiceOf(choices, row.job_id);
      let checkoutOk = true;
      let branchOk = true;

      if (choice.removeCheckout) {
        const outcome = await onReclaim(row.job_id);
        if (outcome.ok) {
          if (outcome.reclaimed !== undefined) gaveBack[row.job_id] = outcome.reclaimed;
          // A locked checkout answers ok and stays on disk; its receipt says why.
          if (outcome.reclaimed?.worktree.removed === false) checkoutOk = false;
        } else {
          checkoutOk = false;
          failed.push({ jobId: row.job_id, title: row.job_title, act: "checkout", outcome });
        }
      }

      if (choice.deleteBranch) {
        const unmerged = unmergedOf(row);
        if (unmerged !== null) {
          const outcome = await onDeleteBranch(row.job_id, unmerged.tip);
          if (outcome.ok) {
            if (outcome.branchDeleted !== undefined) deletedBranches[row.job_id] = outcome.branchDeleted;
          } else {
            branchOk = false;
            failed.push({ jobId: row.job_id, title: row.job_title, act: "branch", outcome });
          }
        }
      }

      if (choice.forget && checkoutOk && branchOk) {
        const outcome = await onForget(row.job_id);
        if (!outcome.ok) failed.push({ jobId: row.job_id, title: row.job_title, act: "record", outcome });
      }
    }

    setReceipts(gaveBack);
    setBranchDeletions(deletedBranches);
    setRefused(failed);
    setChoices({});
    // Any row refused is a refusal: the alerts above say which.
    const answered: ButtonAnswer = failed.length === 0 ? "accepted" : "refused";
    setAnswer(answered);
    tap(patternFor(answered));
    setSending(false);
  }

  const nothingHeld = groups.deciding.length === 0 && groups.waiting.length === 0;

  return (
    <div className="armada-screen__pane">
      {back}
      {refused.map((one) => (
        <Alert key={`${one.jobId}-${one.act}`} tone="escalated" title={refusalTitle(one)}>
          {refusedSaid(one.outcome)}
        </Alert>
      ))}

      {nothingHeld ? (
        <NothingToDecide automatic={groups.automatic.length} />
      ) : (
        <Card>
          <CardHeader>
            <CardTitle>Waiting on you</CardTitle>
          </CardHeader>
          <CardContent>
            <p>
              Fleet has already given back everything it could prove nobody needs. These are
              what is left — each one failed a safety test, which is not the same as being
              unsafe, and the reason under each row is what the decision is made on.
            </p>
            <ul>
              {groups.deciding.map((one) => {
                const choice = choiceOf(choices, one.job_id);
                return (
                  <HeldWorktree
                    key={one.job_id}
                    held={namedByHandle(one, jobs)}
                    choice={choice}
                    offered={offeredOn(one, choice)}
                    onChoose={choose}
                    reclaimed={receipts[one.job_id]}
                    branchDeleted={branchDeletions[one.job_id]}
                    sitting={sitting(one.last_moved_at, now) ?? undefined}
                    onCopied={onCopied}
                  />
                );
              })}
            </ul>
            <Button
              variant="secondary"
              pending={sending}
              answer={answer}
              disabled={picked.length === 0}
              onClick={() => setConfirming(true)}
            >
              {sending
                ? "Cleaning up…"
                : picked.length === 0
                  ? "Clean up what you choose"
                  : `Clean up ${picked.length === 1 ? "1 row" : `${picked.length} rows`}`}
            </Button>
          </CardContent>
        </Card>
      )}

      {groups.waiting.length === 0 ? null : (
        <Card>
          <CardHeader>
            <CardTitle>Still running</CardTitle>
          </CardHeader>
          <CardContent>
            <p>
              A drone may still be writing in these, so fleet refuses to reclaim them and
              nothing here offers to. They are listed so that a worktree missing from the
              group above reads as a job still going rather than as disk already returned.
            </p>
            <ul>
              {groups.waiting.map((one) => (
                <HeldWorktree
                  key={one.job_id}
                  held={namedByHandle(one, jobs)}
                  sitting={sitting(one.last_moved_at, now) ?? undefined}
                  onCopied={onCopied}
                />
              ))}
            </ul>
          </CardContent>
        </Card>
      )}

      {groups.automatic.length === 0 ? null : (
        <Card>
          <CardHeader>
            <CardTitle>Fleet takes these on its own</CardTitle>
          </CardHeader>
          <CardContent>
            <p>
              Every safety test passed, so these come back on the next sweep without anybody
              deciding. Drawn rather than hidden: a worktree that is simply absent from this
              page cannot be told from one already given back.
            </p>
            <ul>
              {groups.automatic.map((one) => (
                <HeldWorktree
                  key={one.job_id}
                  held={one}
                  sitting={sitting(one.last_moved_at, now) ?? undefined}
                  onCopied={onCopied}
                />
              ))}
            </ul>
          </CardContent>
        </Card>
      )}

      <Dialog
        open={confirming}
        width="wide"
        title={confirmTitle(picked.length)}
        confirmLabel="Clean up"
        onCancel={() => setConfirming(false)}
        onConfirm={() => void cleanUp()}
      >
        <WhatItCosts plan={plan} now={now} />
      </Dialog>
    </div>
  );
}

/** A refusal named by the row it is about and which of its three acts it was. */
type RowFailure = { jobId: string; title: string; act: "checkout" | "branch" | "record"; outcome: Outcome };

/** What a failed act reads on its alert. */
function refusalTitle(failure: RowFailure): string {
  const acted =
    failure.act === "checkout"
      ? "the checkout was not removed"
      : failure.act === "branch"
        ? "the branch was not deleted"
        : "the record was not forgotten";
  return `${failure.title}: ${acted}`;
}

/**
 * What a refused act says. **Fleet's own sentence where it sent one** — a
 * `delete_branch` 409 arrives as `refused` with a `WireError`, and `said`'s
 * generic case for that is blank by design (`copy.ts`); every other refusal
 * here still reads through `said`.
 */
function refusedSaid(outcome: Outcome): string {
  return !outcome.ok && outcome.why === "refused" ? outcome.error.message : said(outcome);
}

/**
 * What the confirmation says, which is what is lost and never how much disk
 * comes back.
 *
 * **The destroyed files are named, one by one.** They are the only thing on
 * this screen a checkout removal ends, no branch carries them, and a count
 * would be a number a person cannot check against what they remember writing.
 */
function WhatItCosts({ plan, now }: { plan: Planned; now: number }) {
  const files = filesDestroyed(plan);
  const nothingLost =
    plan.destroying.length === 0 && plan.deletingBranches.length === 0 && plan.forgetting.length === 0;
  return (
    <>
      <p>{confirmOpening(plan)}</p>

      {nothingLost ? <p>{NOTHING_IS_LOST}</p> : null}

      {plan.destroying.length === 0 ? null : (
        <>
          <p>
            <strong>
              {files === 1 ? "One file is destroyed" : `${files} files are destroyed`}
            </strong>{" "}
            — written and committed nowhere, so the checkout is the only copy and nothing
            gets them back.
          </p>
          {plan.destroying.map((one) => (
            <div key={one.jobId}>
              {/* How long they have sat, on the confirmation as well as the
                  row: this is the last screen before they are gone, and it is
                  where a person recognises work they had forgotten. */}
              <p>{sittingSaid(one.title, one.lastMovedAt, now)}</p>
              <ul>
                {one.files.map((file) => (
                  <li key={file} className="mono">
                    {file}
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </>
      )}

      {plan.deletingBranches.length === 0 ? null : (
        <>
          <p>
            <strong>
              {plan.deletingBranches.length === 1
                ? "One branch is deleted"
                : `${plan.deletingBranches.length} branches are deleted`}
            </strong>
            , at the tip below — recoverable from that commit alone once the branch itself is
            gone.
          </p>
          <ul>
            {plan.deletingBranches.map((one) => (
              <li key={one.jobId} className="mono">
                {one.branch} · {one.commits === 1 ? "1 commit" : `${one.commits} commits`} ·{" "}
                {one.tip}
              </li>
            ))}
          </ul>
        </>
      )}

      {plan.keeping.length === 0 ? null : (
        <>
          <p>
            These branches are kept, with the commits that kept them. Nothing here deletes
            work nobody has taken — merge or delete them by hand when you are ready.
          </p>
          <ul>
            {plan.keeping.map((one) => (
              <li key={one.jobId} className="mono">
                {one.branch} · {one.commits === 1 ? "1 commit" : `${one.commits} commits`} ·{" "}
                {one.tip}
              </li>
            ))}
          </ul>
        </>
      )}

      {plan.forgetting.length === 0 ? null : (
        <>
          <p>
            <strong>
              {plan.forgetting.length === 1 ? "One record is forgotten" : `${plan.forgetting.length} records are forgotten`}
            </strong>{" "}
            — there is no undo, and a forgotten job cannot be opened again.
          </p>
          <ul>
            {plan.forgetting.map((one) => (
              <li key={one.jobId}>{one.title}</li>
            ))}
          </ul>
        </>
      )}
    </>
  );
}

/**
 * The job, and how long its checkout has been sitting.
 *
 * **The age is on the same line as the title, not a footnote.** It is the fact
 * that separates work somebody is mid-way through from work they have
 * forgotten, and it is the last chance to notice the difference.
 */
function sittingSaid(title: string, at: string, now: number): string {
  const age = sitting(at, now);
  return age === null ? title : `${title} — last moved ${age} ago`;
}

/**
 * Fleet is holding nothing that needs a person.
 *
 * **Not "no data".** An empty list here is the rule working: everything fleet
 * held could be proved safe and has been given back, which is a reading of the
 * machine and not a gap in this page.
 */
function NothingToDecide({ automatic }: { automatic: number }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Nothing is waiting on you</CardTitle>
      </CardHeader>
      <CardContent>
        <p>
          Fleet reclaims a worktree the moment it can prove nobody needs it — the job has
          ended, the base already reaches its branch, nothing in it is uncommitted, nobody is
          piloting it and nothing depends on it. None of what it is holding right now failed
          one of those tests.
        </p>
        <p>
          {automatic === 0
            ? "It is holding no disk at all."
            : "The ones below come back on the next sweep, without anybody deciding."}
        </p>
      </CardContent>
    </Card>
  );
}
