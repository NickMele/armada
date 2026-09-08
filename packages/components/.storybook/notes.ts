// A reviewer's running commentary on a story, read by an agent session rather
// than by another person — so the record is a JSON line per note, not prose.
import { existsSync, mkdirSync, appendFileSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
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
  // Unset until a screenshot lands beside a note. Present in the shape now so
  // that day's route and record do not change, only this field's value does.
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
              const payload = JSON.parse(body) as Partial<NotePayload>;
              if (
                typeof payload.story !== "string" ||
                typeof payload.titled !== "string" ||
                typeof payload.note !== "string"
              ) {
                res.statusCode = 400;
                res.end("expected { story, titled, note }");
                return;
              }
              appendNote(file, { ...(payload as NotePayload), at: new Date().toISOString() });
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
