// A `window.armada` with no main process behind it, answering from a scenario.
//
// **Typed as `BridgeApi` member by member, never a `Proxy`**, so a capability
// added to the preload fails typecheck here until this answers it. What each
// kind of call answers, and why no more: `docs/practices/running-locally.md`,
// *Bridge on a mock Fleet*.

import { PROTOCOL_VERSION } from "@armada/protocol";
import type { JobSummary, Outcome } from "@armada/protocol";

import type { BridgeApi } from "../../../shared/api";
import type { BridgeState, Summons } from "../../../shared/bridge";
import { unanswered } from "./scenario";
import type { Scenario } from "./scenario";
import { keeping } from "./studio-fleet";

const OK: Outcome = { ok: true };

/** A window's own `window.armada`, over one scenario. Each call makes a fresh one. */
export function fakeBridge(scenario: Scenario): BridgeApi {
  let state: BridgeState = scenario.state;
  const listeners = new Set<(state: BridgeState) => void>();
  const summoners = new Set<(to: Summons) => void>();
  let summoned = false;

  /** Publish a change — on a microtask, because main's reach a window over IPC, never inside the call. */
  function publish(change: Partial<BridgeState>): void {
    state = { ...state, ...change };
    const now = state;
    queueMicrotask(() => listeners.forEach((listener) => listener(now)));
  }

  /** One field on one Job, and on its detail where that Job is the one open. */
  function move(jobId: string, change: Partial<JobSummary>): void {
    const watched = state.watched;
    publish({
      jobs: state.jobs.map((job) => (job.id === jobId ? { ...job, ...change } : job)),
      watched:
        watched.state === "read" && watched.jobId === jobId
          ? { ...watched, detail: { ...watched.detail, job: { ...watched.detail.job, ...change } } }
          : watched,
    });
  }

  function forget(jobIds: readonly string[]): void {
    publish({ jobs: state.jobs.filter((job) => !jobIds.includes(job.id)) });
  }

  /** This window's Fleet for Studios, over the Studios the scenario names. */
  const studios = keeping(scenario.studios).routes({ state: () => state, publish });

  const readsOf = (jobId: string) => scenario.reads[jobId];
  const path = (jobId: string, route = "") => `/jobs/${encodeURIComponent(jobId)}${route}`;
  const failed = (jobId: string, route: string) =>
    ({ state: "failed", jobId, outcome: unanswered(path(jobId, route)) }) as const;
  const refused = (route: string) => ({ ok: false, outcome: unanswered(route) }) as const;
  const unread = (route: string) => ({ state: "failed", outcome: unanswered(route) }) as const;
  const nothing = { state: "none" } as const;

  const api: BridgeApi = {
    protocolVersion: () => PROTOCOL_VERSION,
    state: async () => state,
    subscribe: (onState) => {
      listeners.add(onState);
      return () => listeners.delete(onState);
    },

    // Accepted, and no Job appears: one Fleet made would carry an id and a plan invented here.
    proposeJob: async () => OK,
    proposeFromRequest: async (request) => ({
      ok: false,
      why: "faulted",
      request,
      outcome: unanswered("/jobs/propose"),
    }),
    stopProposal: async () => OK,
    stageAttachment: async (_bytes, filename) => ({ path: filename }),
    searchFiles: async () => [],

    approveDispatch: async (jobId) => (move(jobId, { status: "queued" }), OK),
    redispatchJob: async () => OK,
    killDrone: async () => OK,
    killJob: async (jobId) => (move(jobId, { status: "killed" }), OK),
    clearTerminalJobs: async (jobIds) => {
      const at = new Date().toISOString();
      jobIds.forEach((jobId) => move(jobId, { reclaimed_at: at }));
      return { reclaimed: [], failed: [] };
    },
    forgetTerminalJobs: async (jobIds) => (forget(jobIds), { cleared: [...jobIds], failed: [] }),
    reclaimWorktree: async (jobId) => (move(jobId, { reclaimed_at: new Date().toISOString() }), OK),
    deleteBranch: async () => OK,
    forgetJob: async (jobId) => (forget([jobId]), OK),
    redirectDrone: async () => OK,
    answerQuestion: async () => OK,
    answerCommand: async () => OK,
    answerHelmCall: async () => OK,
    explainCommand: async (jobId, callId) => refused(path(jobId, `/calls/${callId}/explain`)),
    setWhenBlocked: async () => OK,
    answerJudge: async () => OK,
    setWhenRefused: async () => OK,
    setModel: async (jobId, model) => {
      if (model !== null) move(jobId, { model });
      return OK;
    },
    setReviewModel: async () => OK,
    removeAllowedCommand: async () => OK,
    restartStep: async () => OK,
    overrideVerdict: async () => OK,
    rerunGate: async () => OK,
    rerunChecks: async () => OK,
    showAgain: async () => OK,
    raiseCostCap: async () => OK,
    raiseTurnCap: async () => OK,
    saveLimits: async () => OK,
    savePreference: async () => OK,
    fileReport: async () => OK,
    addTask: async (jobId) => refused(path(jobId, "/plan")),
    dropTask: async (jobId) => refused(path(jobId, "/plan")),

    // The footprint and the hand-in are pushed about the open Job, so they arrive with it.
    watchJob: async (jobId) => {
      const reads = jobId === null ? undefined : readsOf(jobId);
      publish({
        watched: jobId === null ? nothing : (reads?.watched ?? failed(jobId, "")),
        footprint: reads?.recorded.footprint ?? nothing,
        handed: reads?.recorded.handed ?? nothing,
      });
    },
    observeJob: async (jobId) =>
      publish({ observed: jobId === null ? nothing : (readsOf(jobId)?.observed ?? nothing) }),
    followCheckOutput: async () => publish({ followed: nothing }),
    // A fixture with no history is one whose story never asked for it, and `none` is what it draws.
    readHistory: async (jobId) =>
      publish({ history: jobId === null ? nothing : (readsOf(jobId)?.history ?? nothing) }),
    readResources: async (jobId) =>
      publish({
        resources: jobId === null ? nothing : (readsOf(jobId)?.resources ?? failed(jobId, "/resources")),
      }),
    watchRunSheet: async (jobId) =>
      publish({ runSheet: jobId === null ? nothing : failed(jobId, "/runs/sheet") }),
    observeRun: async () => publish({ runFollowed: nothing }),
    startRun: async () => OK,
    stopRun: async () => OK,
    undoRun: async () => OK,
    listRuns: async (jobId) => refused(path(jobId, "/runs")),
    getRunOutput: async (jobId, runId) => refused(path(jobId, `/runs/${runId}/output`)),

    watchCheckoutRunSheet: async (want) =>
      publish({ checkoutRunSheet: want ? unread("/manifest/runs/sheet") : nothing }),
    observeCheckoutRun: async () => publish({ checkoutRunFollowed: nothing }),
    startCheckoutRun: async () => OK,
    stopCheckoutRun: async () => OK,
    undoCheckoutRun: async () => OK,
    listCheckoutRuns: async () => refused("/manifest/runs"),
    getCheckoutRunOutput: async (runId) => refused(`/manifest/runs/${runId}/output`),
    getCheckoutRunDiff: async (runId) => refused(`/manifest/runs/${runId}/diff`),
    watchManifestDrift: async (want) => publish({ manifestDrift: want ? unread("/manifest/drift") : nothing }),
    // Held for the life of the window: a failure here would draw every surface's Fleet panel in trouble.
    watchOverview: async () => undefined,
    startCheckoutVerify: async () => OK,

    readManifestFile: async () => refused("/manifest/file"),
    saveManifestFile: async () => unread("/manifest/file"),
    editManifest: async () => unread("/manifest/edit"),
    readManifestSpend: async () => refused("/manifest/spend"),

    readRepositoryScan: async () => refused("/repositories/scan"),
    readManifestProposals: async () => refused("/manifest/proposals"),
    editManifestProposal: async () => unread("/manifest/proposals"),
    writeManifestProposal: async () => unread("/manifest/proposals"),

    listRepositoryAllowedCommands: async () => refused("/manifest/allowed-commands"),
    removeRepositoryAllowedCommand: async () => refused("/manifest/allowed-commands"),

    readKitInventory: async () => refused("/kit/inventory"),
    listKitServers: async () => refused("/kit/servers"),
    addKitServer: async () => refused("/kit/servers"),
    forgetKitServer: async () => refused("/kit/servers"),
    setKitServerReach: async () => refused("/kit/servers"),
    setManifestServerReach: async () => refused("/kit/servers"),

    // The rail's pick is this window's own, so it moves here as it does in main.
    pickRepository: async (root) => publish({ repository: root }),

    chooseFolder: async () => null,
    resolveFolder: async () => null,
    addRepository: async () => unread("/repositories"),
    cloneRepository: async () => unread("/repositories/clone"),

    startServer: async () => OK,
    stopServer: async () => OK,
    openServerLink: async () => ({ ok: false, why: "no_address" }),
    // The capture window is a second window main opens — #1294. The mock has
    // none, so this says what a Studio nothing is holding would say.
    openCaptureWindow: async () => ({ ok: false, why: "no_studio" }),
    captureWindow: {
      read: () => () => {},
      arm: async () => null,
      aim: async () => null,
      hold: async () => null,
      release: async () => {},
      save: async () => OK,
      reload: async () => {},
      followRefused: async () => {},
      scroll: () => {},
    },
    // A Studio starting one entry — #1289, #1345. The mock Fleet answers, so
    // what these do here is what every other unstubbed act does: nothing.
    startStudioRun: async () => ({ ok: true }) as Outcome,
    startStudioServer: async () => ({ ok: true }) as Outcome,
    examineJob: async (jobId) => publish({ examination: failed(jobId, "/examine") }),
    readEvidence: async (jobId) =>
      publish({
        evidence: jobId === null ? nothing : (readsOf(jobId)?.recorded.evidence ?? failed(jobId, "/evidence")),
      }),
    readDiff: async (jobId) =>
      publish({ diff: jobId === null ? nothing : (readsOf(jobId)?.recorded.diff ?? failed(jobId, "/diff")) }),
    readCall: async (jobId, callId) =>
      readsOf(jobId)?.calls[callId] ?? refused(path(jobId, `/calls/${callId}`)),
    readCheckOutput: async (jobId, kept) =>
      readsOf(jobId)?.checkOutputs[kept] ?? refused(path(jobId, `/checks/${kept}/output`)),
    readFrame: async (jobId, kept) => readsOf(jobId)?.frames[kept] ?? refused(path(jobId, `/frames/${kept}`)),
    readComposing: async (repository) => refused(`/composing?repository=${encodeURIComponent(repository)}`),
    // The app's own spelling, which a browser has no handler for — `props.ts`' reason.
    frameStreamUrl: (jobId, kept) => `armada-frame://frame/${jobId}/${kept}`,
    readReports: async (want) => publish({ reports: want ? unread("/reports") : nothing }),
    readHeld: async (want) => publish({ held: want ? unread("/worktrees/held") : nothing }),
    // Every scenario keeps Studios, so the surface opens wherever it is reached. A scenario naming
    // none keeps an empty list and draws its empty state, never a read failure — #1341.
    ...studios,
    approveReview: async () => OK,
    mergePullRequest: async () => OK,
    rerunFailedChecks: async () => OK,
    investigateFailedChecks: async () => OK,
    queueAfterFinding: async () => OK,
    fileFindingIssue: async () => OK,
    openFindingIssue: async () => ({ ok: false, why: "no_address" }),
    requestChanges: async () => OK,
    rejectWork: async (jobId) => (move(jobId, { status: "rejected" }), OK),
    readRemarks: async (jobId) =>
      publish({
        remarks: jobId === null ? nothing : (readsOf(jobId)?.recorded.remarks ?? failed(jobId, "/remarks")),
      }),
    takeUpRemarks: async () => OK,
    dismissFinding: async () => OK,
    openArtifact: async () => ({ ok: false, why: "no_repository" }),
    openPullRequest: async () => ({ ok: false, why: "no_address" }),
    openRemarkLink: async () => ({ ok: false, why: "no_address" }),

    // A scenario opens its Job the way a pressed notification does — once, though `StrictMode` registers twice.
    onSummoned: (onGo) => {
      summoners.add(onGo);
      if (scenario.opens !== undefined && !summoned) {
        summoned = true;
        const to = { jobId: scenario.opens };
        queueMicrotask(() => summoners.forEach((summoner) => summoner(to)));
      }
      return () => summoners.delete(onGo);
    },

    askHelm: async () => OK,
    // No session behind a mock Fleet, so the record is the refusal a window
    // with nothing connected already draws — never an invented record.
    helmDebugInfo: async () => refused("/helm/debug"),
    startHelmFresh: async () => OK,
    pointHelm: async () => undefined,
    // The mock provides no haptics, so nothing calls this; answered for the type.
    tap: () => undefined,
  };
  return { ...api, ...scenario.behaves?.({ state: () => state, publish }) };
}
