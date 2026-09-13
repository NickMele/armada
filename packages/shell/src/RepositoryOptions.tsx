// The repositories Fleet serves, as a picker's options: the rail's, and the ask a surface that
// needs one repository puts on All. One list, so the two cannot name a repository differently.

import type { RepositorySummary } from "@armada/protocol";
import { repositoryLabel } from "./repository-label";

/** The rail's value for All repositories. No root is empty, so it cannot collide with one. */
export const ALL_REPOSITORIES = "";

/** Set up first, then the rest in a group: the rail clips a long label, and picking one opens Setup. */
export function RepositoryOptions({ repositories }: { repositories: readonly RepositorySummary[] }) {
  const optionOf = (repository: RepositorySummary) => (
    <option key={repository.root} value={repository.root} title={repository.root}>
      {repositoryLabel(repository, repositories)}
    </option>
  );
  const loose = repositories.filter((one) => one.manifest === undefined);
  return (
    <>
      {repositories.filter((one) => one.manifest !== undefined).map(optionOf)}
      {loose.length === 0 ? null : <optgroup label="Not set up">{loose.map(optionOf)}</optgroup>}
    </>
  );
}
