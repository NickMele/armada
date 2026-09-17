// The Job's diff as the Produced panel draws it: by folder, biggest first. #1187.
import { describe, expect, it } from "vitest";

import type { ChangedFile } from "@armada/components";

import { changeSummaryOf, foldersOf, moreSaid } from "./change-summary";

/** The #796 Job's Implement step, as the frames drew it. */
const DRAWN: ChangedFile[] = [
  { path: "crates/fleet/src/daemon/fittings.rs", change: "modified", added: 6, deleted: 1 },
  { path: "crates/fleet/src/dispatch.rs", change: "modified", added: 4, deleted: 4 },
  { path: "crates/fleet/src/ending.rs", change: "modified", added: 3, deleted: 3 },
  { path: "crates/fleet/src/evidence.rs", change: "modified", added: 31, deleted: 9 },
  { path: "crates/fleet/src/settling.rs", change: "modified", added: 14, deleted: 6 },
  { path: "crates/store/src/lib.rs", change: "modified", added: 2 },
  { path: "crates/store/src/migrations.rs", change: "modified", added: 12 },
  { path: "crates/store/src/pending_evidence.rs", change: "added", added: 82 },
];

describe("which folder a file is drawn under", () => {
  it("joins a directory holding one file to the nearest one above it that holds more", () => {
    const folders = foldersOf(DRAWN.map((file) => file.path));
    expect(folders.get("crates/fleet/src/daemon/fittings.rs")).toBe("crates/fleet/src");
    expect(folders.get("crates/store/src/lib.rs")).toBe("crates/store/src");
  });

  it("keeps a lone directory where nothing above it changed", () => {
    expect(foldersOf(["docs/notes.md", "crates/a/src/lib.rs"]).get("crates/a/src/lib.rs")).toBe(
      "crates/a/src",
    );
  });

  it("draws a file at the top of the repository under `.`", () => {
    expect(foldersOf(["README.md"]).get("README.md")).toBe(".");
  });
});

describe("the summary", () => {
  it("is the frames' drawing: folders and files biggest first, with folder totals", () => {
    const { folders, more } = changeSummaryOf(DRAWN, 12);
    expect(more).toBe(0);
    expect(folders.map((folder) => [folder.path, folder.added, folder.deleted])).toEqual([
      ["crates/store/src", 96, undefined],
      ["crates/fleet/src", 58, 23],
    ]);
    expect(folders[0]?.files.map((file) => file.name)).toEqual([
      "pending_evidence.rs",
      "migrations.rs",
      "lib.rs",
    ]);
    expect(folders[1]?.files.map((file) => file.name)).toEqual([
      "evidence.rs",
      "settling.rs",
      "dispatch.rs",
      "daemon/fittings.rs",
      "ending.rs",
    ]);
  });

  it("puts a file nothing counted last, rather than reading it as the smallest", () => {
    const { folders } = changeSummaryOf(
      [
        { path: "a/logo.png", change: "added" },
        { path: "a/one.rs", change: "modified", added: 1 },
      ],
      12,
    );
    expect(folders[0]?.files.map((file) => file.name)).toEqual(["one.rs", "logo.png"]);
  });

  it("draws the biggest files and says how many more, keeping each folder's whole total", () => {
    const { folders, more } = changeSummaryOf(DRAWN, 2);
    expect(more).toBe(6);
    expect(moreSaid(more)).toBe("and 6 more files, in the diff");
    expect(folders.map((folder) => [folder.path, folder.files.length, folder.added])).toEqual([
      ["crates/store/src", 1, 96],
      ["crates/fleet/src", 1, 58],
    ]);
  });
});
