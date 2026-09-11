// A reviewer's running commentary on a story, read by an agent session rather
// than by another person — so the record is a JSON line per note, not prose.
import { existsSync, mkdirSync, appendFileSync, readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { spawnSync } from "node:child_process";
import type { Plugin } from "vite";
import type { Picked } from "./picker-types.ts";

export interface NotePayload {
  story: string;
  titled: string;
  note: string;
  // Absent for a note against the whole story — the picker (`picker.ts`)
  // is what fills this in, never required.
  picked?: Picked;
}

export interface NoteRecord extends NotePayload {
  at: string;
  // The screenshot pasted with the note, as a path from the worktree root.
  // The image itself lands under `.notes/shots/`, ignored with the rest of
  // `.notes/`, and never inside the JSON line.
  shot?: string;
}

// Storybook runs with cwd `packages/components`, several levels below the
// worktree root a note has to land in. A worktree's `.git` is a file (it
// points at `.git/worktrees/<name>` in the checkout above); a plain
// checkout's is a directory — testing existence rather than type is what
// makes the same walk work for both, so a second worktree needs no
// configuration of its own.
export function findWorktreeRoot(startDir: string): string {
  let dir = startDir;
  for (;;) {
    if (existsSync(join(dir, ".git"))) return dir;
    const parent = dirname(dir);
    if (parent === dir) {
      throw new Error(`No .git found walking up from ${startDir}`);
    }
    dir = parent;
  }
}

function notesFile(worktreeRoot: string): string {
  return join(worktreeRoot, ".notes", "storybook.jsonl");
}

// The `.gitignore` line is necessary but not sufficient — a reviewer's note
// names whatever they were looking at, and this repository is public. Asking
// git directly, rather than trusting the file, catches the ignore rule being
// edited or missing in a worktree cut before it existed.
function assertNotesIgnored(worktreeRoot: string, file: string): void {
  const result = spawnSync("git", ["check-ignore", "--quiet", file], {
    cwd: worktreeRoot,
  });
  if (result.status === 0) return;
  if (result.status === 1) {
    throw new Error(
      `${file} is not gitignored. Add ".notes/" to .gitignore before ` +
        "the Notes panel can write anything — this repository is public.",
    );
  }
  throw new Error(
    `git check-ignore failed (exit ${result.status}): ${result.stderr?.toString() ?? ""}`,
  );
}

function readNotesFor(file: string, story: string): NoteRecord[] {
  if (!existsSync(file)) return [];
  return readFileSync(file, "utf8")
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line) as NoteRecord)
    .filter((record) => record.story === story);
}

// A pasted screenshot as the panel sends it: a data URL, decoded here and
// written beside the notes file.
const SHOT = /^data:image\/(png|jpeg|webp|gif);base64,([A-Za-z0-9+/=]+)$/;
const SHOT_LIMIT = 15 * 1024 * 1024;
const SHOT_TYPES: Record<string, string> = {
  png: "image/png",
  jpg: "image/jpeg",
  webp: "image/webp",
  gif: "image/gif",
};

function shotsDir(worktreeRoot: string): string {
  return join(worktreeRoot, ".notes", "shots");
}

function saveShot(worktreeRoot: string, dataUrl: string): string {
  const match = SHOT.exec(dataUrl);
  if (match === null) throw new Error("a screenshot has to be a PNG, JPEG, WebP or GIF");
  const bytes = Buffer.from(match[2] ?? "", "base64");
  if (bytes.length > SHOT_LIMIT) throw new Error("a screenshot has to be under 15 MB");
  const ext = match[1] === "jpeg" ? "jpg" : (match[1] ?? "png");
  const name = `${new Date().toISOString().replace(/[:.]/g, "-")}.${ext}`;
  mkdirSync(shotsDir(worktreeRoot), { recursive: true });
  writeFileSync(join(shotsDir(worktreeRoot), name), bytes);
  return join(".notes", "shots", name);
}

function appendNote(file: string, record: NoteRecord): void {
  mkdirSync(dirname(file), { recursive: true });
  appendFileSync(file, `${JSON.stringify(record)}\n`);
}

export function notesPlugin(): Plugin {
  return {
    name: "armada-notes",
    configureServer(server) {
      const root = findWorktreeRoot(process.cwd());
      const file = notesFile(root);
      assertNotesIgnored(root, file);

      server.middlewares.use((req, res, next) => {
        if (!req.url?.startsWith("/__notes")) return next();
        const url = new URL(req.url, "http://localhost");

        // A note's screenshot, for the panel to draw under the note. A bare
        // file name only, so nothing outside `.notes/shots/` is reachable.
        if (req.method === "GET" && url.pathname.startsWith("/__notes/shots/")) {
          const name = basename(url.pathname);
          const path = join(shotsDir(root), name);
          const type = SHOT_TYPES[name.split(".").pop() ?? ""];
          if (!/^[\w.-]+$/.test(name) || type === undefined || !existsSync(path)) {
            res.statusCode = 404;
            res.end();
            return;
          }
          res.setHeader("Content-Type", type);
          res.end(readFileSync(path));
          return;
        }

        if (req.method === "GET") {
          const story = url.searchParams.get("story") ?? "";
          res.setHeader("Content-Type", "application/json");
          res.end(JSON.stringify(readNotesFor(file, story)));
          return;
        }

        if (req.method === "POST") {
          let body = "";
          req.on("data", (chunk) => (body += chunk));
          req.on("end", () => {
            try {
              const { shot, ...payload } = JSON.parse(body) as Partial<NotePayload> & {
                shot?: unknown;
              };
              if (
                typeof payload.story !== "string" ||
                typeof payload.titled !== "string" ||
                typeof payload.note !== "string"
              ) {
                res.statusCode = 400;
                res.end("expected { story, titled, note }");
                return;
              }
              const record: NoteRecord = { ...(payload as NotePayload), at: new Date().toISOString() };
              if (typeof shot === "string" && shot !== "") record.shot = saveShot(root, shot);
              appendNote(file, record);
              res.statusCode = 204;
              res.end();
            } catch (err) {
              res.statusCode = 400;
              res.end(String(err));
            }
          });
          return;
        }

        next();
      });
    },
  };
}
