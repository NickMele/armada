// The two callers this ask serves, and the one thing that tells them apart —
// #959: the Manifest surface's own ask still picks a repository nobody has
// set up, to open Setup on it; New job's own use shows the same entry greyed
// out and cannot pick it, since a Job needs a Manifest and nothing here may
// route anywhere.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import type { RepositorySummary } from "@armada/protocol";
import { AskRepository } from "./AskRepository";
import { mount, unmount } from "./mounted";

afterEach(unmount);

const SET_UP: RepositorySummary = {
  root: "/Users/user/armada",
  records_root: "/records/armada",
  manifest: {
    id: "armada",
    repository: "armada",
    path: "/Users/user/armada/armada.yml",
    records_root: "/records/armada",
    version: 1,
    checks: [],
  },
};
const NOT_SET_UP: RepositorySummary = { root: "/Users/user/scratch", records_root: "/records/scratch" };

test("the Manifest surface's own ask can pick a repository nobody has set up", async () => {
  const picked: string[] = [];
  mount(
    <AskRepository
      repositories={[SET_UP, NOT_SET_UP]}
      title="Pick a repository to open its Manifest"
      next="Picking it opens Setup."
      onPick={(root) => picked.push(root)}
    />,
  );

  const select = page.getByLabelText("Repository");
  await expect.element(page.getByRole("option", { name: "scratch" })).not.toBeDisabled();
  await select.selectOptions(NOT_SET_UP.root);

  expect(picked).toEqual([NOT_SET_UP.root]);
});

test("New job's own ask shows a repository nobody has set up, greyed out and unpickable", async () => {
  const picked: string[] = [];
  mount(
    <AskRepository
      repositories={[SET_UP, NOT_SET_UP]}
      title="Pick the repository this Job is for"
      next="The Board stays on All."
      onPick={(root) => picked.push(root)}
      onlySetUp
    />,
  );

  await expect.element(page.getByRole("option", { name: "scratch — Set it up first" })).toBeDisabled();
  const select = page.getByLabelText("Repository");
  await select.selectOptions(SET_UP.root);

  // Selecting the disabled option is not offered at all — only the one Manifest is.
  expect(picked).toEqual([SET_UP.root]);
});
