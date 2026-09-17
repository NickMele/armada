// The files the annotation layer writes, #1226. Dev only.
//
// **No Electron import, on purpose.** The vite plugin in
// `annotations-server.ts` runs this in a plain Node process beside a dev
// server, and a module that pulled in `electron` would not load there. What
// main needs from Electron it passes in.
//
// Main registers these handlers only when `app.isPackaged` is false, and the
// preload exposes `window.armadaDev` only when main passed `ANNOTATE_FLAG`. A
// packaged Bridge has neither end of the channel.

import { existsSync } from "node:fs";
import { mkdir, readdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";

import {
  ANNOTATION_CHANNELS,
  ANNOTATIONS_DIR,
  byCreation,
  isAnnotation,
  isAnnotationId,
  serializeAnnotation,
  type Annotation,
} from "../shared/annotations";

/**
 * The repository root above `start`: the first directory holding both
 * `pnpm-workspace.yaml` and `Cargo.toml`. In a worktree that is the worktree,
 * so notes left on a worktree's Bridge land in that worktree.
 */
export function repositoryRoot(start: string): string | null {
  let at = resolve(start);
  for (;;) {
    if (existsSync(join(at, "pnpm-workspace.yaml")) && existsSync(join(at, "Cargo.toml"))) return at;
    const up = dirname(at);
    if (up === at) return null;
    at = up;
  }
}

export function annotationsDir(root: string): string {
  return join(root, ...ANNOTATIONS_DIR);
}

/**
 * Every note in the directory, oldest first. A file that does not parse or is
 * not a note is skipped rather than failing the list — one hand-edited file
 * must not take every pin off the screen.
 */
export async function listAnnotations(dir: string): Promise<Annotation[]> {
  let names: string[];
  try {
    names = await readdir(dir);
  } catch {
    return [];
  }
  const notes: Annotation[] = [];
  for (const name of names) {
    if (!name.endsWith(".json")) continue;
    try {
      const parsed: unknown = JSON.parse(await readFile(join(dir, name), "utf8"));
      if (isAnnotation(parsed) && `${parsed.id}.json` === name) notes.push(parsed);
    } catch {
      // Unreadable or half-written; skipped, see above.
    }
  }
  return notes.sort(byCreation);
}

/** Writes one note to `<id>.json`, through a rename so a reader never sees half of it. */
export async function saveAnnotation(dir: string, note: unknown): Promise<void> {
  if (!isAnnotation(note)) throw new Error("not an annotation");
  await mkdir(dir, { recursive: true });
  const path = join(dir, `${note.id}.json`);
  const partial = `${path}.partial`;
  await writeFile(partial, serializeAnnotation(note), "utf8");
  await rename(partial, path);
}

export async function removeAnnotation(dir: string, id: unknown): Promise<void> {
  if (!isAnnotationId(id)) throw new Error("not an annotation id");
  await rm(join(dir, `${id}.json`), { force: true });
}

/** The part of `ipcMain` this uses, so the module needs no Electron to load. */
type Handles = {
  handle: (channel: string, listener: (event: unknown, ...args: unknown[]) => unknown) => void;
};

/**
 * Registers the three channels against the repository above `appPath`. Returns
 * the directory, or null when no repository was found — a Bridge run from
 * somewhere that is not this repository has nowhere to write, and says so.
 */
export function handleAnnotations(ipc: Handles, appPath: string): string | null {
  const root = repositoryRoot(appPath);
  if (root === null) {
    console.warn(`annotations: no repository above ${appPath}; the layer cannot save`);
    return null;
  }
  const dir = annotationsDir(root);
  ipc.handle(ANNOTATION_CHANNELS.list, () => listAnnotations(dir));
  ipc.handle(ANNOTATION_CHANNELS.save, (_event, note) => saveAnnotation(dir, note));
  ipc.handle(ANNOTATION_CHANNELS.remove, (_event, id) => removeAnnotation(dir, id));
  return dir;
}
