// A Fleet with Helm pointed at a repository and a conversation already in it —
// the one moment every other scenario is missing. Every scenario before this
// published `helm: { state: "none" }`, so the dock's own controls — the thread,
// the composer in its pointed state, *Start fresh*, and the record's split
// button — could not be reached by opening the mock at all. Three agents in one
// night built the state inline to screenshot it, and the last of them threw the
// scenario away again (#1488).
//
// **The conversation is about the Job on the Board behind it**, and nothing in
// it is invented past what that fixture already says: `escalatedGateFailure()`
// is Job 77 stopped on Regression check with `cargo_nextest` failed three
// times, and Helm's reply renders that check run's own `expected` and
// `produced` rather than paraphrasing them —
// `docs/contracts/agent-copy.md`, *Render a record, never paraphrase it*.

import { PROTOCOL_VERSION } from "@armada/protocol";
import type { HelmDebugInfo, HelmThreadItem } from "@armada/protocol";
import { escalatedGateFailure } from "@armada/screens/src/fixtures/build/index";
import { MANIFEST_ID, repository } from "@armada/screens/src/fixtures/build/base";

import { connected } from "./moment";
import type { Scenario } from "./moment";

/** The Job the whole conversation is about — the same one the Board behind the dock draws. */
const JOB = escalatedGateFailure();

/**
 * What Helm answered, whole. **Every fact in it is the fixture's**: the step
 * and its ordinal, the three attempts, the check's `expected` and `produced`
 * strings verbatim, and the three recourses `stuck` names. No "I" — reading a
 * record is not something Helm did.
 */
const ANSWER = [
  "Job 77 stopped on step 4 of 6, Regression check. cargo_nextest failed on all three attempts.",
  "Expected — cargo nextest run --workspace exits 0",
  "Produced — exit 101 — 3 of 2034 tests failed",
  "The worktree is still on disk. Override the verdict, redirect the drone, or redispatch.",
].join("\n\n");

/**
 * Why the second ask never came back. **Fleet's own sentence**, the one
 * `helm::hosting` writes when the agent's door cannot be published — and the
 * sharpest thing a person can press *Copy debug info* about, since `door` is a
 * field of the record itself.
 *
 * **A nearer failure was tried and is not this.** The agent CLI failing to
 * launch names the CLI, which the vendor-literal rule refuses outside
 * `crates/adapters` — a fixture is not an adapter. The reply-budget timeout
 * would now draw as well as this one does: neither surface frames a `why` any
 * more, which is what `crates/fleet/src/helm/unanswered.rs` settled.
 */
const WENT_QUIET = "Helm's door would not be configured: Permission denied (os error 13)";

/**
 * The conversation, as `helmArrived` would have folded it off the socket: an
 * ask, the reply and its cost, a second ask, and no answer to it. **The failed
 * answer is the point** — the record's *Copy debug info* and *Details* exist
 * for a bad answer somebody wants to carry to whoever could fix it (#1367), and
 * a conversation where nothing went wrong never shows them in their own moment.
 *
 * Ids are the sequence numbers a live socket would have counted, one per item.
 */
const CONVERSATION: HelmThreadItem[] = [
  { kind: "asked", id: "1", ts: "2026-09-10T14:31:06.000Z", text: "why did 77 stop" },
  {
    kind: "row",
    id: "2",
    turn: {
      ts: "2026-09-10T14:31:19.000Z",
      seq: 2,
      // `by` is read for nothing in a Helm thread — `helm-thread.ts` draws
      // every reply row as Helm — and `drone` is what the fold defaults an
      // unstamped row to, so that is what a row off the wire carries here.
      by: "drone",
      saw: { event: "said", text: ANSWER },
    },
  },
  {
    kind: "row",
    id: "3",
    turn: {
      ts: "2026-09-10T14:31:21.000Z",
      seq: 3,
      by: "drone",
      saw: { event: "ended", turns: 4, cost_micros: 21_400, refusals: 0 },
    },
  },
  {
    kind: "asked",
    id: "4",
    ts: "2026-09-10T14:33:48.000Z",
    text: "which test is it, and does it fail on main too",
  },
  { kind: "unanswered", id: "5", ts: "2026-09-10T14:33:50.000Z", why: WENT_QUIET },
];

/**
 * The brief, as this session was given it. **Fleet's own opening and its
 * repository block**, from `crates/fleet/src/helm/brief.rs`; the whole brief
 * runs to nine blocks and the rest of them say nothing this scenario is for.
 */
const BRIEF = [
  "You are Helm, in Armada. A person asks you about the work in one repository, and about the " +
    "repository itself. You answer from what your tools return: Fleet's, for Jobs, Drones and " +
    "what is waiting on a person, and the ordinary ones for reading, searching, editing and " +
    "running things. Saying that you did something does not do it. Only a tool call does.",
  `THIS REPOSITORY\n\nEvery question in this conversation is about Manifest ${MANIFEST_ID}, read ` +
    "from armada. The Fleet tools answer inside it and reach nothing outside it, so when you are " +
    "asked about another repository, say it cannot be answered here.",
].join("\n\n");

/**
 * The record `GET /helm/debug` answers for this session — the same conversation
 * from Fleet's side, so what the split button copies and what the thread draws
 * are about one session rather than two unrelated fixtures.
 */
const RECORD: HelmDebugInfo = {
  manifest_id: MANIFEST_ID,
  checkout: "/Users/user/armada",
  authority: "acting",
  model: "sonnet",
  brief: BRIEF,
  door: "armada-fleet",
  tools: ["list_jobs", "get_job", "get_check_output", "get_diff", "get_events_since", "ask_person_to_approve"],
  servers: 3,
  session: "01M2C1TJ8G00HELMSESSION01",
  thread: [
    { at: "2026-09-10T14:31:06.000Z", line: "asked", text: { text: "why did 77 stop" } },
    { at: "2026-09-10T14:31:11.000Z", line: "called", tool: "get_job", detail: JOB.job.id },
    { at: "2026-09-10T14:31:19.000Z", line: "said", text: { text: ANSWER } },
    { at: "2026-09-10T14:31:21.000Z", line: "ended", turns: 4, cost_micros: 21_400, refusals: 0 },
    {
      at: "2026-09-10T14:33:48.000Z",
      line: "asked",
      text: { text: "which test is it, and does it fail on main too" },
    },
    { at: "2026-09-10T14:33:50.000Z", line: "unanswered", why: WENT_QUIET },
  ],
  cut: 0,
  polled: {
    from: 812,
    upto: 851,
    kinds: [
      { kind: "job.checking", count: 3 },
      { kind: "job.state_changed", count: 1 },
    ],
  },
  run_id: "01M2C1TJ8G00FLEETRUN0001",
  protocol_version: PROTOCOL_VERSION,
  at: "2026-09-10T14:34:11.402Z",
};

/**
 * Helm pointed at `armada`, mid-conversation about the Job on the Board.
 *
 * **One repository set up, so the composer draws no switch**: pointed at a
 * repository, one is nothing to switch between, and a second served here would
 * only be there to make a control appear. `fleet-not-running` through
 * `every-state` are where the unpointed dock and its switch are read.
 *
 * `behaves` answers `GET /helm/debug` alone. **Asking still moves nothing** —
 * a reply is Fleet's to decide and the fake invents none — so Send clears the
 * field and the thread stands, which is `running-locally.md`'s rule for every
 * other act a Fleet would have to answer.
 */
export function talking(): Scenario {
  return {
    name: "helm-talking",
    says: "Helm pointed at a repository, with a reply given and one that never came",
    state: {
      ...connected([JOB.job], JOB.workflows, [repository()]),
      helm: { state: "open", manifestId: MANIFEST_ID, replying: false, skipped: 0, missed: 0, items: CONVERSATION },
    },
    reads: { [JOB.job.id]: JOB },
    behaves: () => ({ helmDebugInfo: async () => ({ ok: true, record: RECORD }) }),
  };
}
