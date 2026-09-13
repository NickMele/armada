// What a surface that needs one repository draws on All repositories: the question, and the
// rail's own options. Picking answers it by picking in the rail, so one pick is all there is.

import type { RepositorySummary } from "@armada/protocol";
import { Alert, Select } from "@armada/components";
import { RepositoryOptions } from "@armada/shell";

/** No root is empty, so this value is never a repository. */
const UNANSWERED = "";

export function AskRepository({
  repositories,
  title,
  next,
  onPick,
}: {
  repositories: readonly RepositorySummary[];
  /** What the surface needs one repository for. Sentence case, no Wh- opener. */
  title: string;
  /** What picking does. */
  next: string;
  /** A root Fleet lists. */
  onPick: (root: string) => void;
}) {
  return (
    <Alert tone="neutral" title={title}>
      <p>{next}</p>
      <Select
        label="Repository"
        value={UNANSWERED}
        onChange={(event) => {
          if (event.target.value !== UNANSWERED) onPick(event.target.value);
        }}
      >
        <option value={UNANSWERED} disabled>
          Choose a repository
        </option>
        <RepositoryOptions repositories={repositories} />
      </Select>
    </Alert>
  );
}
