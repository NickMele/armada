// The pictures the Notes on one Studio kept — #1352. `frames.ts`'s shape, for a
// Studio: the bytes cross the preload, become a `blob:` this module owns, and
// every one it minted is revoked when the Studio changes or the window goes.
//
// **Held here rather than fetched in a component** for exactly that reason. An
// object URL is held by the document until it is revoked, so a person walking
// six Studios would leave six boards of screenshots in memory.

import { useCallback, useEffect, useRef } from "react";

import { useHeldReads } from "./held-reads";

import type { StudioNodeFrame } from "@armada/components";
import type { FrameRead } from "@armada/protocol";

/**
 * Reading one Note's frame, as the screen's caller hands it in. **An argument
 * and not a global**, which is `ReadFrame`'s rule: the bytes come from the
 * process that can reach Fleet, and a story passes its own.
 */
export type ReadStudioFrame = (studioId: string, nodeId: string) => Promise<FrameRead>;

/** What one Studio's frames come to: what is held for a node, and how to ask. */
export type StudioFrames = {
  /** What to draw on a node, or `undefined` where nothing has asked for it. */
  of: (nodeId: string) => StudioNodeFrame | undefined;
  /** Ask for every node in this list that has not been asked for yet. */
  want: (nodeIds: readonly string[]) => void;
};

/**
 * Hold one Studio's fetched frames, and revoke what it minted.
 *
 * The Studio id is a dependency rather than an argument, for `useFrames`'
 * reason: a node id is only meaningful under the Studio whose directory holds
 * its frame.
 */
export function useStudioFrames(read: ReadStudioFrame, studioId: string): StudioFrames {
  // An effect, because a `blob:` is held by the document rather than by React,
  // and synchronising with something outside React is what an effect is for.
  const minted = useRef<string[]>([]);
  useEffect(() => {
    return () => {
      for (const src of minted.current) URL.revokeObjectURL(src);
      minted.current = [];
    };
  }, [studioId]);

  const { of, fetch } = useHeldReads<FrameRead, StudioNodeFrame>({
    read,
    // `useHeldReads` calls its scope `jobId`; here the scope is the Studio.
    jobId: studioId,
    settle: (answer) => drawn(answer, minted),
    asking: READING,
    // A rejected invoke is main gone, which is the window closing.
    failed: { why: NOT_ANSWERED },
  });

  const want = useCallback(
    (nodeIds: readonly string[]) => {
      for (const nodeId of nodeIds) fetch(nodeId);
    },
    [fetch],
  );

  return { of, want };
}

/**
 * What came back, as the node will draw it. **The URL is minted here and
 * recorded in the same breath**, so there is one place a `blob:` comes into
 * existence and one list to revoke.
 */
function drawn(read: FrameRead, minted: { current: string[] }): StudioNodeFrame {
  if (!read.ok) {
    const refused = !read.outcome.ok && read.outcome.why === "refused";
    return { why: refused ? NOT_ON_DISK : NOT_ANSWERED };
  }
  if (!read.type.startsWith("image/")) return { why: CANNOT_DRAW };
  const src = URL.createObjectURL(new Blob([read.bytes as BlobPart], { type: read.type }));
  minted.current.push(src);
  return { src };
}

/** Asked for, nothing back yet. The node draws its box and says it is reading. */
const READING: StudioNodeFrame = {};

/**
 * The Note names a frame and the file is not there. The 422, in the app's
 * voice: a Studio is kept until a person deletes it, and a directory swept off
 * the disk under it is the case this sentence was written for.
 */
const NOT_ON_DISK = "This frame is on the Note and no longer on disk.";

/** Fleet did not answer. The same sentence a step's frames use. */
const NOT_ANSWERED = "Fleet did not answer for this frame.";

/** Bytes that are not an image. Nothing writes one, so this says so plainly. */
const CANNOT_DRAW = "Bridge does not know how to draw this kind of file.";
