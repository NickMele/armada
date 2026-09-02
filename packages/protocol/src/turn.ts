// One Drone's turns, as TypeScript sees them. `crates/ipc/src/turn.rs`.
//
// Its own file for the reason the Rust side gives it one: this is the only
// query whose transport is a socket, and it is read-only all the way down —
// nothing here has a request half, and nothing here reaches a Drone.
//
// Hand-written like `protocol.ts`, and a second statement of the Rust shapes
// for the same reason: the codegen that would emit both does not exist yet.

import type { ChangedFile, CheckRun, Missed } from "./protocol";
import type { ProtocolVersion } from "./version";

/** One message on a Job's Observe socket. `crates/ipc/src/turn.rs`. */
export type TurnMessage =
  | ({ message: "opened" } & Opening)
  | ({ message: "row"; ts: string; step?: string; by?: Voice } & Saw)
  | ({ message: "missed" } & Missed)
  | ({ message: "closed" } & Closed);

/**
 * The first message on every connection, before any row.
 *
 * **`Opening`, not `Opened`** — `artifacts.ts` has an `Opened` and it is the
 * one every caller outside this file means. Two types of one name in a package
 * every other package imports whole is an ambiguity nobody can resolve at the
 * call site.
 */
export type Opening = {
  protocol_version: ProtocolVersion;
  job_id: string;
  /** Whether a Drone was writing when this opened. `false` is ordinary. */
  live: boolean;
  /** Older rows the bounded backfill left out. Never a silent truncation. */
  skipped: number;
};

/**
 * Who a row is. The three actors a step's story has.
 *
 * A step is a conversation: Armada opens it with an instruction, the Drone
 * works, and Fleet runs the Checks and reads what came out. Only the middle one
 * used to be written down, so the activity log could say what the Drone did and
 * nothing about what it had been asked or what was made of it.
 *
 * **Absent is `drone`.** Every row written before Fleet stamped this field
 * decoded from a Drone's own output, so an older row read back without it is
 * read back correctly.
 */
export type Voice = "armada" | "drone" | "fleet";

/** Nothing more is coming, and why. A socket that simply stops says nothing. */
export type Closed = {
  /** `drone_ended` or `nothing_writing`. */
  because: string;
};

/**
 * One row of a Drone's transcript, as a viewer is shown it.
 *
 * The step a row was written under travels beside this rather than inside it,
 * as `step` on the row message: it is true of every kind, and it is optional
 * because a row written before Fleet recorded one carries no step and nothing
 * can recover which it was.
 *
 * The tag is `event` and not `kind`, because `unrecognised` already carries a
 * `kind`. Two of the file's kinds never arrive here — `quota_moved`, which is
 * dispatch gating rather than this Job's business, and the sink's own
 * `missed` — so no case is written for them.
 *
 * **`ended` was a third, and the reason given for it was false.** It was
 * withheld because the Job's rail was said to state a run's cost and turn
 * count; no rail ever did, and nothing else on the wire carries either. It
 * arrives since protocol 4.10, and `turns.ts` is where it is drawn.
 */
export type Saw =
  | { event: "started"; session: string; model: string; mcp_servers: number }
  /**
   * The Drone reached for a tool, and what it reached for it with.
   *
   * **Both fields always arrive.** `crates/ipc/src/turn.rs` declares `detail`
   * and `truncated` on every `Saw::Called`, and the Fleet that predates them is
   * a Fleet behind, which Bridge refuses rather than reads. So neither is
   * optional here, and **empty is a value rather than an absence**: it means
   * the vocabulary had no name for that tool's arguments, which is what the
   * pane falls back to the call id for.
   *
   * The detail is bounded and may be cut — a `Write` argument is a whole file —
   * and `truncated` is how a row says so, because a command can legitimately
   * end in an ellipsis.
   *
   * **`whole` is on the Rust variant and is not here.** `Shown` drops it before
   * the socket, so no viewer ever receives it; declaring it would be a field a
   * surface could read and never find. The rest comes back over HTTP, once, for
   * the one call somebody opened — `CallArguments` below.
   */
  | {
      event: "called";
      tool: string;
      call: string;
      detail: string;
      truncated: boolean;
      /**
       * How many characters the argument had, before anything was cut.
       *
       * **What `truncated` on its own could not say.** The flag reports an
       * absence and a size reports a proportion, so an opened row reads
       * *showing 200 of 14,320 characters* instead of counting the string it
       * was handed — which is the cut length, and would call every truncated
       * row complete.
       *
       * **Absent is a row written before Fleet stamped this field**, whose true
       * size nothing recovers — `step` and `by` carry the same absence for the
       * same reason. Never an argument measured at nought, and never a reason
       * to withhold the fetch: such a row says nothing about size and still
       * offers the rest.
       */
      detail_length?: number;
    }
  | { event: "answered"; call: string; failed: boolean }
  | { event: "said"; text: string }
  | { event: "refused"; tool: string; call: string; because: string }
  /**
   * The last row a Drone writes: what the run cost, how many turns it took,
   * and how many of its calls the harness refused.
   *
   * **A run's own total and never a Job's.** A Job that retried has one of
   * these per Drone, and nothing here adds them up — a Job-wide figure is a
   * question somebody decides rather than a number the wire carries.
   *
   * `cost_micros` is millionths of a dollar, an integer because a budget that
   * is compared and accumulated as a float is a budget that drifts.
   */
  | { event: "ended"; turns: number; cost_micros: number; refusals: number }
  /**
   * A turn Armada put into the Drone's session, whole — chapter one of a
   * step's story. `occasion` is spelled as the constructor that built it:
   * `opening`, `outcome`, `redirect`, `answer`, `drift`, `report`, `poke`.
   *
   * The text is not bounded. It is Fleet's own rendering of a template it
   * holds, so nothing here is a size a Drone chose.
   *
   * `headings` names which lines of `text` its writer wrote as block headings,
   * zero-based into `text.split("\n")`. **It is the one fact about the turn's
   * shape a reader cannot recover**: `fleet::briefing` writes every block as a
   * heading, a blank line, then the body, and a surface handed the text alone
   * can only guess — the first line of a block, or a line in capitals. Both
   * guesses are wrong on briefs Fleet already writes, so a short body line
   * would draw as a heading and nothing would catch it.
   *
   * **Absent is a turn with no headed blocks, and also a row written before
   * Fleet stamped this field** — deliberately the same to a reader, because
   * what an older row draws as is what an unheaded turn draws as. Every turn
   * but the opening brief is one block of prose.
   */
  | { event: "instructed"; occasion: string; text: string; headings?: number[] }
  /**
   * One declared Check, as Fleet ran it. **The Drone never runs these** — a
   * Drone reporting its own tests is a claim rather than a result — so a Check
   * appears nowhere in a Drone's own output.
   */
  | { event: "checked"; run: CheckRun }
  /**
   * What the step's work came to, read at the step boundary. The only per-step
   * reading of what a Drone wrote there is: `job.files_changed` has no
   * boundary attached and a footprint is the whole Job's.
   */
  | { event: "produced"; files: ChangedFile[] }
  | { event: "unrecognised"; kind: string }
  | { event: "unreadable"; line: string; why: string };

/**
 * One tool call's arguments, as `GET /jobs/:job_id/calls/:call_id` answered.
 * `crates/ipc/src/turn.rs`.
 *
 * **The other half of `Saw.called`, and the reason a cut row is not a dead
 * end.** The socket carries a line and a size; this carries the argument. It is
 * asked for once, by the person who opened one row — the split `get_diff`
 * already makes against `job.files_changed`, for the same reason: the stream is
 * bounded and lossy by design, and a row big enough to evict its neighbours
 * loses the short form too.
 *
 * It is here rather than in `protocol.ts` because it is the other end of a row,
 * and a row is this file's subject. Nothing here has a request half either.
 */
export type CallArguments = {
  /** The tool, as its own vocabulary spells it. */
  tool: string;
  /** The call id the row carried, which is what was asked for. */
  call: string;
  /**
   * The argument as the Drone sent it — whitespace and newlines intact, because
   * a heredoc read as one line is not the thing that was run. It goes into the
   * same pre block the payload already draws.
   */
  arguments: string;
  /**
   * Whether `arguments` is all of it. **False only where the record itself is
   * short**: a row written before the file kept the whole carries the bounded
   * line and nothing behind it.
   *
   * Stated rather than inferred from the two lengths agreeing, because a
   * surface that inferred it would call a partial answer complete on every row
   * where the count was also missing.
   */
  whole: boolean;
  /**
   * How many characters the argument had, where the record knows. Absent is an
   * old row again, not an argument of no length — a surface holding
   * `whole: false` and no length has what there is and no way to say how much
   * is missing, and says nothing about size rather than inventing one.
   */
  length?: number;
};
