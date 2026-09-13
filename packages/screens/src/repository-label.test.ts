// What a repository is called in the rail's picker — #886.

import { describe, expect, it } from "vitest";

import type { RepositorySummary } from "@armada/protocol";
import { repositoryLabel } from "@armada/shell/src/repository-label";

const manifest = (id: string, root: string) => ({ id, repository: id, path: `${root}/armada.yml`, records_root: `/records/${id}`, version: 1, checks: [] });
const setUp = (id: string, root: string): RepositorySummary => ({ root, records_root: `/records/${id}`, manifest: manifest(id, root) });
const loose = (root: string): RepositorySummary => ({ root, records_root: `/records${root}` });

describe("the picker's label", () => {
  it("reads a set-up repository as its Manifest id alone, however its folder collides", () => {
    const every = [setUp("storefront", "/Users/user/code/web"), setUp("web", "/Users/user/old/web"), loose("/Users/user/code/storefront")];
    expect(repositoryLabel(every[0]!, every)).toBe("storefront");
    expect(repositoryLabel(every[1]!, every)).toBe("web");
  });

  it("reads one not set up as its folder, even where a set-up id shares the name", () => {
    const every = [setUp("api", "/Users/user/services/api"), loose("/Users/user/code/api/")];
    expect(repositoryLabel(every[1]!, every)).toBe("api");
  });

  it("adds the parent only where two not set up share a folder, and the root past that", () => {
    const every = [loose("/Users/user/code/api"), loose("/Users/user/old/api"), loose("/Volumes/code/api")];
    expect(repositoryLabel(every[1]!, every)).toBe("old/api");
    expect(repositoryLabel(every[0]!, every)).toBe("/Users/user/code/api");
    expect(repositoryLabel(every[2]!, every)).toBe("/Volumes/code/api");
  });
});
