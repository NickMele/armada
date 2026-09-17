// The two notification tones, written as WAV files beside this script.
//
// The shape of each is `docs/contracts/design-system.md`, Sound. The files are
// checked in, because electron-builder copies them into the bundle and a build
// should not synthesise audio. The names are the ones `src/main/tones.ts` hands
// to a notification's `sound`, and `tones.test.ts` holds the two to each other.
//
// `node sounds/tones.mjs` rewrites both. `--check` writes nothing and fails
// naming a file that is not what a fresh run would produce.

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const RATE = 44100;
const NOTE_MS = 200;
const ATTACK_MS = 12;
/** Where the decay has reached by the note's end: -60 dB, so cutting it there does not click. */
const FLOOR = 0.001;
/** Peak, as a fraction of full scale. Both tones share it: loudness is macOS's setting. */
const LEVEL = 0.5;

/** Equal temperament from A4 = 440 Hz. */
const hz = (semitonesFromA4) => 440 * 2 ** (semitonesFromA4 / 12);
const A4 = hz(0);
const G4 = hz(-2);
const D4 = hz(-7);

const TONES = {
  "armada-blocked": [
    { at: 0, hz: G4 },
    { at: 170, hz: D4 },
  ],
  "armada-waiting": [{ at: 0, hz: A4 }],
};

function envelope(ms) {
  if (ms < ATTACK_MS) return ms / ATTACK_MS;
  const tau = (NOTE_MS - ATTACK_MS) / Math.log(1 / FLOOR);
  return Math.exp(-(ms - ATTACK_MS) / tau);
}

function render(notes) {
  const endMs = Math.max(...notes.map((note) => note.at + NOTE_MS));
  const samples = new Float64Array(Math.round((endMs / 1000) * RATE));
  for (const note of notes) {
    const start = Math.round((note.at / 1000) * RATE);
    const length = Math.round((NOTE_MS / 1000) * RATE);
    for (let i = 0; i < length && start + i < samples.length; i += 1) {
      const seconds = i / RATE;
      samples[start + i] += Math.sin(2 * Math.PI * note.hz * seconds) * envelope(seconds * 1000);
    }
  }
  const peak = samples.reduce((most, value) => Math.max(most, Math.abs(value)), 0);
  return samples.map((value) => (value / peak) * LEVEL);
}

/** 16-bit mono PCM, the one format every sound lookup on macOS reads. */
function wav(samples) {
  const data = samples.length * 2;
  const out = Buffer.alloc(44 + data);
  out.write("RIFF", 0, "ascii");
  out.writeUInt32LE(36 + data, 4);
  out.write("WAVE", 8, "ascii");
  out.write("fmt ", 12, "ascii");
  out.writeUInt32LE(16, 16);
  out.writeUInt16LE(1, 20);
  out.writeUInt16LE(1, 22);
  out.writeUInt32LE(RATE, 24);
  out.writeUInt32LE(RATE * 2, 28);
  out.writeUInt16LE(2, 32);
  out.writeUInt16LE(16, 34);
  out.write("data", 36, "ascii");
  out.writeUInt32LE(data, 40);
  samples.forEach((value, i) => out.writeInt16LE(Math.round(value * 32767), 44 + i * 2));
  return out;
}

const here = dirname(fileURLToPath(import.meta.url));
const check = process.argv.includes("--check");
let stale = 0;
for (const [name, notes] of Object.entries(TONES)) {
  const path = join(here, `${name}.wav`);
  const bytes = wav(render(notes));
  if (!check) {
    writeFileSync(path, bytes);
    console.log(`wrote ${name}.wav, ${bytes.length} bytes`);
    continue;
  }
  let held = null;
  try {
    held = readFileSync(path);
  } catch {
    // Missing reads as stale, below.
  }
  if (held === null || !held.equals(bytes)) {
    stale += 1;
    console.error(`stale: ${name}.wav is not what this script writes. Run it without --check`);
  }
}
if (check && stale === 0) console.log("both tones are what this script writes");
process.exitCode = stale === 0 ? 0 : 1;
