// A note sent to Fleet, #1250: proposed as a Job through the describe-the-work
// path the composer uses, so it lands at the approval gate like any request and
// nothing runs until a person approves it.

import { said } from "@armada/screens/src/copy";
import type { StagedAttachment } from "@armada/protocol";

import type { BridgeApi } from "../../../shared/api";
import { requestOf, type Annotation, type Box, type Sent } from "../../../shared/annotations";
import type { Sink } from "./sink";

/** The two calls a send makes on Fleet's seam. */
export type Proposer = Pick<BridgeApi, "stageAttachment" | "proposeFromRequest">;

export type SendAnswer = { ok: true; sent: Sent } | { ok: false; saying: string };

/** Why Send is not offered here, or null where it is. */
export function unsendable(sink: Sink): string | null {
  return sink.via === "main" ? null : "Sending needs Bridge and its Fleet; in the mock the note stays a file";
}

/**
 * Stage a screenshot of where the note points, where one can be taken, and
 * propose the note's request against the repository the notes are about.
 * **Nothing is saved here**: the caller writes `sent` onto the note, so a send
 * that fails leaves the note exactly as it was.
 */
export async function sendToFleet(note: Annotation, box: Box, sink: Sink, fleet: Proposer, at: Date): Promise<SendAnswer> {
  const root = await sink.root();
  if (root === null) return { ok: false, saying: "No repository was found above Bridge, so there is nothing to send the note against" };

  const attachments: StagedAttachment[] = [];
  const png = await sink.capture(box).catch(() => null);
  if (png !== null) {
    const filename = `annotation-${note.id}.png`;
    const { path } = await fleet.stageAttachment(png, filename, "image/png");
    attachments.push({ path, filename, mimeType: "image/png" });
  }

  const answer = await fleet.proposeFromRequest(requestOf(note), attachments, root);
  if (!answer.ok) return { ok: false, saying: said(answer.outcome) };
  const job = answer.jobs[0];
  if (job === undefined) return { ok: false, saying: "Fleet answered with no Job" };
  return { ok: true, sent: { jobId: job.id, handle: job.handle, at: at.toISOString() } };
}
