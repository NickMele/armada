// A Studio on the wire: the list, one Studio with its graph, and the acts a
// client asks of one. `crates/ipc/src/studio.rs`. Since 14.6, #1285; a scout's
// Finding and its three acts since 14.7, #1292, `crates/ipc/src/scouting.rs`.
//
// Hand-written, `protocol.ts`'s rules. **One exception to leaving a closed set
// as `string`**: a node's `kind` is the tag its fields hang off, so the content
// is a union on it. Its state, an edge's kind and its standing stay `string`.

import type { CheckoutRunUnderway } from "./rehearsal";

/** Every Studio one repository keeps, the last touched first — `GET /studios`. */
export type StudioList = {
  studios: StudioSummary[];
};

/** One Studio as the list names it. Names no Workspace. */
export type StudioSummary = {
  id: string;
  manifest_id: string;
  /** Absent on a Studio nobody has named yet. */
  name?: string;
  created_at: string;
  touched_at: string;
};

/**
 * One Studio and its whole graph — `GET /studios/:studio_id`, the answer to
 * every write on one, and the body of `studio.changed`. A client replaces the
 * Studio it holds with it.
 */
export type Studio = StudioSummary & {
  /** `person` or `helm`. Absent when untitled, or named before this was kept. Since 14.7. */
  named_by?: string;
  /** Oldest first. */
  nodes: StudioNode[];
  /** Oldest first. */
  edges: StudioEdge[];
};

/** What a node holds, tagged by its kind. `docs/concepts/studio.md`, Nodes. */
export type StudioNodeContent =
  /**
   * A reference to the run, never its status. `kept` is absent while the run
   * is still there to read; present is partial — the run has been swept and
   * this is all there is. Since 14.10, #1289.
   */
  | { kind: "run"; run_id: string; kept?: StudioRunKept }
  /** What a person pointed at and said, fixed at capture. `capture` since 14.11, #1290. */
  | { kind: "note"; said: string; capture?: StudioCapture }
  | { kind: "cluster"; title: string }
  /** What a scout was asked, and from its start what it read. Since 14.7. */
  | ({ kind: "finding" } & StudioFinding)
  | { kind: "contradiction"; first: string; second: string }
  | { kind: "sketch"; body: string }
  | { kind: "link"; address: string }
  | { kind: "deferral"; what: string }
  | { kind: "outline"; body: string }
  | { kind: "issue_draft"; title: string; body: string }
  /** A reference to the Job, never its status. */
  | { kind: "job"; job_id: string };

/**
 * What a Run node kept of its run once retention swept it: the result, and the
 * log's last lines. `exit_code`, `expect_exit_code` and `stopped` are the three
 * a run's colour is derived from, so a swept run reads the same as it did while
 * it ran. Since 14.10, #1289.
 */
export type StudioRunKept = {
  name: string;
  command: string;
  /** Absent where the run was killed before it exited. */
  exit_code?: number;
  expect_exit_code: number;
  stopped: boolean;
  duration_ms: number;
  /** The log's last lines, oldest first. */
  lines: string[];
  /** How many lines the log held in all, whether or not they are here. */
  total_lines: number;
  /** Whether `lines` is the whole log rather than its tail. */
  whole: boolean;
};

/** Where a person left a node, in whole canvas units. */
export type StudioPosition = { x: number; y: number };

export type StudioNode = StudioNodeContent & {
  id: string;
  /**
   * Absent on a kind with no states, and always on a Run or a Job: their state
   * is read off the run or the Job.
   */
  state?: string;
  position: StudioPosition;
  created_at: string;
  /** `person` or `helm`. Absent only on a node added before it was kept. Since 14.7. */
  added_by?: string;
};

/** One edge. `proposed` is drawn dashed until a person accepts it. */
export type StudioEdge = {
  id: string;
  from: string;
  to: string;
  /** `produced`, `same_as`, `blocks` or `answers`. */
  kind: string;
  /** `proposed` or `accepted`. */
  standing: string;
  created_at: string;
  /** `person` or `helm`. Absent only on an edge kept before it was. Since 14.7. */
  added_by?: string;
};

/** A Studio that is gone — `delete_studio`'s answer and `studio.deleted`'s body. */
export type StudioDeleted = {
  id: string;
  manifest_id: string;
};

/**
 * `studio.helm_acted`'s body: one act Helm took on a Studio, published after
 * the write's `studio.changed`. A person's act publishes `studio.changed`
 * alone, so the two are told apart by kind. Since 14.7, #1288.
 */
export type StudioHelmActed = HelmStudioAct & {
  studio_id: string;
  manifest_id: string;
  at: string;
};

/** Which of Helm's unasked acts, with the id of what it added or the name given. */
export type HelmStudioAct =
  | { act: "added_node"; node_id: string }
  | { act: "proposed_edge"; edge_id: string }
  | { act: "named"; name: string };

/** `POST /studios/create?manifest_id=`. A blank or absent name is untitled. */
export type CreateStudio = { name?: string };

/** `POST /studios/:studio_id/rename`. A blank name is refused. */
export type RenameStudio = { name: string };

/**
 * `POST /studios/:studio_id/add_node`. A node starts in its kind's first state.
 * `produced_by` names the node that made this one, and the Studio draws the edge.
 */
export type AddStudioNode = StudioNodeContent & {
  position: StudioPosition;
  produced_by?: string;
};

/**
 * What a Note keeps of where a person pointed — the development annotation
 * layer's own fields (`apps/desktop/src/shared/annotations.ts`) plus the four
 * it does not record. Since 14.11, #1290, `crates/ipc/src/capturing.rs`.
 */
export type StudioCapture = {
  /** The innermost React component under the press, where one was found. */
  component?: string;
  /** The components above it, nearest first — the parent chain. */
  owners?: string[];
  selector: string;
  element: { tag: string; text: string; label?: string };
  screen?: string;
  layer?: string;
  location: string;
  /** Where the element sat in the window, in CSS pixels. */
  bounds: { x: number; y: number; width: number; height: number };
  window: { width: number; height: number };
  /** `getComputedStyle`, for the properties Bridge declares it reads. */
  styles?: Record<string, string>;
  markup: string;
  /**
   * **Absent unless the build exposes one.** React 19 fibers carry no
   * `_debugSource`, so no path is guessed for one.
   */
  source?: string;
  /** The frame Fleet kept beside the Studio's records. */
  frame?: { filename: string; byte_size: number; width: number; height: number };
};

/** The PNG Bridge took, written to disk before the request. Never read back. */
export type StagedFrame = { staged_path: string; width: number; height: number };

/**
 * `POST /studios/:studio_id/capture_note`. A person's act and no agent's:
 * pointing at something wrong needs a pointer.
 */
export type CaptureStudioNote = {
  said: string;
  /** Where they pointed, without the frame — Fleet fills that in from `frame`. */
  capture: StudioCapture;
  position: StudioPosition;
  frame?: StagedFrame;
  produced_by?: string;
};

/** `POST /studios/:studio_id/move_node`. Position only. */
export type MoveStudioNode = { node_id: string; position: StudioPosition };

/** `POST /studios/:studio_id/remove_node`. Takes the node's edges with it. */
export type RemoveStudioNode = { node_id: string };

/**
 * `POST /studios/:studio_id/propose_edge`. Lands `proposed`. `produced` does not
 * decode: the Studio draws that edge itself.
 */
export type ProposeStudioEdge = {
  from: string;
  to: string;
  kind: "same_as" | "blocks" | "answers";
};

/** `POST /studios/:studio_id/decide_edge`. A rejected edge is removed. */
export type DecideStudioEdge = { edge_id: string; accepted: boolean };

/**
 * A Finding. `asked` alone while Proposed; `checkout` from the scout's start;
 * `ended` once it stops, however it stopped. Since 14.7.
 */
export type StudioFinding = {
  asked: string;
  /** The commit read, and whether anything uncommitted sat on top of it. */
  checkout?: ScoutCheckout;
  /** Every file read, relative to the checkout, in the order first read — a file a search returned lines of included. */
  read?: string[];
  /** Every search run, as its pattern and where it looked. */
  searched?: string[];
  /** What the scout said last. */
  learned?: string;
  ended?: ScoutEnded;
};

export type ScoutCheckout = { commit: string; uncommitted: boolean };

/**
 * How a scout ended. `cost_micros` is millionths of a dollar, **absent where
 * the agent reported none** — never nought in its place.
 */
export type ScoutEnded = (
  | { outcome: "answered" }
  | { outcome: "stopped" }
  | { outcome: "failed"; why: string }
) & { cost_micros?: number };

/**
 * `POST /studios/:studio_id/ask_scout`. A person's ask: a Finding is added
 * Gathering and its scout starts. Bridge only.
 */
export type AskScout = {
  asked: string;
  position: StudioPosition;
  produced_by?: string;
};

/** `POST /studios/:studio_id/start_scout`. Starts a Proposed Finding. */
export type StartScout = { node_id: string };

/**
 * `POST /studios/:studio_id/stop_scout`. Answers the Studio before the Finding
 * freezes; the frozen one arrives on `studio.changed`.
 */
export type StopScout = { node_id: string };

/**
 * `POST /studios/:studio_id/start_run`. Runs one Manifest entry in the
 * checkout of the repository this Studio belongs to, so it names no
 * repository. `produced_by` names the node it was started from, and the Studio
 * draws the `produced` edge. Since 14.10, #1289.
 */
export type StartStudioRun = {
  name: string;
  /** A directory below the root whose own file declares `name`. */
  workspace?: string;
  position: StudioPosition;
  produced_by?: string;
};

/** `start_studio_run`'s answer, 202: the Studio whole, the node it made, and
 * the run underway. Since 14.10, #1289. */
export type StudioRunStarted = {
  studio: Studio;
  node_id: string;
  run: CheckoutRunUnderway;
};
