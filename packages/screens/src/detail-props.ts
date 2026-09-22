// What a caller must hand `JobDetail`, and nothing else.
//
// **Fifty-three fields, which is the finding rather than the inconvenience.**
// A screen takes props in and sends callbacks out — `index.ts` states that as
// the package's whole posture — so the surface is wide by design: every act a
// person can take on a Job arrives as one more `on…`. What it is not is part
// of arranging the screen, and `JobDetail.tsx` says its own job is to hold the
// open state of one reading and to arrange the regions.

// **Split out because that file was cut twice and grew back both times.** Its
// own header says so. The earlier cuts moved what a region *says* —
// `heading.tsx`, `step.tsx`, `chapters.tsx`, `Sheets.tsx`. This moves what the
// screen is *given*, the one large thing left that was neither state nor
// arrangement.

// **Re-exported from `JobDetail.tsx` rather than imported from here.** A caller
// names the screen it is configuring, not the file the type happens to sit in,
// which is why that file already re-exports `ConfirmableAct`, `FoldedReads`
// and `Render` from their own modules.

import type { ActAnswer, ActingAct, DecidingAct } from "./pending";

import type {
  CommandAnswer,
  Examination,
  FileReport,
  FollowedLog,
  History,
  Holds,
  JobSummary,
  Journalled,
  JudgeAnswer,
  ManifestSummary,
  ModelChoices,
  Observed,
  Outcome,
  Watched,
  WhenBlocked,
  WhenRefused,
  WorkflowSummary,
} from "@armada/protocol";

import type { ConfirmableAct, HeldAct } from "./Acts";
import type { JobDraft } from "./draft/held";
import type { ShowAgainCall } from "./again";
import type { ExplainCommand, ReadCall } from "./calls";
import type { FrameSrc, ReadFrame } from "./frames";
import type { FoldedReads } from "./mine";
import type { OpenArtifact, OpenPullRequest } from "./opening";
import type { FollowCheckOutput, ReadCheckOutput } from "./outputs";
import type { AddTask, DropTask, PlanEditAnswer } from "./plan-edits";
import type { RunSheetSlice } from "./rehearsal";
import type { OpenStudioFrom } from "./work";

export type JobDetailProps = {
  job: JobSummary;
  /** `GET /jobs/:job_id` for this Job, as main published it. */
  watched: Watched;
  workflows: readonly WorkflowSummary[];
  manifests: readonly ManifestSummary[];
  /** True while what is shown is not live. Every control is refused. */
  stale: boolean;
  /** Now, injected. A whole-Job elapsed is read, so it has to move. */
  now: number;
  /** In flight. A second press does not send a second command. */
  acting: boolean;
  /** An approval already sent for this Job. */
  approving: boolean;
  /**
   * This Job's stopped step is having its Checks run again. **Its own flag
   * beside `acting`'s**: the re-run can take minutes, and the step panel says
   * so for as long as it runs rather than only reading generically busy.
   */
  rerunningChecks: boolean;
  /** A decision on this Job's work already in flight. */
  deciding: boolean;
  /** Which act at the review gate that is, so its own control waits. #1117. */
  decidingAct?: DecidingAct | undefined;
  /** Which act `acting` is, so its own control waits. #1117. */
  actingAct?: ActingAct | undefined;
  /** What Fleet said to the last act on this Job, so the control that sent it answers. */
  answered?: ActAnswer | undefined;
  /** Ask for a confirmation. Nothing destructive is one press from here. */
  onAct: (act: ConfirmableAct, jobId: string) => void;
  /**
   * Kill, straight through. **The hold on the header's own kill was the
   * confirmation**, so this sends where `onAct` asks.
   */
  onActHeld: (act: HeldAct, jobId: string) => void;
  /** Send a redirect straight through — its own dialog is the confirmation. */
  onRedirect: (jobId: string, instruction: string) => void;
  /**
   * Answer the question this Job's drone asked, by the label picked.
   *
   * **Straight through, with no confirmation.** Picking an option and pressing
   * send are already two deliberate acts on a closed set the drone chose, and a
   * dialog on top would be a third press for the ordinary path.
   */
  onAnswer: (jobId: string, questionId: string, chose: string) => void;
  /**
   * Allow or reject a command the drone was not given, by its call id. The
   * same call answers a command a drone is waiting on and a refused row on a
   * stopped job. Straight through, for `onAnswer`'s reason.
   *
   * **`rule` rides only with `always_allow`**, and only where the command
   * offered one to pick from — one of `CommandInFlight.rules` or
   * `Refusal.rules`.
   */
  onAnswerCommand: (
    jobId: string,
    call: string,
    answer: CommandAnswer,
    note?: string,
    rule?: string,
  ) => void;
  /** Ask what one command does. It decides nothing; absent draws no control. */
  onExplainCommand?: ExplainCommand;
  /** Answer the question a judge refusal opened. One press is the whole answer. */
  onAnswerJudge: (jobId: string, askedAt: string, answer: JudgeAnswer, note?: string) => void;
  /** How this job meets the next such command. Live; nothing restarts. */
  onSetWhenBlocked: (jobId: string, whenBlocked: WhenBlocked) => void;
  /** How this job meets the next judge criterion that refuses. Live; nothing restarts. */
  onSetWhenRefused: (jobId: string, whenRefused: WhenRefused) => void;
  /** The model this job's later steps start on, or `null` for the workflow's. Live. */
  onSetModel: (jobId: string, model: string | null) => void;
  /** Choose the model the Job's review step runs on, or clear it. #903. */
  onSetReviewModel: (jobId: string, model: string | null) => void;
  /** Take back a command allowed for this job. A line in `armada.yml` is not touched. */
  onRemoveAllowedCommand: (jobId: string, run: string) => void;
  /** What `list_models` offers, for the Job settings panel. `null` until it is read. */
  models: ModelChoices | null;
  /** Overrule a Judge that refused the work, with the reason. */
  onOverrule: (jobId: string, reason: string) => void;
  /**
   * Send a step a gaming flag held back to be worked again, with an optional
   * note for the next Drone. **The restart, answered from the card**, where the
   * Drone has gone; where it still holds its session the card redirects it
   * through `onRedirect` instead. The card's answer is its own confirmation,
   * as the redirect's dialog is. #1079.
   */
  onSendBack: (jobId: string, note?: string) => void;
  /**
   * Give this job a higher cost ceiling, in millionths of a dollar. **The one
   * handler here that moves no part of the job** — it stops the next dispatch
   * being refused for money, and the job stays exactly where it was.
   */
  onRaiseCap: (jobId: string, costCapMicros: number) => void;
  /**
   * Let this job take more turns. **The other ceiling `over_budget` folds**,
   * and it moves no part of the job either — it stops the next dispatch being
   * refused for turns. A plain turn count: the cost cap converts and this one
   * does not.
   */
  onRaiseTurnCap: (jobId: string, turnCap: number) => void;
  /** Ask the gate again on a step it could not decide. Nothing is at stake. */
  onRerun: (jobId: string) => void;
  /**
   * Run a stopped step's Checks again, on the worktree as it stands. No
   * Drone spawns and no retry is spent — `#1105`, `onRerun`'s shape for the
   * other trigger.
   */
  onRerunChecks: (jobId: string) => void;
  /**
   * Ask the Job to show its work again. **Answered to this screen**, like
   * `onReport`, because what a press came to is said beside its control.
   * Absent draws no control, only the sets earlier presses kept.
   */
  onShowAgain?: ShowAgainCall;
  /**
   * Which Job's diff the host should hold open, or `null` for none.
   *
   * **It has to be stable**: an effect depends on it, and a lambda rebuilt on
   * every tick of the clock would reopen the read on every tick with it.
   */
  onReadDiff: (jobId: string | null) => void;
  /**
   * The host calls this screen makes on a person's behalf, handed in.
   *
   * **Every one of them is a round trip to a process with a filesystem or a
   * socket.** This screen decides what they mean and when to make them; it
   * does not reach for the thing that makes them, because a screen that did
   * could not be rendered anywhere but inside the app.
   */
  onOpenArtifact: OpenArtifact;
  /**
   * Open the pull request this Job opened, in whatever browses the web here.
   *
   * **Beside `onOpenArtifact` and not folded into it.** That one takes a word
   * from a closed set and lands a file in an editor; this takes no word at all
   * and leaves the app. One prop taking which would read as one act and perform
   * two — and the two fail at different things, which is why `Followed` is not
   * `Opened`.
   */
  onOpenPullRequest: OpenPullRequest;
  /**
   * Open another Job in this same surface — today, the one that replaced this
   * one. `#1439`.
   *
   * **Not a round trip like the two above it.** It is the shell's own
   * navigation, which is why it is optional: a story or a harness that draws
   * this screen alone has nowhere to go, and the callout then says what
   * happened without offering a press that would do nothing.
   */
  onOpenJob?: (jobId: string) => void;
  /**
   * Open the Studio this Job was dispatched from, landing on its own node.
   * `#1362`.
   *
   * **`onOpenJob`'s shape, and optional for its reason**: it is the shell's
   * own navigation, so a story or a harness drawing this screen alone offers
   * no press rather than a dead one.
   */
  onOpenStudio?: OpenStudioFrom;
  onReadCall: ReadCall;
  /**
   * Read one Check's own output. **`onReadCall`'s shape one record over**, and
   * its own prop for the same reason: it is a round trip to the process that
   * has the file, and the screen does the reading rather than the fetching.
   */
  onReadCheckOutput: ReadCheckOutput;
  /**
   * Read one frame a step's harness produced. **`onReadCheckOutput`'s shape one
   * record over** — the bytes come from the process that can reach Fleet, and
   * the screen is handed the way to ask rather than the way to connect.
   */
  onReadFrame: ReadFrame;
  /**
   * Where a recording streams from. **Beside `onReadFrame` rather than folded
   * into it**: that one answers with bytes and this one answers with an
   * address, because a video cannot be held whole before it plays.
   */
  onFrameSrc: FrameSrc;
  /** Stable, like `onReadDiff` — an effect in the decision block depends on it. */
  onNeedMaterial: (jobId: string | null) => void;
  /**
   * Open or close the read of the pull request's comments. Stable for
   * `onNeedMaterial`'s reason, and its own prop because it is its own read: one
   * reaches a record and this one reaches a forge.
   */
  onNeedRemarks: (jobId: string | null) => void;
  /** Say this job failed in error, with the record attached. */
  onReport: (jobId: string, filing: FileReport) => Promise<Outcome>;
  /** Let this Job run. Sent on the press, with no confirmation. */
  onApprove: (jobId: string) => void;
  /**
   * The four answers to a Job at `awaiting_review`. Four props, not one — each
   * does something different to the Job, and one prop taking which would read
   * as one act and perform four.
   *
   * `onMergePullRequest` is drawn only where the Job's record holds a pull
   * request, which `Decide` decides from the detail rather than from a flag.
   */
  onMergePullRequest: (jobId: string) => void;
  /** Start the pull request's failed CI runs again. #905. */
  onRerunFailedChecks: (jobId: string) => void;
  /** Send the branch back for a Drone to find out why CI failed. #905. */
  onInvestigateFailedChecks: (jobId: string) => void;
  /** Queue a Job after this one lands, from a For context finding. #906. */
  onQueueAfterFinding: (jobId: string, finding: string) => void;
  /** File the issue a person confirmed, drafted from a For context finding. #906. */
  onFileFindingIssue: (jobId: string, finding: string, title: string, body: string) => void;
  /** Open the issue a finding became. #906. */
  onOpenFindingIssue: (jobId: string, finding: string) => void;
  onApproveReview: (jobId: string) => void;
  onRequestChanges: (jobId: string, note: string) => void;
  onReject: (jobId: string) => void;
  /**
   * The fifth answer at the same gate: the comments a person picked off the
   * pull request reach a Drone. **Handles and never words** — Fleet reads the
   * pull request again on the press, so nothing here decides what a Drone is
   * told.
   */
  onTakeUpRemarks: (jobId: string, remarks: string[]) => void;
  /** Dismiss a finding the review raised, with the reason. #907. */
  onDismissFinding: (jobId: string, finding: string, reason: string) => void;
  /** Open one comment on the forge. `Decide`'s own note on why this is a Job
   *  id and a comment id, never an address. */
  onOpenRemarkLink: (jobId: string, remarkId: string) => void;
  /**
   * What the second socket has said. **Opened for every Job that is open**, not
   * on a press: the activity log is a chapter of the step's story and a chapter
   * that filled only after somebody asked is the tab this screen removed.
   */
  observed: Observed;
  /**
   * What the Job's own log has said. **The third voice, and the one thing there
   * is to read before a Drone exists** — `observed` is a Drone's transcript and
   * is empty for the whole of preparation, which is precisely when somebody
   * opens this screen to find out what is going on.
   */
  journalled: Journalled;
  /**
   * The running Check's log main is following, as it is written. Optional,
   * because a surface drawing a Job with no gate running has nothing to follow.
   */
  followed?: FollowedLog;
  /** Follow one running Check's log, or `null` to stop. */
  onFollowCheckOutput?: FollowCheckOutput;
  /**
   * What the open Job holds on this machine. **Opened with the Job**, like the
   * two sockets above: the panel it draws answers *is this working*, which is
   * the question somebody opening a Job they suspect has wedged came with.
   */
  resources: Holds;
  /**
   * The Job's history, for the one line of Pulse that says what a person last
   * did. Optional: without it, Pulse draws the latest of the Drone and Fleet.
   */
  history?: History;
  /**
   * What the last look found, where somebody pressed for one. **Never opened
   * with the Job** — an answer that appeared unasked would be the machine
   * noticing on its own, which is a different capability.
   */
  examination: Examination;
  /** Ask Fleet to go and look at this Job now. It costs no model call. */
  onExamine: (jobId: string) => void;
  /** The reads the panel's chapters draw from. */
  recorded: FoldedReads;
  onCopied: (value: string) => void;
  /**
   * Say a sentence to the person. **Mostly a failure** — an open that did
   * nothing is the defect `#246` is about, and success is usually the file
   * being in front of them — **and, since `#897`, the one pair of successes
   * that need a word anyway**: adding or dropping a task changes a row among
   * several, so the act that changed it says so once, briefly.
   */
  onSaid: (sentence: string) => void;
  /**
   * Add a task to this Job's plan, from the Plan region's own eyebrow act.
   * `after` is always `""` — the end — because reordering is not in this
   * milestone; the owner chose add and drop alone. Refused by name:
   * `fleet.no_plan`, a blank title.
   */
  onAddTask: (jobId: string, add: AddTask) => Promise<PlanEditAnswer>;
  /**
   * Drop a task from this Job's plan, with a reason, from the row a cursor or
   * the keyboard is on. Refused by name: `fleet.no_plan`,
   * `fleet.no_such_task`, `fleet.task_already_settled` on a `done` or already
   * `dropped` task, a blank reason.
   */
  onDropTask: (jobId: string, drop: DropTask) => Promise<PlanEditAnswer>;
  rehearsal: RunSheetSlice; // The run sheet, Journey 9 — bundled, `recorded`'s precedent
  /**
   * Whether Where things are is open — Fleet's own preference,
   * `BridgeState.preferences.where_things_are_open`, `#927`. This package
   * stays free of Electron, so the caller reads and saves it; until
   * preferences arrive, the caller's own default is closed.
   */
  whereOpen: boolean;
  onOpenWhere: (open: boolean) => void;
  /**
   * Open the composer — `n` in `actions.toml`, scope `anywhere`. **The Board's
   * own key, answered here too**: `new_job` is the one contextual act that
   * acts on nothing on screen, so every contextual surface reaches the same
   * composer through the same handler rather than each opening its own.
   */
  onCompose: () => void;
  /**
   * What this Job's boards draw that Fleet cannot serve yet — `#1532`'s draft
   * schema, carried by the mock and absent against a real Fleet.
   *
   * **Absent is not empty.** A destination handed nothing derives what today's
   * wire can answer instead, which is what keeps a board built on the drafts
   * rendering thinner rather than blank. It goes away in `#1545`, when a shape
   * is promoted and arrives on the Job's own reads.
   */
  draft?: JobDraft;
};
