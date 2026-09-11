// What one Job's screen is read from, and which of those reads each way of
// coming back takes again.
//
// Beside `connection.ts` rather than inside it, for the reason `command.ts`,
// `reader.ts` and `review.ts` are: the connection is a socket, a runtime file
// and a state machine, and a classification of reads is none of those. Nothing
// is held here — every read and every socket is handed in, so this cannot drift
// from what the connection believes it has open.
//
// # Why the list is one list
//
// #472 was the open Job's detail being re-read when Fleet came back and what
// the Job holds on this machine not being. Two reads, one screen, one region
// each, and only one of them in the path — so the screen half-recovered and the
// half that did not was the panel reporting the outage, which went on saying
// Fleet was not answering until somebody pressed Refresh.
//
// Each occasion used to name its own reads at its own call site. That is the
// shape of the defect rather than an instance of it: a third read handled a
// fourth way is the same bug in a month. So the occasions are a type, the reads
// are one list, and adding a region to a Job's screen means adding a row here.
//
// # And not everything, every time
//
// A reconnection that refetches every open surface is its own problem, and the
// patch is where it bites — `crates/ipc/src/work.rs` splits the diff off
// `get_job` precisely because the bytes are large. So the reads are classified
// by what keeps each current, and what fires is what somebody has open: `again`
// on a read holding no Job is a no-op, and a socket nobody opened is not
// opened.

/**
 * One per-Job read: which Job it is holding, and how to take it again.
 *
 * Structural rather than `JobReader<T>` so this file needs none of the four
 * shapes the reads answer with. What it classifies is when a read is taken,
 * which is the same question whatever comes back.
 */
type Region = {
  readonly jobId: string | null;
  again(port: number): Promise<void>;
};

/** One per-Job socket: whether it is up, and how to open it. */
type Channel = {
  attached(): boolean;
  open(port: number, jobId: string): void;
};

/** Both halves of the review, which are the reads no event ever takes again. */
type Reviewed = {
  reread(port: number): Promise<void>;
  repair(port: number): Promise<void>;
};

/**
 * Every region of one Job's screen, handed in as the connection holds it.
 *
 * | What a Job's screen draws | What keeps it current | Taken again |
 * |---|---|---|
 * | The Job whole, `detail` | Every event naming it | Every occasion |
 * | What it holds, `resources` | Every event naming it | Every occasion |
 * | Its transition history | Every event naming it, where unfolded | Every occasion |
 * | Its turns, and its own log | Their own sockets | Reopened where down |
 * | What its Drones claimed, and the patch | A press, and nothing else | On a reconnection where it is showing a failure; on Refresh always |
 *
 * **Three per-Job things are deliberately absent, and each is absent for its
 * own reason.** The footprint is pushed by `job.files_changed` and has no route
 * to read, so it comes back on the next event and a resync cannot rebuild it. A
 * look's verdict is the answer to a press that writes a line in the Job's own
 * log, and repeating an act nobody asked for is not a recovery. A recorded
 * call's arguments answer the caller and are never held, so nothing can go
 * stale. A region added to a Job's screen belongs on the table or in this
 * paragraph.
 *
 * **The comments half of the row above has two triggers the table does not
 * carry.** `job.remarks_changed` (`#661`) re-takes the remarks alone, where
 * they are the ones open — `connection.ts` calls
 * `ReviewMaterial.remarksChanged` straight off that event rather than
 * routing it through an `Again` occasion here. `remarks-poll.ts`'s 20 s
 * timer (`#667`) calls the same method from `index.ts`, for as long as one
 * Job's panel is on screen. Both are narrower than every occasion above:
 * those classify a whole screen's worth of reads by what caused the moment,
 * and each of these is one route woken by one Job's pull request, which the
 * classification this file exists for has nothing to add to.
 */
export type Screen = {
  detail: Region;
  resources: Region;
  history: Region;
  turns: Channel;
  notes: Channel;
  /** The Job whose turns are open, and whose log is. `null` is neither. */
  observing: string | null;
  reading: string | null;
  review: Reviewed;
};

/**
 * Why the open Job's reads are being taken again. **Four occasions, and what
 * separates them is which reads have gone stale — not how urgent it is.**
 */
export type Again =
  /**
   * An event named a Job. **Only that Job's reads**, because the stream carries
   * every Job and nothing else on any screen moved.
   */
  | { because: "job_moved"; jobId: string }
  /**
   * The stream dropped events under a connection that held. Everything events
   * keep current, for whatever Job is open — HTTP never stopped answering, so
   * the reads no event feeds are still good.
   */
  | { because: "stream_gap" }
  /**
   * A socket that was down is up. Everything events keep current, **and the
   * reads nothing else takes again where they are showing a failure** — every
   * read attempted while Fleet was unreachable failed, and a surface holding
   * one of those failures has no other way back.
   */
  | { because: "fleet_came_back" }
  /**
   * Somebody pressed Refresh. Everything, **including both halves of the review
   * whether or not either is stuck** — that is the one thing a press adds to a
   * reconnection, and it adds it because a person asking has said the bytes are
   * worth it.
   */
  | { because: "a_person_asked" };

/**
 * Every read one Job's screen is drawn from, taken again together.
 *
 * **Together, not in sequence**: these are the regions of one screen, and a
 * person watching them repair one at a time is watching #472 from the other
 * side.
 */
export async function takeAgain(port: number, again: Again, screen: Screen): Promise<void> {
  const only = again.because === "job_moved" ? again.jobId : null;
  /** Whether a read or a socket held for this Job is one this occasion takes. */
  const mine = (jobId: string | null): jobId is string =>
    jobId !== null && (only === null || jobId === only);

  // **A step advancing is a new Drone, and Fleet's socket ended with the last
  // one.** So the event that says this Job moved is what reopens the transcript
  // — there is no timer here on purpose: reopening resets the rows and
  // republishes `opening`, so a loop would blank the log on every tick. Only
  // where the socket is down; reopening a working one restarts the transcript
  // from the top, and a resync arrives after every dropped event too. #324.
  const observing = screen.observing;
  if (mine(observing) && !screen.turns.attached()) screen.turns.open(port, observing);
  // The log socket does not end when a step advances — nothing about a Job's
  // own log is per-Drone — so this only catches one that closed because Fleet
  // could not read the file, on the next occasion that reaches here.
  const reading = screen.reading;
  if (mine(reading) && !screen.notes.attached()) screen.notes.open(port, reading);

  // `review.ts` owns why a reconnection takes only what is stuck and a press
  // takes both. The short of it is the megabyte.
  const review =
    again.because === "a_person_asked"
      ? screen.review.reread(port)
      : again.because === "fleet_came_back"
        ? screen.review.repair(port)
        : undefined;

  await Promise.all([
    // A resync says nothing about the open Job's steps, and neither does an
    // event carrying only a Board row.
    mine(screen.detail.jobId) ? screen.detail.again(port) : undefined,
    // The machine reading moves with the Job rather than on a clock of its own:
    // a poll would pay a process table per tick.
    mine(screen.resources.jobId) ? screen.resources.again(port) : undefined,
    // A history that is unfolded grows as the Job moves, so the move that was
    // just delivered is read back rather than left off the end of the list.
    mine(screen.history.jobId) ? screen.history.again(port) : undefined,
    review,
  ]);
}
