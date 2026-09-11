// A Job recorded off a real Fleet, replayed into what `JobDetail` is given.
//
// **Replayed through the app's own fold.** `scripts/record-job.mjs` writes what
// Fleet answered and nothing else. Every read here becomes the `JobRead` Bridge's
// main process would have published — `read` on a 2xx, `failed` with the outcome
// `refusedWith` makes otherwise — and each socket's messages go through
// `observedFrom` and `journalledFrom`, which fold with the functions main folds
// with. So a story drawn from a recording is the app's reading of that Job, not
// somebody's drawing of it.
//
// **The casts are the wire's, made once each.** A recording is JSON, and JSON
// has no types. Main makes the same assertion at the same seam — `body as
// JobDetail` — because the body is Fleet's, and Fleet is the authority on it.

import { journalledFrom, observedFrom, refusedWith } from "@armada/protocol";
import type {
  CallArguments,
  CheckOutput,
  JobDetail as WireDetail,
  JobDiff,
  JobEvidence,
  JobHistory,
  JobRead,
  JobRemarks,
  JobResources,
  JobSummary,
  JournalMessage,
  ManifestSummary,
  Outcome,
  TurnMessage,
  WorkflowSummary,
} from "@armada/protocol";

import type { JobFixture } from "./fixture";
import { RECORDINGS } from "./recorded/index";

/** One answer, as the recorder kept it — the status, and the body parsed where it parsed. */
type Answered = { status: number; body: unknown };

/** A frame's answer: its bytes as base64 on a 2xx, and the refusal otherwise. */
type Photographed = Answered & { type?: string; base64?: string };

/** What one socket carried, and whether it was still open when the recording stopped. */
type Streamed<Message> = { open: boolean; messages: Message[] };

/** A recording as `scripts/record-job.mjs` writes it. */
export type Recording = {
  says: string;
  recorded_at: string;
  now: number;
  summary: JobSummary;
  reads: Record<"detail" | "resources" | "events" | "evidence" | "diff" | "remarks", Answered>;
  workflows: Answered;
  manifests: Answered;
  observe: Streamed<TurnMessage>;
  log: Streamed<JournalMessage>;
  calls: Record<string, Answered>;
  checkOutputs: Record<string, Answered>;
  frames: Record<string, Photographed>;
};

/** One recording, replayed, by the directory it was recorded into. */
export function recorded(slug: string): JobFixture {
  const recording = RECORDINGS[slug];
  if (recording === undefined) throw new Error(`no recording named ${slug}`);
  return replayed(recording as Recording);
}

/** Every recording's directory, in the order the recorder listed them. */
export const RECORDED_SLUGS: readonly string[] = Object.keys(RECORDINGS);

export function replayed(recording: Recording): JobFixture {
  const jobId = recording.summary.id;
  const at = (route: string) => `/jobs/${encodeURIComponent(jobId)}${route}`;
  const { reads } = recording;
  return {
    name: recording.says,
    job: recording.summary,
    watched: read(jobId, at(""), reads.detail, (body) => ({ detail: body as WireDetail })),
    // Board reads, and main keeps the last good answer when one fails. A
    // recording has no last answer, so a failed one is an empty roster.
    workflows: ok(recording.workflows) ? (recording.workflows.body as WorkflowSummary[]) : [],
    manifests: ok(recording.manifests) ? (recording.manifests.body as ManifestSummary[]) : [],
    observed: observedFrom(jobId, recording.observe.messages, recording.observe.open),
    journalled: journalledFrom(jobId, recording.log.messages, recording.log.open),
    resources: read(jobId, at("/resources"), reads.resources, (body) => ({
      resources: body as JobResources,
    })),
    history: read(jobId, at("/events"), reads.events, (body) => ({
      moves: (body as JobHistory).moves,
    })),
    recorded: {
      // Pushed on the board's socket as `job.files_changed` and never read, and
      // only while the Job is open. A recording opens the Job after the fact,
      // which is the reading Bridge has on a Job opened late.
      footprint: { state: "none" },
      evidence: read(jobId, at("/evidence"), reads.evidence, (body) => ({
        steps: (body as JobEvidence).steps,
      })),
      diff: read(jobId, at("/diff"), reads.diff, (body) => ({ work: (body as JobDiff).work })),
      remarks: read(jobId, at("/remarks"), reads.remarks, (body) => ({ review: body as JobRemarks })),
    },
    calls: each(recording.calls, (answer, id) =>
      settled(answer, at(`/calls/${id}`), (body) => ({ call: body as CallArguments })),
    ),
    checkOutputs: each(recording.checkOutputs, (answer, kept) =>
      settled(answer, at(`/checks/${kept}/output`), (body) => ({ output: body as CheckOutput })),
    ),
    frames: each(recording.frames, (answer, kept) =>
      ok(answer) && answer.base64 !== undefined
        ? { ok: true, bytes: bytesOf(answer.base64), type: answer.type ?? "application/octet-stream" }
        : { ok: false, outcome: outcomeOf(answer, at(`/frames/${kept}`)) },
    ),
    now: recording.now,
  };
}

/** A per-Job read, in the state main's `JobReader` would have published it in. */
function read<Read extends object>(
  jobId: string,
  path: string,
  answer: Answered,
  keeps: (body: unknown) => Read,
): JobRead<Read> {
  return ok(answer)
    ? { state: "read", jobId, ...keeps(answer.body) }
    : { state: "failed", jobId, outcome: outcomeOf(answer, path) };
}

/** An on-demand read, in the shape `request.ts` answers one. */
function settled<Read extends object>(
  answer: Answered,
  path: string,
  keeps: (body: unknown) => Read,
): ({ ok: true } & Read) | { ok: false; outcome: Outcome } {
  return ok(answer) ? { ok: true, ...keeps(answer.body) } : { ok: false, outcome: outcomeOf(answer, path) };
}

function ok(answer: Answered): boolean {
  return answer.status >= 200 && answer.status < 300;
}

/** The outcome main would have drawn, from the body as Fleet sent it. */
function outcomeOf(answer: Answered, path: string): Outcome {
  const text = typeof answer.body === "string" ? answer.body : JSON.stringify(answer.body);
  return refusedWith(answer.status, text, { method: "GET", path });
}

function each<From, To>(
  record: Record<string, From>,
  map: (value: From, key: string) => To,
): Record<string, To> {
  return Object.fromEntries(Object.entries(record).map(([key, value]) => [key, map(value, key)]));
}

/** `atob` rather than `Buffer`: this runs in Storybook's browser as well as node. */
function bytesOf(base64: string): Uint8Array {
  return Uint8Array.from(atob(base64), (char) => char.charCodeAt(0));
}
