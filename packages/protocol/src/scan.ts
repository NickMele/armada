// What Scan found in a repository nobody set up for Armada.
// `crates/ipc/src/scan.rs`, served at `GET /repository/scan`.
//
// **Evidence, never a proposal.** Every value is copied out of a file the
// repository already had and carries that file, because every line of a
// proposal built on it cites one. What was not read is said beside it — a tool
// Scan has never heard of reads as not followed, never as clean.
//
// The header rules in `protocol.ts` hold here: hand-written, and every closed
// set is left as `string`.

/** One read-only pass over a checkout, every workspace at once. */
export type RepositoryScan = {
  /** The directory that was read, as Fleet resolved it. */
  checkout: string;
  /** Every workspace, the root (`.`) first and always present. */
  workspaces: ScannedWorkspace[];
  /**
   * What the repository's CI jobs run, each with its file and job. Evidence,
   * not a Check. Absent from a Fleet that predates it.
   */
  ci_commands?: CiCommand[];
  /**
   * What belongs to no workspace and was not read, and why. **Never evidence
   * of absence** — a checkout that would not list says so here rather than
   * answering with nothing.
   */
  not_read: NotRead[];
};

/** One directory that can hold a Manifest of its own. */
export type ScannedWorkspace = {
  /** Relative to the checkout, `.` for the root. */
  dir: string;
  /** The workspace-pattern entries naming it; empty where it was walked to. */
  declared_by: WorkspaceGlob[];
  /**
   * What the picker ticks by: `strong` where a file names something runnable,
   * `thin` where files were read and none does, `not_followed` where nothing
   * here could be read. Only `strong` is ticked by default.
   */
  evidence: string;
  manifests: ToolFile[];
  /** Cited where they sit — a shared lockfile is on the root only. */
  lockfiles: ToolFile[];
  /** Scripts and aliases, by name. Where a file writes them, not what gates code. */
  runnables: Runnable[];
  tools: ToolSection[];
  services: ComposeService[];
  /** Empty is the ordinary answer: a port is evidence only where a file declares one. */
  ports: DeclaredPort[];
  /**
   * A runnable name every strong sibling declares and this one does not —
   * computed over the batch the picker ticks by default. Never every absence,
   * and always empty on the root, which is not a sibling.
   */
  missing: MissingName[];
  not_read: NotRead[];
};

export type WorkspaceGlob = { file: string; entry: string };

/** A file and the tool it belongs to. `tool` is rendered, never matched on. */
export type ToolFile = { file: string; tool: string };

/** `key` is where in the file — `scripts.test`, `alias.xtask`. */
export type Runnable = { file: string; key: string; name: string; run: string };

export type ToolSection = { file: string; key: string; tool: string };

export type ComposeService = { file: string; key: string; name: string };

/** `container` is the port inside the container, or the one a process listens on. */
export type DeclaredPort = { file: string; key: string; name: string; container: number };

export type MissingName = { name: string; declared_in: string[] };

/**
 * `key` is where in the file — `jobs.test.steps[2].run`. `cell` names the one
 * matrix cell it was read as, never one finding per cell; rendered, never matched on.
 */
export type CiCommand = { file: string; job: string; key: string; run: string; cell?: string };

/** `why` is rendered, never matched on. */
export type NotRead = { file: string; why: string };
