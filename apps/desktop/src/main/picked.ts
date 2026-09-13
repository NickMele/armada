// Which repository a per-repository call names — the rail's pick, held in main.
//
// **One place builds every such path**, so a call added later cannot forget to name its
// repository: an absent parameter is not an error at Fleet, it is the repository Fleet was
// started in, which is the wrong one the moment a second is picked. `picked.test.ts` holds
// every per-repository route in `src/main` to it.

import type { RepositorySummary } from "@armada/protocol";

export class Picked {
  private root: string | null = null;
  private listed: readonly RepositorySummary[] = [];

  /** The picked repository, or `null` before Fleet has listed any. */
  get repository(): RepositorySummary | null {
    return this.listed.find((one) => one.root === this.root) ?? null;
  }

  /** The picked root, as `BridgeState.repository` carries it. */
  get picked(): string | null {
    return this.repository?.root ?? null;
  }

  /**
   * What Fleet lists. The pick stays where it is still served and otherwise falls to the
   * first, the one Fleet started in. **Answers whether what calls name moved** — a root, or
   * a Manifest id that appeared when Write put one down.
   */
  hold(listed: readonly RepositorySummary[]): boolean {
    const was = this.key();
    this.listed = listed;
    if (this.repository === null) this.root = listed[0]?.root ?? null;
    return this.key() !== was;
  }

  /** Pick a listed root. A root Fleet does not list is ignored, and so answers `false`. */
  pick(root: string): boolean {
    if (!this.listed.some((one) => one.root === root)) return false;
    const was = this.key();
    this.root = root;
    return this.key() !== was;
  }

  /**
   * A route acting on the picked repository's Manifest, naming it by `?manifest_id=`.
   * **`null` where the picked repository has none yet**: sending nothing would act on
   * another repository. Before anything is listed the route goes bare, as it always did.
   */
  manifest(path: string): string | null {
    const repository = this.repository;
    if (repository === null) return path;
    if (repository.manifest === undefined) return null;
    return named(path, "manifest_id", repository.manifest.id);
  }

  /** Scan and its proposals, which name a repository by its root — it may have no Manifest. */
  scan(path: string): string {
    const repository = this.repository;
    return repository === null ? path : named(path, "repository", repository.root);
  }

  /** Whether a `manifest.reread` is about the picked repository. Anything, before a pick. */
  reads(path: string): boolean {
    const repository = this.repository;
    if (repository === null) return true;
    return path === repository.manifest?.path || path.startsWith(`${repository.root}/`);
  }

  private key(): string {
    const repository = this.repository;
    return repository === null ? "" : `${repository.root}\n${repository.manifest?.id ?? ""}`;
  }
}

function named(path: string, key: string, value: string): string {
  return `${path}${path.includes("?") ? "&" : "?"}${key}=${encodeURIComponent(value)}`;
}
