/* The one seam between `shoot` and a browser.
 *
 * `shoot` knows about marks, pairs and drawings. It knows nothing about how a
 * page becomes a PNG. Everything that does is on the other side of this file,
 * in `browser-electron.mjs`, so replacing the capture stack is replacing one
 * file rather than rewriting the tool.
 *
 * That separation is not tidiness. #209 puts visual evidence behind a harness
 * the repository declares, and says Armada must hold no opinion about
 * Playwright versus Cypress versus an Electron driver, the same way `checks`
 * holds none about cargo versus pnpm. Electron is the right choice *here* —
 * Bridge already ships it, so nothing new is installed to render pages the
 * workspace can already render — and it is a choice this repository made, not
 * one the tool is built out of.
 *
 * ## The contract
 *
 *   browse({ page, capture, into, width, height }) -> Promise<Read>
 *
 *   page      absolute path to an HTML file on disk. It may reach only files
 *             beside it; nothing here is allowed to fetch.
 *   capture   write a PNG per mark. False reads the page and writes nothing.
 *   into      directory the PNGs go in, one per mark, named `<mark>.png`.
 *   width     viewport width in CSS pixels. Height is a hint only — a shot may
 *   height    be taller than the window.
 *
 *   Read = {
 *     marks:  [{ mark, x, y, width, height }]   every [data-shot], in CSS px
 *     frames: [{ id, marked, heading, candidate }]  a drawing's units, for
 *                                                   refusing and for --suggest
 *     page:   { width, height }
 *     written:[{ mark, file, width, height }]   empty when capture is false
 *     failures: [string]                        what the page complained about
 *   }
 *
 * An implementation owes: two device pixels per CSS pixel, fonts loaded and
 * images decoded before any capture, and a shot clipped to the mark's box —
 * or, where the marked element has a direct `.stage` child, to that child, so
 * the gallery's caption stays out of the frame. */
import { spawn } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../..");

/* Bridge's Electron, resolved from Bridge's own package rather than installed
   again at the root. One Chromium in the workspace, and it is already there.

   The `electron` package's main export is the path to its binary — it is what
   `electron-vite` and every other runner uses, and it throws a readable error
   when the binary was never downloaded, which is worth more than assembling
   the path by hand and getting an ENOENT out of `spawn`. */
function electronBinary() {
  return createRequire(join(root, "apps/desktop/package.json"))("electron");
}

export async function browse({ page, capture = false, into = null, width = 1440, height = 1200 }) {
  const scratch = join(root, ".shots/.run");
  mkdirSync(scratch, { recursive: true });
  const request = join(scratch, "request.json");
  const reply = join(scratch, "reply.json");
  writeFileSync(request, JSON.stringify({ page, capture, into, reply, width, height }), "utf8");

  const code = await new Promise((done, fail) => {
    const child = spawn(electronBinary(), [join(here, "browser-electron.mjs"), request], {
      stdio: ["ignore", "inherit", "pipe"],
      /* Electron's own security warnings are addressed to an app author and
         this is a screenshot tool loading one file from disk. */
      env: { ...process.env, ELECTRON_DISABLE_SECURITY_WARNINGS: "1" },
    });
    /* Chromium logs its own internals to stderr — a GPU shared-image warning
       per capture on a machine with no display attached, dozens of lines for
       one run. None of it is about the page and none of it is a failure, and
       burying the one line that is under thirty that are not is how a real
       problem gets scrolled past. Anything not stamped with Chromium's
       `[pid:timestamp:LEVEL:file]` prefix is passed through untouched. */
    let held = "";
    child.stderr.setEncoding("utf8");
    child.stderr.on("data", (chunk) => {
      held += chunk;
      const lines = held.split("\n");
      held = lines.pop() ?? "";
      for (const line of lines)
        if (!/^\[\d+:\d{4}\/\d{6}\.\d+:[A-Z]+:/.test(line)) process.stderr.write(`${line}\n`);
    });
    child.on("error", (e) => fail(new Error(`the browser would not start: ${e.message}`)));
    child.on("exit", done);
  });
  if (code !== 0) throw new Error(`the browser exited ${code} without capturing anything`);
  return JSON.parse(readFileSync(reply, "utf8"));
}
