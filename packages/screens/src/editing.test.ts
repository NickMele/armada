// The file view's folds. Each case is a rule about not losing somebody's edit
// or the change that landed under it, rather than a detail of the shapes.

import { describe, expect, it } from "vitest";

import type { ManifestSaved } from "@armada/protocol";
import {
  fileAnswered,
  fileNameOf,
  saveAnswered,
  settledBy,
  takenOnDisk,
  type HeldFile,
} from "./editing";

const PATH = "/work/armada/armada.yml";
const DISK = "version: 1\nid: armada\n";
const EDIT = "version: 1\nid: armada\nbase: main\n";
const PULLED = "version: 1\nid: armada\nbase: trunk\n";

function open(over: Partial<Extract<HeldFile, { state: "open" }>> = {}): HeldFile {
  return { state: "open", path: PATH, read: DISK, text: DISK, saving: false, ...over };
}

const SAVED: ManifestSaved = { path: PATH, at: "2026-09-12T14:20:03.120Z" };

describe("the toggle's name", () => {
  it("is the file's own name, off the path Fleet resolved", () => {
    expect(fileNameOf(PATH)).toBe("armada.yml");
    expect(fileNameOf("armada.yml")).toBe("armada.yml");
  });
});

describe("whether a reading answers a save", () => {
  it("does only when Fleet read the file after the write landed", () => {
    expect(settledBy(SAVED, { path: PATH, at: "2026-09-12T14:20:04.700Z" })).toBe(true);
  });

  it("does not for a reading of the file before the save", () => {
    expect(settledBy(SAVED, { path: PATH, at: "2026-09-12T14:11:40.000Z" })).toBe(false);
    expect(settledBy(SAVED, { path: PATH, at: SAVED.at })).toBe(false);
    expect(settledBy(SAVED, null)).toBe(false);
  });
});

describe("a read, folded", () => {
  it("opens the file with nothing edited", () => {
    const held = fileAnswered({ state: "reading" }, { ok: true, file: { path: PATH, text: DISK } });
    expect(held).toEqual(open());
  });

  it("never replaces an edit in progress, and never moves what it started from", () => {
    const editing = open({ text: EDIT });
    expect(fileAnswered(editing, { ok: true, file: { path: PATH, text: PULLED } })).toBe(editing);
  });

  it("shows a file changed elsewhere when nothing is being edited", () => {
    const held = fileAnswered(open(), { ok: true, file: { path: PATH, text: PULLED } });
    expect(held).toMatchObject({ read: PULLED, text: PULLED });
  });

  it("does not blank an open file when a read fails", () => {
    const editing = open({ text: EDIT });
    expect(fileAnswered(editing, { ok: false, outcome: { ok: false, why: "not_connected" } })).toBe(
      editing,
    );
  });

  it("fills the disk side of a save that found the file gone, once it is back", () => {
    const held = fileAnswered(open({ text: EDIT, moved: { onDisk: null } }), {
      ok: true,
      file: { path: PATH, text: PULLED },
    });
    expect(held).toMatchObject({ text: EDIT, read: DISK, moved: { onDisk: PULLED } });
  });
});

describe("a save, folded", () => {
  it("counts what was sent as written, not what has been typed since", () => {
    const typedOn = open({ text: `${EDIT}# more\n`, saving: true });
    const held = saveAnswered(typedOn, { read: DISK, text: EDIT }, { state: "saved", saved: SAVED });
    expect(held).toMatchObject({ read: EDIT, text: `${EDIT}# more\n`, saving: false, saved: SAVED });
  });

  it("keeps the edit and the disk apart when the file moved", () => {
    const held = saveAnswered(
      open({ text: EDIT, saving: true, saved: SAVED }),
      { read: DISK, text: EDIT },
      { state: "moved", onDisk: PULLED },
    );
    expect(held).toMatchObject({ read: DISK, text: EDIT, moved: { onDisk: PULLED }, saving: false });
    expect(held).not.toHaveProperty("saved", SAVED);
  });

  it("keeps the edit when the bytes would not go down", () => {
    const held = saveAnswered(
      open({ text: EDIT, saving: true }),
      { read: DISK, text: EDIT },
      { state: "failed", outcome: { ok: false, why: "not_connected" } },
    );
    expect(held).toMatchObject({ read: DISK, text: EDIT, saving: false });
  });
});

describe("taking what is on disk", () => {
  it("carries on from the disk, and only where there is one", () => {
    expect(takenOnDisk(open({ text: EDIT, moved: { onDisk: PULLED } }))).toEqual(
      open({ read: PULLED, text: PULLED }),
    );
    const gone = open({ text: EDIT, moved: { onDisk: null } });
    expect(takenOnDisk(gone)).toBe(gone);
  });
});
