// What one Job's readers have fetched, by key, and nothing about what it means.
//
// **Three reads were the same thirty lines.** A call's arguments, a Check's
// output and a step's frames are each fetched when somebody asks, asked for
// once however often the story is rebuilt, and dropped when the Job changes —
// `calls.ts`, `outputs.ts` and `frames.ts` each spelled that out again. What
// differs is what an answer draws as, and that stays in each file.
//
// **Held for one Job, and the Job is part of what is held.** A key names a row
// under the Job it was read for, so a map for one Job reads as empty under the
// next — during render, rather than from an effect a frame later — and an
// answer that lands after the Job changed is dropped rather than written in.

import { useCallback, useRef, useState } from "react";

/** What a reader needs: what is held for a key, and how to ask for one. */
export type HeldReads<State> = {
  of: (key: string) => State | undefined;
  /** Ask for a key. A second ask while one is out, or once answered, sends nothing. */
  fetch: (key: string) => void;
  /** Hold an answer that needs no round trip, where the reader already has it. */
  hold: (key: string, state: State) => void;
};

export type HeldReadsOf<Read, State> = {
  /** The round trip, as the screen's caller hands it in. */
  read: (jobId: string, key: string) => Promise<Read>;
  jobId: string;
  /** What an answer draws as. */
  settle: (read: Read) => State;
  /** What a key reads as while it is out. */
  asking: State;
  /** What a rejected round trip reads as — main gone, which is the window closing. */
  failed: State;
};

type Held<State> = { jobId: string; keys: Record<string, State> };

export function useHeldReads<Read, State>({
  read,
  jobId,
  settle,
  asking,
  failed,
}: HeldReadsOf<Read, State>): HeldReads<State> {
  const [held, setHeld] = useState<Held<State>>({ jobId, keys: {} });
  const mine = held.jobId === jobId ? held.keys : NOTHING;

  // Read through refs inside the callbacks, so an answer landing does not
  // re-create them: the story is rebuilt on every tick of the clock, and a
  // callback that changed with it would ask again every second.
  const current = useRef(mine);
  current.current = mine;
  const settling = useRef({ settle, asking, failed });
  settling.current = { settle, asking, failed };

  const hold = useCallback(
    (key: string, state: State) =>
      setHeld((was) =>
        was.jobId === jobId
          ? { jobId, keys: { ...was.keys, [key]: state } }
          : { jobId, keys: { [key]: state } },
      ),
    [jobId],
  );

  const fetch = useCallback(
    (key: string) => {
      if (current.current[key] !== undefined) return;
      current.current = { ...current.current, [key]: settling.current.asking };
      hold(key, settling.current.asking);
      // An answer for a Job that is no longer open is nobody's, so it is
      // dropped rather than held under the Job that replaced it.
      const land = (state: State) =>
        setHeld((was) => (was.jobId === jobId ? { jobId, keys: { ...was.keys, [key]: state } } : was));
      void read(jobId, key).then(
        (answer) => land(settling.current.settle(answer)),
        () => land(settling.current.failed),
      );
    },
    [read, jobId, hold],
  );

  const of = useCallback((key: string) => current.current[key], []);
  return { of, fetch, hold };
}

const NOTHING: Record<string, never> = {};
