// Whether the machine panel offers to ask Fleet, and the failures where it
// must not.
//
// **A person opened a job Fleet had not answered about and the one control on
// the screen asked Fleet a question.** The panel read "Nobody has asked whether
// this job is working. Looking costs no model call." over a live `Look now`,
// because a read that failed and a read nobody had made were the same reading.
// #462.
//
// The fix turns on the failure and not on the fact of failure, and on which of
// two ways an attempt could not work. A Fleet that is not there is one; a Fleet
// that is up and answering something Bridge cannot read is the other, and it
// withdraws the act for the opposite reason — the disagreement is in the
// builds, so the same request meets it again however alive Fleet is.
//
// A refusal and a wait that ran out keep the act. Getting that wrong is just as
// bad as the original defect: a panel that withdrew its control on a timeout
// would take the next move away from somebody whose Fleet is fine.

import { describe, expect, it } from "vitest";

import type {
  Holds,
  JobExamined,
  JobProcess,
  JobResources as Held,
  Look,
  Outcome,
} from "@armada/protocol";
import { nothingToAsk, summarised, tailOf } from "./resources";
import type { LogRow } from "./story";

const JOB = "01M130Y1380016YK5S0JXBXDQ5";

function failed(outcome: Outcome): Holds {
  return { state: "failed", jobId: JOB, outcome };
}

/** A transport failure, with the route it was about, as `request.ts` builds one. */
function transport(
  why: "timed_out" | "unreachable" | "unanswerable",
  over: { status?: number } = {},
): Holds {
  const fault =
    why === "timed_out"
      ? ({ why, method: "GET", path: `/jobs/${JOB}/resources`, waitedMs: 5000 } as const)
      : why === "unanswerable"
        ? ({
            why,
            method: "GET",
            path: `/jobs/${JOB}/resources`,
            status: over.status ?? 502,
          } as const)
        : ({ why, method: "GET", path: `/jobs/${JOB}/resources` } as const);
  return failed({ ok: false, why: "transport", detail: "fetch failed", fault });
}

describe("nothing to ask", () => {
  it("says Fleet is not there where Bridge holds no connection", () => {
    expect(nothingToAsk(failed({ ok: false, why: "not_connected" }))).toBe("no_answer");
  });

  it("says Fleet is not there where the request could not be sent", () => {
    expect(nothingToAsk(transport("unreachable"))).toBe("no_answer");
  });

  // Fleet is demonstrably up and it still withdraws the act, because what
  // stops the answer is the two builds and not the daemon. A second press
  // sends the same request down the same route to the same disagreement.
  it("says the answer was unreadable where Fleet answered a status", () => {
    expect(nothingToAsk(transport("unanswerable"))).toBe("unreadable");
  });

  // The two readings have opposite fixes — start Fleet, against rebuild the
  // pair — so a caller that folded them into one flag would send somebody to
  // restart a Fleet that is already running.
  it("tells the two apart rather than reporting that something failed", () => {
    expect(nothingToAsk(transport("unreachable"))).not.toBe(
      nothingToAsk(transport("unanswerable")),
    );
  });

  // Fleet may well have carried the read out. It is there, and a second press
  // is the reasonable move rather than a dead end.
  it("keeps the act where the read timed out", () => {
    expect(nothingToAsk(transport("timed_out"))).toBeUndefined();
  });

  it("keeps the act where Fleet refused the route", () => {
    expect(
      nothingToAsk(
        failed({
          ok: false,
          why: "refused",
          error: {
            code: "job.not_found",
            message: "No job with that id.",
            run_id: "01M1RUN000000000000000000",
            fields: {},
            chain: [],
          },
        }),
      ),
    ).toBeUndefined();
  });

  // The states that are not a failure at all. `read` is the reading in hand,
  // and `keepsLastGood` means a re-read that fails leaves it there.
  it("offers the act at every state that is not a failure", () => {
    expect(nothingToAsk({ state: "none" })).toBeUndefined();
    expect(nothingToAsk({ state: "reading", jobId: JOB })).toBeUndefined();
  });
});

// # The summary under the run
//
// The reading moved to a sheet and five lines took its place, so the block and
// the reading it opens are two drawings of one answer. Every case below is a
// place they could have disagreed.

function held(over: Partial<Held> = {}): Held {
  return {
    job_id: JOB,
    read_at: "2026-09-04T09:16:52.402Z",
    held: "running",
    processes: [],
    worktree: { path: `.armada/worktrees/${JOB}`, branch: "fix/x", bytes: 1_288_490_188 },
    ...over,
  };
}

function looked(looks: Look[]): JobExamined {
  return {
    job_id: JOB,
    looked_at: "2026-09-04T09:16:52.402Z",
    found: "working",
    looks,
    resources: held(),
  };
}

const RUNNING: JobProcess = {
  pid: 41233,
  command: "node",
  cpu_percent: 8.2,
  memory_bytes: 402_653_184,
  running_for: "06:11",
  recorded: true,
};

describe("what the summary says", () => {
  // The split the whole block turns on. A reading that never arrived must not
  // render as a Job that holds nothing: one is a silent seam and the other is
  // an answer, and only the first has a next step.
  it("says nothing where no reading arrived", () => {
    expect(summarised(null, null)).toBeNull();
  });

  it("counts what is running, and zero is an answer", () => {
    expect(summarised(held({ processes: [RUNNING] }), null)?.processes).toBe(1);
    expect(summarised(held(), null)?.processes).toBe(0);
  });

  // The rule lives in `JobResources` and is imported. If this ever disagrees
  // with the sheet, the two have grown a second copy of it.
  it("calls nothing running a fault on the readings that are one", () => {
    expect(summarised(held({ held: "gone" }), null)?.nothingRunningIsWrong).toBe(true);
    expect(summarised(held({ held: "none" }), null)?.nothingRunningIsWrong).toBe(false);
  });

  // `healthy` is a finding, and a block that promoted a directory on disk to a
  // verdict would spend a person's suspicion and return nothing.
  it("says the worktree is on disk, and calls it healthy only where a look did", () => {
    expect(summarised(held(), null)?.worktree).toBe("on disk");
    expect(
      summarised(held(), looked([{ asked: "worktree", found: "working", said: "on disk" }]))
        ?.worktree,
    ).toBe("healthy");
  });

  it("says the worktree is gone, and marks it wrong", () => {
    const said = summarised(
      held(),
      looked([{ asked: "worktree", found: "not_working", said: "the worktree is not there" }]),
    );
    expect(said?.worktree).toBe("gone");
    expect(said?.worktreeIsWrong).toBe(true);
  });

  // A check that could not tell is not a failure and is not a pass. Drawing it
  // as either is what the full reading's own `cannot_tell` arm refuses.
  it("does not mark a look that could not tell as a fault", () => {
    const said = summarised(
      held(),
      looked([{ asked: "worktree", found: "cannot_tell", said: "the check would not run" }]),
    );
    expect(said?.worktree).toBe("could not be read");
    expect(said?.worktreeIsWrong).toBeUndefined();
  });

  it("resolves the unit, and never draws a byte count", () => {
    expect(summarised(held(), null)?.size).toBe("1.2 GiB");
  });

  // A walk that ran past its bound is itself worth knowing, and it is not zero.
  it("says a size was not measured rather than showing one", () => {
    expect(
      summarised(held({ worktree: { path: "p", branch: "b", bytes: undefined } }), null)?.size,
    ).toBe("not measured");
  });

  // No checkout means no size row at all. `sized(0)` is a figure, and a figure
  // claims something was measured.
  it("drops the size where there is nothing on disk", () => {
    const said = summarised(held({ worktree: undefined }), null);
    expect(said?.worktree).toBe("none on disk");
    expect(said?.size).toBeUndefined();
  });
});

describe("the tail", () => {
  const rows: LogRow[] = [
    { id: "1", at: "09:14:02", actor: "fleet", kind: "note", message: "Worktree cut", payload: [] },
    {
      id: "2",
      at: "09:14:02",
      actor: "fleet",
      kind: "note",
      message: "Preparation began",
      payload: [],
    },
    {
      id: "3",
      at: "09:16:47",
      actor: "fleet",
      kind: "note",
      message: "A preparation command failed",
      payload: [{ text: "exit  1", named: "failed" }],
    },
  ];

  // Two, and the newest first. The region this replaced bounded at 15rem and
  // still pushed the run below the fold.
  it("takes the last two, newest first", () => {
    expect(tailOf(rows).map((line) => line.said)).toEqual([
      "A preparation command failed",
      "Preparation began",
    ]);
  });

  it("names who wrote each line as a person reads it", () => {
    expect(tailOf(rows)[0].actor).toBe("Fleet");
  });

  // The level is already a rendering by the time it gets here: `notesOf` names
  // an error payload `failed`. Reading that answer is what keeps the tail's
  // hue and the log's hue the same hue.
  it("marks a line whose payload was written at error level", () => {
    expect(tailOf(rows)[0].wrong).toBe(true);
    expect(tailOf(rows)[1].wrong).toBeUndefined();
  });

  it("draws nothing where the log carries nothing", () => {
    expect(tailOf([])).toEqual([]);
  });
});
