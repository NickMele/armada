// What a save answered, told apart. The case that matters is the one that is
// not a failure: a file that moved under the edit carries what is on disk now,
// and losing it would leave a person with nothing to reconcile against.

import { describe, expect, it } from "vitest";

import { saveAnswerOf } from "./editing";

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
