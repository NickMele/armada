// The whole preload surface: every capability the renderer may reach.
//
// **Split out of `bridge.ts`**, which held the state, the capabilities and the
// channel names in one file at 897 lines. `BridgeState` and `NOTHING_YET`
// answer *what the renderer reads*; this answers *what the renderer may ask
// for* — a different question, and the file's own size was the two questions
// sharing one answer.

import type {
  Artifact,
  CallRead,
  CheckOutputRead,
  ClearOutcome,
  CommandAnswer,
  Draft,
  FileReport,
  Followed,
  FrameRead,
  Opened,
  Outcome,
  Proposed,
  ProtocolVersion,
  ReclaimOutcome,
  RunListRead,
  RunOutputRead,
  StagedAttachment,
  StartRun,
  WhenBlocked,
} from "@armada/protocol";
import type { BridgeState, Summons } from "./bridge";

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
  proposeFromRequest: (
    request: string,
    attachments: readonly StagedAttachment[],
  ) => Promise<Proposed>;
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
  /**
   * Paths under the checkout narrowed against typed text, for the `@` mention
   * popup. **Empty rather than a fault**, on a call that could not be made or
   * on nothing connected — a person is typing, and a toast over a popup they
   * may not even have open would be Bridge announcing a failure nobody asked
   * to hear about.
   */
  searchFiles: (query: string) => Promise<string[]>;
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
   * Allow or reject a command the job's drone reached for and was not given,
   * by the call id and one of the answers Fleet offered for it.
   *
   * **The call id says which of the two places it is.** A command a drone is
   * waiting on is answered in place and the job stays `running`; a refused row
   * on a job stopped at `blocked_by_policy` moves the job on. Fleet refuses 409
   * where the call names nothing waiting or refused, and where the answer was
   * not offered.
   */
  answerCommand: (jobId: string, call: string, answer: CommandAnswer) => Promise<Outcome>;
  /**
   * Change how one job meets a command its drone was not given. **Live**: the
   * next command the drone reaches for reads it, and no drone is respawned.
   */
  setWhenBlocked: (jobId: string, whenBlocked: WhenBlocked) => Promise<Outcome>;
  /**
   * Choose the model one job's later steps start on, or `null` for the one its
   * workflow gives each. **Live**: the step running now keeps its model.
   */
  setModel: (jobId: string, model: string | null) => Promise<Outcome>;
  /**
   * Take back a command a person allowed for one job, by the command exactly as
   * it was allowed. A line in `armada.yml` is not touched.
   */
  removeAllowedCommand: (jobId: string, run: string) => Promise<Outcome>;
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
  /**
   * Read one Job whole and keep it current, or `null` to stop.
   *
   * The renderer says which Job is open; main does the reading and republishes
   * it whenever an event names that Job. One call per open, not one per event.
   */
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
  /** Read the run sheet — Journey 9 — or `null` to stop. Opened by the sheet,
   * not the Job: `readDiff`'s rule, one read on demand. */
  watchRunSheet: (jobId: string | null) => Promise<void>;
  /** One run's output, or `null` to stop — opened for a run `startRun` just
   * began, or one the sheet is reopening onto in flight. */
  observeRun: (jobId: string | null, runId: string | null) => Promise<void>;
  /** Run one Check or Command in this Job's own worktree, narrowed or whole.
   * **A rehearsal**: no Evidence, nothing on the Job moves. Opens
   * `observeRun` for the caller the moment the run exists. */
  startRun: (jobId: string, body: StartRun) => Promise<Outcome>;
  /** End a run's process group. Its log keeps what printed. */
  stopRun: (jobId: string, runId: string) => Promise<Outcome>;
  /** Put back the files one run changed, from the snapshot taken just before
   * it. Refused while a Drone is working, while a run is in flight, on a run
   * already undone or with no snapshot. */
  undoRun: (jobId: string, runId: string) => Promise<Outcome>;
  /** Every earlier run from the sheet, newest first, and what would not read. */
  listRuns: (jobId: string) => Promise<RunListRead>;
  /** One run's log, read back as a window that says it is one. */
  getRunOutput: (jobId: string, runId: string) => Promise<RunOutputRead>;
  /** Start a declared server — this Job's worktree, or the main checkout with
   * no Job. `server.serving`/`server.exited` follow as events. */
  startServer: (name: string, jobId?: string) => Promise<Outcome>;
  /** End a server's process group. Its log keeps what printed. */
  stopServer: (serverId: string) => Promise<Outcome>;
  /** Open one server link in the system browser. **No surface navigates.** A
   * server id and the link's own address — main checks it against the links
   * it holds for that server before handing anything to the OS. */
  openServerLink: (serverId: string, url: string) => Promise<Followed>;
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
   * Send the branch back for a Drone that can edit files to bring it current
   * with main. `#663`. Fleet runs the rebase; only a conflict spawns a Drone,
   * on the step before the one that delivers, never the gate's own.
   */
  resolvePullRequestConflict: (jobId: string) => Promise<Outcome>;
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
   * Hand the comments a person picked off the pull request to a Drone.
   * Nothing is written back onto the pull request.
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
   * Open one comment on the pull request, in whatever browses the web on this
   * machine.
   *
   * **A Job id and a comment id, never an address** — `openPullRequest`'s rule
   * exactly. Main looks the comment up in the remarks reading it published,
   * reads its own `url` off that and checks it is a web address before handing
   * it over, so the renderer never holds the string that decides what opens.
   *
   * `undefined` where the comment carries no address: the caller draws no link
   * for one, rather than a link that opens nothing.
   */
  openRemarkLink: (jobId: string, remarkId: string) => Promise<Followed>;
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
