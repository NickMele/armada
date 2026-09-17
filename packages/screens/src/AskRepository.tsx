// What a surface that needs one repository draws on All repositories: the question, and the
// rail's own options. Picking answers it by picking in the rail, so one pick is all there is.
//
// **Two callers, one gate between them.** The Manifest surface's own ask picks a repository
// nobody has set up yet to open Setup on it — `RepositoryOptions` is built for exactly that. New
// job cannot: a Job needs a Manifest, and #959 decided that picking one here must not route
// anywhere or narrow the Board, so `onlySetUp` shows it greyed out and unpickable instead.

import type { ReactNode } from "react";
import type { RepositorySummary } from "@armada/protocol";
import { Alert, Select } from "@armada/components";
import { repositoryLabel, RepositoryOptions } from "@armada/shell";

/** No root is empty, so this value is never a repository. */
const UNANSWERED = "";

export function AskRepository({
  repositories,
  title,
  next,
  onPick,
  onlySetUp = false,
  action,
}: {
  repositories: readonly RepositorySummary[];
  /** What the surface needs one repository for. Sentence case, no Wh- opener. */
  title: string;
  /** What picking does. */
  next: string;
  /** A root Fleet lists. */
  onPick: (root: string) => void;
  /**
   * New job's own use — #959: a repository nobody has set up yet has no
   * Manifest for the Job to belong to, so it is shown rather than hidden, and
   * cannot be picked. `false`, the default, is the Manifest surface's own
   * ask, which still picks one to open Setup on it.
   */
  onlySetUp?: boolean;
  /**
   * The one control the ask carries at its trailing edge — the composer's own
   * way out, which has no card header to sit in here. Absent draws none.
   */
  action?: ReactNode;
}) {
  const loose = repositories.filter((one) => one.manifest === undefined);
  return (
    <Alert tone="neutral" title={title} action={action}>
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
        {onlySetUp ? (
          <>
            {repositories
              .filter((one) => one.manifest !== undefined)
              .map((repository) => (
                <option key={repository.root} value={repository.root} title={repository.root}>
                  {repositoryLabel(repository, repositories)}
                </option>
              ))}
            {loose.length === 0 ? null : (
              <optgroup label="Not set up">
                {loose.map((repository) => (
                  <option key={repository.root} value={repository.root} title={repository.root} disabled>
                    {repositoryLabel(repository, repositories)} — Set it up first
                  </option>
                ))}
              </optgroup>
            )}
          </>
        ) : (
          <RepositoryOptions repositories={repositories} />
        )}
      </Select>
    </Alert>
  );
}
