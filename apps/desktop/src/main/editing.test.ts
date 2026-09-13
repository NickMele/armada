// What a save and an edit answered, told apart. The case that matters is the one that is
// not a failure: a file that moved under the edit carries what is on disk now,
// and losing it would leave a person with nothing to reconcile against.

import { describe, expect, it } from "vitest";

import { editAnswerOf, saveAnswerOf } from "./editing";

function refused(code: string, fields: Record<string, unknown>) {
  return {
    ok: false as const,
    outcome: {
      ok: false as const,
      why: "refused" as const,
      error: { code, message: "armada.yml changed after this edit read it", run_id: "r", fields, chain: [] },
    },
  };
}

describe("a save's answer", () => {
  it("is saved where Fleet wrote the bytes", () => {
    const saved = { path: "armada.yml", at: "2026-09-12T14:20:03.120Z" };
    expect(saveAnswerOf({ ok: true, body: saved })).toEqual({ state: "saved", saved });
  });

  it("carries what is on disk now where the file moved", () => {
    const answer = saveAnswerOf(
      refused("fleet.manifest_moved_under_the_edit", { on_disk: "version: 1\n" }),
    );
    expect(answer).toEqual({ state: "moved", onDisk: "version: 1\n" });
  });

  it("says the file is gone rather than handing back an empty text", () => {
    const answer = saveAnswerOf(refused("fleet.manifest_moved_under_the_edit", {}));
    expect(answer).toEqual({ state: "moved", onDisk: null });
  });

  it("is a failure for any other refusal, carried whole", () => {
    const other = refused("fleet.manifest_unwritable", {});
    expect(saveAnswerOf(other)).toEqual({ state: "failed", outcome: other.outcome });
  });
});

describe("an edit's answer", () => {
  it("is edited where Fleet wrote the file, carrying what it wrote", () => {
    const edited = { path: "armada.yml", at: "2026-09-12T14:20:03.120Z", text: "version: 1\n", declared: {} };
    expect(editAnswerOf({ ok: true, body: edited })).toEqual({ state: "edited", edited });
  });

  it("reads a result that would not load as refused, fault by fault", () => {
    const answer = editAnswerOf(
      refused("fleet.manifest_edit_refused", {
        faults: [["checks.lint.requires", "names `db`, which no Command declares"], "not a pair"],
      }),
    );
    expect(answer).toEqual({
      state: "refused",
      saying: "armada.yml changed after this edit read it",
      faults: [{ key: "checks.lint.requires", fault: "names `db`, which no Command declares" }],
    });
  });

  it("reads a misnamed entry as refused with Fleet's sentence and no faults", () => {
    const answer = editAnswerOf(refused("fleet.manifest_edit_misnamed", { key: "checks.build" }));
    expect(answer).toMatchObject({ state: "refused", faults: [] });
  });

  it("carries what is on disk now where the file moved, as a save does", () => {
    const answer = editAnswerOf(refused("fleet.manifest_moved_under_the_edit", { on_disk: "id: x\n" }));
    expect(answer).toEqual({ state: "moved", onDisk: "id: x\n" });
  });

  it("is a failure for any other refusal, carried whole", () => {
    const other = refused("fleet.manifest_unwritable", {});
    expect(editAnswerOf(other)).toEqual({ state: "failed", outcome: other.outcome });
  });
});
