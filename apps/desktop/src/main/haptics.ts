// Playing a trackpad pattern from outside the renderer, which cannot reach one.
//
// **The seam names the two patterns and not how they are played.** A `Performer`
// takes a pattern and returns at once. The one here writes one pattern name a
// line to a long-running JXA script; a Swift helper reading the same lines on
// stdin replaces it by implementing `Performer` and changing `pickPerformer`,
// with no caller changed. The rule is `docs/contracts/design-system.md`, Touch.

import { spawn } from "node:child_process";

import { CHANNELS } from "../shared/bridge";
import { isPattern } from "../shared/haptics";
import type { Pattern } from "../shared/haptics";

export type Performer = {
  /** Play one pattern. Never waits, and a pattern that cannot be played is lost and nothing else. */
  perform: (pattern: Pattern) => void;
  dispose: () => void;
};

/** What the script performer needs of a child process, so a test can hand it a fake. */
export type Child = {
  stdin: {
    write: (line: string) => unknown;
    end: () => unknown;
    on: (event: "error", listener: (error: Error) => void) => unknown;
  } | null;
  on: (event: "exit" | "error", listener: () => void) => unknown;
};

/**
 * `NSHapticFeedbackManager` from JXA, one pattern name a line on stdin. It exits
 * on EOF, so Bridge quitting ends it. AppKit's patterns: Generic 0, Alignment 1,
 * LevelChange 2; performance time Now is 1.
 */
export const SCRIPT = `
ObjC.import("AppKit");
ObjC.import("Foundation");
const input = $.NSFileHandle.fileHandleWithStandardInput;
const performer = $.NSHapticFeedbackManager.defaultPerformer;
const APPKIT = { alignment: 1, level_change: 2 };
let partial = "";
while (true) {
  const data = input.availableData;
  if (data.length == 0) break;
  partial += $.NSString.alloc.initWithDataEncoding(data, $.NSUTF8StringEncoding).js;
  const lines = partial.split("\\n");
  partial = lines.pop();
  for (const line of lines) {
    const pattern = APPKIT[line.trim()];
    if (pattern !== undefined) performer.performFeedbackPatternPerformanceTime(pattern, 1);
  }
}
`;

/**
 * Starts the script on the first tap rather than at launch, and again on the
 * tap after it has died. Measured warm at 0.2ms a tap against ~140ms for an
 * `osascript` per tap, which is launch alone.
 */
export class ScriptPerformer implements Performer {
  private child: Child | null = null;

  constructor(private readonly start: () => Child) {}

  perform(pattern: Pattern): void {
    const child = this.running();
    try {
      child?.stdin?.write(`${pattern}\n`);
    } catch {
      this.lose(child);
    }
  }

  dispose(): void {
    const child = this.child;
    this.child = null;
    try {
      child?.stdin?.end();
    } catch {
      // Already gone, which is what ending it was for.
    }
  }

  private running(): Child | null {
    if (this.child !== null) return this.child;
    let child: Child;
    try {
      child = this.start();
    } catch {
      return null;
    }
    this.child = child;
    child.on("exit", () => this.lose(child));
    child.on("error", () => this.lose(child));
    // A write to a script that has just died is EPIPE, raised here and not at the write.
    child.stdin?.on("error", () => this.lose(child));
    return child;
  }

  /** Forget a child that died, unless a newer one has already replaced it. */
  private lose(child: Child | null): void {
    if (this.child === child) this.child = null;
  }
}

const NOTHING: Performer = { perform: () => undefined, dispose: () => undefined };

/** The one performer, chosen once. Only macOS has a trackpad this can reach. */
export function pickPerformer(platform: NodeJS.Platform = process.platform): Performer {
  if (platform !== "darwin") return NOTHING;
  return new ScriptPerformer(() =>
    spawn("osascript", ["-l", "JavaScript", "-e", SCRIPT], { stdio: ["pipe", "ignore", "ignore"] }),
  );
}

/** The narrow part of `ipcMain` and `app` this registers on. */
export type Hosts = {
  ipc: { on: (channel: string, listener: (event: unknown, pattern: unknown) => void) => unknown };
  app: { on: (event: "will-quit", listener: () => void) => unknown };
};

/** Listen for the renderer's taps, and end the script as Bridge quits. */
export function handleTaps({ ipc, app }: Hosts, performer: Performer = pickPerformer()): void {
  ipc.on(CHANNELS.tap, (_event, pattern) => {
    if (isPattern(pattern)) performer.perform(pattern);
  });
  app.on("will-quit", () => performer.dispose());
}
