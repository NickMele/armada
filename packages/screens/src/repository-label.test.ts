// What a repository is called in the picker — #886 — and on a Board row on All repositories.

import { describe, expect, it } from "vitest";

import type { JobSummary, RepositorySummary } from "@armada/protocol";
import { manifestLabel, repositoryLabel } from "@armada/shell/src/repository-label";
import { BOARD_COLUMNS, columnsFor, REPOSITORY_COLUMN, repositoryOf, TASKS_COLUMN } from "./board";

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

  it("names a Job's repository by the Manifest id it carries, served or not", () => {
    const every = [setUp("armada", "/Users/user/armada")];
    expect(manifestLabel("armada", every)).toBe("armada");
    expect(manifestLabel("gone", every)).toBe("gone");
  });
});

describe("a Board row's repository", () => {
  const job = { owner_manifest_id: "storefront" } as JobSummary;
  const two = [setUp("armada", "/Users/user/armada"), setUp("storefront", "/Users/user/storefront")];

  it("is named, with its column, on All where Fleet serves more than one", () => {
    expect(repositoryOf(job, two, true)).toBe("storefront");
    expect(columnsFor([], two, true)).toEqual([...BOARD_COLUMNS, REPOSITORY_COLUMN]);
    for (const served of [null, [], two.slice(1)]) {
      expect(repositoryOf(job, served, true)).toBeUndefined();
      expect(columnsFor([], served, true)).toEqual(BOARD_COLUMNS);
    }
  });

  it("is not named with one repository picked, however many are served", () => {
    expect(repositoryOf(job, two, false)).toBeUndefined();
    expect(columnsFor([], two, false)).toEqual(BOARD_COLUMNS);
  });

  it("comes before Tasks, so a row without a plan leaves only the trailing cell blank", () => {
    const planned = { tasks: { done: 1, working: 0, open: 1, dropped: 0 } } as JobSummary;
    expect(columnsFor([planned, job], two, true)).toEqual([...BOARD_COLUMNS, REPOSITORY_COLUMN, TASKS_COLUMN]);
  });
});
