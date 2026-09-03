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
// one nobody should reach for from a screen. So the control here is a checkbox
// per row and a confirmation that reads out what each chosen row costs; there
// is no select-all, and adding one would be adding the act this surface exists
// to replace.
//
// # The confirmation says what is lost, not how much disk comes back
//
// Bytes are not the decision. Which commits go, whether anything else has them,
// and which uncommitted files exist nowhere but the checkout is — and only the
// last of those is destroyed at all, because there is no force on this seam.
// `held.ts` computes it and is unit-tested, because every sentence in it is read
// immediately before something is destroyed.

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

import type { HeldWorktrees, Outcome, WorktreeHeld, WorktreeReclaimed } from "@armada/protocol";
import { said } from "./copy";
import { confirmOpening, confirmTitle, divided, filesDestroyed, losing, NOTHING_IS_LOST } from "./held";

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
   * Give one worktree back, and answer with what the two halves did.
   *
   * **One id at a time, and a promise rather than a callback.** There is no
   * bulk route on the wire and there should not be: each is independent, one
   * refusing does not stop the rest, and the receipt belongs to the press that
   * asked for it — publishing it as app state would make one person's gesture
   * part of what every surface re-renders on.
   */
  onReclaim: (jobId: string) => Promise<Outcome>;
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
export function Worktrees({ onWant, held, onReclaim, onCopied }: WorktreesProps) {
  useEffect(() => {
    onWant(true);
    return () => onWant(false);
  }, []);

  /** Chosen, by job id. **Not on the rows** — a bulk act needs one set. */
  const [chosen, setChosen] = useState<string[]>([]);
  /** Whether the confirmation is up. */
  const [confirming, setConfirming] = useState(false);
  /** What each reclaim answered, by job id. Kept until the list is re-read. */
  const [receipts, setReceipts] = useState<Record<string, WorktreeReclaimed>>({});
  /** Refusals, which are the half a receipt cannot carry. */
  const [refused, setRefused] = useState<{ jobId: string; outcome: Outcome }[]>([]);
  /** One act at a time, so a second press does not send the set twice. */
  const [sending, setSending] = useState(false);

  if (held.state === "failed") {
    return (
      <Alert tone="escalated" title="What fleet is holding could not be read">
        {said(held.outcome)}
      </Alert>
    );
  }
  // `none` is the frame before the effect above has run. It says the same thing
  // as `reading` rather than drawing an empty list, which here would claim
  // fleet is holding nothing — the one answer on this page nobody should be
  // given by accident.
  if (held.state !== "read") {
    return <p className="text-fg-muted">Reading what fleet is holding.</p>;
  }

  const groups = divided(held.held.worktrees);
  const picked = groups.deciding.filter((one) => chosen.includes(one.job_id));
  const cost = losing(picked);

  function choose(jobId: string, selected: boolean): void {
    setChosen((was) => (selected ? [...was, jobId] : was.filter((id) => id !== jobId)));
  }

  /**
   * Send one `reclaim_worktree` per chosen id, in turn.
   *
   * **Each is independent.** There is no bulk route, and one refusing — a
   * status that moved between the press and the call, a repository that would
   * not open — must not stop the rest. What comes back is kept per job, because
   * a person reading the answer is reading it row by row.
   */
  async function reclaim(): Promise<void> {
    setConfirming(false);
    setSending(true);
    const gaveBack: Record<string, WorktreeReclaimed> = {};
    const failed: { jobId: string; outcome: Outcome }[] = [];
    for (const one of picked) {
      const outcome = await onReclaim(one.job_id);
      if (outcome.ok && outcome.reclaimed !== undefined) gaveBack[one.job_id] = outcome.reclaimed;
      else if (!outcome.ok) failed.push({ jobId: one.job_id, outcome });
    }
    setReceipts(gaveBack);
    setRefused(failed);
    setChosen([]);
    setSending(false);
  }

  const nothingHeld = groups.deciding.length === 0 && groups.waiting.length === 0;

  return (
    <div className="flex flex-col gap-6">
      {refused.map((one) => (
        <Alert key={one.jobId} tone="escalated" title="One worktree was not given back">
          {said(one.outcome)}
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
            <ul className="flex flex-col gap-4">
              {groups.deciding.map((one) => (
                <HeldWorktree
                  key={one.job_id}
                  held={one}
                  selected={chosen.includes(one.job_id)}
                  onSelect={choose}
                  reclaimed={receipts[one.job_id]}
                  onCopied={onCopied}
                />
              ))}
            </ul>
            <Button
              variant="secondary"
              disabled={picked.length === 0 || sending}
              onClick={() => setConfirming(true)}
            >
              {picked.length === 0
                ? "Reclaim what you choose"
                : `Reclaim ${picked.length === 1 ? "1 worktree" : `${picked.length} worktrees`}`}
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
            <ul className="flex flex-col gap-4">
              {groups.waiting.map((one) => (
                <HeldWorktree key={one.job_id} held={one} onCopied={onCopied} />
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
            <ul className="flex flex-col gap-4">
              {groups.automatic.map((one) => (
                <HeldWorktree key={one.job_id} held={one} onCopied={onCopied} />
              ))}
            </ul>
          </CardContent>
        </Card>
      )}

      <Dialog
        open={confirming}
        width="wide"
        title={confirmTitle(picked.length)}
        confirmLabel="Reclaim"
        onCancel={() => setConfirming(false)}
        onConfirm={() => void reclaim()}
      >
        <WhatItCosts picked={picked} cost={cost} />
      </Dialog>
    </div>
  );
}

/**
 * What the confirmation says, which is what is lost and never how much disk
 * comes back.
 *
 * **The destroyed files are named, one by one.** They are the only thing on
 * this screen the act ends, no branch carries them, and a count would be a
 * number a person cannot check against what they remember writing.
 */
function WhatItCosts({
  picked,
  cost,
}: {
  picked: readonly WorktreeHeld[];
  cost: ReturnType<typeof losing>;
}) {
  const files = filesDestroyed(cost);
  return (
    <>
      <p>{confirmOpening(cost)}</p>

      {cost.destroying.length === 0 ? (
        <p>{NOTHING_IS_LOST}</p>
      ) : (
        <>
          <p>
            <strong>
              {files === 1 ? "One file is destroyed" : `${files} files are destroyed`}
            </strong>{" "}
            — written and committed nowhere, so the checkout is the only copy and nothing
            gets them back.
          </p>
          {cost.destroying.map((one) => (
            <div key={one.jobId}>
              <p>{one.title}</p>
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

      {cost.keeping.length === 0 ? null : (
        <>
          <p>
            These branches are kept, with the commits that kept them. Nothing here deletes
            work nobody has taken — merge or delete them by hand when you are ready.
          </p>
          <ul>
            {cost.keeping.map((one) => (
              <li key={one.jobId} className="mono">
                {one.branch} · {one.commits === 1 ? "1 commit" : `${one.commits} commits`} ·{" "}
                {one.tip}
              </li>
            ))}
          </ul>
        </>
      )}

      {picked.length === 0 ? <p>Nothing is chosen, so nothing happens.</p> : null}
    </>
  );
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
