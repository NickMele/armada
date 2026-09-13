// Locate's wire half and the one rule it restates — `docs/journeys/set-up-a-project-manifest.md`,
// *Getting in*. No React, so main imports the answer's shape.

import type { Outcome, RepositorySummary } from "@armada/protocol";

/** What an add or a clone came to. `busy` is Bridge's own: one is already out. */
export type LocateAnswer =
  | { state: "located"; repository: RepositorySummary }
  | { state: "refused"; code: string; saying: string }
  | { state: "busy" }
  | { state: "failed"; outcome: Outcome };

/** Fleet's code for a clone destination that is already there and not empty. */
export const DESTINATION_OCCUPIED = "fleet.destination_occupied";

/**
 * The folder `git clone` makes for `url`: the last segment, less `.git`. **Fleet's
 * `folder_named_by` restated**, in `crates/fleet/src/repositories/cloning.rs`, so the dialog
 * names the folder before anything is sent. The two are coupled by this comment.
 */
export function folderNamedBy(url: string): string | null {
  let end = url.trim().replace(/\/+$/, "");
  if (end.endsWith("/.git")) end = end.slice(0, -"/.git".length).replace(/\/+$/, "");
  const last = end.slice(Math.max(end.lastIndexOf("/"), end.lastIndexOf(":")) + 1);
  const name = last.endsWith(".git") ? last.slice(0, -".git".length) : last;
  return name === "" || name === "." || name === ".." ? null : name;
}

/** Where a clone lands, or `null` where the parent is not absolute or the URL names no folder. */
export function landsIn(url: string, parent: string): string | null {
  const name = folderNamedBy(url);
  const under = parent.trim();
  if (name === null || !isAbsolute(under)) return null;
  return `${under.replace(/\/+$/, "")}/${name}`;
}

/** Fleet refuses a relative path, so the dialog does not offer to send one. */
export function isAbsolute(path: string): boolean {
  return path.trim().startsWith("/");
}
