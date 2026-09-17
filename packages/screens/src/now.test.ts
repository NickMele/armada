// What the live phase's header says the Drone is doing. #1196.
//
// **The case that matters is the call that has not come back**, because that
// is the one a log cannot show: its `answered` row does not exist yet, so a
// header reading the last row would name the call before it instead.
import { describe, expect, it } from "vitest";

import { answered, called, said } from "./fixtures/build/base";
import { doingNow } from "./now";

const STEP = "implement";
const TREE = "~/Development/armada/.armada/worktrees/01K5/";

function at(seconds: number): string {
  return new Date(Date.parse("2026-09-15T08:27:40Z") + seconds * 1000).toISOString();
}

const NOW = Date.parse(at(3));

describe("what the Drone is doing", () => {
  it("is the call with no answer, with its verb, its path and how long it has been open", () => {
    expect(
      doingNow(
        [
          called(STEP, at(-20), "c1", "Read", `${TREE}crates/fleet/src/ending.rs`),
          answered(STEP, at(-19), "c1"),
          called(STEP, at(0), "c2", "Edit", `${TREE}crates/fleet/src/settling.rs +2 -2`),
        ],
        STEP,
        NOW,
      ),
    ).toEqual({
      verb: "Editing",
      detail: "crates/fleet/src/settling.rs",
      mono: true,
      took: "3s",
    });
  });

  it("is the Drone's own sentence between calls, and carries no verb there", () => {
    expect(
      doingNow(
        [
          called(STEP, at(-20), "c1", "Bash", "cargo test -p fleet"),
          answered(STEP, at(-19), "c1"),
          said(STEP, at(-1), "Now I will check the settling path."),
        ],
        STEP,
        NOW,
      ),
    ).toEqual({ detail: "Now I will check the settling path." });
  });

  it("says nothing at all on a step whose Drone has done nothing", () => {
    expect(doingNow([], STEP, NOW)).toBeUndefined();
  });

  it("leaves a command whole, so a Bash call reads as the command it ran", () => {
    expect(doingNow([called(STEP, at(0), "c1", "Bash", "cargo test -p fleet")], STEP, NOW)).toEqual({
      verb: "Running",
      detail: "cargo test -p fleet",
      mono: true,
      took: "3s",
    });
  });

  it("falls back to the tool's own name where the roster has no verb for it", () => {
    expect(
      doingNow([called(STEP, at(0), "c1", "WebFetch", "https://docs.rs/tokio")], STEP, NOW),
    ).toEqual({
      verb: "WebFetch",
      detail: "https://docs.rs/tokio",
      mono: true,
      took: "3s",
    });
  });
});
