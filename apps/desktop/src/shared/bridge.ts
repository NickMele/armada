// What the three processes agree on: the state main publishes, the operations
// the renderer may initiate, and the channel names those two travel over.
//
// Types, the channel constants, and the empty state both ends start from.
// Nothing here runs in more than one process — the preload is a wire and not an
// import path, and this file is the shape of what crosses it.

import type {
  BridgeIdentity,
  CallRead,
  ClearOutcome,
  Connection,
  Diff,
  Draft,
  Evidence,
  Footprint,
  HeldWorktrees,
  History,
  Holdings,
  Journalled,
  Observed,
  Outcome,
  Proposed,
  ProtocolVersion,
  Reports,
  Skew,
  Watched,
  connectedTo,
} from "@armada/protocol";
import type { FileReport, FleetCapacity, JobDetail, JobFilesChanged, JobSummary, ProposalInFlight, Report, ReportList, UnreadableJob, WireError } from "@armada/protocol";
import type { ManifestSummary, ModelChoices, WorkflowSummary } from "@armada/protocol";
import type { Artifact, Opened } from "@armada/protocol";
import type { Recorded } from "@armada/protocol";
import type { Submitted, Work } from "@armada/protocol";
import type { CallArguments, Saw, Voice } from "@armada/protocol";
import { connects, skew, spoken } from "@armada/protocol";
import { PROTOCOL_VERSION } from "@armada/protocol";




/**
 * The state with its identity current, which today means Fleet's version.
 *
 * **Here rather than at the five places a failure is built**, for the reason
 * `connectedTo` is here: a fact derived from the connection is derived once, by
 * the thing that owns the connection, so no surface can publish a state whose
 * identity disagrees with it. Four of the five failure builders are handed no
 * connection at all, and a refusal — the one Fleet itself answered — was the
 * payload most obviously wrong to omit Fleet's version from.
 *
 * The identity is rewritten only when the version moves, so a state whose
 * connection did not change keeps the same object and nothing redraws for it.
 *
 * `null` for the three connection states that never read a runtime file. Absent
 * rather than guessed: a version written in for a Fleet Bridge never identified
 * would be the one row of that payload nobody could check.
 */
export function identifying(state: BridgeState): BridgeState {
  const fleetProtocol =
    "fleet" in state.connection ? spoken(state.connection.fleet.protocolVersion) : null;
  if (fleetProtocol === state.bridge.fleetProtocol) return state;
  return { ...state, bridge: { ...state.bridge, fleetProtocol } };
}

/**
 * Where a pressed notification lands.
 *
 * **A job, or the set.** A telling about one job opens that job. A telling
 * about several cannot open one of them without choosing for somebody, so it
 * lands on the Needs-you tab — which is the set the notification was derived
 * from, and the same one keyboard `2` reaches.
 *
 * It travels main → renderer only. Nothing the renderer sends can produce one.
 */
export type Summons = { jobId: string | null };

/** Everything the renderer draws, published by main and never assembled twice. */
export type BridgeState = {
  connection: Connection;
  /** Where a Bridge failure points. Never a Job's identity. */
  bridge: BridgeIdentity;
  jobs: JobSummary[];
  /** Rows the store refused. Shown, never merged into `jobs` as a placeholder. */
  unreadable: UnreadableJob[];
  /**
   * How full the fleet is, and what holds the next Drone back.
   *
   * **`null` until Fleet has answered once**, which is a Bridge that has not
   * asked rather than a Fleet with room — the two are drawn differently and a
   * zeroed placeholder would make them one. Re-read whenever a Job moves,
   * because a Job moving is the only thing that changes the occupancy, and the
   * machine reading rides along on the same call.
   */
  capacity: FleetCapacity | null;
  /** Events Fleet dropped before Bridge saw them, since the window opened. */
  missed: number;
  /** When the Jobs above were last current, in epoch milliseconds. */
  readAt: number | null;
  /** Jobs with an approval in flight. What stops a second dispatch. */
  approving: string[];
  /**
   * The proposal this window is waiting on, or `null` where it is waiting on
   * none.
   *
   * **The one piece of state here that is not about a Job**, because a proposal
   * is the interval before any Job exists. It appears when Fleet says the call
   * went out, moves as the call gets somewhere, and is `null` again the moment
   * the call comes back — however it came back.
   *
   * **This window's own, matched on the token it sent.** Fleet publishes every
   * proposal on one stream and two windows may be dispatching at once; a state
   * that folded whichever arrived last would draw somebody else's call as
   * yours, and offer a stop that killed it.
   */
  proposing: ProposalInFlight | null;
  /**
   * What Fleet holds, and therefore what a proposal may name.
   *
   * Read over the one connection like everything else here. The composer used
   * to offer a text field for a pasted id, because nothing served these — and a
   * pasted id was accepted by Fleet unchecked. Both halves of that are fixed:
   * Fleet refuses an id it does not hold, and this is what the form offers so
   * nobody has to guess at one.
   */
  holds: Holdings;
  /**
   * The one Job read whole, where a detail is open.
   *
   * **Published, not fetched by the component that draws it.** The detail is
   * re-read whenever an event names its Job, which is what makes the rail move
   * without a reload — a renderer holding its own copy would go stale the
   * moment a step advanced.
   */
  watched: Watched;
  /**
   * The Job being watched turn by turn, where somebody opened one.
   *
   * **Its own socket, and its own piece of state.** Transcript rows arrive at
   * Drone speed, and putting them on the stream the Board is drawn from would
   * evict the state changes that draw it. Separate here for the same reason.
   */
  observed: Observed;
  /**
   * What Fleet has done to the Job being watched — its own log, live.
   *
   * **Its own piece of state beside `observed`, and its own socket.** The two
   * answer different questions and neither substitutes for the other: a Drone's
   * transcript exists only while a Drone does, and this is the only thing there
   * is to draw while a worktree is being cut. A Job with all its steps
   * `not_started` has an empty `observed` and a full `journalled`, which is
   * exactly the moment somebody opens the panel.
   */
  journalled: Journalled;
  /**
   * What the open Job's Drone has changed in its worktree.
   *
   * **Only the open Job's, and only while the event arrives.**
   * `job.files_changed` is published for every Job on the one stream; keeping
   * every Job's footprint would make the Board pay for a detail nobody has
   * open, which is the thing this read is meant to stay off.
   */
  footprint: Footprint;
  /**
   * One Job's transition history, where a surface asked for one.
   *
   * **Read when it is asked for, not on every open.** It is its own operation
   * for that reason: a detail is fetched to draw a summary and a history has no
   * bound — it grows for as long as the Job lives, and a retried step is a row
   * per attempt plus the moves around it.
   */
  history: History;
  /**
   * What one Job's Drones claimed, where a surface asked for it. The cheap half
   * of the pair, and still asked for rather than paid for on every open.
   */
  evidence: Evidence;
  /**
   * One Job's worktree against the branch it was cut from, where a surface asked
   * for it. **The expensive half, and the one place the patch bytes are spent.**
   * `crates/adapter-traits/src/work_product.rs` splits it off the file list
   * because the bytes are large and most steps ask no semantic question; this is
   * read on the act they were split for, never folded into `watched`, which is
   * re-read every time an event names the open Job.
   */
  diff: Diff;
  /**
   * Every report filed, and the calibration counts, where a surface asked.
   *
   * **Not per Job, and that is the point of it.** A report outlives the Job it
   * is about — `armada clean` forgets the Job and the report stays whole — so
   * this is the one read here that no Job id scopes, and the one that would be
   * lost if it were reachable only through a Job.
   *
   * Read when the surface that draws it opens, like the folded reads under a
   * Job and for the same reason: the bodies travel with the list, so nothing
   * pays for them until somebody is reading them.
   */
  reports: Reports;
  /**
   * What Fleet is holding disk for, where a surface asked.
   *
   * **The second read here no Job scopes**, and not for the reports' reason: a
   * report outlives the Job it names, while this is a question that only makes
   * sense of the set — which of these to give back. A field on a Job could
   * carry the reasons and could not carry the choice.
   *
   * Read when the surface opens and dropped when it closes, like the reports.
   */
  held: HeldWorktrees;
};



/** The whole preload surface, and therefore everything the renderer can reach. */
export type BridgeApi = {
  protocolVersion: () => ProtocolVersion;
  state: () => Promise<BridgeState>;
  subscribe: (onState: (state: BridgeState) => void) => () => void;
  proposeJob: (draft: Draft) => Promise<Outcome>;
  /**
   * Describe the work and let the Job proposer decide what it is: which
   * workflow, what to call it, and whether it is one Job or several.
   *
   * **The same gate as `proposeJob`, and this adds none.** Every Job comes back
   * at `awaiting_approval` and each takes its own approval in turn — approving
   * one of several accepts a plan and starts nothing else.
   *
   * `proposeJob` stays the override, not a fallback: a person who knows which
   * workflow they want names it themselves and no model is asked.
   *
   * The two refusals are separate arms of `Proposed` because a person does
   * different things about them — a request nothing fits is said again
   * differently or hand-entered, and a call that could not be made is simply
   * asked again.
   */
  proposeFromRequest: (request: string) => Promise<Proposed>;
  /**
   * Stop the proposal this window is waiting on.
   *
   * **It kills the call rather than stopping the wait.** A window that merely
   * dropped the request would leave the proposer running inside Fleet, spending
   * against the budget, with nobody left to read what it decided.
   *
   * Takes no id: `BridgeState.proposing` is what this window is waiting on, and
   * an id from the renderer would let one window stop another's call. Answers
   * whether there was still one to stop — pressing this a beat after the Jobs
   * landed is being late rather than failing, and the surface says so.
   */
  stopProposal: () => Promise<Outcome>;
  /**
   * Write pasted or picked bytes to a staging file before a Job exists —
   * there is no Job id yet to key storage on; one is minted at `propose`
   * time. Returns the absolute path written, which the caller carries on
   * `Draft.attachments` until `proposeJob` sends it as a `staged_path`.
   */
  stageAttachment: (
    bytes: ArrayBuffer,
    filename: string,
    mimeType: string,
  ) => Promise<{ path: string }>;
  approveDispatch: (jobId: string) => Promise<Outcome>;
  /**
   * Kill the failed Job and mint its replacement. **Nothing resumes** — the
   * Job it is called on ends at `killed` and a new one is created carrying
   * `redispatched_from`, whose id comes back on the outcome.
   */
  redispatchJob: (jobId: string) => Promise<Outcome>;
  /** Kill the process. The Job survives, with its worktree held. */
  killDrone: (jobId: string) => Promise<Outcome>;
  /** End the Job at `killed`. Terminal, and nothing resumes it. */
  killJob: (jobId: string) => Promise<Outcome>;
  /**
   * Delete every terminal Job's whole record, one `forget_job` per id. **Real
   * deletion, not a status** — there is no undo, and a cleared Job cannot be
   * opened again. The caller decides which ids are terminal; this sends
   * exactly the ids it is given and refuses none of them itself.
   */
  clearTerminalJobs: (jobIds: readonly string[]) => Promise<ClearOutcome>;
  /**
   * Give one terminal Job's worktree and branch back, **without waiting for
   * Fleet to stop**. `armada clean` is the same act from the CLI and refuses
   * while the daemon is running, which is exactly when a person wants the disk.
   *
   * **The record survives.** This takes the directory and the branch;
   * `clearTerminalJobs` takes the row. Sending both is ordinary and the order
   * does not matter.
   *
   * **A branch nothing has merged is kept**, always — there is no force here —
   * so the outcome's `reclaimed` says which half happened rather than reducing
   * both to one flag.
   */
  reclaimWorktree: (jobId: string) => Promise<Outcome>;
  /**
   * Inject an instruction into the Drone that is there. **Legal only on an
   * escalated Job that still holds one** — Fleet refuses 409 where the Drone
   * is gone, naming `restartStep` as the act that applies. Nothing is
   * spawned; the Job comes back `running` with the same session.
   */
  redirectDrone: (jobId: string, instruction: string) => Promise<Outcome>;
  /**
   * Answer the question the job's drone asked, by picking one of the labels it
   * offered.
   *
   * **The answer is a choice, never prose.** There is no free-text parameter
   * and fleet refuses a label it did not offer — a person who needs to say
   * something the options do not cover uses `redirectDrone`, which is the one
   * route their own words reach a drone by.
   *
   * The job comes back unchanged: it was `running` while it waited and is
   * `running` now. Fleet refuses 409 where nothing is waiting, where the id
   * names a question already answered, and where the label was not offered.
   */
  answerQuestion: (
    jobId: string,
    questionId: string,
    chose: string,
  ) => Promise<Outcome>;
  /**
   * Put a fresh Drone on the surviving worktree, at the step that stopped, and
   * say what to do differently where there is something to say.
   * **Legal only where the Drone is gone** — Fleet refuses 409 where one is
   * still alive, or where the worktree itself is gone.
   *
   * **The note is optional and the plain restart sends no body at all.** It
   * does not reach a session — there is none — it waits on the job and opens
   * the brief of the drone this asks for, which is where `requestChanges`'s
   * note goes. A blank one is not sent: `undefined` and `""` are both a
   * restart with nothing said, because a drone handed an empty instruction
   * starts over with exactly what was not enough.
   *
   * Fleet answers 409 where a note is already waiting on the job, quoting it
   * back rather than overwriting it.
   */
  restartStep: (jobId: string, note?: string) => Promise<Outcome>;
  /**
   * Overrule a machine that stopped the work, and let the Job go on.
   *
   * **Not an approval, and its own entry for that reason.** `approveReview`
   * answers a gate nothing objected to; this answers one that stopped the step,
   * and it says a machine was wrong rather than that the work was right. The
   * step advances still carrying `failed`, and the reason is written to the
   * Job's log where it stays.
   *
   * Legal on an escalated Job whose step stopped on `gate_failure` — the Judge
   * refusing a criterion — or on `evidence_suspect`, the gaming check calling
   * the evidence untrustworthy. Both are a machine's decision, which is what a
   * person may overrule. Fleet refuses 409 for `gate_undecided`, where nothing
   * weighed the work, and for a step that stopped on anything else; 422 for a
   * blank reason. Whether the Drone is still there decides only how the Job
   * carries on.
   */
  overrideVerdict: (jobId: string, reason: string) => Promise<Outcome>;
  /**
   * Read one Job whole and keep it current, or `null` to stop.
   *
   * The renderer says which Job is open; main does the reading and republishes
   * it whenever an event names that Job. One call per open, not one per event.
   */
  /**
   * Ask the gate again, on the evidence the step already submitted.
   *
   * **Not an override and not a widening of one.** `overrideVerdict` lifts a
   * decision a machine made; `gate_undecided` is a gate that made none — it
   * could not derive what it needed to read — so there is nothing to disagree
   * with and nothing to lift. This asks the question that failed to be asked,
   * which is why it carries no reason: nothing is being disputed, so there is
   * no sentence to record that the second reading will not say for itself.
   *
   * Legal on an escalated Job whose stopped step carries `gate_undecided`.
   * Fleet refuses 409 on any other trigger — that one is an override or
   * nothing — and on a Job it is no longer standing at, because the baseline
   * the gate reads against lives in that Job's slot and a Fleet restarted since
   * the escalation has none. Fleet stands at several Jobs at once, so the
   * refusal is about this Job's slot being gone rather than about the one slot
   * holding somebody else. Where the cause has not gone away the gate
   * is undecided again and **nothing moves**, which is not a failure.
   */
  rerunGate: (jobId: string) => Promise<Outcome>;
  /**
   * Say that this Job failed in error, in your own words, and file the Job's
   * own record with it.
   *
   * **Not an act on the Job.** Nothing moves, nothing is spawned and nothing is
   * dispatched: a report is a record of what a person concluded, and an entry
   * that also moved the Job would make disagreeing with a verdict a way of
   * getting past one — which is `overrideVerdict`, a different act with a
   * different refusal.
   *
   * The sentence is required and blank is refused before the request is sent,
   * matching the 422 Fleet would give it. What comes back on the outcome is the
   * report, because the rendered record is what a person does the next thing
   * with — Armada does not file it anywhere, and says so.
   */
  fileReport: (jobId: string, filing: FileReport) => Promise<Outcome>;
  watchJob: (jobId: string | null) => Promise<void>;
  /**
   * Watch one Job's turns, or `null` to stop.
   *
   * **Nothing is sent to the Drone and nothing can be.** This opens a socket
   * that only reads, closes it when the window closes it, and leaves the Job
   * exactly as it found it — a capability that could intervene would be Pilot,
   * which is a different act with a transition on the record.
   */
  observeJob: (jobId: string | null) => Promise<void>;
  /**
   * Read one Job's transition history, or `null` to stop.
   *
   * **Its own entry because it is its own operation.** A history is not on
   * `JobDetail`, so folding it into `watchJob` would make every Job opened pay
   * for a surface that is folded away by default. Read-only, like the two above
   * it: a recorded move is a fact, and nothing here can add one.
   */
  readHistory: (jobId: string | null) => Promise<void>;
  /**
   * Read what one Job's Drones claimed, or `null` to stop. Read-only, and its
   * own entry rather than folded into `readDiff`: they are two operations on
   * the Rust side because a surface wanting only the claims would otherwise
   * fetch a megabyte to read four lines.
   */
  readEvidence: (jobId: string | null) => Promise<void>;
  /**
   * Read one Job's worktree against its branch, or `null` to stop. Read-only.
   * **The one capability here that spends the patch bytes**, and deliberately
   * not reachable by opening a Job — the renderer calls it from the surface
   * that draws a diff, which is the act the bytes were separated for.
   */
  readDiff: (jobId: string | null) => Promise<void>;
  /**
   * Read one recorded tool call's arguments — the whole of what the socket cut.
   *
   * **The one read here that answers rather than publishes**, and the one that
   * names something smaller than a Job. Every other read is held open and kept
   * current because the thing it draws moves; a recorded argument is finished,
   * and a person opening one row is asking about that row. So it takes a call
   * id, answers once, and nothing is left held.
   *
   * Read-only, like the reads above it. Nothing on it reaches a Drone, and the
   * call id is one Fleet already put on a row this window was streamed.
   */
  readCall: (jobId: string, callId: string) => Promise<CallRead>;
  /**
   * Read every filed report and the counts beside them, or `false` to drop it.
   *
   * **Read-only, and the only read here that names no Job.** A report is about
   * a Job but does not belong to one — it survives the Job being forgotten — so
   * a listing reached through a Job would lose exactly the reports that most
   * need reading. Nothing about this files, edits or withdraws one; filing is
   * `fileReport`, on the Job it is about.
   *
   * A boolean rather than an id for that reason: there is nothing to scope it
   * to, only whether somebody is looking.
   */
  readReports: (want: boolean) => Promise<void>;
  /**
   * Read what Fleet is holding disk for, or drop it.
   *
   * **Read-only, and the reasons are the payload.** What comes back is every
   * worktree Fleet is holding and the test each one failed, so a person can
   * decide about them one at a time — Fleet has already given back everything
   * that passed all five, without being asked.
   *
   * **A piloted Job's checkout is not in it.** Fleet drops it before answering,
   * so there is nothing here to hide and nothing that could be drawn by
   * mistake: a person is at an unrestricted toolset in that directory.
   *
   * A boolean rather than an id, for `readReports`'s reason: there is nothing
   * to scope it to, only whether somebody is looking.
   */
  readHeld: (want: boolean) => Promise<void>;
  /**
   * Take the work. **The counterpart to `approveDispatch`, at the other end of
   * the Job.** On the workflow's last step Fleet commits and delivers before
   * recording the Job done. Legal only at `awaiting_review`, like the two below.
   */
  approveReview: (jobId: string) => Promise<Outcome>;
  /**
   * Send the work back with a note. **The Job comes back `running`**, same step,
   * same Drone — nothing is spawned and nothing done is thrown away.
   */
  requestChanges: (jobId: string, note: string) => Promise<Outcome>;
  /**
   * A verdict on the work, and the Job is over. **Terminal, and it ends the
   * Drone** — that is what separates it from `requestChanges`, and it is not
   * `killJob`, which clears the Board and carries no verdict at all. Three
   * entries and not one taking which: that would read as one act and perform
   * three, and the three differ by whether anything survives.
   */
  rejectWork: (jobId: string) => Promise<Outcome>;
  /**
   * Open one of a Job's artifacts in whatever the OS opens it with.
   *
   * **One entry taking which, where the kills and the decisions are three.**
   * Those are split because they differ in what survives them; these three
   * differ only in which file, they change nothing about the Job, and a Job id
   * plus a word from a closed set is deliberately less than a path — main
   * derives the path, so the renderer never holds the argument that matters.
   *
   * The branch is absent from `Artifact` and stays a copy: it is served rather
   * than derived, and it is not a path.
   */
  openArtifact: (jobId: string, what: Artifact) => Promise<Opened>;
  /**
   * Where a pressed notification says to go.
   *
   * **The one entry here the renderer cannot initiate.** Every other capability
   * is the window asking Fleet for something; this is main handing over a press
   * that happened outside the window — possibly while there was no window — and
   * the renderer's only part is to go there.
   *
   * It grants nothing: what arrives is a job id this window already draws, or
   * `null`. Subscribing twice is two callbacks, and the returned function is
   * how one stops.
   */
  onSummoned: (onGo: (to: Summons) => void) => () => void;
};

/**
 * What Bridge holds before anything has answered.
 *
 * **One statement, not two.** Main and the renderer each used to declare their
 * own, and the two drifted the first time a field was added — a renderer
 * missing a key main publishes reads as a field that is always absent.
 */
export const NOTHING_YET: BridgeState = {
  connection: { state: "reading" },
  // Main resolves the log path from the home it can see. Until it answers, the
  // renderer does not know it and does not name one.
  bridge: { auditPath: null, fleetProtocol: null },
  jobs: [],
  unreadable: [],
  capacity: null,
  missed: 0,
  readAt: null,
  approving: [],
  proposing: null,
  holds: { workflows: [], manifests: [], models: null },
  watched: { state: "none" },
  observed: { state: "none" },
  journalled: { state: "none" },
  footprint: { state: "none" },
  history: { state: "none" },
  evidence: { state: "none" },
  diff: { state: "none" },
  reports: { state: "none" },
  held: { state: "none" },
};

/** The channels the preload is allowed to name. There is no general `invoke`. */
export const CHANNELS = {
  state: "bridge:state",
  changed: "bridge:changed",
  proposeJob: "bridge:propose-job",
  proposeFromRequest: "bridge:propose-from-request",
  stopProposal: "bridge:stop-proposal",
  stageAttachment: "bridge:stage-attachment",
  approveDispatch: "bridge:approve-dispatch",
  redispatchJob: "bridge:redispatch-job",
  killDrone: "bridge:kill-drone",
  killJob: "bridge:kill-job",
  clearTerminalJobs: "bridge:clear-terminal-jobs",
  reclaimWorktree: "bridge:reclaim-worktree",
  redirectDrone: "bridge:redirect-drone",
  answerQuestion: "bridge:answer-question",
  restartStep: "bridge:restart-step",
  overrideVerdict: "bridge:override-verdict",
  rerunGate: "bridge:rerun-gate",
  fileReport: "bridge:file-report",
  watchJob: "bridge:watch-job",
  observeJob: "bridge:observe-job",
  readHistory: "bridge:read-history",
  readEvidence: "bridge:read-evidence",
  readDiff: "bridge:read-diff",
  readCall: "bridge:read-call",
  readReports: "bridge:read-reports",
  readHeld: "bridge:read-held",
  approveReview: "bridge:approve-review",
  requestChanges: "bridge:request-changes",
  rejectWork: "bridge:reject-work",
  openArtifact: "bridge:open-artifact",
  summoned: "bridge:summoned",
} as const;
