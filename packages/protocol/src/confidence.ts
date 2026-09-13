/**
 * Armada's review of a change, as a person reads it on the job. Since 13.22, #903.
 * Not `review`: `JobDetail.review` is the text Fleet composes for the pull request.
 * `crates/ipc/src/detail/confidence.rs`.
 */

export type Says = "confident" | "not_confident";

export type JobConfidence = {
  says: Says;
  reasons: string[];
  areas: AreaRow[];
  /** Absent where the change touches no test and the reviewer named nothing untested. */
  tests?: TestsSection;
  needs_you: FindingRow[];
  small_fixes: FindingRow[];
  for_context: FindingRow[];
};

export type AreaRow = {
  name: string;
  what: string;
  files: string[];
};

/** Tests in the change. It opens itself where a test was removed or loosened with no reason. */
export type TestsSection = {
  opened_because?: OpenedBecause;
  proves: ProvesRow[];
  changed: ChangedTestRow[];
  untested: UntestedRow[];
};

/** Why a section opened itself, naming what to look at. */
export type OpenedBecause =
  | { kind: "test_removed"; name: string }
  | { kind: "test_loosened"; name: string };

export type ProvesRow = {
  area: string;
  what: string;
  tests: number;
};

/** A test the change removed or loosened. `flagged` where it was given no reason. */
export type ChangedTestRow = {
  name: string;
  change: string;
  replaced_by?: string;
  why?: string;
  flagged: boolean;
};

export type UntestedRow = {
  code: string;
  why: string;
};

export type FindingRow = {
  finding: string;
  why: string;
};
