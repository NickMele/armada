// What the state main publishes is, and the channel names it and every
// capability travel over.
//
// **The capabilities are `api.ts`, split out at 897 lines.** This file kept
// growing because a state field and a capability answer different questions
// and one file was answering both; `BridgeApi` is what the renderer may ask
// for, and what is left here is what it reads without asking.
//
// Nothing here runs in more than one process — the preload is a wire and not an
// import path, and this file is the shape of what crosses it.

import type {
  BridgeIdentity,
  Connection,
  Diff,
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
  Remarks,
  Reports,
  Watched,
} from "@armada/protocol";
import type { FleetCapacity, JobSummary, ProposalInFlight, UnreadableJob } from "@armada/protocol";
import type { ManifestReading } from "@armada/protocol";
import type { RunFollowed, RunSheetRead, ServerList } from "@armada/protocol";
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
  /** The run sheet — Journey 9. Opened with the sheet, not the Job, `diff`'s
   * rule: most Jobs are never rehearsed, so nothing pays for one nobody asked. */
  runSheet: RunSheetRead;
  /** The run a window is reading, as it prints — its own socket, `followed`'s
   * shape one subject over. One at a time. */
  runFollowed: RunFollowed;
  /**
   * Every server Fleet holds, each Job's and the main checkout's.
   *
   * **Read once per connection, kept current by folding `server.*` off
   * `/events`**, `capacity`'s terms. Not scoped to the open Job — *Where
   * things are* keeps a *Serving* row up after the sheet that started it
   * closes.
   */
  servers: ServerList;
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
  runSheet: { state: "none" },
  runFollowed: { state: "none" },
  servers: { servers: [] },
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
  answerCommand: "bridge:answer-command",
  setWhenBlocked: "bridge:set-when-blocked",
  setModel: "bridge:set-model",
  removeAllowedCommand: "bridge:remove-allowed-command",
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
  watchRunSheet: "bridge:watch-run-sheet",
  observeRun: "bridge:observe-run",
  startRun: "bridge:start-run",
  stopRun: "bridge:stop-run",
  undoRun: "bridge:undo-run",
  listRuns: "bridge:list-runs",
  getRunOutput: "bridge:get-run-output",
  startServer: "bridge:start-server",
  stopServer: "bridge:stop-server",
  openServerLink: "bridge:open-server-link",
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
  resolvePullRequestConflict: "bridge:resolve-pull-request-conflict",
  requestChanges: "bridge:request-changes",
  rejectWork: "bridge:reject-work",
  takeUpRemarks: "bridge:take-up-remarks",
  openArtifact: "bridge:open-artifact",
  openPullRequest: "bridge:open-pull-request",
  openRemarkLink: "bridge:open-remark-link",
  summoned: "bridge:summoned",
} as const;
