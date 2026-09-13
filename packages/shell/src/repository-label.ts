// What a repository is called in the rail's picker.
// No React, so a node test reaches it.

import type { RepositorySummary } from "@armada/protocol";

/**
 * A set-up repository reads as its Manifest id and nothing else: Fleet refuses to serve one id
 * twice. One not set up reads as its folder, with the parent only where another not set up
 * shares that folder name, and the whole root past that. The root is always the hover.
 */
export function repositoryLabel(
  repository: RepositorySummary,
  repositories: readonly RepositorySummary[],
): string {
  if (repository.manifest !== undefined) return repository.manifest.id;
  const alike = repositories.filter((other) => other.root !== repository.root && other.manifest === undefined);
  const folder = folderOf(repository.root);
  if (!alike.some((other) => folderOf(other.root) === folder)) return folder;
  const placed = placedOf(repository.root);
  return alike.some((other) => placedOf(other.root) === placed) ? repository.root : placed;
}

function placedOf(root: string): string {
  const parts = root.replace(/\/+$/, "").split("/");
  return `${parts.at(-2) ?? ""}/${parts.at(-1) ?? root}`;
}

function folderOf(root: string): string {
  return root.replace(/\/+$/, "").split("/").pop() || root;
}
