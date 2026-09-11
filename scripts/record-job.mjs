#!/usr/bin/env node
// Record one Job off a running Fleet, as the wire carried it, for Storybook.
//
// **Raw, and folded later.** What is written is what Fleet answered — each
// read's status and body, and every message on the Job's two sockets — and
// nothing here decides what any of it means. `packages/screens` replays a
// recording through `@armada/protocol`'s fold, the one Bridge's main process
// runs, so a story drawn from it cannot disagree with the app. A recorder that
// folded as it went would be a second fold, and the one that drifts.
//
// **Scrubbed before it is written, and refused if the scrub missed.** A Job's
// transcript names the machine it ran on — home paths, the account name — and
// this repository is public. Whatever the scrub cannot account for stops the
// write rather than being committed.
//
//   node scripts/record-job.mjs <job id or handle> <slug> "<the state, as a sentence>"
//                               [--dev-fleet <scratch-dir>]
//
// `--dev-fleet` reads the Fleet `scripts/dev-fleet` started in that directory.
// Without it, the Fleet running as you.
// `docs/practices/running-locally.md` says when to run it and what it prints.

import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, realpathSync, writeFileSync } from "node:fs";
import { homedir, hostname, userInfo } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "..");
const RECORDED = join(REPO, "packages/screens/src/fixtures/recorded");
/** Where a recording of the whole Board goes. `Screens/Board` replays it. */
const BOARDS = join(REPO, "packages/screens/src/fixtures/boards");
/** How long a socket may stay quiet before a stream still running is cut. */
const QUIET_MS = 2000;
/** How long one read may take. A Fleet this slow is worth hearing about. */
const READ_MS = 10_000;

const args = process.argv.slice(2);
const devFleet = option("--dev-fleet");
const boardSlug = option("--board");
const home = devFleet === undefined ? homedir() : join(devFleet, "home/user");
const [wanted, slug, says] = args;
const asked =
  boardSlug === undefined
    ? Boolean(wanted && slug && says && /^[a-z0-9-]+$/.test(slug))
    : /^[a-z0-9-]+$/.test(boardSlug);
if (!asked) {
  console.error(
    'usage: node scripts/record-job.mjs <job id or handle> <slug> "<the state, as a sentence>" [--dev-fleet <scratch-dir>]\n' +
      "       node scripts/record-job.mjs --board <slug> [--dev-fleet <scratch-dir>]\n" +
      "  <slug> is the recording's directory: lower case, digits and hyphens",
  );
  process.exit(2);
}

const runtime = join(home, "Library/Application Support/Armada/fleet.json");
if (!existsSync(runtime)) fail(`no Fleet is running as ${home} — it has no runtime file`);
const { port, pid } = JSON.parse(readFileSync(runtime, "utf8"));
try {
  process.kill(pid, 0);
} catch {
  fail(`the runtime file names pid ${pid}, and nothing is running as it`);
}
const BASE = `http://127.0.0.1:${port}`;

// **`--board` records the Board rather than one Job**: every row `list_jobs`
// serves, and the workflows and manifests those rows name, scrubbed and checked
// the way a Job's recording is. `Screens/Board` replays it beside the rows it
// builds, so a real Board's mix of states is in Storybook too.
if (boardSlug !== undefined) {
  const [listedJobs, heldWorkflows, heldManifests] = await Promise.all([
    get("/jobs"),
    get("/workflows"),
    get("/manifests"),
  ]);
  const board = {
    jobs: listOf(listedJobs.body, "jobs"),
    workflows: listOf(heldWorkflows.body, "workflows"),
    manifests: listOf(heldManifests.body, "manifests"),
  };
  const { text, replaced } = scrub(JSON.stringify(board, null, 1));
  const left = leaks(text);
  if (left.length > 0) fail(`nothing written — the scrub left ${left.join(", ")}`);
  mkdirSync(BOARDS, { recursive: true });
  writeFileSync(join(BOARDS, `${boardSlug}.json`), `${text}\n`);
  console.log(`recorded the Board as ${boardSlug}`);
  console.log(`  jobs       ${board.jobs.length}`);
  console.log(`  workflows  ${board.workflows.length}`);
  console.log(`  scrubbed   ${replaced} occurrences; ${Math.round(text.length / 1024)} KiB written`);
  await new Promise((flushed) => process.stdout.write("", flushed));
  process.exit(0);
}

const listed = await get("/jobs");
const jobs = Array.isArray(listed.body) ? listed.body : (listed.body?.jobs ?? []);
const summary = jobs.find((job) => job.id === wanted || job.handle === wanted);
if (summary === undefined) fail(`no Job ${wanted} on this Fleet — it holds ${jobs.length}`);
const id = encodeURIComponent(summary.id);

// Every read job detail makes, under the routes Bridge's main process uses.
const reads = {};
for (const [name, route] of [
  ["detail", ""],
  ["resources", "/resources"],
  ["events", "/events"],
  ["evidence", "/evidence"],
  ["diff", "/diff"],
  ["remarks", "/remarks"],
]) {
  reads[name] = await get(`/jobs/${id}${route}`);
}
const workflows = await get("/workflows");
const manifests = await get("/manifests");
const [observe, log] = await Promise.all([
  listen(`/jobs/${id}/observe`),
  listen(`/jobs/${id}/log`),
]);

// The reads a person opens on demand, and only the ones this Job's panel can
// offer: a cut call's arguments, each Check's output, each frame a step kept.
const detail = reads.detail.status === 200 ? reads.detail.body : { steps: [] };
const calls = {};
for (const message of observe.messages) {
  if (message.message === "row" && message.event === "called" && message.truncated) {
    calls[message.call] ??= await get(`/jobs/${id}/calls/${encodeURIComponent(message.call)}`);
  }
}
const checkOutputs = {};
const frames = {};
for (const step of detail.steps ?? []) {
  for (const run of step.check_runs ?? []) {
    if (!run.output_path) continue;
    // The basename, because that is what the Checks chapter asks for.
    const kept = basename(run.output_path);
    checkOutputs[kept] ??= await get(`/jobs/${id}/checks/${encodeURIComponent(kept)}/output`);
  }
  for (const frame of step.frames ?? []) {
    const path = frame.kept.split("/").map(encodeURIComponent).join("/");
    frames[frame.kept] ??= await getBytes(`/jobs/${id}/frames/${path}`);
  }
}

const recording = {
  says,
  recorded_at: new Date().toISOString(),
  now: Date.now(),
  summary,
  reads,
  workflows,
  manifests,
  observe,
  log,
  calls,
  checkOutputs,
  frames,
};
const { text, replaced } = scrub(JSON.stringify(recording, null, 1));
const left = leaks(text);
if (left.length > 0) fail(`nothing written — the scrub left ${left.join(", ")}`);

mkdirSync(join(RECORDED, slug), { recursive: true });
writeFileSync(join(RECORDED, slug, "recording.json"), `${text}\n`);
writeIndex();

console.log(`recorded ${summary.handle ?? summary.id} (${summary.status}) as ${slug} — "${says}"`);
for (const [name, read] of Object.entries(reads)) console.log(`  ${name.padEnd(10)} ${read.status}`);
console.log(`  workflows  ${workflows.status}\n  manifests  ${manifests.status}`);
console.log(`  observe    ${observe.messages.length} messages, ${observe.open ? "still open" : "closed"}`);
console.log(`  log        ${log.messages.length} messages, ${log.open ? "still open" : "closed"}`);
console.log(
  `  on demand  ${Object.keys(calls).length} calls, ${Object.keys(checkOutputs).length} Check outputs, ` +
    `${Object.keys(frames).length} frames`,
);
console.log(`  scrubbed   ${replaced} occurrences; ${Math.round(text.length / 1024)} KiB written`);

// Exit rather than wait for the event loop to drain. A socket cut while still
// open waits on Fleet to answer the close, and a Fleet that never does would
// hold the recording's own terminal after the file is already written.
await new Promise((flushed) => process.stdout.write("", flushed));
process.exit(0);

/** A list read, whether Fleet answered it bare or under its own name. */
function listOf(body, key) {
  return Array.isArray(body) ? body : (body?.[key] ?? []);
}

/** One option and its value, taken out of `args` so the positionals are left. */
function option(name) {
  const at = args.indexOf(name);
  if (at === -1) return undefined;
  const [, value] = args.splice(at, 2);
  return value;
}

async function get(path) {
  const answer = await fetch(BASE + path, { signal: AbortSignal.timeout(READ_MS) });
  const text = await answer.text();
  let body = text;
  try {
    body = JSON.parse(text);
  } catch {
    // Not JSON is recorded as the text it was — a refusal the wire could not
    // parse is a thing the story should draw, not a thing to drop.
  }
  return { status: answer.status, body };
}

/** A frame is a file, so it is kept as bytes and its media type. */
async function getBytes(path) {
  const answer = await fetch(BASE + path, { signal: AbortSignal.timeout(READ_MS) });
  if (!answer.ok) return { status: answer.status, body: await answer.text() };
  return {
    status: answer.status,
    type: answer.headers.get("content-type") ?? "application/octet-stream",
    base64: Buffer.from(await answer.arrayBuffer()).toString("base64"),
  };
}

/**
 * Every message on one of the Job's sockets, until Fleet closes it or it goes
 * quiet. `open` is which: a finished Job's stream replays and closes, and a
 * running one's has no end, so it is cut and the story draws it still watching.
 */
function listen(path) {
  return new Promise((done) => {
    const messages = [];
    let quiet;
    let finished = false;
    const socket = new WebSocket(`ws://127.0.0.1:${port}${path}`);
    const finish = (open) => {
      if (finished) return;
      finished = true;
      clearTimeout(quiet);
      if (open) socket.close();
      done({ open, messages });
    };
    const wait = () => {
      clearTimeout(quiet);
      quiet = setTimeout(() => finish(true), QUIET_MS);
    };
    socket.addEventListener("open", wait);
    socket.addEventListener("message", (event) => {
      messages.push(JSON.parse(String(event.data)));
      wait();
    });
    socket.addEventListener("close", () => finish(false));
    socket.addEventListener("error", () => finish(false));
  });
}

/**
 * The machine taken out of the text, most specific first.
 *
 * A dev Fleet serves a clone under its scratch directory, so that clone's path
 * becomes the checkout it was cloned from, and the dev Fleet's home becomes
 * yours — then yours becomes `~`, which is what the app would show you anyway.
 */
function scrub(text) {
  const swaps = [];
  if (devFleet !== undefined) {
    for (const form of forms(join(devFleet, "repo"))) swaps.push([form, mainCheckout()]);
    for (const form of forms(home)) swaps.push([form, homedir()]);
    // Anything else of the dev Fleet's own — its do-nothing agent, say.
    for (const form of forms(devFleet)) swaps.push([form, "<dev-fleet>"]);
  }
  swaps.push([/[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/g, "someone@example.com"]);
  // An agent's own session directory, which a Drone that is itself an agent
  // names in its tool output — its temp root carries the account's uid.
  swaps.push([/\/(private\/)?tmp\/claude-\d+\//g, "<session-tmp>/"]);
  for (const form of forms(homedir())) {
    swaps.push([form, "~"]);
    // The form a project directory takes when its separators become hyphens.
    swaps.push([form.replaceAll("/", "-"), "~"]);
  }
  // Whatever home directory is left — another account's, or yours cut short
  // where a tool's output was truncated mid-path, which the whole path above
  // cannot match. `user` is the name the privacy gate lets a committed path use.
  swaps.push([/\/(Users|home)\/(?!user\b)[^/\s"\\]+/g, "/$1/user"]);
  swaps.push([/-(Users|home)-(?!user\b)[A-Za-z0-9._]+/g, "-$1-user"]);
  swaps.push([new RegExp(`\\b${escaped(userInfo().username)}\\b`, "g"), "owner"]);
  swaps.push([new RegExp(escaped(hostname().split(".")[0]), "g"), "host"]);

  let replaced = 0;
  for (const [from, to] of swaps) {
    const pattern = typeof from === "string" ? new RegExp(escaped(from), "g") : from;
    text = text.replace(pattern, (...found) => {
      replaced += 1;
      // `$1` in a replacement is the pattern's first group, as `String.replace`
      // would expand it — the callback is only here to count.
      return to.replace(/\$(\d)/g, (_, n) => found[Number(n)] ?? "");
    });
  }
  return { text, replaced };
}

/**
 * What would name this machine if it were still in the text, each with the
 * first place it was found — a refusal that does not say where is one nobody
 * can fix.
 */
function leaks(text) {
  const checks = [
    ["your home directory", new RegExp(escaped(homedir()))],
    // The shapes the privacy gate refuses, so a recording it would fail is
    // one this never writes.
    ["a home directory", /\/(Users|home)\/(?!user\b)[^/\s"\\]+/],
    ["a home directory, mangled", /-(Users|home)-(?!user\b)[A-Za-z0-9._]+/],
    ["your account name", new RegExp(`\\b${escaped(userInfo().username)}\\b`)],
    ["this machine's name", new RegExp(escaped(hostname().split(".")[0]))],
    ["a session scratch directory", /\/(private\/)?tmp\/claude-\d+/],
  ];
  const found = [];
  for (const [name, pattern] of checks) {
    const at = text.search(pattern);
    if (at !== -1) found.push(`${name}, first at …${text.slice(Math.max(0, at - 40), at + 60)}…`);
  }
  return found;
}

/** A path as written and as the filesystem resolves it — `/tmp` is a link. */
function forms(path) {
  const real = existsSync(path) ? realpathSync(path) : path;
  return [...new Set([real, path, real.replace(/^\/private\//, "/")])].sort((a, b) => b.length - a.length);
}

function mainCheckout() {
  const common = execFileSync("git", ["rev-parse", "--path-format=absolute", "--git-common-dir"], {
    cwd: REPO,
    encoding: "utf8",
  });
  return dirname(common.trim());
}

function escaped(text) {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** The list the stories read. Rewritten whole, so a recording is never left out. */
function writeIndex() {
  const slugs = readdirSync(RECORDED, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() && existsSync(join(RECORDED, entry.name, "recording.json")))
    .map((entry) => entry.name)
    .sort();
  const lines = [
    "// Written by `scripts/record-job.mjs`, which rewrites it whole on every",
    "// recording. Editing it by hand is lost on the next one.",
    "",
    ...slugs.map((name, at) => `import r${at} from "./${name}/recording.json";`),
    "",
    "/** Every recording, by its directory. Replayed by `../recorded.ts`. */",
    "export const RECORDINGS: Record<string, unknown> = {",
    ...slugs.map((name, at) => `  "${name}": r${at},`),
    "};",
    "",
  ];
  writeFileSync(join(RECORDED, "index.ts"), lines.join("\n"));
}

function fail(why) {
  console.error(`record-job: ${why}`);
  process.exit(1);
}
