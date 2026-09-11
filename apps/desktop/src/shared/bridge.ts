// What the three processes agree on: the state main publishes, the operations
// the renderer may initiate, and the channel names those two travel over.
//
// Types, the channel constants, and the empty state both ends start from.
// Nothing here runs in more than one process — the preload is a wire and not an
// import path, and this file is the shape of what crosses it.

import type {
  BridgeIdentity,
  CallRead,
  CheckOutputRead,
  FrameRead,
  ClearOutcome,
  Connection,
  Diff,
  Draft,
  Evidence,
  Examination,
  Footprint,
  Holds,
  HeldWorktrees,
  History,
  Holdings,
  FollowedLog,
  Journalled,
  Observed,
  Outcome,
  Proposed,
  ProtocolVersion,
  ReclaimOutcome,
  Remarks,
  Reports,
  Watched,
} from "@armada/protocol";
import type { FileReport, FleetCapacity, JobSummary, ProposalInFlight, UnreadableJob } from "@armada/protocol";
import type { ManifestReading } from "@armada/protocol";
import type { Artifact, Followed, Opened } from "@armada/protocol";
import { spoken } from "@armada/protocol";




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
  /**
   * What Fleet's last read of `armada.yml` came to, or `null` because there
   * has not been one.
   *
   * **The second piece of state here that is not about a Job**, beside
   * `proposing` and for a related reason: a Manifest is Fleet's own, so there
   * is no row it belongs to. Unlike `proposing` it is not this window's — every
   * window watching this Fleet holds the same reading, because every one of
   * them is running against the same configuration.
   *
   * **Read on connect as well as folded from `manifest.reread`.** A refusal is
   * a standing condition, not an instant: the file and the values in force go
   * on disagreeing until somebody fixes the file, so a window opened a minute
   * after the save has to be able to find out. An event alone would tell only
   * whoever happened to be looking.
   */
  manifestReading: ManifestReading | null;
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
   * The running Check's log somebody opened, as it is written. **Its own
   * socket**, for `journalled`'s reason: a Check's output is a file still
   * growing, and the event stream is bounded to keep payloads that size off
   * it. `following.ts`.
   */
  followed: FollowedLog;
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
   * What people wrote on one Job's open pull request, where the surface a
   * person decides on asked for it. **The one read in this state that costs a
   * forge** — nothing takes it on a timer and no event refreshes it, so it is
   * opened by that surface and dropped when it closes.
   */
  remarks: Remarks;
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
   * What the open Job holds on this machine — its processes, what each is
   * burning, and the disk its worktree has taken.
   *
   * **Opened with the Job and re-read while it is open**, which is what makes
   * it a live panel rather than a snapshot: a figure that stopped moving while
   * a Job ran would be a panel claiming a stall that is not there. It keeps its
   * last good reading through a failed re-read for `watched`'s reason — a
   * blanked panel reads as a Job holding nothing, which is the exact answer
   * this exists to make loud.
   */
  resources: Holds;
  /**
   * What Fleet found when somebody pressed for a look — and only then.
   *
   * **Not read on opening a Job.** It is a thing a person did, it costs a
   * process table and a directory walk, and an answer that appeared without
   * anybody asking would be the automatic bound rather than the person's half
   * of it. It stays on screen until they press again or leave the Job.
   */
  examination: Examination;
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
   * Reclaim every terminal Job's worktree and branch at once, one
   * `reclaim_worktree` per id. **Every row survives** — this takes the
   * directory and the branch, and `forgetTerminalJobs` below is the act that
   * takes the row. The caller decides which ids are terminal; this sends
   * exactly the ids it is given and refuses none of them itself.
   *
   * **A branch nothing has merged is kept**, always — there is no force here —
   * so a failed id in the outcome may still have given its disk back; read
   * `reclaimed.branch` on the per-Job act for which half happened.
   */
  clearTerminalJobs: (jobIds: readonly string[]) => Promise<ReclaimOutcome>;
  /**
   * Delete every terminal Job's whole record at once, one `forget_job` per
   * id. **Real deletion, not a status** — there is no undo, and a Job this
   * reaches cannot be opened again. The caller decides which ids are
   * terminal; this sends exactly the ids it is given and refuses none of them
   * itself.
   */
  forgetTerminalJobs: (jobIds: readonly string[]) => Promise<ClearOutcome>;
  /**
   * Give one terminal Job's worktree and branch back, **without waiting for
   * Fleet to stop**. `armada clean` is the same act from the CLI and refuses
   * while the daemon is running, which is exactly when a person wants the disk.
   *
   * **The record survives.** This takes the directory and the branch;
   * `forgetJob` takes the row. Sending both is ordinary and the order does
   * not matter.
   *
   * **A branch nothing has merged is kept**, always — there is no force here —
   * so the outcome's `reclaimed` says which half happened rather than reducing
   * both to one flag.
   */
  reclaimWorktree: (jobId: string) => Promise<Outcome>;
  /**
   * Delete one terminal Job's whole record. **Real deletion, not a status** —
   * there is no undo, and the Job cannot be opened again. The per-Job half of
   * `forgetTerminalJobs`.
   */
  forgetJob: (jobId: string) => Promise<Outcome>;
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
   * Ask a Job to show its work: Fleet reruns the last spec a Drone named, in
   * the Job's worktree, and keeps what it captured as a set of its own beside
   * the step's frames. **It moves nothing on the Job.**
   *
   * **It answers when the press has landed**, which may be as long as the app
   * takes to start. Fleet runs it off the turn loop and bounds it by its own
   * Check budget, so Bridge does not add a wait of its own — see `NO_WAIT`.
   * Fleet refuses 409 before anything runs where it cannot, naming what is
   * missing; a press that ran and captured nothing answers with why.
   */
  showAgain: (jobId: string) => Promise<Outcome>;
  /**
   * Give one job a higher cost ceiling than the tier above it allows.
   *
   * **The act the `over_budget` label has always pointed at.** A job past its
   * cost cap waits at `queued` reading that label until somebody raises the cap
   * — the one reason a queued job carries that does not clear on its own — and
   * the only remedy before this was a machine-wide setting that governs every
   * job and takes on a restart.
   *
   * **It moves nothing.** No status, no step, no drone: admission was going to
   * start one and was refused for money, so the next turn starts it. What comes
   * back is the job, and the field worth reading is `queued_reason` —
   * `over_budget` before, absent or `waiting_on_resources` after.
   *
   * The figure is millionths of a dollar, which is the unit `JobSpend` reads
   * in. **It raises only**: a value at or under the cap in force is refused
   * before the request is sent, matching the 422 Fleet would answer, because a
   * press that reports success and leaves the job stopped is the failure this
   * exists against. Fleet refuses 409 on a terminal job, which has nothing left
   * to spend.
   */
  raiseCostCap: (jobId: string, costCapMicros: number) => Promise<Outcome>;
  /**
   * Let one Job take more turns than the tier above it allows.
   *
   * **The other ceiling `over_budget` folds, and until now the one with no
   * remedy.** A Job at its turn cap waits at `queued` reading the same label a
   * Job out of money reads, and raising the cost cap does not start it.
   *
   * **It moves nothing**, on `raiseCostCap`'s terms. What comes back is the
   * Job, and the field worth reading is `queued_reason`.
   *
   * The figure is a plain turn count, which is the unit `JobSpend` reads it in
   * — there is no conversion on this act. **It raises only**: a value at or
   * under the cap in force is refused before the request is sent, matching the
   * 422 Fleet would answer. Fleet refuses 409 on a terminal Job, which has no
   * turns left to take.
   */
  raiseTurnCap: (jobId: string, turnCap: number) => Promise<Outcome>;
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
   * Read one running Check's log as it is written, or `null` to stop.
   *
   * **Read-only, like `observeJob`**, and its own entry because it is its own
   * socket. `kept` is the last component of a `CheckUnderway.output_path`, and
   * Fleet resolves it against the Checks it is running before opening anything.
   */
  followCheckOutput: (jobId: string | null, kept: string | null) => Promise<void>;
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
   * Read what the open Job holds on this machine, or `null` to stop.
   *
   * **Read-only and its own entry**, because it is its own operation on the
   * Rust side and for the same reason: `watchJob` is re-read on every event
   * naming the Job, and this walks a process table and a directory.
   */
  readResources: (jobId: string | null) => Promise<void>;
  /**
   * Ask Fleet to go and look at this Job now, and say what it found.
   *
   * **The rung below intervene.** Every other act on a Job here changes it, so
   * a person who suspected one was wedged had one move. This one moves nothing
   * — what it leaves is a line in the Job's own log.
   *
   * It costs no model call. The answer arrives on `examination` rather than
   * coming back from the call, so a window reopened while a look was out still
   * gets it.
   */
  examineJob: (jobId: string) => Promise<void>;
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
   * Read one Check's own output, whole enough to read on the screen it is on.
   *
   * **`readCall`'s shape one record over.** `CheckRun.output_path` has always
   * said where the file is and `openArtifact` hands it to the operating system;
   * this is what brings the lines in, so a suite that went green for the wrong
   * reason can be argued with without leaving the app.
   *
   * `kept` is the row's own file name, off `output_path`. **Nothing here
   * composes a path** — `artifacts.ts` owns that rule — and Fleet resolves the
   * name against its own record, so this reaches no file the record does not
   * name.
   *
   * Read-only, like the reads above it.
   */
  readCheckOutput: (jobId: string, kept: string) => Promise<CheckOutputRead>;
  readFrame: (jobId: string, kept: string) => Promise<FrameRead>;
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
   * Merge the pull request this Job's branch went out on, then take the work.
   *
   * **The only entry on this surface that writes into a repository Armada does
   * not own**, and the reason it exists under `auto_merge: never`: the policy
   * says no machine decides that work lands, and a person who merges on the
   * forge instead skips the checks Armada would have run against what landed.
   *
   * Its own entry rather than a flag on `approveReview`, for the reason the
   * three below are three: they differ in what happens to the world, and one
   * entry taking which would read as one act and perform four.
   */
  mergePullRequest: (jobId: string) => Promise<Outcome>;
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
   * What people wrote on one Job's open pull request, or `null` to stop.
   *
   * **Read-only, and the one read on this surface that reaches a forge.** It is
   * its own entry beside `readEvidence` and `readDiff` for their reason: three
   * operations, opened by the surfaces that draw them, and this one costs a
   * process and a network rather than a record or a worktree.
   */
  readRemarks: (jobId: string | null) => Promise<void>;
  /**
   * Hand the comments a person picked off the pull request to a Drone, and let
   * one reply on the pull request say which.
   *
   * **A fifth entry rather than a flag on `requestChanges`.** What reaches
   * Fleet is a set of handles off a forge, not a person's words — the words are
   * read from the forge on the press, so nothing here decides what a Drone is
   * told. What it does to the Job is what `requestChanges` does.
   */
  takeUpRemarks: (jobId: string, remarks: string[]) => Promise<Outcome>;
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
   * Open the pull request Fleet opened for this Job, in whatever browses the
   * web on this machine.
   *
   * **A Job id and nothing else, which is `openArtifact`'s rule.** Main reads
   * the address off the reading it published and checks it is a web address
   * before handing it over, so the renderer never holds the argument that
   * decides what opens — an address arriving from a click handler is the one
   * string that would turn this into arbitrary URL handling.
   *
   * **Its own entry rather than a fourth `Artifact`.** That set is files, main
   * derives a path for each, and every one of them lands on this machine; this
   * one is served rather than derived and it leaves the app. One capability
   * covering both would read as one act and perform two.
   */
  openPullRequest: (jobId: string) => Promise<Followed>;
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
  manifestReading: null,
  missed: 0,
  readAt: null,
  approving: [],
  proposing: null,
  holds: { workflows: [], manifests: [], models: null },
  watched: { state: "none" },
  observed: { state: "none" },
  journalled: { state: "none" },
  followed: { state: "none" },
  footprint: { state: "none" },
  history: { state: "none" },
  evidence: { state: "none" },
  diff: { state: "none" },
  remarks: { state: "none" },
  reports: { state: "none" },
  resources: { state: "none" },
  examination: { state: "none" },
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
  forgetTerminalJobs: "bridge:forget-terminal-jobs",
  reclaimWorktree: "bridge:reclaim-worktree",
  forgetJob: "bridge:forget-job",
  redirectDrone: "bridge:redirect-drone",
  answerQuestion: "bridge:answer-question",
  restartStep: "bridge:restart-step",
  overrideVerdict: "bridge:override-verdict",
  rerunGate: "bridge:rerun-gate",
  showAgain: "bridge:show-again",
  raiseCostCap: "bridge:raise-cost-cap",
  raiseTurnCap: "bridge:raise-turn-cap",
  fileReport: "bridge:file-report",
  watchJob: "bridge:watch-job",
  observeJob: "bridge:observe-job",
  followCheckOutput: "bridge:follow-check-output",
  readHistory: "bridge:read-history",
  readEvidence: "bridge:read-evidence",
  readResources: "bridge:read-resources",
  examineJob: "bridge:examine-job",
  readDiff: "bridge:read-diff",
  readRemarks: "bridge:read-remarks",
  readCall: "bridge:read-call",
  readCheckOutput: "bridge:read-check-output",
  readFrame: "bridge:read-frame",
  readReports: "bridge:read-reports",
  readHeld: "bridge:read-held",
  approveReview: "bridge:approve-review",
  mergePullRequest: "bridge:merge-pull-request",
  requestChanges: "bridge:request-changes",
  rejectWork: "bridge:reject-work",
  takeUpRemarks: "bridge:take-up-remarks",
  openArtifact: "bridge:open-artifact",
  openPullRequest: "bridge:open-pull-request",
  summoned: "bridge:summoned",
} as const;
