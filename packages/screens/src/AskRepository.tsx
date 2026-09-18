// What a surface that needs one repository draws on All repositories: the question, and the
// rail's own options. Picking answers it by picking in the rail, so one pick is all there is.
//
// **Two callers, one gate between them.** The Manifest surface's own ask picks a repository
// nobody has set up yet to open Setup on it — `RepositoryOptions` is built for exactly that. New
// job cannot: a Job needs a Manifest, and #959 decided that picking one here must not route
// anywhere or narrow the Board, so `onlySetUp` shows it greyed out and unpickable instead.
//
// **`action` is the composer's alone, and a rail surface passes none.** The composer is drawn
// over whatever was showing — the rail stays marked on it — so its Close has a place to return
// to and #1366 put it on the ask's own title. Studios and Manifest are rail destinations: `goTo`
// clears every other surface before it sets one, nothing opens either over anything, and the rail
// marks the surface itself. A Close there would have no state to close to. The design system says
// the same for the trail — *a surface reached from Navigation carries no trail*, because
// Navigation is already the answer to where you are — and `actions.bridge_surfaces` (`⌘1`–`⌘9`)
// is the registered way off one. `actions.close` is for an overlay or a detail route, and the ask
// here is neither: it is the surface's first step, answered in place by picking.

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
   * The one control the ask carries — the composer's own way out. It sits at
   * the trailing edge of the title's line, which is where the two card states
   * of the same composer draw theirs. Absent draws none, which is what the two
   * rail surfaces pass — the head says why.
   */
  action?: ReactNode;
}) {
  const loose = repositories.filter((one) => one.manifest === undefined);
  return (
    <Alert tone="neutral" title={title} action={action} actionOn="title">
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
