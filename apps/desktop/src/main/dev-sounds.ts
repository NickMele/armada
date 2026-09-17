// The two tones, where a development Bridge's notifications can find them.
//
// A notification names its sound, and macOS looks the name up in the running
// app's bundle and then in `~/Library/Sounds`. A packaged Armada.app carries
// both files in its Resources; `electron-vite preview` runs Electron's own
// bundle, which carries neither, so a dev Bridge copies them here instead.
// Measured 17 Sep 2026: `NSSound.soundNamed` resolved a name only once its
// file sat in `~/Library/Sounds`. A packaged Bridge never calls this.

import { copyFileSync, mkdirSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

/**
 * Copy every `.wav` under `from` into `into`, skipping a file already there
 * byte for byte. Returns what was written.
 *
 * **A failure costs the tone and nothing else**, the same as a refused
 * notification, so it is swallowed rather than stopping Bridge from opening.
 */
export function installSounds(from: string, into: string): string[] {
  const written: string[] = [];
  try {
    mkdirSync(into, { recursive: true });
    for (const name of readdirSync(from).filter((file) => file.endsWith(".wav"))) {
      const source = join(from, name);
      const target = join(into, name);
      if (same(source, target)) continue;
      copyFileSync(source, target);
      written.push(name);
    }
  } catch {
    // Unreadable source or unwritable home: the notification falls back to macOS's chime.
  }
  return written;
}

function same(source: string, target: string): boolean {
  try {
    return readFileSync(source).equals(readFileSync(target));
  } catch {
    return false;
  }
}
