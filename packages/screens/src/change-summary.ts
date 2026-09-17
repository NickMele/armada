// The Job's real diff as the Produced panel draws it: by folder, biggest change
// first. #1187.
//
// **Grouped by where a file sits, and nothing cleverer.** A file's folder is
// its own directory, except where that directory holds only this one changed
// file and a directory above it holds others: then it joins that one, named by
// the rest of its path. `crates/fleet/src/daemon/fittings.rs` beside four files
// in `crates/fleet/src` reads `daemon/fittings.rs` under it, as the frames drew.
import type { ChangedFile, ProducedFolder, ProducedFile } from "@armada/components";

/** The folder the top of the repository is drawn under. */
export const TOP = ".";

/** Directories are compared by string, so this is the whole of a path's parent. */
function parentOf(path: string): string {
  const at = path.lastIndexOf("/");
  return at < 0 ? TOP : path.slice(0, at);
}

/**
 * Which folder each path is drawn under, keyed by path.
 *
 * **One level of joining**, never a chain: a directory joins its nearest
 * ancestor that holds changed files directly, and that ancestor stays itself.
 */
export function foldersOf(paths: readonly string[]): Map<string, string> {
  const holding = new Map<string, number>();
  for (const path of paths) holding.set(parentOf(path), (holding.get(parentOf(path)) ?? 0) + 1);
  const folders = new Map<string, string>();
  for (const path of paths) {
    const own = parentOf(path);
    if ((holding.get(own) ?? 0) > 1 || own === TOP) {
      folders.set(path, own);
      continue;
    }
    let above = own;
    let joined: string | undefined;
    while (above !== TOP) {
      above = parentOf(above);
      if (holding.has(above) && above !== TOP) {
        joined = above;
        break;
      }
    }
    folders.set(path, joined ?? own);
  }
  return folders;
}

/** A path as it reads under its folder — `daemon/fittings.rs`. */
export function nameUnder(path: string, folder: string): string {
  return folder === TOP ? path : path.slice(folder.length + 1);
}

/** Lines gained and lost, or nothing where none of them was counted. */
function sizeOf(file: ChangedFile): number | undefined {
  return file.added === undefined && file.deleted === undefined
    ? undefined
    : (file.added ?? 0) + (file.deleted ?? 0);
}

/**
 * Biggest first. **A file nothing counted goes last**, rather than reading as
 * the smallest change: absent is not zero. Ties keep path order.
 */
function bySize(a: { size?: number; path: string }, b: { size?: number; path: string }): number {
  if (a.size === undefined || b.size === undefined) {
    if (a.size !== b.size) return a.size === undefined ? 1 : -1;
    return a.path.localeCompare(b.path);
  }
  return b.size - a.size || a.path.localeCompare(b.path);
}

/** A total over the files that carried one, or nothing where none did. */
function total(files: readonly ChangedFile[], of: (file: ChangedFile) => number | undefined) {
  const counted = files.map(of).filter((n): n is number => n !== undefined);
  return counted.length === 0 ? undefined : counted.reduce((a, b) => a + b, 0);
}

/**
 * The summary: the biggest `most` files, grouped by folder, folders biggest
 * first, and how many were left out.
 *
 * **A folder's total is over every file in it**, drawn or not, so the numbers
 * beside a folder are never shrunk by the cut.
 */
export function changeSummaryOf(
  files: readonly ChangedFile[],
  most: number,
): { folders: ProducedFolder[]; more: number } {
  const folderOf = foldersOf(files.map((file) => file.path));
  const drawn = [...files]
    .map((file) => ({ file, size: sizeOf(file), path: file.path }))
    .sort(bySize)
    .slice(0, most);
  const byFolder = new Map<string, ProducedFile[]>();
  for (const { file } of drawn) {
    const folder = folderOf.get(file.path) ?? TOP;
    const row: ProducedFile = {
      path: file.path,
      name: nameUnder(file.path, folder),
      change: file.change,
      ...(file.added === undefined ? {} : { added: file.added }),
      ...(file.deleted === undefined ? {} : { deleted: file.deleted }),
      ...(file.outsidePlan === true ? { outsidePlan: true } : {}),
    };
    byFolder.set(folder, [...(byFolder.get(folder) ?? []), row]);
  }
  const folders = [...byFolder.entries()].map(([path, rows]) => {
    const inIt = files.filter((file) => (folderOf.get(file.path) ?? TOP) === path);
    const added = total(inIt, (file) => file.added);
    const deleted = total(inIt, (file) => file.deleted);
    return {
      path,
      ...(added === undefined ? {} : { added }),
      ...(deleted === undefined ? {} : { deleted }),
      files: rows,
      size: added === undefined && deleted === undefined ? undefined : (added ?? 0) + (deleted ?? 0),
    };
  });
  folders.sort(bySize);
  return {
    folders: folders.map(({ size: _size, ...folder }) => folder),
    more: files.length - drawn.length,
  };
}

/** `and 6 more files`, or nothing where every file is drawn. */
export function moreSaid(more: number): string | undefined {
  if (more <= 0) return undefined;
  return `and ${more} more ${more === 1 ? "file" : "files"}, in the diff`;
}
