import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";

import { installSounds } from "./dev-sounds";

const made: string[] = [];
function dir(): string {
  const path = mkdtempSync(join(tmpdir(), "armada-sounds-"));
  made.push(path);
  return path;
}
afterEach(() => {
  for (const path of made.splice(0)) rmSync(path, { recursive: true, force: true });
});

describe("installing the tones for a dev Bridge", () => {
  it("copies the shipped tones, and only the .wav files", () => {
    const into = join(dir(), "Sounds");
    const from = join(__dirname, "../../sounds");
    expect(installSounds(from, into).sort()).toEqual(["armada-blocked.wav", "armada-waiting.wav"]);
    expect(readdirSync(into).sort()).toEqual(["armada-blocked.wav", "armada-waiting.wav"]);
  });

  it("writes nothing when the same bytes are already there", () => {
    const from = dir();
    const into = dir();
    writeFileSync(join(from, "armada-waiting.wav"), "tone");
    installSounds(from, into);
    expect(installSounds(from, into)).toEqual([]);
  });

  it("replaces a tone that has changed since it was installed", () => {
    const from = dir();
    const into = dir();
    writeFileSync(join(from, "armada-waiting.wav"), "new");
    writeFileSync(join(into, "armada-waiting.wav"), "old");
    expect(installSounds(from, into)).toEqual(["armada-waiting.wav"]);
    expect(readFileSync(join(into, "armada-waiting.wav"), "utf8")).toBe("new");
  });

  it("gives up quietly when there is nothing to read", () => {
    expect(installSounds(join(dir(), "missing"), dir())).toEqual([]);
  });
});
