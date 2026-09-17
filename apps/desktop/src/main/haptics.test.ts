// The script performer against a fake child process: when the script starts,
// what it is written, and what happens when it has died. What the trackpad
// feels is AppKit's, and no test here can say it.

import { describe, expect, it } from "vitest";

import { CHANNELS } from "../shared/bridge";
import { handleTaps, pickPerformer, ScriptPerformer } from "./haptics";
import type { Child, Performer } from "./haptics";

type Fake = Child & { lines: string[]; ended: boolean; die: () => void; breakPipe: () => void };

function fakeChild(): Fake {
  const exits: (() => void)[] = [];
  const pipeErrors: ((error: Error) => void)[] = [];
  const fake: Fake = {
    lines: [],
    ended: false,
    stdin: {
      write: (line) => fake.lines.push(line),
      end: () => {
        fake.ended = true;
      },
      on: (_event, listener) => pipeErrors.push(listener),
    },
    on: (_event, listener) => exits.push(listener),
    die: () => exits.forEach((exit) => exit()),
    breakPipe: () => pipeErrors.forEach((listener) => listener(new Error("EPIPE"))),
  };
  return fake;
}

function performer(): { performer: ScriptPerformer; started: Fake[] } {
  const started: Fake[] = [];
  return {
    started,
    performer: new ScriptPerformer(() => {
      const child = fakeChild();
      started.push(child);
      return child;
    }),
  };
}

describe("the script", () => {
  it("does not start until the first tap", () => {
    const { performer: taps, started } = performer();
    expect(started).toHaveLength(0);
    taps.perform("alignment");
    expect(started).toHaveLength(1);
  });

  it("is written one pattern a line, and started once for many taps", () => {
    const { performer: taps, started } = performer();
    taps.perform("alignment");
    taps.perform("level_change");
    taps.perform("alignment");
    expect(started).toHaveLength(1);
    expect(started[0]?.lines).toEqual(["alignment\n", "level_change\n", "alignment\n"]);
  });

  it("starts again on the tap after it died, and loses only the tap it missed", () => {
    const { performer: taps, started } = performer();
    taps.perform("alignment");
    started[0]?.die();
    taps.perform("level_change");
    expect(started).toHaveLength(2);
    expect(started[1]?.lines).toEqual(["level_change\n"]);
  });

  it("starts again after a write to a dead script broke the pipe", () => {
    const { performer: taps, started } = performer();
    taps.perform("alignment");
    started[0]?.breakPipe();
    taps.perform("alignment");
    expect(started).toHaveLength(2);
  });

  it("does not forget a live script when an older one's exit arrives late", () => {
    const { performer: taps, started } = performer();
    taps.perform("alignment");
    const first = started[0]!;
    first.breakPipe();
    taps.perform("alignment");
    first.die();
    taps.perform("alignment");
    expect(started).toHaveLength(2);
    expect(started[1]?.lines).toHaveLength(2);
  });

  it("loses the tap and nothing else when the script cannot start", () => {
    const taps = new ScriptPerformer(() => {
      throw new Error("osascript: not found");
    });
    expect(() => taps.perform("alignment")).not.toThrow();
  });

  it("ends the script's input on dispose, which is what makes it exit", () => {
    const { performer: taps, started } = performer();
    taps.perform("alignment");
    taps.dispose();
    expect(started[0]?.ended).toBe(true);
    taps.perform("alignment");
    expect(started).toHaveLength(2);
  });

  it("is never started on a machine that is not a Mac", () => {
    const nothing = pickPerformer("linux");
    expect(() => nothing.perform("alignment")).not.toThrow();
  });
});

describe("the channel", () => {
  function hosted(): { send: (pattern: unknown) => void; quit: () => void; played: string[]; disposed: () => boolean } {
    let listener: (event: unknown, pattern: unknown) => void = () => undefined;
    let quitting: () => void = () => undefined;
    const played: string[] = [];
    let disposed = false;
    const recording: Performer = {
      perform: (pattern) => played.push(pattern),
      dispose: () => {
        disposed = true;
      },
    };
    handleTaps(
      {
        ipc: {
          on: (channel, handler) => {
            if (channel === CHANNELS.tap) listener = handler;
          },
        },
        app: { on: (_event, handler) => (quitting = handler) },
      },
      recording,
    );
    return { send: (pattern) => listener({}, pattern), quit: () => quitting(), played, disposed: () => disposed };
  }

  it("plays the two patterns and nothing else the renderer sends", () => {
    const { send, played } = hosted();
    send("alignment");
    send("level_change");
    send("generic");
    send({ pattern: "alignment" });
    expect(played).toEqual(["alignment", "level_change"]);
  });

  it("disposes the performer as Bridge quits", () => {
    const { quit, disposed } = hosted();
    quit();
    expect(disposed()).toBe(true);
  });
});
