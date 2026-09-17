// A Studio on the wire: the list, one Studio with its graph, and the acts a
// client asks of one. `crates/ipc/src/studio.rs`. Since 14.6, #1285.
//
// Hand-written, `protocol.ts`'s rules. **One exception to leaving a closed set
// as `string`**: a node's `kind` is the tag its fields hang off, so the content
// is a union on it. Its state, an edge's kind and its standing stay `string`.

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
  /** Oldest first. */
  nodes: StudioNode[];
  /** Oldest first. */
  edges: StudioEdge[];
};

/** What a node holds, tagged by its kind. `docs/concepts/studio.md`, Nodes. */
export type StudioNodeContent =
  /** A reference to the run, never its status. */
  | { kind: "run"; run_id: string }
  /** What a person pointed at and said, fixed at capture. */
  | { kind: "note"; said: string }
  | { kind: "cluster"; title: string }
  | { kind: "finding"; asked: string }
  | { kind: "contradiction"; first: string; second: string }
  | { kind: "sketch"; body: string }
  | { kind: "link"; address: string }
  | { kind: "deferral"; what: string }
  | { kind: "outline"; body: string }
  | { kind: "issue_draft"; title: string; body: string }
  /** A reference to the Job, never its status. */
  | { kind: "job"; job_id: string };

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
