// What a step's frames draw as: the rows, married to what has been fetched.
//
// **The arithmetic half.** What is asserted here is the reading — wire order,
// the weight, and what a collapsed chapter's summary says. The half that mints
// and revokes object URLs is a hook, so it is mounted, and it is in
// `frames.test.tsx` for the reason this package's config gives.

import { describe, expect, it } from "vitest";

import type { KeptFrame } from "@armada/protocol";

import {
  framesSummary,
  pairedFrames,
  pairedSummary,
  shownFrames,
  NO_FRAMES,
} from "./frames";

function frame(over: Partial<KeptFrame> = {}): KeptFrame {
  return {
    attempt: 1,
    name: "home.png",
    path: ".armada/frames/12-a-job/show.1/home.png",
    bytes: 41_002,
    kept: "show.1/home.png",
    ...over,
  };
}

describe("what a step's frames draw as", () => {
  it("keeps wire order, because that ordering is the record's", () => {
    const rows = [
      frame({ kept: "show.1/home.png", name: "home.png" }),
      frame({ kept: "show.1/a.png", name: "a.png" }),
    ];
    // Sorted, `a.png` would lead — and a reader comparing two runs would be
    // reading them in an order Fleet never answered in.
    expect(shownFrames(rows, NO_FRAMES).map((shown) => shown.name)).toEqual([
      "home.png",
      "a.png",
    ]);
  });

  it("carries the weight before the bytes, so a slow one reads as a large file", () => {
    const shown = shownFrames([frame({ bytes: 41_002 })], NO_FRAMES);
    expect(shown[0]!.weight).toBe("40.0 KB");
    // Neither drawn nor failed: the plate says the wait. A frame nothing has
    // asked for and one still in flight are the same wait from where the
    // person is sitting, and this is what makes them read the same.
    expect(shown[0]!.content).toBeUndefined();
    expect(shown[0]!.why).toBeUndefined();
  });

  it("carries the run on every frame rather than implying it by position", () => {
    const shown = shownFrames(
      [frame(), frame({ attempt: 3, kept: "show.3/home.png" })],
      NO_FRAMES,
    );
    expect(shown.map((one) => one.attempt)).toEqual([1, 3]);
    // The same file name from two runs, told apart by `kept` and never by the
    // harness's own name for it.
    expect(shown.map((one) => one.kept)).toEqual(["show.1/home.png", "show.3/home.png"]);
  });

  it("weighs a small file in bytes and a large one in megabytes", () => {
    expect(shownFrames([frame({ bytes: 900 })], NO_FRAMES)[0]!.weight).toBe("900 B");
    expect(shownFrames([frame({ bytes: 4_200_000 })], NO_FRAMES)[0]!.weight).toBe("4.0 MB");
  });
});

describe("what a collapsed chapter says about them", () => {
  it("says nothing about a step with no frames", () => {
    expect(framesSummary([])).toBeUndefined();
  });

  it("counts one frame in the singular", () => {
    expect(framesSummary([frame()])).toBe("1 frame");
  });

  it("leaves the runs out where a step was worked once", () => {
    expect(framesSummary([frame(), frame({ kept: "show.1/b.png" })])).toBe("2 frames");
  });

  /**
   * Two frames from two runs and two frames from one are different things to
   * be looking at. A summary that read `2 frames` for both would hide the fact
   * that decides whether what a reader is about to open is current.
   */
  it("says how many runs where a step was worked more than once", () => {
    expect(framesSummary([frame(), frame({ attempt: 2, kept: "show.2/home.png" })])).toBe(
      "2 frames · 2 runs",
    );
  });

  /**
   * Whether there is a before is the fact that decides what a collapsed
   * chapter is worth opening for: with both sides it answers what the change
   * did to the screen, with one it answers only what the screen is.
   */
  it("says whether there is a before, where there is one", () => {
    expect(
      framesSummary([
        frame({ kept: "show.1.base/home.png", side: "base" }),
        frame({ kept: "show.1.branch/home.png", side: "branch" }),
      ]),
    ).toBe("2 frames · before and after");
    expect(framesSummary([frame({ side: "branch" })])).toBe("1 frame");
  });
});

describe("which side a frame is a photograph of", () => {
  /**
   * **`before` and `after`, not `base` and `branch`.** The wire's words name
   * the two checkouts Fleet had to serve; what a reviewer wants to know is
   * which of these is how the screen was, and the translation happens once so
   * no component learns what a base branch is.
   */
  it("says the reader's word for each side where there are two", () => {
    const shown = shownFrames(
      [
        frame({ kept: "show.1.base/home.png", side: "base" }),
        frame({ kept: "show.1.branch/home.png", side: "branch" }),
      ],
      NO_FRAMES,
    );
    expect(shown.map((one) => one.side)).toEqual(["before", "after"]);
  });

  /**
   * A step with one side is a repository with no base, a base run that would
   * not start, or a Fleet older than 9.5. `after` written on every frame of a
   * set with no before is a word that says nothing and implies a missing half.
   */
  it("labels nothing where there is only one side to be on", () => {
    const shown = shownFrames(
      [
        frame({ kept: "show.1.branch/home.png", side: "branch" }),
        frame({ kept: "show.1.branch/settings.png", side: "branch" }),
      ],
      NO_FRAMES,
    );
    expect(shown.map((one) => one.side)).toEqual([undefined, undefined]);
  });

  /** A Fleet older than 9.5 sends no side at all, and those rows are branch. */
  it("reads a row with no side as the branch, which is what it is", () => {
    const shown = shownFrames(
      [frame({ kept: "show.1.base/home.png", side: "base" }), frame({ side: undefined })],
      NO_FRAMES,
    );
    expect(shown.map((one) => one.side)).toEqual(["before", "after"]);
  });
});


// Which pairs fold, and the one direction this is allowed to be wrong in.
//
// **Folding is the only act on this surface that can hide something.** A pair
// drawn when it did not need to be is noise a reader glances past; a pair
// folded when it moved is the change they came for, gone. So every unknown
// answers *not the same*, and the cases below are mostly unknowns.
describe("which pairs fold away", () => {
  function pair(over: Partial<KeptFrame> = {}, side: "base" | "branch" = "branch"): KeptFrame {
    return {
      attempt: 1,
      name: "home.png",
      path: `.armada/frames/12-a-job/show.1.${side}/home.png`,
      bytes: 41_002,
      kept: `show.1.${side}/home.png`,
      side,
      digest: "a1b2c3d4e5f60718",
      ...over,
    };
  }

  const both = (over: Partial<KeptFrame> = {}) => [
    pair({ kept: "show.1.base/home.png", ...over }, "base"),
    pair({}, "branch"),
  ];

  it("marries the two sides by name, within one run", () => {
    const pairs = pairedFrames(both(), NO_FRAMES);
    expect(pairs).toHaveLength(1);
    expect(pairs[0]!.name).toBe("home.png");
    expect(pairs[0]!.before).toBeDefined();
    expect(pairs[0]!.after).toBeDefined();
  });

  it("folds a pair whose digest and size both agree", () => {
    expect(pairedFrames(both(), NO_FRAMES)[0]!.same).toBe(true);
  });

  it("draws a pair whose digests differ", () => {
    expect(pairedFrames(both({ digest: "ffffffffffffffff" }), NO_FRAMES)[0]!.same).toBe(false);
  });

  // The digest is sixty-four bits and the size is already on the row. Two
  // comparisons that must both hold cost nothing, and remove the one failure
  // that would be silent.
  it("draws a pair whose digests agree and sizes do not", () => {
    expect(pairedFrames(both({ bytes: 9 }), NO_FRAMES)[0]!.same).toBe(false);
  });

  // A frame kept before the digest existed carries none. Two absences reading
  // as agreement would fold away exactly the old Jobs nobody can re-photograph.
  it("never folds on an empty digest, including two of them", () => {
    expect(pairedFrames(both({ digest: "" }), NO_FRAMES)[0]!.same).toBe(false);
    const neither = [
      pair({ kept: "show.1.base/home.png", digest: "" }, "base"),
      pair({ digest: "" }, "branch"),
    ];
    expect(pairedFrames(neither, NO_FRAMES)[0]!.same).toBe(false);
  });

  it("never folds a pair with only one side", () => {
    const added = pairedFrames([pair({}, "branch")], NO_FRAMES);
    expect(added[0]!.same).toBe(false);
    expect(added[0]!.before).toBeUndefined();
    expect(added[0]!.after).toBeDefined();

    const removed = pairedFrames([pair({ kept: "show.1.base/home.png" }, "base")], NO_FRAMES);
    expect(removed[0]!.same).toBe(false);
    expect(removed[0]!.after).toBeUndefined();
  });

  // A step worked three times captured three sets. Pairing across runs would
  // put this run's after beside the last run's before — a comparison nobody
  // asked for, and one a reader cannot tell from the real thing.
  it("never pairs across runs", () => {
    const pairs = pairedFrames(
      [
        pair({ attempt: 1, kept: "show.1.base/home.png" }, "base"),
        pair({ attempt: 2, kept: "show.2.branch/home.png" }, "branch"),
      ],
      NO_FRAMES,
    );
    expect(pairs).toHaveLength(2);
    expect(pairs.some((one) => one.same)).toBe(false);
  });

  it("takes each side once, so a name written twice does not overwrite", () => {
    const twice = [...both(), pair({ digest: "ffffffffffffffff" }, "branch")];
    const pairs = pairedFrames(twice, NO_FRAMES);
    expect(pairs).toHaveLength(1);
    expect(pairs[0]!.same).toBe(true);
  });

  it("keeps wire order, and a base-only name lands where the record had it", () => {
    const pairs = pairedFrames(
      [pair({ name: "gone.png", kept: "show.1.base/gone.png" }, "base"), ...both()],
      NO_FRAMES,
    );
    expect(pairs.map((one) => one.name)).toEqual(["gone.png", "home.png"]);
  });
});

describe("what a chapter says about two sides", () => {
  function shot(name: string, side: "base" | "branch", digest: string): KeptFrame {
    return {
      attempt: 1,
      name,
      path: `.armada/frames/12-a-job/show.1.${side}/${name}`,
      bytes: 41_002,
      kept: `show.1.${side}/${name}`,
      side,
      digest,
    };
  }

  const of = (rows: KeptFrame[]) => pairedSummary(pairedFrames(rows, NO_FRAMES));

  it("says nothing about a step with no frames", () => {
    expect(of([])).toBeUndefined();
  });

  // Ten screens photographed twice is twenty images and one sentence worth
  // reading. A summary counting the images counts the work, not the answer.
  it("counts what moved against what was compared", () => {
    expect(
      of([
        shot("home.png", "base", "aaaa"),
        shot("home.png", "branch", "aaaa"),
        shot("checks.png", "base", "aaaa"),
        shot("checks.png", "branch", "bbbb"),
      ]),
    ).toBe("1 of 2 changed");
  });

  it("drops the denominator where everything moved", () => {
    expect(of([shot("a.png", "base", "aaaa"), shot("a.png", "branch", "bbbb")])).toBe("1 frame");
  });

  // A real answer, and the one outcome on a shown step that asks a question:
  // the change touched no screen the spec photographs.
  it("says so plainly where nothing moved at all", () => {
    expect(of([shot("a.png", "base", "aaaa"), shot("a.png", "branch", "aaaa")])).toBe(
      "nothing moved",
    );
  });
});
