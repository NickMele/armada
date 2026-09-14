// One stream message, folded into `FleetConnection`'s state.
//
// Pulled out of `connection.ts` because the switch on `message.event.kind` was
// the largest thing left in that file and grows by one arm per new kind of
// event Fleet publishes — the same shape of growth #891 named. **Kept whole,
// not split by kind**: `connection.ts`'s own history already tried that
// question and answered it — every arm reaches the same state and the same
// `publish`, so cutting the switch into several files would put one machine's
// transitions in several places rather than one. What moved instead is the
// switch's *address*: it now takes its state through `ArrivalHost`, a narrow
// view of `FleetConnection` built once in its constructor, so the switch
// itself carries no private field of the class it serves.
//
// `reread`, `readCapacity` and `readLimits` travel with it: each is a stream
// arm's own read-and-publish, used nowhere else the switch is not.

import { connectedTo } from "@armada/protocol";
import { connects, PROTOCOL_VERSION, skew } from "@armada/protocol";
import type { Connection, JobSummary, ServerState, StreamMessage } from "@armada/protocol";
import type { BridgeState } from "../shared/bridge";
import type { Questions } from "./questions";
import type { RehearsalConnection } from "./rehearsal";
import { ask, capacityOf, limitsOf, preferencesOf } from "./request";
import type { ReviewMaterial } from "./review";
import type { RepositoryReads } from "./repositories";
import type { Again } from "./screen";
import type { BridgeStateFleet } from "./socket";

/** What the arrival switch reaches on `FleetConnection`, and nothing more. */
export interface ArrivalHost {
  current(): BridgeState;
  now(): number;
  greeted(): boolean;
  setGreeted(value: boolean): void;
  proposalRef(): string | null;
  setProposalRef(value: string | null): void;
  watchedJobId(): string | null;
  readonly repositories: RepositoryReads;
  readonly rehearsal: RehearsalConnection;
  /** Every open window's own Overview read again — `connection.ts`'s `windowFacades`. */
  overviewAgain(port: number): Promise<void>;
  readonly questions: Questions;
  readonly helm: { reconnected(port: number): void };
  readonly material: ReviewMaterial;
  readonly socket: { close(): void; resetUnreachable(): void };
  publish(change: Partial<BridgeState>): void;
  fold(job: JobSummary): void;
  forget(jobId: string): void;
  settle(connection: Connection): void;
  refresh(port: number, jobId: string): void;
  takeAgain(port: number, again: Again): Promise<void>;
}

/** Re-read the Board whole, brought current by a status move nobody folded. */
export async function reread(
  port: number,
  publish: (change: Partial<BridgeState>) => void,
  now: () => number,
): Promise<void> {
  const answer = await ask(port, "GET", "/jobs");
  if (answer.ok !== true) return;
  const list = answer.body as { jobs: JobSummary[]; unreadable?: [] };
  publish({ jobs: list.jobs, unreadable: list.unreadable ?? [], readAt: now() });
}

/**
 * How full the fleet is. **A failed read publishes `null`**, which draws as
 * nothing rather than as the last count — the bar must not keep saying
 * "2 of 2" off an answer it could not get.
 */
export async function readCapacity(
  port: number,
  publish: (change: Partial<BridgeState>) => void,
): Promise<void> {
  publish({ capacity: await capacityOf(port) });
}

/**
 * Fleet's three admission limits. **Once per connection**, `readManifest`'s
 * terms: nothing but a save changes them, and `saveLimits` publishes the new
 * reading itself rather than asking this to run again.
 */
export async function readLimits(
  port: number,
  publish: (change: Partial<BridgeState>) => void,
): Promise<void> {
  publish({ limits: await limitsOf(port) });
}

/**
 * A person's Bridge preferences. **Once per connection**, `readLimits`'
 * terms: nothing but a save changes them, and `Preferring.save` publishes the
 * new reading itself.
 *
 * **A failed read publishes nothing**, unlike `readLimits` — `preferencesOf`'s
 * reason: `BridgeState.preferences` is never `null`, so there is no "not read
 * yet" to draw, and overwriting it with the shipped default on a glitch would
 * draw a person's own choice as though it had never been saved.
 */
export async function readPreferences(
  port: number,
  publish: (change: Partial<BridgeState>) => void,
): Promise<void> {
  const read = await preferencesOf(port);
  if (read !== null) publish({ preferences: read });
}

export function applyArrival(host: ArrivalHost, text: string, fleet: BridgeStateFleet): void {
  let message: StreamMessage;
  try {
    message = JSON.parse(text) as StreamMessage;
  } catch {
    // The stream carries no error message, so an unparseable one is a
    // connection to drop rather than a state to fold.
    host.socket.close();
    return;
  }

  if (message.message === "resync") {
    // Again, because a Fleet restarted under a live socket is not the one
    // the runtime file described.
    const reading = skew({ fleet: message.protocol_version, bridge: PROTOCOL_VERSION });
    if (!connects(reading)) {
      host.socket.close();
      const speaks = message.protocol_version;
      const expected = PROTOCOL_VERSION;
      host.settle({ state: "version_skew", fleet, why: reading, speaks, expected });
      return;
    }
    host.socket.resetUnreachable();
    // Read before it is set, because both readings arrive as this message and
    // only the order tells them apart. See the field.
    const cameBack = !host.greeted();
    host.setGreeted(true);
    host.publish({
      connection: connectedTo(fleet, message.cursor),
      jobs: message.jobs.jobs,
      unreadable: message.jobs.unreadable ?? [],
      readAt: host.now(),
    });
    // What a proposal may name: read once per connection, because it changes
    // when Fleet restarts rather than when a Job moves.
    void host.repositories.readHoldings(fleet.port);
    // And how full the fleet is, which changes when a Job moves and is
    // therefore read again below on every status move.
    void readCapacity(fleet.port, host.publish);
    // And what Fleet's last read of `armada.yml` came to. **Once per
    // connection and never again**, unlike capacity: it changes when somebody
    // saves a file, and `manifest.reread` is what says so. This read is for
    // the window that opened after the save — which is most windows, since a
    // refusal stands until the file is corrected.
    void host.repositories.readManifestForEveryWindow(fleet.port);
    // And Fleet's three admission limits, once per connection: nothing but a
    // save changes them, and that act publishes its own new reading.
    void readLimits(fleet.port, host.publish);
    // And a person's Bridge preferences, once per connection, `readLimits`'
    // reason.
    void readPreferences(fleet.port, host.publish);
    // And every server Fleet holds, once per connection — `server.*` on
    // `/events` carries each row whole from here on.
    void host.rehearsal.readServers(fleet.port);
    // And every question waiting on a person, from every repository: events carry only what is
    // asked next, so what was already waiting is read off the Jobs that can hold one.
    void host.questions.readAll(fleet.port);
    // **And the open Job's screen, whole.** A resync says where every Job is
    // and nothing about what any one of them holds, so every region of the
    // Job somebody has open is taken again together — `screen.ts` is the
    // list, and it is one list so that a read added later is classified
    // rather than left out. #472.
    void host.takeAgain(fleet.port, { because: cameBack ? "fleet_came_back" : "stream_gap" });
    // And Helm's own socket, on a repository already targeted — a second
    // socket to the same peer, `takeAgain`'s reason repeated one seam over.
    host.helm.reconnected(fleet.port);
    return;
  }

  if (message.message === "missed") {
    // The count alone cannot repair what Bridge holds. A resync always
    // follows; until it lands the screen says how many were lost.
    host.publish({ missed: host.current().missed + message.dropped });
    return;
  }

  const event = message.event;
  const connection: Connection = connectedTo(fleet, message.cursor);

  if (event.kind === "job.created") {
    // The row travels whole, so the list gains it without a round trip — a
    // Job proposed over the API used to publish nothing and never appear.
    host.publish({ connection });
    host.fold(event.job);
    return;
  }
  if (event.kind === "job.step_advanced") {
    // **The row is replaced, not patched.** `current_step_id` has already
    // moved on the Job travelling with the event, and `event.status` is the
    // status the move happened *beneath* rather than a transition — folding
    // either by hand is how half a row goes stale.
    host.publish({ connection });
    host.fold(event.job);
    // A question asked on the step it left is gone with it.
    host.questions.moved(event.job);
    host.refresh(fleet.port, event.job.id);
    return;
  }
  if (event.kind === "drone.spawned" || event.kind === "drone.exited") {
    // **`job.step_advanced`'s shape, and for its reason.** `assigned_drone`
    // is a field of the row, so the summary travels whole and the Board gains
    // or loses the Drone without a round trip.
    //
    // **The detail is re-read, and on the exit that read is the point.** Fleet
    // writes the Drone's spend row before publishing an exit — see
    // `crates/fleet/src/allowance.rs` — so this is the first message on which
    // a whole figure can be read back. It is re-read rather than folded for
    // `job.judging`'s reason: what a Drone cost is served on the Job's own
    // `spend`, and a second copy carried here would give one fact two homes.
    //
    // Both kinds were arriving already and neither was matched, so each fell
    // through to the tail below, read `event.job_id` as `undefined` and was
    // taken for a Job this window had never seen — a full `GET /jobs` twice
    // per step boundary, and the open Job never re-read at all.
    host.publish({ connection });
    host.fold(event.job);
    host.questions.moved(event.job);
    host.refresh(fleet.port, event.job.id);
    return;
  }
  if (event.kind === "job.files_changed") {
    // **Only the open Job's, and the whole list rather than a fold.** The
    // reading replaces what is held, so a file that stopped being changed
    // leaves by not being in the next one — a stream of additions could never
    // say that. A reading about a Job nobody has open is dropped: nothing on
    // the Board changes when a file does.
    const mine = host.watchedJobId() === event.job_id;
    host.publish({
      connection,
      ...(mine ? { footprint: { state: "read" as const, jobId: event.job_id, reading: event } } : {}),
    });
    return;
  }
  if (event.kind === "evidence.submitted") {
    // **The moment, held; the submission, fetched.** Fleet publishes this the
    // instant the Evidence call lands, and it carries no part of what was
    // submitted — `GET /jobs/:job_id/evidence` is that read, and `review.ts`
    // says why it is asked for rather than pushed. `#813`.
    //
    // Only the open Job's, `job.files_changed`'s terms: nothing on the
    // Board's row moves when a Drone hands in. The step's rail draws it until
    // `job.checking` arrives with the gate's own reading, which is the window
    // that used to say nothing at all.
    //
    // The claims are taken again where somebody is already holding them — a
    // step sent back and submitted a second time is the case, and there the
    // panel is open on the Job it just changed.
    const mine = host.watchedJobId() === event.job_id;
    host.publish({
      connection,
      ...(mine ? { handed: { state: "heard" as const, jobId: event.job_id, moment: event } } : {}),
    });
    void host.material.evidenceSubmitted(fleet.port, event.job_id);
    return;
  }
  if (event.kind === "job.judging" || event.kind === "job.checking" || event.kind === "job.dry_run") {
    // `job.checking` is the same answer one tier along: `StepDetail.checking`
    // is re-read, and a running Check's elapsed time is counted, not re-read.
    // `job.dry_run` is that again for a Drone's own run, `StepDetail.dry_run`.
    // **Re-read rather than fold.** The call is served on the open Job's own
    // field, `StepDetail.judging`, which is what a Bridge opened mid-call
    // already reads — so folding it into a second copy would give one fact
    // two homes and a surface would take whichever arrived last. The event is
    // the wake-up; the detail is the answer.
    //
    // Only the open Job's, for `job.files_changed`'s reason: nothing on the
    // Board changes when a Judge call goes out. Two reads per call.
    host.publish({ connection });
    host.refresh(fleet.port, event.job_id);
    return;
  }
  if (event.kind === "job.asking" || event.kind === "job.command_waiting") {
    // **The row does change here, unlike a Judge call, and it was not being
    // changed.** `JobSummary.asking` is the second arm of the Needs-you rule:
    // a Job whose Drone has asked something is `running` with `who_is_acting`
    // = `Drone`, and only that flag lifts it out of Running. Fleet builds it
    // off the working slot, so every summary that travels with an event has
    // it absent — which is why the wire publishes this event at all.
    //
    // Nothing folded it, so a question moved the row only on the next full
    // re-read: the tab that exists to stop a question going unseen was the
    // one thing that did not see it. The detail is still re-read, because
    // what was asked and what each answer commits to live there; this is the
    // one bit the Board needs and cannot get any other way.
    //
    // Absent `asking` is the question coming back, and `false` says so
    // outright. A waiting command is the same arm, `waiting` for `asking`.
    const waiting =
      event.kind === "job.asking" ? event.asking !== undefined : event.waiting !== undefined;
    host.publish({
      connection,
      jobs: host.current().jobs.map((job) =>
        job.id === event.job_id ? { ...job, asking: waiting } : job,
      ),
    });
    // The dock's copy, whole off the event, for every repository's Job.
    if (event.kind === "job.asking") host.questions.asking(event.job_id, event.asking);
    else host.questions.commandWaiting(event.job_id, event.waiting);
    host.refresh(fleet.port, event.job_id);
    return;
  }
  if (event.kind === "job.plan_changed") {
    // **The row's own terms, `job.asking`'s shape.** The counts ride on the
    // event and patch the row directly; the plan itself does not, so the open
    // Job's detail is re-read for `work_plan` — `work-plan.ts` says why.
    host.publish({
      connection,
      jobs: host.current().jobs.map((job) =>
        job.id === event.job_id ? { ...job, tasks: event.tasks } : job,
      ),
    });
    host.refresh(fleet.port, event.job_id);
    return;
  }
  if (event.kind === "proposal.moved") {
    // **This window's own, matched on the token it sent.** Fleet publishes
    // every proposal on one stream and two windows may be dispatching at
    // once — folding whichever arrived last would draw somebody else's call
    // as yours and offer a stop that killed it.
    //
    // Absent `proposing` is the call coming back, however it came back, and
    // clears the state. The Jobs it produced arrive as `job.created` and are
    // folded there; nothing here puts a row on the board.
    if (event.client_ref !== undefined && event.client_ref === host.proposalRef()) {
      const proposing = event.proposing ?? null;
      if (proposing === null) host.setProposalRef(null);
      host.publish({ connection, proposing });
      return;
    }
    // Somebody else's, or one this window did not start. The connection is
    // still current, which is what the publish says and all it says.
    host.publish({ connection });
    return;
  }
  if (event.kind === "job.landed") {
    // **`job.step_advanced`'s shape, and for its reason.** The row travels
    // whole with `landed` already on it, so the board redraws without a
    // round trip — and the status has not moved, so there is nothing to fold
    // by hand. The detail is re-read because the pull request's state is
    // also a field on `delivery`, which the open Job draws.
    host.publish({ connection });
    host.fold(event.job);
    host.questions.moved(event.job);
    host.refresh(fleet.port, event.job.id);
    return;
  }
  if (event.kind === "job.remarks_changed") {
    // Moves no row, `job.files_changed`'s terms — `review.ts` owns whether
    // anybody is looking, and this only wakes that read where they are.
    host.publish({ connection });
    void host.material.remarksChanged(fleet.port, event.job_id);
    return;
  }
  if (event.kind === "job.forgotten") {
    // The opposite of `job.created`: the id, and nothing to fold — the row
    // is gone at Fleet by the time this arrives, so it is dropped here
    // rather than replaced. Covers a forget made from another window, or a
    // window that raced the event past its own call's answer.
    host.publish({ connection });
    host.forget(event.job_id);
    host.questions.forgotten(event.job_id);
    return;
  }
  if (event.kind === "manifest.reread") {
    // **Above the tail below, because there is no Job to find.** The tail
    // reads `event.job_id` and treats a Job it does not hold as a missed
    // message, so falling through to it would make every save Bridge sees
    // trigger a full re-read of the board.
    //
    // The whole reading replaces what is held rather than merging into it.
    // A read that took after one that was refused leaves nothing of the
    // refusal standing, and a merge would keep the old fault on screen
    // beside the news that the file is now fine.
    // Overview's drift spans every repository on All, so it reads whichever file this was.
    void host.overviewAgain(fleet.port);
    host.publish({ connection });
    // Every window whose own pick reads this file draws it, the Manifest reading and drift both.
    host.repositories.manifestReread(event);
    host.rehearsal.onManifestReread(event.path, fleet.port);
    return;
  }
  if (event.kind === "repositories.changed") {
    // Carried whole, so an add made in another window or at the CLI lands without a round trip.
    host.publish({ connection });
    return void host.repositories.listed(event, fleet.port);
  }
  if (event.kind === "run.finished") {
    host.publish({ connection });
    host.rehearsal.onRunFinished(event.job_id, fleet.port);
    return;
  }
  if (event.kind === "checkout_run.finished") {
    // **Above the tail below, because there is no Job to find**, which is
    // `manifest.reread`'s reason on this same switch: the tail reads
    // `event.job_id` and treats a Job it does not hold as a missed message,
    // so falling through would make every run in the checkout trigger a full
    // re-read of the board.
    host.publish({ connection });
    host.rehearsal.onCheckoutRunFinished(fleet.port);
    return;
  }
  if (
    event.kind === "server.starting" ||
    event.kind === "server.serving" ||
    event.kind === "server.exited"
  ) {
    // Replaced, never patched: the event carries the whole `ServerState`. The list alone moved no
    // sheet, so Run and Stop on a server drew nothing until a reload.
    const row: ServerState = event;
    const servers = host.rehearsal.onServerEvent(host.current().servers.servers, row);
    host.publish({ connection, servers: { servers } });
    host.rehearsal.onServerMoved(row, fleet.port);
    return;
  }

  if (event.kind !== "job.state_changed") {
    // A newer Fleet's kind, or one that moves no row: never folded as a move.
    host.publish({ connection });
    return;
  }
  const held = host.current().jobs.find((job) => job.id === event.job_id);
  if (held === undefined) {
    // `job.created` covers the ordinary case, so a move about a Job this
    // window has never seen means a message was missed.
    host.publish({ connection });
    void reread(fleet.port, host.publish, host.now);
    return;
  }
  const moved: JobSummary = { ...held, status: event.to, reason: event.reason };
  host.publish({
    connection,
    jobs: host.current().jobs.map((job) => (job.id === moved.id ? moved : job)),
    readAt: host.now(),
  });
  host.refresh(fleet.port, moved.id);
  // A Judge refusal is carried by no event, so a Job held for review has its detail read for one.
  host.questions.moved(moved);
  if (moved.status === "awaiting_review") void host.questions.read(fleet.port, moved.id);
  // **A status move is the only thing that changes the occupancy**, so this
  // is where the reading is taken rather than on a timer. The machine half
  // rides along on the same call, which means a disk that fills while nothing
  // moves is not noticed until something does — and something moving is the
  // moment it starts mattering, because that is when admission next asks.
  void readCapacity(fleet.port, host.publish);
}
