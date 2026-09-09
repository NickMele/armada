// The frames a step's harness produced, fetched so the panel can lead with
// them.
//
// **`outputs.ts`'s shape, and one deliberate departure from it.** A Check's
// output is fetched when somebody opens that Check, because output is the
// payload the event stream is bounded to keep off itself and pre-fetching every
// row would spend exactly what the split avoids. Frames are fetched when the
// step is opened, without anybody pressing: the whole claim of `#209` is that a
// change whose point is not the code is reviewed by *looking at it*, and a
// panel that leads with a button saying there are pictures has not led with
// anything. The bound is the shape of the thing — one step is open at a time
// and a spec captures a handful of screens, where one Check prints a whole
// test run.
//
// **Keyed by `kept`, which is Fleet's own id for a row**: the run's directory
// and the file name, composed once on the Rust side and sent back as it was
// given. Nothing here derives it from a path.
//
// # The bytes become a `blob:` and this owns it
//
// What crosses the preload is an array, because the renderer never opens a
// socket and a base64 in between would inflate it by a third to say the same
// thing. `URL.createObjectURL` is what turns that into something an `img` can
// draw, and every one it mints has to be revoked — an object URL is held by the
// document until it is, so a person walking six Jobs would leave six sets of
// screenshots in memory. The revoking is this module's whole reason for holding
// state rather than fetching in a component.

import { useCallback, useEffect, useRef, useState } from "react";

import type { ShownFrame } from "@armada/components";
import type { FrameRead, KeptFrame } from "@armada/protocol";

/**
 * What one frame is, as this window has it.
 *
 * `undefined` — what the map answers for a frame nothing has asked for — is the
 * moment before the step's fetch goes out, and it draws as `reading…` for the
 * same reason a started one does: from a reader's side they are the same wait.
 */
export type FrameState =
  /** Asked for, nothing back yet. */
  | { state: "fetching" }
  /** An object URL this module minted and will revoke. */
  | { state: "got"; src: string }
  /**
   * Nothing came back, and the reader says which in one sentence.
   *
   * **The frame's own, never the step's.** A frame the record holds and the
   * disk does not is a 422 about one file; the other frames of that run are
   * still drawn, because a failed read is one frame's business.
   */
  | { state: "absent"; note: string };

/** What a chapter needs: what is held, and how to ask for a step's worth. */
export type Frames = {
  of: (kept: string) => FrameState | undefined;
  /** Ask for every frame in this list that has not been asked for yet. */
  want: (frames: KeptFrame[]) => void;
};

/**
 * Reading one frame, as the screen's caller hands it in.
 *
 * **An argument, not a global**, which is `ReadCheckOutput`'s rule: the bytes
 * come from the process that can reach Fleet, and the screen is handed the way
 * to ask rather than the way to connect.
 */
export type ReadFrame = (jobId: string, kept: string) => Promise<FrameRead>;

/**
 * Hold one Job's fetched frames, and revoke what it minted.
 *
 * The Job id is a dependency rather than an argument, for `useCheckOutputs`'
 * reason: a `kept` is only meaningful under the Job whose `.armada/frames`
 * holds it, and carried into the next Job it would name a file that Job never
 * wrote.
 */
export function useFrames(read: ReadFrame, jobId: string): Frames {
  const [held, setHeld] = useState<Record<string, FrameState>>({});

  // Read through a ref inside the callbacks so an answer landing does not
  // re-create them. The story is rebuilt on every tick of the clock, and a
  // `want` that changed with it would ask again on every second.
  const current = useRef(held);
  current.current = held;

  // **Every object URL this window minted, revoked when the Job changes or the
  // window goes.** Held beside the state rather than read out of it because
  // the cleanup runs after the state has already been replaced — a cleanup
  // that read `held` would be revoking the new Job's URLs, or nothing at all.
  const minted = useRef<string[]>([]);
  useEffect(() => {
    return () => {
      for (const src of minted.current) URL.revokeObjectURL(src);
      minted.current = [];
    };
  }, [jobId]);
  useEffect(() => setHeld({}), [jobId]);

  const want = useCallback(
    (frames: KeptFrame[]) => {
      const asking = frames.filter((frame) => current.current[frame.kept] === undefined);
      if (asking.length === 0) return;
      setHeld((was) => {
        const next = { ...was };
        for (const frame of asking) next[frame.kept] = { state: "fetching" };
        return next;
      });
      for (const frame of asking) {
        const kept = frame.kept;
        void read(jobId, kept).then(
          (answer) => setHeld((was) => ({ ...was, [kept]: drawn(answer, minted) })),
          // A rejected invoke is main gone, which is the window closing.
          // Recorded as an absence so the plate is not left saying `reading…`
          // for the rest of its life.
          () => setHeld((was) => ({ ...was, [kept]: { state: "absent", note: NOT_ANSWERED } })),
        );
      }
    },
    [read, jobId],
  );

  const of = useCallback((kept: string) => current.current[kept], []);
  return { of, want };
}

/**
 * What came back, as the plate will draw it.
 *
 * **The URL is minted here and recorded in the same breath**, so there is one
 * place a `blob:` comes into existence and one list that has to be revoked. A
 * component calling `createObjectURL` in a render would mint one per paint.
 */
function drawn(read: FrameRead, minted: { current: string[] }): FrameState {
  if (!read.ok) {
    const refused = !read.outcome.ok && read.outcome.why === "refused";
    return { state: "absent", note: refused ? NOT_ON_DISK : NOT_ANSWERED };
  }
  // The type is Fleet's, read off the file's own name and sent with `nosniff`.
  // A second opinion composed here would be the one that is wrong.
  const src = URL.createObjectURL(new Blob([read.bytes as BlobPart], { type: read.type }));
  minted.current.push(src);
  return { state: "got", src };
}

/**
 * The row is on the record and the file is not. The 422, in the app's voice.
 *
 * **A frame's whole content is the image**, so this is the sentence for a
 * `.armada/frames` directory that was reclaimed after the record was written —
 * which is the case the refusal was written for.
 */
const NOT_ON_DISK = "This frame is on the record and no longer on disk.";

/** Fleet did not answer. The same sentence the brief and the outputs use. */
const NOT_ANSWERED = "Fleet did not answer for this frame.";

/**
 * What the two sides are called on the screen.
 *
 * **The reader's words and not the wire's.** `base` and `branch` name the two
 * checkouts Fleet had to serve; what a person reading a step wants to know is
 * which of these is how the screen was. The translation happens once, here, so
 * no component learns what a base branch is.
 */
const asBefore = "before";
const asAfter = "after";

/**
 * The rows the record holds, married to what this window has fetched.
 *
 * **Wire order, and no sort.** Fleet answers oldest run first and that ordering
 * is the record's; re-sorting on arrival is the column flip-flop the failure
 * log named, and here it would silently reorder the runs a person is comparing.
 *
 * A frame nothing has asked for yet carries neither `src` nor `why`, which the
 * plate draws as `reading…` — the honest reading, because from where the person
 * is sitting a fetch that has not gone out and one that has not come back are
 * the same wait.
 */
export function shownFrames(frames: KeptFrame[], held: Frames): ShownFrame[] {
  // **Labelled only where there is something to compare against.** A step with
  // frames from one side is a repository with no base, a base run that would
  // not start, or a Fleet older than 9.5 — and `after` written on every frame
  // of a set with no before is a word that says nothing and implies a missing
  // half.
  const both = new Set(frames.map((frame) => frame.side ?? "branch")).size > 1;
  return frames.map((frame) => {
    const state = held.of(frame.kept);
    return {
      kept: frame.kept,
      name: frame.name,
      attempt: frame.attempt,
      weight: weighs(frame.bytes),
      ...(both ? { side: (frame.side ?? "branch") === "base" ? asBefore : asAfter } : {}),
      ...(state?.state === "got" ? { src: state.src } : {}),
      ...(state?.state === "absent" ? { why: state.note } : {}),
    };
  });
}

/**
 * What the chapter's header says about a step's frames.
 *
 * **A count, and the runs it spans where it spans more than one.** Three frames
 * from one run and three frames from three runs are different things to be
 * looking at, and a collapsed chapter that said `3 frames` for both would hide
 * exactly the fact that decides whether what a reader is about to open is
 * current.
 */
export function framesSummary(frames: KeptFrame[]): string | undefined {
  if (frames.length === 0) return undefined;
  const runs = new Set(frames.map((frame) => frame.attempt));
  const counted = frames.length === 1 ? "1 frame" : `${frames.length} frames`;
  const said = runs.size <= 1 ? counted : `${counted} · ${runs.size} runs`;
  // **And whether there is a before, which is the fact that decides what a
  // collapsed chapter is worth opening for.** A set with both sides answers
  // what the change did to the screen; one with only the branch answers what
  // the screen is, which is the reading #209 was written against.
  const both = new Set(frames.map((frame) => frame.side ?? "branch")).size > 1;
  return both ? `${said} · ${asBefore} and ${asAfter}` : said;
}

// ------------------------------------------------------------ the two sides
//
// **Pairing is here rather than in the component**, because what makes two
// frames a pair is a fact about the record — the name, the run, the side and
// the digest — and a component that knew any of those would be the second place
// the rule is written. What crosses is `Paired`, which says only: here is a
// before, here is an after, and whether they are the same picture.

/**
 * One frame's two sides, married by name within one run.
 *
 * **The name is the join and the run is the scope.** A harness names its own
 * frames, so `home.png` from the base and `home.png` from the branch are one
 * screen photographed twice — but only within one attempt. A step worked three
 * times captured three sets, and pairing across runs would put this run's after
 * beside the last run's before, which is a comparison nobody asked for and
 * cannot tell from the real one.
 */
export type Paired = {
  /** The name both sides share, or the one side's where it has no pair. */
  name: string;
  attempt: number;
  /** The base's frame, absent where the change added this screen. */
  before?: ShownFrame;
  /** The branch's frame, absent where the change removed it. */
  after?: ShownFrame;
  /**
   * Whether the two sides carry the same picture, and so may be folded away.
   *
   * **False wherever it cannot be established**, which is the whole of what
   * makes folding safe. One side missing, a frame kept before the digest
   * existed, a size that disagrees — none of those is *the same*, and each one
   * draws.
   */
  same: boolean;
};

/** One side of a pair, with what decides whether it matches the other. */
type Half = { shown: ShownFrame; digest: string; bytes: number };

/**
 * What a step's frames come to, pair by pair.
 *
 * **Wire order.** Fleet answers in the record's order and re-sorting is the
 * column flip-flop the failure log named; a pair takes the position of
 * whichever of its sides the record listed first, so a name that exists only on
 * the base still lands where it belongs rather than at the end.
 *
 * **A side is only ever taken once.** Two frames of one name on one side of one
 * run is a harness that wrote the same file twice, which one directory cannot
 * hold — so the first wins and nothing here has to choose between them.
 */
export function pairedFrames(frames: KeptFrame[], held: Frames): Paired[] {
  const shown = shownFrames(frames, held);
  const order: string[] = [];
  const building = new Map<string, { name: string; attempt: number; before?: Half; after?: Half }>();

  frames.forEach((frame, at) => {
    const key = `${frame.attempt} ${frame.name}`;
    let pair = building.get(key);
    if (pair === undefined) {
      pair = { name: frame.name, attempt: frame.attempt };
      building.set(key, pair);
      order.push(key);
    }
    const side = (frame.side ?? "branch") === "base" ? "before" : "after";
    if (pair[side] !== undefined) return;
    pair[side] = {
      shown: shown[at]!,
      digest: frame.digest ?? "",
      bytes: frame.bytes,
    };
  });

  return order.map((key) => {
    const pair = building.get(key)!;
    return {
      name: pair.name,
      attempt: pair.attempt,
      ...(pair.before === undefined ? {} : { before: pair.before.shown }),
      ...(pair.after === undefined ? {} : { after: pair.after.shown }),
      same: theSamePicture(pair.before, pair.after),
    };
  });
}

/**
 * Whether a pair is the same picture twice.
 *
 * **Every unknown answers no, and that is the design.** Folding is the only act
 * on this surface that can hide something, so it is taken only where both sides
 * are present, both carry a digest, and the digest *and* the byte count agree.
 * Anything else draws — at worst a pair a reader glances past, which is what
 * the surface did before any of this existed.
 *
 * **The size is compared beside the digest** rather than trusted to it. Sixty-
 * four bits over data nobody is choosing adversarially is not a risk anybody
 * meets, and the count is already on the row: two comparisons that must both
 * hold cost nothing and remove the one failure that would be silent.
 *
 * **An empty digest never matches, including another empty.** A frame kept
 * before the field existed carries none, and two absences reading as agreement
 * would fold away exactly the old Jobs nobody can photograph again.
 */
function theSamePicture(before: Half | undefined, after: Half | undefined): boolean {
  if (before === undefined || after === undefined) return false;
  if (before.digest === "" || after.digest === "") return false;
  return before.digest === after.digest && before.bytes === after.bytes;
}

/**
 * What a chapter says about a set that has two sides.
 *
 * **The count that matters is what moved, not what was taken.** Ten screens
 * photographed twice is twenty images and one sentence worth reading: how many
 * of the ten are not the same picture. A summary that said `20 frames` would be
 * counting the work rather than the answer.
 */
export function pairedSummary(pairs: Paired[]): string | undefined {
  if (pairs.length === 0) return undefined;
  const moved = pairs.filter((pair) => !pair.same).length;
  const runs = new Set(pairs.map((pair) => pair.attempt));
  const said =
    moved === 0
      ? "nothing moved"
      : moved === pairs.length
        ? `${moved} ${moved === 1 ? "frame" : "frames"}`
        : `${moved} of ${pairs.length} changed`;
  return runs.size <= 1 ? said : `${said} · ${runs.size} runs`;
}

/**
 * A holder that has nothing and asks for nothing.
 *
 * **For a caller whose subject is not the frames** — a test of the chapter
 * ordering, a story of a step that captured none. It is not a fallback: a
 * screen that reached for this rather than being handed a real one would draw
 * every frame as `reading…` forever, which is why `chaptersOf` requires the
 * argument rather than defaulting to it.
 */
export const NO_FRAMES: Frames = { of: () => undefined, want: () => {} };

/** What a file weighs, at the precision a person reads rather than counts. */
function weighs(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
