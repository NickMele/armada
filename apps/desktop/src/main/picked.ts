// Which repository a per-repository call names — the rail's pick, held in main.
//
// **One place builds every such path**, so a call added later cannot forget to name its
// repository: an absent parameter is not an error at Fleet, it is the repository Fleet was
// started in, which is the wrong one the moment a second is picked. `picked.test.ts` holds
// every per-repository route in `src/main` to it.
//
// **All repositories is a pick of none.** Bridge opens on it, and on it every builder below
// answers `null`: a surface that needs one repository asks which before anything is sent.

import type { RepositorySummary } from "@armada/protocol";

export class Picked {
  /** The picked root, or `null` for All repositories. */
  private root: string | null = null;
  private listed: readonly RepositorySummary[] = [];

  /** The picked repository, or `null` on All repositories and before Fleet has listed any. */
  get repository(): RepositorySummary | null {
    return this.listed.find((one) => one.root === this.root) ?? null;
  }

  /** The picked root, as `BridgeState.repository` carries it: `null` is All repositories. */
  get picked(): string | null {
    return this.repository?.root ?? null;
  }

  /**
   * What Fleet lists. The pick stays where it is still served and otherwise falls back to All.
   * **Answers whether what calls name moved** — a root, or a Manifest id that appeared when
   * Write put one down.
   */
  hold(listed: readonly RepositorySummary[]): boolean {
    const was = this.key();
    this.listed = listed;
    if (this.repository === null) this.root = null;
    return this.key() !== was;
  }

  /** Pick a listed root, or `null` for All. A root Fleet does not list is ignored, and so answers `false`. */
  pick(root: string | null): boolean {
    if (root !== null && !this.listed.some((one) => one.root === root)) return false;
    const was = this.key();
    this.root = root;
    return this.key() !== was;
  }

  /**
   * A route acting on the picked repository's Manifest, naming it by `?manifest_id=`.
   * **`null` where the picked repository has none yet, and on All**: sending nothing would act
   * on another repository. Before anything is listed the route goes bare, as it always did.
   */
  manifest(path: string): string | null {
    const repository = this.repository;
    if (repository === null) return this.bare(path);
    if (repository.manifest === undefined) return null;
    return named(path, "manifest_id", repository.manifest.id);
  }

  /**
   * Verify and what its panel reads: the main checkout, which may have no root Manifest. Named by
   * `?manifest_id=` where it has one, and by root where it has none. `null` on All.
   */
  checkout(path: string): string | null {
    const repository = this.repository;
    if (repository === null) return this.bare(path);
    return repository.manifest === undefined
      ? named(path, "repository", repository.root)
      : named(path, "manifest_id", repository.manifest.id);
  }

  /**
   * A Manifest route for every repository a scoped surface reads — the picked one, or each listed on
   * All — naming each by its own `?manifest_id=`. `null` beside one with no Manifest yet.
   */
  each(path: string): { repository: RepositorySummary; path: string | null }[] {
    const scope = this.repository === null ? this.listed : [this.repository];
    return scope.map((repository) => ({
      repository,
      path: repository.manifest === undefined ? null : named(path, "manifest_id", repository.manifest.id),
    }));
  }

  /** Scan and its proposals, which name a repository by its root — it may have no Manifest. `null` on All. */
  scan(path: string): string | null {
    const repository = this.repository;
    return repository === null ? this.bare(path) : named(path, "repository", repository.root);
  }

  /** Whether a `manifest.reread` is about the picked repository. Anything before a listing, nothing on All. */
  reads(path: string): boolean {
    const repository = this.repository;
    if (repository === null) return this.listed.length === 0;
    return path === repository.manifest?.path || path.startsWith(`${repository.root}/`);
  }

  /** No repository picked: bare before Fleet lists any, as an older Fleet needs, and nothing on All. */
  private bare(path: string): string | null {
    return this.listed.length === 0 ? path : null;
  }

  private key(): string {
    const repository = this.repository;
    return repository === null ? "" : `${repository.root}\n${repository.manifest?.id ?? ""}`;
  }
}

function named(path: string, key: string, value: string): string {
  return `${path}${path.includes("?") ? "&" : "?"}${key}=${encodeURIComponent(value)}`;
}
