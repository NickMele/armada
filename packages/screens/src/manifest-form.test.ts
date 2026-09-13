// An edit's answer, folded into the forms. The case that matters: a Fleet that
// does not say what it wrote leaves the form reading the file, not drawing a guess.

import { describe, expect, it } from "vitest";
import type { ManifestDeclared } from "@armada/protocol";

import { draftOf } from "./form-edits";
import { editAnswered, formAnswered, type Held } from "./manifest-form";

const DECLARED: ManifestDeclared = {
  checks: [{ name: "build", check: { run: "cargo build" } }],
  commands: [],
  ports: [],
  auto_merge: { written: "never", offered: ["never", "always"] },
  review_gate: { written: "human_always", offered: ["human_always"] },
};
const WROTE = { ...DECLARED, checks: [{ name: "build", check: { run: "cargo build --locked" } }] };

function open(): Held {
  const draft = draftOf(DECLARED);
  draft.checks[0]!.run = "cargo build --locked";
  return { state: "open", path: "armada.yml", read: "old", declared: DECLARED, draft, saving: true };
}

describe("an edit's answer", () => {
  it("redraws the form from what Fleet says it wrote", () => {
    const held = editAnswered(open(), {
      state: "edited",
      edited: { path: "armada.yml", at: "2026-09-12T14:20:03.120Z", text: "new", declared: WROTE },
    });
    expect(held).toMatchObject({ state: "open", read: "new", declared: WROTE, saving: false });
  });

  it("reads the file again where Fleet did not say what it wrote, and keeps the receipt", () => {
    const edited = { path: "armada.yml", at: "2026-09-12T14:20:03.120Z", text: "new" };
    const reading = editAnswered(open(), { state: "edited", edited });
    expect(reading.state).toBe("reading");

    const redrawn = formAnswered(reading, {
      ok: true,
      file: { path: "armada.yml", text: "new", declared: WROTE },
    });
    expect(redrawn).toMatchObject({ state: "open", read: "new", declared: WROTE });
    expect(redrawn.state === "open" && redrawn.saved).toBeTruthy();
    expect(redrawn.state === "open" && redrawn.draft).toEqual(draftOf(WROTE));
  });
});
